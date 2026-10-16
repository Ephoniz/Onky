/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    minwavertstream.h

Abstract:

    Definition of wavert miniport stream class.
--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Main/minwavertstream.h). Modifications Copyright (c) 2026 Audio
// Program contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_MINWAVERTSTREAM_H_
#define _AUDIOPROGRAMCABLE_MINWAVERTSTREAM_H_

//
// Structure to store notifications events in a protected list
//
typedef struct _NotificationListEntry
{
    LIST_ENTRY  ListEntry;
    PKEVENT     NotificationEvent;
} NotificationListEntry;

EXT_CALLBACK   TimerNotifyRT;

//=============================================================================
// Referenced Forward
//=============================================================================
class CMiniportWaveRT;
typedef CMiniportWaveRT *PCMiniportWaveRT;

//=============================================================================
// Classes
//=============================================================================
///////////////////////////////////////////////////////////////////////////////
// CMiniportWaveRTStream
//
//  The stream clock is simulated from the performance counter, as in the
//  sample. Each position update moves the elapsed bytes between the WaveRT
//  buffer and the cable: render streams write them into the cable, capture
//  streams read them out of it.
//
class CMiniportWaveRTStream :
    public IMiniportWaveRTStreamNotification,
    public IMiniportWaveRTInputStream,
    public IMiniportWaveRTOutputStream,
    public CUnknown
{
protected:
    PPORTWAVERTSTREAM           m_pPortStream;
    LIST_ENTRY                  m_NotificationList;
    PEX_TIMER                   m_pNotificationTimer;
    ULONG                       m_ulNotificationIntervalMs;

public:
    DECLARE_STD_UNKNOWN();
    DEFINE_STD_CONSTRUCTOR(CMiniportWaveRTStream);
    ~CMiniportWaveRTStream();

    IMP_IMiniportWaveRTStream;
    IMP_IMiniportWaveRTStreamNotification;
    IMP_IMiniportWaveRTInputStream;
    IMP_IMiniportWaveRTOutputStream;

    NTSTATUS                    Init
    (
        _In_  PCMiniportWaveRT    Miniport,
        _In_  PPORTWAVERTSTREAM   Stream,
        _In_  ULONG               Channel,
        _In_  BOOLEAN             Capture,
        _In_  PKSDATAFORMAT       DataFormat
    );

    // Friends
    friend class                CMiniportWaveRT;
    friend EXT_CALLBACK         TimerNotifyRT;
protected:
    CMiniportWaveRT*            m_pMiniport;
    CLoopbackCable*             m_pCable;
    ULONG                       m_ulPin;
    BOOLEAN                     m_bCapture;
    BOOLEAN                     m_bUnregisterStream;
    ULONG                       m_ulDmaBufferSize;
    BYTE*                       m_pDmaBuffer;
    ULONG                       m_ulNotificationsPerBuffer;
    KSSTATE                     m_KsState;
    ULONGLONG                   m_ullPlayPosition;
    ULONGLONG                   m_ullWritePosition;
    ULONGLONG                   m_ullLinearPosition;
    ULONGLONG                   m_ullPresentationPosition;
    ULONG                       m_ulLastOsReadPacket;
    ULONG                       m_ulLastOsWritePacket;
    LONGLONG                    m_llPacketCounter;
    ULONGLONG                   m_ullDmaTimeStamp;
    LARGE_INTEGER               m_ullPerformanceCounterFrequency;
    ULONGLONG                   m_hnsElapsedTimeCarryForward;
    ULONGLONG                   m_ullLastDPCTimeStamp;
    ULONGLONG                   m_hnsDPCTimeCarryForward;
    ULONG                       m_byteDisplacementCarryForward;
    ULONG                       m_ulDmaMovementRate;
    PWAVEFORMATEXTENSIBLE       m_pWfExt;
    KSPIN_LOCK                  m_PositionSpinLock;

public:

    //presentation
    NTSTATUS GetPresentationPosition
    (
        _Out_  KSAUDIO_PRESENTATION_POSITION *_pPresentationPosition
    );

private:

    //
    // Helper functions.
    //

#pragma code_seg()

    VOID WriteBytes
    (
        _In_ ULONG ByteDisplacement
    );

    VOID ReadBytes
    (
        _In_ ULONG ByteDisplacement
    );

    VOID UpdatePosition
    (
        _In_ LARGE_INTEGER ilQPC
    );

    NTSTATUS SetCurrentWritePositionInternal
    (
        _In_  ULONG ulCurrentWritePosition
    );

    NTSTATUS GetPositions
    (
        _Out_opt_  ULONGLONG *      _pullLinearBufferPosition,
        _Out_opt_  ULONGLONG *      _pullPresentationPosition,
        _Out_opt_  LARGE_INTEGER *  _pliQPCTime
    );
};
typedef CMiniportWaveRTStream *PCMiniportWaveRTStream;
#endif // _AUDIOPROGRAMCABLE_MINWAVERTSTREAM_H_
