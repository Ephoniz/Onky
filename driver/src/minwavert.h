/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    minwavert.h

Abstract:

    Definition of wavert miniport class.
--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Main/minwavert.h). Modifications Copyright (c) 2026 Audio Program
// contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_MINWAVERT_H_
#define _AUDIOPROGRAMCABLE_MINWAVERT_H_

//=============================================================================
// Referenced Forward
//=============================================================================
class CMiniportWaveRTStream;
typedef CMiniportWaveRTStream *PCMiniportWaveRTStream;

//=============================================================================
// Classes
//=============================================================================
///////////////////////////////////////////////////////////////////////////////
// CMiniportWaveRT
//
class CMiniportWaveRT :
    public IMiniportWaveRT,
    public IMiniportAudioSignalProcessing,
    public CUnknown
{
private:
    ULONG                               m_ulSystemAllocated;
    ULONG                               m_ulMaxSystemStreams;

    PCFILTER_DESCRIPTOR                 m_FilterDesc;
    PIN_DEVICE_FORMATS_AND_MODES *      m_DeviceFormatsAndModes;
    KSPIN_LOCK                          m_DeviceFormatsAndModesLock; // To serialize access.
    KIRQL                               m_DeviceFormatsAndModesIrql;
    ULONG                               m_DeviceFormatsAndModesCount;

protected:
    // Strong reference: the adapter owns the cable ring and the miniport pair.
    PADAPTERCOMMON                      m_pAdapterCommon;
    CLoopbackCable *                    m_pCable;
    eDeviceType                         m_DeviceType;
    PENDPOINT_MINIPAIR                  m_pMiniportPair;

public:
    NTSTATUS ValidateStreamCreate
    (
        _In_    ULONG   _Pin,
        _In_    BOOLEAN _Capture
    );

    NTSTATUS StreamCreated
    (
        _In_ ULONG                  _Pin,
        _In_ PCMiniportWaveRTStream _Stream
    );

    NTSTATUS StreamClosed
    (
        _In_ ULONG                  _Pin,
        _In_ PCMiniportWaveRTStream _Stream
    );

    NTSTATUS IsFormatSupported
    (
        _In_ ULONG          _ulPin,
        _In_ BOOLEAN        _bCapture,
        _In_ PKSDATAFORMAT  _pDataFormat
    );

    static NTSTATUS GetAttributesFromAttributeList
    (
        _In_ const KSMULTIPLE_ITEM *_pAttributes,
        _In_ size_t _Size,
        _Out_ GUID* _pSignalProcessingMode
    );

public:
    DECLARE_STD_UNKNOWN();

#pragma code_seg("PAGE")
    CMiniportWaveRT(
        _In_            PUNKNOWN                                UnknownAdapter,
        _In_            PENDPOINT_MINIPAIR                      MiniportPair
    )
        :CUnknown(0),
        m_ulSystemAllocated(0),
        m_ulMaxSystemStreams(0),
        m_DeviceFormatsAndModes(MiniportPair->PinDeviceFormatsAndModes),
        m_DeviceFormatsAndModesCount(MiniportPair->PinDeviceFormatsAndModesCount),
        m_pAdapterCommon(NULL),
        m_pCable(NULL),
        m_DeviceType(MiniportPair->DeviceType),
        m_pMiniportPair(MiniportPair)
    {
        PAGED_CODE();

        RtlZeroMemory(&m_FilterDesc, sizeof(m_FilterDesc));

        if (UnknownAdapter != NULL)
        {
            m_pAdapterCommon = (PADAPTERCOMMON)UnknownAdapter;
            m_pAdapterCommon->AddRef();
        }

        if (MiniportPair->WaveDescriptor)
        {
            RtlCopyMemory(&m_FilterDesc, MiniportPair->WaveDescriptor, sizeof(m_FilterDesc));

            //
            // Get the max # of pin instances.
            //
            if (IsRenderDevice())
            {
                if (m_FilterDesc.PinCount > KSPIN_WAVE_RENDER_SOURCE)
                {
                    m_ulMaxSystemStreams = m_FilterDesc.Pins[KSPIN_WAVE_RENDER_SINK_SYSTEM].MaxFilterInstanceCount;
                }
            }
            else
            {
                //
                // capture bridge pin comes first in the enumeration
                //
                if (m_FilterDesc.PinCount > KSPIN_WAVEIN_HOST)
                {
                    m_ulMaxSystemStreams = m_FilterDesc.Pins[KSPIN_WAVEIN_HOST].MaxFilterInstanceCount;
                }
            }
        }

        KeInitializeSpinLock(&m_DeviceFormatsAndModesLock);
        m_DeviceFormatsAndModesIrql = PASSIVE_LEVEL;
    }

#pragma code_seg()

    ~CMiniportWaveRT();

    IMP_IMiniportWaveRT;
    IMP_IMiniportAudioSignalProcessing;

    // Friends
    friend class        CMiniportWaveRTStream;

    friend NTSTATUS PropertyHandler_WaveFilter
    (
        _In_ PPCPROPERTY_REQUEST      PropertyRequest
    );

public:
    NTSTATUS PropertyHandlerProposedFormat
    (
        _In_ PPCPROPERTY_REQUEST PropertyRequest
    );

    NTSTATUS PropertyHandlerProposedFormat2
    (
        _In_ PPCPROPERTY_REQUEST PropertyRequest
    );

    CLoopbackCable * GetCable()
    {
        return m_pCable;
    }

private:
    _IRQL_raises_(DISPATCH_LEVEL)
    _Acquires_lock_(m_DeviceFormatsAndModesLock)
    _Requires_lock_not_held_(m_DeviceFormatsAndModesLock)
    _IRQL_saves_global_(SpinLock, m_DeviceFormatsAndModesIrql)
    VOID AcquireFormatsAndModesLock();

    _Releases_lock_(m_DeviceFormatsAndModesLock)
    _Requires_lock_held_(m_DeviceFormatsAndModesLock)
    _IRQL_restores_global_(SpinLock, m_DeviceFormatsAndModesIrql)
    VOID ReleaseFormatsAndModesLock();

    _Post_satisfies_(return > 0)
    ULONG GetPinSupportedDeviceFormats(_In_ ULONG PinId, _Outptr_opt_result_buffer_(return) KSDATAFORMAT_WAVEFORMATEXTENSIBLE **ppFormats);

    _Success_(return != 0)
    ULONG GetPinSupportedDeviceModes(_In_ ULONG PinId, _Outptr_opt_result_buffer_(return) _On_failure_(_Deref_post_null_) MODE_AND_DEFAULT_FORMAT **ppModes);

protected:
    BOOL IsRenderDevice()
    {
        return m_DeviceType == eCableRenderDevice ? TRUE : FALSE;
    }

    BOOL IsSystemRenderPin(ULONG nPinId);

    BOOL IsSystemCapturePin(ULONG nPinId);

    BOOL IsBridgePin(ULONG nPinId);
};

typedef CMiniportWaveRT *PCMiniportWaveRT;

NTSTATUS
CreateMiniportWaveRTCable
(
    _Out_       PUNKNOWN *,
    _In_        REFCLSID,
    _In_opt_    PUNKNOWN,
    _In_        POOL_FLAGS,
    _In_        PUNKNOWN,
    _In_opt_    PVOID,
    _In_        PENDPOINT_MINIPAIR
);

#endif // _AUDIOPROGRAMCABLE_MINWAVERT_H_
