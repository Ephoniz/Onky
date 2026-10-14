/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    minwavertstream.cpp

Abstract:

    Implementation of wavert miniport stream.
--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Main/minwavertstream.cpp). Modifications Copyright (c) 2026 Audio
// Program contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#include "definitions.h"
#include <limits.h>
#include <ks.h>
#include "endpoints.h"
#include "loopback.h"
#include "minwavert.h"
#include "minwavertstream.h"
#define MINWAVERTSTREAM_POOLTAG 'SRdA'

#pragma warning (disable : 4127)

//=============================================================================
// CMiniportWaveRTStream
//=============================================================================

//=============================================================================
#pragma code_seg("PAGE")
CMiniportWaveRTStream::~CMiniportWaveRTStream
(
    void
)
/*++

Routine Description:

  Destructor for wavertstream

--*/
{
    PAGED_CODE();

    if (m_pNotificationTimer)
    {
        ExDeleteTimer
        (
            m_pNotificationTimer,
            TRUE, // Cancel the timer if it is currently set.
            TRUE, // Wait for the timer to finish expiring and for any callback to a ExTimerCallback routine to finish.
            NULL
         );
        m_pNotificationTimer = NULL;
    }

    // Since we just cancelled the notification timer, wait for all queued
    // DPCs to complete before we tear the stream down.
    //
    KeFlushQueuedDpcs();

    while (!IsListEmpty(&m_NotificationList))
    {
        PLIST_ENTRY leCurrent = RemoveHeadList(&m_NotificationList);
        NotificationListEntry* nleCurrent = CONTAINING_RECORD(leCurrent, NotificationListEntry, ListEntry);
        ExFreePoolWithTag(nleCurrent, MINWAVERTSTREAM_POOLTAG);
    }

    if (NULL != m_pMiniport)
    {
        if (m_bUnregisterStream)
        {
            m_pMiniport->StreamClosed(m_ulPin, this);
            m_bUnregisterStream = FALSE;
        }

        m_pCable = NULL;
        m_pMiniport->Release();
        m_pMiniport = NULL;
    }

    if (m_pWfExt)
    {
        ExFreePoolWithTag( m_pWfExt, MINWAVERTSTREAM_POOLTAG );
        m_pWfExt = NULL;
    }

    DPF_ENTER(("[CMiniportWaveRTStream::~CMiniportWaveRTStream]"));
} // ~CMiniportWaveRTStream

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CMiniportWaveRTStream::Init
(
    _In_ PCMiniportWaveRT           Miniport_,
    _In_ PPORTWAVERTSTREAM          PortStream_,
    _In_ ULONG                      Pin_,
    _In_ BOOLEAN                    Capture_,
    _In_ PKSDATAFORMAT              DataFormat_,
    _In_ GUID                       SignalProcessingMode
)
/*++

Routine Description:

  Initializes the stream object.

--*/
{
    PAGED_CODE();

    PWAVEFORMATEX pWfEx = NULL;
    NTSTATUS ntStatus = STATUS_SUCCESS;

    m_pMiniport = NULL;
    m_pCable = NULL;
    m_ulPin = 0;
    m_bUnregisterStream = FALSE;
    m_bCapture = FALSE;
    m_ulDmaBufferSize = 0;
    m_pDmaBuffer = NULL;
    m_ulNotificationsPerBuffer = 0;
    m_KsState = KSSTATE_STOP;
    m_llPacketCounter = 0;
    m_ullPlayPosition = 0;
    m_ullWritePosition = 0;
    m_ullDmaTimeStamp = 0;
    m_ullPerformanceCounterFrequency.QuadPart = 0;
    m_hnsElapsedTimeCarryForward = 0;
    m_ullLastDPCTimeStamp = 0;
    m_hnsDPCTimeCarryForward = 0;
    m_ulDmaMovementRate = 0;
    m_byteDisplacementCarryForward = 0;
    m_pWfExt = NULL;
    m_ullLinearPosition = 0;
    m_ullPresentationPosition = 0;
    m_ulCurrentWritePosition = 0;
    m_ulLastOsReadPacket = ULONG_MAX;
    m_ulLastOsWritePacket = ULONG_MAX;
    m_IsCurrentWritePositionUpdated = 0;
    m_SignalProcessingMode = SignalProcessingMode;

    m_pPortStream = PortStream_;
    InitializeListHead(&m_NotificationList);
    m_ulNotificationIntervalMs = 0;

    // Initialize the spinlock to synchronize position updates
    KeInitializeSpinLock(&m_PositionSpinLock);

    m_pNotificationTimer = ExAllocateTimer(
         TimerNotifyRT,
         this,
         EX_TIMER_HIGH_RESOLUTION
    );
    if (!m_pNotificationTimer)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    pWfEx = GetWaveFormatEx(DataFormat_);
    if (NULL == pWfEx)
    {
        return STATUS_UNSUCCESSFUL;
    }

    if (pWfEx->nAvgBytesPerSec == 0 || pWfEx->nBlockAlign == 0)
    {
        return STATUS_INVALID_PARAMETER;
    }

    m_pMiniport = reinterpret_cast<CMiniportWaveRT*>(Miniport_);
    if (m_pMiniport == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }
    m_pMiniport->AddRef();

    m_pCable = m_pMiniport->GetCable();
    if (m_pCable == NULL)
    {
        return STATUS_INVALID_DEVICE_STATE;
    }

    m_ulPin = Pin_;
    m_bCapture = Capture_;
    m_ulDmaMovementRate = pWfEx->nAvgBytesPerSec;

    m_pWfExt = (PWAVEFORMATEXTENSIBLE)ExAllocatePool2(POOL_FLAG_NON_PAGED, sizeof(WAVEFORMATEX) + pWfEx->cbSize, MINWAVERTSTREAM_POOLTAG);
    if (m_pWfExt == NULL)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }
    RtlCopyMemory(m_pWfExt, pWfEx, sizeof(WAVEFORMATEX) + pWfEx->cbSize);

    //
    // Register this stream.
    //
    ntStatus = m_pMiniport->StreamCreated(m_ulPin, this);
    if (NT_SUCCESS(ntStatus))
    {
        m_bUnregisterStream = TRUE;
    }

    return ntStatus;
} // Init

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(NTSTATUS)
CMiniportWaveRTStream::NonDelegatingQueryInterface
(
    _In_ REFIID  Interface,
    _COM_Outptr_ PVOID * Object
)
{
    PAGED_CODE();

    ASSERT(Object);

    if (IsEqualGUIDAligned(Interface, IID_IUnknown))
    {
        *Object = PVOID(PUNKNOWN(PMINIPORTWAVERTSTREAM(this)));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTStream))
    {
        *Object = PVOID(PMINIPORTWAVERTSTREAM(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTStreamNotification))
    {
        *Object = PVOID(PMINIPORTWAVERTSTREAMNOTIFICATION(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTInputStream) && (this->m_bCapture))
    {
        // This interface is supported only on capture streams
        *Object = PVOID(PMINIPORTWAVERTINPUTSTREAM(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTOutputStream) && !(this->m_bCapture))
    {
        // This interface is supported only on render streams
        *Object = PVOID(PMINIPORTWAVERTOUTPUTSTREAM(this));
    }
    else
    {
        *Object = NULL;
    }

    if (*Object)
    {
        PUNKNOWN(*Object)->AddRef();
        return STATUS_SUCCESS;
    }

    return STATUS_INVALID_PARAMETER;
} // NonDelegatingQueryInterface

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::AllocateBufferWithNotification
(
    _In_    ULONG               NotificationCount_,
    _In_    ULONG               RequestedSize_,
    _Out_   PMDL                *AudioBufferMdl_,
    _Out_   ULONG               *ActualSize_,
    _Out_   ULONG               *OffsetFromFirstPage_,
    _Out_   MEMORY_CACHING_TYPE *CacheType_
)
{
    PAGED_CODE();

    ULONG ulBufferDurationMs = 0;

    if ( (0 == RequestedSize_) || (RequestedSize_ < m_pWfExt->Format.nBlockAlign) )
    {
        return STATUS_UNSUCCESSFUL;
    }

    if ((NotificationCount_ == 0) || (RequestedSize_ % NotificationCount_ != 0))
    {
        return STATUS_INVALID_PARAMETER;
    }

    RequestedSize_ -= RequestedSize_ % (m_pWfExt->Format.nBlockAlign);

    PHYSICAL_ADDRESS highAddress;
    highAddress.HighPart = 0;
    highAddress.LowPart = MAXULONG;

    PMDL pBufferMdl = m_pPortStream->AllocatePagesForMdl (highAddress, RequestedSize_);

    if (NULL == pBufferMdl)
    {
        return STATUS_UNSUCCESSFUL;
    }

    // The driver copies between this buffer and the cable ring in software,
    // so the pages are mapped cached (no platform alignment requirements).
    m_pDmaBuffer = (BYTE*)m_pPortStream->MapAllocatedPages(pBufferMdl, MmCached);
    if (m_pDmaBuffer == NULL)
    {
        m_pPortStream->FreePagesFromMdl(pBufferMdl);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    m_ulNotificationsPerBuffer = NotificationCount_;
    m_ulDmaBufferSize = RequestedSize_;
    ulBufferDurationMs = (RequestedSize_ * 1000) / m_ulDmaMovementRate;
    m_ulNotificationIntervalMs = ulBufferDurationMs / NotificationCount_;

    *AudioBufferMdl_ = pBufferMdl;
    *ActualSize_ = RequestedSize_;
    *OffsetFromFirstPage_ = 0;
    *CacheType_ = MmCached;

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg("PAGE")
VOID CMiniportWaveRTStream::FreeBufferWithNotification
(
    _In_        PMDL    Mdl_,
    _In_        ULONG   Size_
)
{
    UNREFERENCED_PARAMETER(Size_);

    PAGED_CODE();

    if (Mdl_ != NULL)
    {
        if (m_pDmaBuffer != NULL)
        {
            m_pPortStream->UnmapAllocatedPages(m_pDmaBuffer, Mdl_);
            m_pDmaBuffer = NULL;
        }

        m_pPortStream->FreePagesFromMdl(Mdl_);
    }

    m_ulDmaBufferSize = 0;
    m_ulNotificationsPerBuffer = 0;

    return;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::RegisterNotificationEvent
(
    _In_ PKEVENT NotificationEvent_
)
{
    PAGED_CODE();

    NotificationListEntry *nleNew = (NotificationListEntry*)ExAllocatePool2(
        POOL_FLAG_NON_PAGED,
        sizeof(NotificationListEntry),
        MINWAVERTSTREAM_POOLTAG);
    if (NULL == nleNew)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    nleNew->NotificationEvent = NotificationEvent_;

    // Fail if the notification event already exists in our list.
    if (!IsListEmpty(&m_NotificationList))
    {
        PLIST_ENTRY leCurrent = m_NotificationList.Flink;
        while (leCurrent != &m_NotificationList)
        {
            NotificationListEntry* nleCurrent = CONTAINING_RECORD( leCurrent, NotificationListEntry, ListEntry);
            if (nleCurrent->NotificationEvent == NotificationEvent_)
            {
                ExFreePoolWithTag( nleNew, MINWAVERTSTREAM_POOLTAG );
                return STATUS_UNSUCCESSFUL;
            }

            leCurrent = leCurrent->Flink;
        }
    }

    InsertTailList(&m_NotificationList, &(nleNew->ListEntry));

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::UnregisterNotificationEvent
(
    _In_ PKEVENT NotificationEvent_
)
{
    PAGED_CODE();

    if (!IsListEmpty(&m_NotificationList))
    {
        PLIST_ENTRY leCurrent = m_NotificationList.Flink;
        while (leCurrent != &m_NotificationList)
        {
            NotificationListEntry* nleCurrent = CONTAINING_RECORD( leCurrent, NotificationListEntry, ListEntry);
            if (nleCurrent->NotificationEvent == NotificationEvent_)
            {
                RemoveEntryList( leCurrent );
                ExFreePoolWithTag( nleCurrent, MINWAVERTSTREAM_POOLTAG );
                return STATUS_SUCCESS;
            }

            leCurrent = leCurrent->Flink;
        }
    }

    return STATUS_NOT_FOUND;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::GetClockRegister
(
    _Out_ PKSRTAUDIO_HWREGISTER Register_
)
{
    UNREFERENCED_PARAMETER(Register_);

    PAGED_CODE();

    return STATUS_NOT_IMPLEMENTED;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::GetPositionRegister
(
    _Out_ PKSRTAUDIO_HWREGISTER Register_
)
{
    UNREFERENCED_PARAMETER(Register_);

    PAGED_CODE();

    return STATUS_NOT_IMPLEMENTED;
}

//=============================================================================
#pragma code_seg("PAGE")
VOID CMiniportWaveRTStream::GetHWLatency
(
    _Out_ PKSRTAUDIO_HWLATENCY  Latency_
)
{
    PAGED_CODE();

    ASSERT(Latency_);

    Latency_->ChipsetDelay = 0;
    Latency_->CodecDelay = 0;
    Latency_->FifoSize = 0;
}

//=============================================================================
#pragma code_seg("PAGE")
VOID CMiniportWaveRTStream::FreeAudioBuffer
(
_In_opt_    PMDL        Mdl_,
_In_        ULONG       Size_
)
{
    UNREFERENCED_PARAMETER(Size_);

    PAGED_CODE();

    if (Mdl_ != NULL)
    {
        if (m_pDmaBuffer != NULL)
        {
            m_pPortStream->UnmapAllocatedPages(m_pDmaBuffer, Mdl_);
            m_pDmaBuffer = NULL;
        }

        m_pPortStream->FreePagesFromMdl(Mdl_);
    }

    m_ulDmaBufferSize = 0;
    m_ulNotificationsPerBuffer = 0;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::AllocateAudioBuffer
(
_In_    ULONG                   RequestedSize_,
_Out_   PMDL                   *AudioBufferMdl_,
_Out_   ULONG                  *ActualSize_,
_Out_   ULONG                  *OffsetFromFirstPage_,
_Out_   MEMORY_CACHING_TYPE    *CacheType_
)
{
    PAGED_CODE();

    if ((0 == RequestedSize_) || (RequestedSize_ < m_pWfExt->Format.nBlockAlign))
    {
        return STATUS_UNSUCCESSFUL;
    }

    RequestedSize_ -= RequestedSize_ % (m_pWfExt->Format.nBlockAlign);

    PHYSICAL_ADDRESS highAddress;
    highAddress.HighPart = 0;
    highAddress.LowPart = MAXULONG;

    PMDL pBufferMdl = m_pPortStream->AllocatePagesForMdl(highAddress, RequestedSize_);

    if (NULL == pBufferMdl)
    {
        return STATUS_UNSUCCESSFUL;
    }

    m_pDmaBuffer = (BYTE*)m_pPortStream->MapAllocatedPages(pBufferMdl, MmCached);
    if (m_pDmaBuffer == NULL)
    {
        m_pPortStream->FreePagesFromMdl(pBufferMdl);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    m_ulDmaBufferSize = RequestedSize_;
    m_ulNotificationsPerBuffer = 0;

    *AudioBufferMdl_ = pBufferMdl;
    *ActualSize_ = RequestedSize_;
    *OffsetFromFirstPage_ = 0;
    *CacheType_ = MmCached;

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::GetPosition
(
    _Out_   KSAUDIO_POSITION    *Position_
)
{
    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);

    if (m_KsState == KSSTATE_RUN)
    {
        //
        // Get the current time and update position.
        //
        LARGE_INTEGER ilQPC = KeQueryPerformanceCounter(NULL);
        UpdatePosition(ilQPC);
    }

    Position_->PlayOffset = m_ullPlayPosition;
    Position_->WriteOffset = m_ullWritePosition;

    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    return STATUS_SUCCESS;
}

//=============================================================================
// CMiniportWaveRTStream::GetReadPacket
//
//  Returns information about the next packet for the OS to read.
//
// Return value
//
//  Returns STATUS_DEVICE_NOT_READY if no new packets are available.
//
// IRQL - PASSIVE_LEVEL
//
// Remarks
//  Although called at passive level, this routine is non-paged code because
//  it is called in the streaming path where page faults should be avoided.
//
#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::GetReadPacket
(
    _Out_ ULONG* PacketNumber,
    _Out_ DWORD* Flags,
    _Out_ ULONG64* PerformanceCounterValue,
    _Out_ BOOL* MoreData
)
{
    ULONG availablePacketNumber;

    // The call must be from event driven mode
    if (m_ulNotificationsPerBuffer == 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    *Flags = 0;

    if (m_KsState < KSSTATE_PAUSE)
    {
        return STATUS_INVALID_DEVICE_STATE;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);

    LONGLONG packetCounter = m_llPacketCounter;
    ULONGLONG ullLinearPosition = m_ullLinearPosition;
    ULONGLONG hnsElapsedTimeCarryForward = m_hnsElapsedTimeCarryForward;
    ULONGLONG ullDmaTimeStamp = m_ullDmaTimeStamp;

    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    // The 0-based number of the last completed packet
    availablePacketNumber = LODWORD(packetCounter - 1);  // Note this might be ULONG_MAX if called during the first packet

    // If no new packets are available...
    if (availablePacketNumber == m_ulLastOsReadPacket)
    {
        return STATUS_DEVICE_NOT_READY;
    }

    // Return next packet number to be read
    *PacketNumber = availablePacketNumber;

    // Compute and return timestamp corresponding to the end of the available
    // packet, extrapolated from the simulated position correlation
    // [m_ullLinearPosition @ m_ullDmaTimeStamp] and the packet counter.
    ULONGLONG linearPositionOfAvailablePacket = packetCounter * (m_ulDmaBufferSize / m_ulNotificationsPerBuffer);
    // Need to divide by (1000 * 10000 because m_ulDmaMovementRate is average bytes per sec
    ULONGLONG carryForwardBytes = (hnsElapsedTimeCarryForward * m_ulDmaMovementRate) / 10000000;
    ULONGLONG deltaLinearPosition = ullLinearPosition + carryForwardBytes - linearPositionOfAvailablePacket;
    ULONGLONG deltaTimeInHns = deltaLinearPosition * 10000000 / m_ulDmaMovementRate;
    ULONGLONG timeOfAvailablePacketInHns = ullDmaTimeStamp - deltaTimeInHns;
    ULONGLONG timeOfAvailablePacketInQpc = timeOfAvailablePacketInHns * m_ullPerformanceCounterFrequency.QuadPart / 10000000;

    *PerformanceCounterValue = timeOfAvailablePacketInQpc;

    // No flags are defined yet
    *Flags = 0;

    // The capture data is produced as the position advances, so there is never
    // more data than revealed by the results from this routine.
    *MoreData = FALSE;

    // Update the last packet read by the OS
    m_ulLastOsReadPacket = availablePacketNumber;

    return STATUS_SUCCESS;
}

#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::SetWritePacket
(
    _In_ ULONG      PacketNumber,
    _In_ DWORD      Flags,
    _In_ ULONG      EosPacketLength
)
{
    UNREFERENCED_PARAMETER(EosPacketLength);
    NTSTATUS ntStatus;

    // The call must be from event driven mode
    if (m_ulNotificationsPerBuffer == 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    ULONG oldLastOsWritePacket = m_ulLastOsWritePacket;

    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
    // 1-based count of completed packets, 0-based packet number of current packet
    LONGLONG currentPacket = m_llPacketCounter;
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    // If not running, the current packet hasn't actually started transfering so OS should be writing
    // to the current packet. If running, then the current packing is already transfering to hardware
    // so the OS should write the packet after the current packet.
    ULONG expectedPacket = LODWORD(currentPacket);
    if (m_KsState == KSSTATE_RUN)
    {
        expectedPacket++;
    }

    // Check if OS PacketNumber is behind or too far ahead of current packet
    LONG deltaFromExpectedPacket = PacketNumber - expectedPacket;   // Modulo arithemetic
    if (deltaFromExpectedPacket < 0)
    {
        return STATUS_DATA_LATE_ERROR;
    }
    else if (deltaFromExpectedPacket > 0)
    {
        return STATUS_DATA_OVERRUN;
    }

    ULONG packetSize = (m_ulDmaBufferSize / m_ulNotificationsPerBuffer);
    ULONG packetIndex = PacketNumber % m_ulNotificationsPerBuffer;
    ULONG ulCurrentWritePosition = packetIndex * packetSize;

    // End-of-stream packets are an offload feature; the cable does not use them.
    if (Flags & KSSTREAM_HEADER_OPTIONSF_ENDOFSTREAM)
    {
        return STATUS_INVALID_PARAMETER;
    }

    m_ulLastOsWritePacket = PacketNumber;

    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
    ntStatus = SetCurrentWritePositionInternal(ulCurrentWritePosition);
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    if (!NT_SUCCESS(ntStatus))
    {
        m_ulLastOsWritePacket = oldLastOsWritePacket;
    }

    return ntStatus;
}

//=============================================================================
#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::GetOutputStreamPresentationPosition
(
    _Out_ KSAUDIO_PRESENTATION_POSITION *pPresentationPosition
)
{
    ASSERT (pPresentationPosition);

    // The call must be from event driven mode
    if(m_ulNotificationsPerBuffer == 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    return GetPresentationPosition(pPresentationPosition);
}

//=============================================================================
#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::GetPacketCount
(
    _Out_ ULONG *pPacketCount
)
{
    ASSERT(pPacketCount);

    // The call must be from event driven mode
    if(m_ulNotificationsPerBuffer == 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);

    if (m_KsState == KSSTATE_RUN)
    {
        // Get the current time and update simulated position.
        LARGE_INTEGER ilQPC = KeQueryPerformanceCounter(NULL);
        UpdatePosition(ilQPC);
    }

    *pPacketCount = LODWORD(m_llPacketCounter);
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    return STATUS_SUCCESS;
}

//linear and presentation positions
#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::GetPositions(
    _Out_opt_  ULONGLONG* _pullLinearBufferPosition,
    _Out_opt_  ULONGLONG* _pullPresentationPosition,
    _Out_opt_  LARGE_INTEGER* _pliQPCTime
)
{
    LARGE_INTEGER   ilQPC;
    KIRQL           oldIrql;

    // Update *_pullLinearBufferPosition with the the number of bytes fetched from waveRT ever since a stream got set into RUN
    // state.
    // Once the stream is set to STOP state, any further read on this call would return zero.

    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
    ilQPC = KeQueryPerformanceCounter(NULL);
    if (m_KsState == KSSTATE_RUN)
    {
        UpdatePosition(ilQPC);
    }
    if (_pullLinearBufferPosition)
    {
        *_pullLinearBufferPosition = m_ullLinearPosition;
    }
    if (_pullPresentationPosition)
    {
        *_pullPresentationPosition = m_ullPresentationPosition;
    }
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);
    if (_pliQPCTime)
    {
        *_pliQPCTime = ilQPC;
    }

    return STATUS_SUCCESS;
}

#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::GetPresentationPosition(_Out_  KSAUDIO_PRESENTATION_POSITION* _pPresentationPosition)
{
    ASSERT(_pPresentationPosition);
    LARGE_INTEGER timeStamp;

    ULONGLONG ullLinearPosition = { 0 };
    ULONGLONG ullPresentationPosition = { 0 };
    NTSTATUS status = STATUS_SUCCESS;

    status = GetPositions(&ullLinearPosition, &ullPresentationPosition, &timeStamp);
    if (!NT_SUCCESS(status))
    {
        return status;
    }

    _pPresentationPosition->u64PositionInBlocks = ullPresentationPosition * m_pWfExt->Format.nSamplesPerSec / m_pWfExt->Format.nAvgBytesPerSec;
    _pPresentationPosition->u64QPCPosition = (UINT64)timeStamp.QuadPart;

    return STATUS_SUCCESS;
}

#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::SetCurrentWritePositionInternal(_In_  ULONG _ulCurrentWritePosition)
{
    if (_ulCurrentWritePosition > m_ulDmaBufferSize)
    {
        return STATUS_INVALID_DEVICE_REQUEST;
    }

    m_ulCurrentWritePosition = _ulCurrentWritePosition;
    InterlockedExchange(&m_IsCurrentWritePositionUpdated, 1);

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::SetState
(
    _In_    KSSTATE State_
)
{
    NTSTATUS        ntStatus        = STATUS_SUCCESS;
    KIRQL oldIrql;

    switch (State_)
    {
        case KSSTATE_STOP:
            KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
            // Reset DMA
            m_llPacketCounter = 0;
            m_ullPlayPosition = 0;
            m_ullWritePosition = 0;
            m_ullLinearPosition = 0;
            m_ullPresentationPosition = 0;
            m_hnsElapsedTimeCarryForward = 0;
            m_byteDisplacementCarryForward = 0;
            m_hnsDPCTimeCarryForward = 0;

            // Reset OS read/write positions
            m_ulLastOsReadPacket = ULONG_MAX;
            m_ulCurrentWritePosition = 0;
            m_ulLastOsWritePacket = ULONG_MAX;

            KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);
            break;

        case KSSTATE_ACQUIRE:
            break;

        case KSSTATE_PAUSE:

            if (m_KsState > KSSTATE_PAUSE)
            {
                //
                // Run -> Pause
                //

                // Pause DMA
                if (m_ulNotificationIntervalMs > 0)
                {
                    ExCancelTimer(m_pNotificationTimer, NULL);
                    KeFlushQueuedDpcs();

                    // If pin is transitioning from RUN, save the time since last buffer completion event was sent
                    // so if the pin goes to RUN state again we can send the buffer completion event at correct time.
                    if (m_ullLastDPCTimeStamp > 0)
                    {
                        LARGE_INTEGER qpc;
                        LARGE_INTEGER qpcFrequency;
                        LONGLONG  hnsCurrentTime;

                        qpc = KeQueryPerformanceCounter(&qpcFrequency);

                        // Convert ticks to 100ns units.
                        hnsCurrentTime = KSCONVERT_PERFORMANCE_TIME(m_ullPerformanceCounterFrequency.QuadPart, qpc);
                        m_hnsDPCTimeCarryForward = hnsCurrentTime - m_ullLastDPCTimeStamp + m_hnsDPCTimeCarryForward;
                    }
                }

                // This call updates the linear buffer and presentation positions.
                GetPositions(NULL, NULL, NULL);
            }
            break;

        case KSSTATE_RUN:
            // Start DMA
            LARGE_INTEGER ullPerfCounterTemp;

            if (m_bCapture)
            {
                // Start each capture session with fresh audio instead of
                // whatever was queued while nobody was recording.
                m_pCable->ResetReader();
            }

            KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
            ullPerfCounterTemp = KeQueryPerformanceCounter(&m_ullPerformanceCounterFrequency);
            m_ullLastDPCTimeStamp = m_ullDmaTimeStamp = KSCONVERT_PERFORMANCE_TIME(m_ullPerformanceCounterFrequency.QuadPart, ullPerfCounterTemp);
            KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

            if (m_ulNotificationIntervalMs > 0)
            {
                // A 1 ms periodic timer emulates the hardware clock and
                // signals the notification events once per packet.
                ExSetTimer
                (
                    m_pNotificationTimer,
                    (-1) * HNSTIME_PER_MILLISECOND,
                    HNSTIME_PER_MILLISECOND, // 1 ms
                    NULL
                 );
            }

            break;
    }

    m_KsState = State_;

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::SetFormat
(
    _In_    KSDATAFORMAT    *DataFormat_
)
{
    UNREFERENCED_PARAMETER(DataFormat_);

    PAGED_CODE();

    return STATUS_NOT_SUPPORTED;
}

//=============================================================================
// Called with m_PositionSpinLock held.
#pragma code_seg()
VOID CMiniportWaveRTStream::UpdatePosition
(
    _In_ LARGE_INTEGER ilQPC
)
{
    if (m_ulDmaBufferSize == 0 || m_pDmaBuffer == NULL)
    {
        return;
    }

    // Convert ticks to 100ns units.
    LONGLONG  hnsCurrentTime = KSCONVERT_PERFORMANCE_TIME(m_ullPerformanceCounterFrequency.QuadPart, ilQPC);

    // Calculate the time elapsed since the last call to GetPosition() or since the
    // DMA engine started.  Note that the division by 10000 to convert to milliseconds
    // may cause us to lose some of the time, so we will carry the remainder forward
    // to the next GetPosition() call.
    //
    ULONGLONG hnsElapsed = hnsCurrentTime - m_ullDmaTimeStamp + m_hnsElapsedTimeCarryForward;
    ULONGLONG TimeElapsedInMS = hnsElapsed / 10000;

    // Carry forward the remainder of this division so we don't fall behind with our position too much.
    //
    m_hnsElapsedTimeCarryForward = hnsElapsed % 10000;

    // Calculate how many bytes in the DMA buffer would have been processed in the elapsed
    // time.  Note that the division by 1000 to convert to milliseconds may cause us to
    // lose some bytes, so we will carry the remainder forward to the next GetPosition() call.
    //
    // need to divide by 1000 because m_ulDmaMovementRate is average bytes per sec.
    ULONGLONG bytesTimesThousand = ((ULONGLONG)m_ulDmaMovementRate * TimeElapsedInMS) + m_byteDisplacementCarryForward;
    ULONGLONG byteDisplacement64 = bytesTimesThousand / 1000;
    m_byteDisplacementCarryForward = (ULONG)(bytesTimesThousand % 1000);

    // Positions only need to stay frame accurate; a stall this long means
    // the stream was starved and older audio is gone anyway.
    ULONG ByteDisplacement = (byteDisplacement64 > MAXULONG / 2) ? (MAXULONG / 2) : (ULONG)byteDisplacement64;
    ByteDisplacement -= ByteDisplacement % m_pWfExt->Format.nBlockAlign;

    m_ullPresentationPosition += ByteDisplacement;

    if (m_bCapture)
    {
        // Fill the WaveRT buffer from the cable.
        WriteBytes(ByteDisplacement);
    }
    else
    {
        // Move what the client rendered into the cable.
        ReadBytes(ByteDisplacement);
    }

    // Increment the DMA position by the number of bytes displaced since the last
    // call to UpdatePosition() and ensure we properly wrap at buffer length.
    //
    m_ullPlayPosition = m_ullWritePosition =
        (m_ullWritePosition + ByteDisplacement) % m_ulDmaBufferSize;

    m_ullLinearPosition += ByteDisplacement;

    // Update the DMA time stamp for the next call to GetPosition()
    //
    m_ullDmaTimeStamp = hnsCurrentTime;
}

//=============================================================================
#pragma code_seg()
VOID CMiniportWaveRTStream::WriteBytes
(
    _In_ ULONG ByteDisplacement
)
/*++

Routine Description:

  Capture: copies ByteDisplacement bytes from the cable into the WaveRT
  buffer at the current position. The cable returns silence when the render
  side is not playing.

--*/
{
    ULONG bufferOffset = m_ullLinearPosition % m_ulDmaBufferSize;

    if (ByteDisplacement > m_ulDmaBufferSize)
    {
        // Only the last buffer's worth can still be read by the client.
        ULONG skipped = ByteDisplacement - m_ulDmaBufferSize;
        m_pCable->ResetReader();
        bufferOffset = (ULONG)((bufferOffset + (ULONGLONG)skipped) % m_ulDmaBufferSize);
        ByteDisplacement = m_ulDmaBufferSize;
    }

    while (ByteDisplacement > 0)
    {
        ULONG runWrite = min(ByteDisplacement, m_ulDmaBufferSize - bufferOffset);

        m_pCable->Read(m_pDmaBuffer + bufferOffset, runWrite);

        bufferOffset = (bufferOffset + runWrite) % m_ulDmaBufferSize;
        ByteDisplacement -= runWrite;
    }
}

//=============================================================================
#pragma code_seg()
VOID CMiniportWaveRTStream::ReadBytes
(
    _In_ ULONG ByteDisplacement
)
/*++

Routine Description:

  Render: copies ByteDisplacement bytes from the WaveRT buffer at the current
  position into the cable, unchanged.

--*/
{
    ULONG bufferOffset = m_ullLinearPosition % m_ulDmaBufferSize;

    if (ByteDisplacement > m_ulDmaBufferSize)
    {
        // The client could not have filled more than one buffer; keep the
        // cable's timing by writing silence for the rest.
        ULONG skipped = ByteDisplacement - m_ulDmaBufferSize;
        m_pCable->Write(NULL, skipped);
        bufferOffset = (ULONG)((bufferOffset + (ULONGLONG)skipped) % m_ulDmaBufferSize);
        ByteDisplacement = m_ulDmaBufferSize;
    }

    while (ByteDisplacement > 0)
    {
        ULONG runWrite = min(ByteDisplacement, m_ulDmaBufferSize - bufferOffset);

        m_pCable->Write(m_pDmaBuffer + bufferOffset, runWrite);

        bufferOffset = (bufferOffset + runWrite) % m_ulDmaBufferSize;
        ByteDisplacement -= runWrite;
    }
}

//=============================================================================
#pragma code_seg()
void
TimerNotifyRT
(
    _In_      PEX_TIMER    Timer,
    _In_opt_  PVOID        DeferredContext
)
{
    LARGE_INTEGER qpc;
    LARGE_INTEGER qpcFrequency;
    BOOL bufferCompleted = FALSE;

    UNREFERENCED_PARAMETER(Timer);

    _IRQL_limited_to_(DISPATCH_LEVEL);

    CMiniportWaveRTStream* _this = (CMiniportWaveRTStream*)DeferredContext;

    if (NULL == _this)
    {
        return;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&_this->m_PositionSpinLock, &oldIrql);

    qpc = KeQueryPerformanceCounter(&qpcFrequency);

    // Convert ticks to 100ns units.
    LONGLONG  hnsCurrentTime = KSCONVERT_PERFORMANCE_TIME(_this->m_ullPerformanceCounterFrequency.QuadPart, qpc);

    // Calculate the time elapsed since the last we ran DPC that matched Notification interval. Note that the division by 10000
    // to convert to milliseconds may cause us to lose some of the time, so we will carry the remainder forward.

    ULONG TimeElapsedInMS = (ULONG)((hnsCurrentTime - _this->m_ullLastDPCTimeStamp + _this->m_hnsDPCTimeCarryForward)/10000);

    if (TimeElapsedInMS >= _this->m_ulNotificationIntervalMs)
    {
        // Carry forward the time greater than notification interval to adjust time to signal next buffer completion event accordingly.
        _this->m_hnsDPCTimeCarryForward = hnsCurrentTime - _this->m_ullLastDPCTimeStamp + _this->m_hnsDPCTimeCarryForward - (_this->m_ulNotificationIntervalMs * 10000);
        // Save the last time DPC ran at notification interval
        _this->m_ullLastDPCTimeStamp = hnsCurrentTime;
        bufferCompleted = TRUE;
    }

    if (!bufferCompleted)
    {
        goto End;
    }

    _this->UpdatePosition(qpc);

    _this->m_llPacketCounter++;

    if (_this->m_KsState != KSSTATE_RUN)
    {
        goto End;
    }

    // Send buffer completion event: the driver consumed a complete packet.
    if (!IsListEmpty(&_this->m_NotificationList))
    {
        PLIST_ENTRY leCurrent = _this->m_NotificationList.Flink;
        while (leCurrent != &_this->m_NotificationList)
        {
            NotificationListEntry* nleCurrent = CONTAINING_RECORD( leCurrent, NotificationListEntry, ListEntry);
            KeSetEvent(nleCurrent->NotificationEvent, 0, 0);

            leCurrent = leCurrent->Flink;
        }
    }

End:
    KeReleaseSpinLock(&_this->m_PositionSpinLock, oldIrql);
    return;
}
//=============================================================================
