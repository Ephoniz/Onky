/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    common.h

Abstract:

    CAdapterCommon class declaration.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Inc/Common.h). Modifications Copyright (c) 2026 Audio Program
// contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_COMMON_H_
#define _AUDIOPROGRAMCABLE_COMMON_H_

//=============================================================================
// Macros
//=============================================================================

#define IF_TRUE_JUMP(condition, label)                          \
    if (condition)                                              \
    {                                                           \
        goto label;                                             \
    }

#define IF_TRUE_ACTION_JUMP(condition, action, label)           \
    if (condition)                                              \
    {                                                           \
        action;                                                 \
        goto label;                                             \
    }

#define IF_FAILED_ACTION_JUMP(ntStatus, action, label)          \
        if (!NT_SUCCESS(ntStatus))                              \
        {                                                       \
            action;                                             \
            goto label;                                         \
        }

#define IF_FAILED_JUMP(ntStatus, label)                         \
    if (!NT_SUCCESS(ntStatus))                                  \
    {                                                           \
        goto label;                                             \
    }

#define SAFE_RELEASE(p) {if (p) { (p)->Release(); (p) = nullptr; } }

// JACKDESC_RGB(r, g, b)
#define JACKDESC_RGB(r, g, b) \
    ((COLORREF)((r << 16) | (g << 8) | (b)))

#define MINWAVERT_POOLTAG           'RWdA'
#define MINTOPORT_POOLTAG           'TPdA'
#define MINADAPTER_POOLTAG          'APdA'
#define CABLE_POOLTAG               'LCdA'

typedef enum
{
    eCableRenderDevice = 0,
    eCableCaptureDevice,
    eMaxDeviceType,
} eDeviceType;

//
// Signal processing modes and default formats structs.
//
typedef struct _MODE_AND_DEFAULT_FORMAT {
    GUID            Mode;
    KSDATAFORMAT*   DefaultFormat;
} MODE_AND_DEFAULT_FORMAT, *PMODE_AND_DEFAULT_FORMAT;

//
// Enumeration of the various types of pins implemented in this driver.
//
typedef enum
{
    NoPin,
    BridgePin,
    SystemRenderPin,
    SystemCapturePin,
} PINTYPE;

//
// PIN_DEVICE_FORMATS_AND_MODES
//
//  Used to specify a pin's type, formats, and modes. Arrays of these
//  structures follow the same order as the filter's pin descriptor array so
//  that KS pin IDs can serve as an index.
//
typedef struct _PIN_DEVICE_FORMATS_AND_MODES
{
    PINTYPE                             PinType;

    KSDATAFORMAT_WAVEFORMATEXTENSIBLE * WaveFormats;
    ULONG                               WaveFormatsCount;

    MODE_AND_DEFAULT_FORMAT *           ModeAndDefaultFormat;
    ULONG                               ModeAndDefaultFormatCount;

} PIN_DEVICE_FORMATS_AND_MODES, *PPIN_DEVICE_FORMATS_AND_MODES;

// forward declaration.
typedef struct _ENDPOINT_MINIPAIR *PENDPOINT_MINIPAIR;
class CLoopbackCable;

// both wave & topology miniport create function prototypes have this form:
typedef NTSTATUS (*PFNCREATEMINIPORT)(
    _Out_           PUNKNOWN                              * Unknown,
    _In_            REFCLSID,
    _In_opt_        PUNKNOWN                                UnknownOuter,
    _In_            POOL_FLAGS                              PoolFlags,
    _In_            PUNKNOWN                                UnknownAdapter,
    _In_opt_        PVOID                                   DeviceContext,
    _In_            PENDPOINT_MINIPAIR                      MiniportPair
);

//
// Endpoint miniport pair (wave/topology) descriptor.
//
typedef struct _ENDPOINT_MINIPAIR
{
    eDeviceType                     DeviceType;

    // Topology miniport.
    PWSTR                           TopoName;               // runtime reference string, unique per cable
    PCWSTR                          TemplateTopoName;       // interface template reference string in the INF
    PFNCREATEMINIPORT               TopoCreateCallback;
    PCFILTER_DESCRIPTOR*            TopoDescriptor;

    // Wave RT miniport.
    PWSTR                           WaveName;               // runtime reference string, unique per cable
    PCWSTR                          TemplateWaveName;       // interface template reference string in the INF
    PFNCREATEMINIPORT               WaveCreateCallback;
    PCFILTER_DESCRIPTOR*            WaveDescriptor;

    PIN_DEVICE_FORMATS_AND_MODES*   PinDeviceFormatsAndModes;
    ULONG                           PinDeviceFormatsAndModesCount;

    // Miniport physical connections.
    PHYSICALCONNECTIONTABLE*        PhysicalConnections;
    ULONG                           PhysicalConnectionCount;

    // Zero-based index of the cable this endpoint belongs to.
    ULONG                           CableIndex;

    // Interface friendly name, shown in Sound settings ("Audio Program Cable N").
    PCWSTR                          FriendlyName;
} ENDPOINT_MINIPAIR;

//=============================================================================
// Defines
//=============================================================================

// {A71A013A-6EAA-4B3B-801E-4B6409734C3B}
DEFINE_GUID(IID_IAdapterCommon,
0xa71a013a, 0x6eaa, 0x4b3b, 0x80, 0x1e, 0x4b, 0x64, 0x09, 0x73, 0x4c, 0x3b);

//=============================================================================
// Interfaces
//=============================================================================

///////////////////////////////////////////////////////////////////////////////
// IAdapterCommon
//
DECLARE_INTERFACE_(IAdapterCommon, IUnknown)
{
    STDMETHOD_(NTSTATUS,        Init)
    (
        THIS_
        _In_  PDEVICE_OBJECT      DeviceObject
    ) PURE;


    STDMETHOD_(PDEVICE_OBJECT,  GetPhysicalDeviceObject)
    (
        THIS
    ) PURE;

    // Registers a render and a capture endpoint for every cable.
    STDMETHOD_(NTSTATUS,        InstallCables)
    (
        THIS_
        _In_opt_    PIRP                Irp
    ) PURE;

    STDMETHOD_(CLoopbackCable*, GetCable)
    (
        THIS_
        _In_        ULONG               CableIndex
    ) PURE;

    STDMETHOD_(VOID,            Cleanup)
    (
        THIS
    ) PURE;
};

typedef IAdapterCommon *PADAPTERCOMMON;

//=============================================================================
// Function Prototypes
//=============================================================================
NTSTATUS
NewAdapterCommon
(
    _Out_       PUNKNOWN *              Unknown,
    _In_        REFCLSID,
    _In_opt_    PUNKNOWN                UnknownOuter,
    _In_        POOL_FLAGS              PoolFlags
);

#endif  //_AUDIOPROGRAMCABLE_COMMON_H_
