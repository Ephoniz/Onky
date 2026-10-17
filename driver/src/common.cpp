/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    common.cpp

Abstract:

    Implementation of the AdapterCommon class.
--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Main/common.cpp). Modifications Copyright (c) 2026 Audio Program
// contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#pragma warning (disable : 4127)

#include <initguid.h>
#include "definitions.h"
#include <devpkey.h>
#include "endpoints.h"
#include "loopback.h"
#include "mintopo.h"
#include "minwavert.h"
#include "cabletables.h"

//=============================================================================
// Per-cable endpoint templates. Runtime copies get unique reference strings,
// a cable index and a friendly name.
//=============================================================================

static
const ENDPOINT_MINIPAIR CableRenderMiniportsTemplate =
{
    eCableRenderDevice,
    NULL,                                       // TopoName, set per cable
    CABLE_TEMPLATE_RENDER_TOPO,
    CreateMiniportTopologyCable,
    &CableRenderTopoMiniportFilterDescriptor,
    NULL,                                       // WaveName, set per cable
    CABLE_TEMPLATE_RENDER_WAVE,
    CreateMiniportWaveRTCable,
    &CableRenderWaveMiniportFilterDescriptor,
    CableRenderPinDeviceFormatsAndModes,
    SIZEOF_ARRAY(CableRenderPinDeviceFormatsAndModes),
    CableRenderTopologyPhysicalConnections,
    SIZEOF_ARRAY(CableRenderTopologyPhysicalConnections),
    0,                                          // CableIndex, set per cable
    NULL                                        // FriendlyName, set per cable
};

static
const ENDPOINT_MINIPAIR CableCaptureMiniportsTemplate =
{
    eCableCaptureDevice,
    NULL,
    CABLE_TEMPLATE_CAPTURE_TOPO,
    CreateMiniportTopologyCable,
    &CableCaptureTopoMiniportFilterDescriptor,
    NULL,
    CABLE_TEMPLATE_CAPTURE_WAVE,
    CreateMiniportWaveRTCable,
    &CableCaptureWaveMiniportFilterDescriptor,
    CableCapturePinDeviceFormatsAndModes,
    SIZEOF_ARRAY(CableCapturePinDeviceFormatsAndModes),
    CableCaptureTopologyPhysicalConnections,
    SIZEOF_ARRAY(CableCaptureTopologyPhysicalConnections),
    0,
    NULL
};

#define CABLE_REFSTRING_CCH     40
#define CABLE_FRIENDLY_CCH      64

typedef struct _CABLE_ENDPOINTS
{
    ENDPOINT_MINIPAIR   RenderPair;
    ENDPOINT_MINIPAIR   CapturePair;
    WCHAR               RenderTopoName[CABLE_REFSTRING_CCH];
    WCHAR               RenderWaveName[CABLE_REFSTRING_CCH];
    WCHAR               CaptureTopoName[CABLE_REFSTRING_CCH];
    WCHAR               CaptureWaveName[CABLE_REFSTRING_CCH];
    WCHAR               FriendlyName[CABLE_FRIENDLY_CCH];
    CLoopbackCable      Cable;
    BOOLEAN             CableInitialized;
} CABLE_ENDPOINTS, *PCABLE_ENDPOINTS;

//=============================================================================
// Classes
//=============================================================================

///////////////////////////////////////////////////////////////////////////////
// CAdapterCommon
//
class CAdapterCommon :
    public IAdapterCommon,
    public IAdapterPowerManagement,
    public CUnknown
{
    private:
        PDEVICE_OBJECT          m_pDeviceObject;
        PDEVICE_OBJECT          m_pPhysicalDeviceObject;
        DEVICE_POWER_STATE      m_PowerState;

        ULONG                   m_CableCount;
        PCABLE_ENDPOINTS        m_Cables;

    public:
        //=====================================================================
        // Default CUnknown
        DECLARE_STD_UNKNOWN();
        DEFINE_STD_CONSTRUCTOR(CAdapterCommon);
        ~CAdapterCommon();

        //=====================================================================
        // Default IAdapterPowerManagement
        IMP_IAdapterPowerManagement;

        //=====================================================================
        // IAdapterCommon methods

        STDMETHODIMP_(NTSTATUS) Init
        (
            _In_  PDEVICE_OBJECT  DeviceObject
        );


        STDMETHODIMP_(PDEVICE_OBJECT)   GetPhysicalDeviceObject(void);

        STDMETHODIMP_(NTSTATUS) InstallCables
        (
            _In_opt_    PIRP                Irp
        );


        STDMETHODIMP_(CLoopbackCable*) GetCable
        (
            _In_        ULONG               CableIndex
        );

        STDMETHODIMP_(VOID) Cleanup();

        //=====================================================================
        // friends
        friend NTSTATUS         NewAdapterCommon
        (
            _Out_       PUNKNOWN *              Unknown,
            _In_        REFCLSID,
            _In_opt_    PUNKNOWN                UnknownOuter,
            _In_        POOL_FLAGS              PoolFlags
        );

    private:

    LIST_ENTRY m_SubdeviceCache;

    static ULONG ReadCableCount();

    VOID FreeCables();

    NTSTATUS InstallSubdevice
    (
        _In_opt_    PIRP                Irp,
        _In_        PWSTR               Name,
        _In_        PCWSTR              TemplateName,
        _In_        PCWSTR              FriendlyName,
        _In_        REFGUID             PortClassId,
        _In_        PFNCREATEMINIPORT   MiniportCreate,
        _In_        PENDPOINT_MINIPAIR  MiniportPair,
        _Out_       PUNKNOWN          * OutPortUnknown,
        _Out_       PUNKNOWN          * OutMiniportUnknown
    );

    NTSTATUS UnregisterSubdevice
    (
        _In_opt_ PUNKNOWN               UnknownPort
    );

    NTSTATUS ConnectTopologies
    (
        _In_ PUNKNOWN                   UnknownTopology,
        _In_ PUNKNOWN                   UnknownWave,
        _In_ PHYSICALCONNECTIONTABLE*   PhysicalConnections,
        _In_ ULONG                      PhysicalConnectionCount
    );

    NTSTATUS DisconnectTopologies
    (
        _In_ PUNKNOWN                   UnknownTopology,
        _In_ PUNKNOWN                   UnknownWave,
        _In_ PHYSICALCONNECTIONTABLE*   PhysicalConnections,
        _In_ ULONG                      PhysicalConnectionCount
    );

    NTSTATUS InstallEndpointFilters
    (
        _In_opt_    PIRP                Irp,
        _In_        PENDPOINT_MINIPAIR  MiniportPair
    );

    NTSTATUS CacheSubdevice
    (
        _In_ PWSTR Name,
        _In_ PUNKNOWN UnknownPort,
        _In_ PUNKNOWN UnknownMiniport
    );

    NTSTATUS RemoveCachedSubdevice
    (
        _In_ PWSTR Name
    );

    VOID EmptySubdeviceCache();

    NTSTATUS CreateAudioInterface
    (
        _In_ PCWSTR                                                 ReferenceString,
        _In_opt_ PCWSTR                                             TemplateReferenceString,
        _In_opt_ PCWSTR                                             FriendlyName,
        _Out_ _At_(AudioSymbolicLinkName->Buffer, __drv_allocatesMem(Mem)) PUNICODE_STRING AudioSymbolicLinkName
    );

    NTSTATUS MigrateDeviceInterfaceTemplateParameters
    (
        _In_ PUNICODE_STRING    SymbolicLinkName,
        _In_opt_ PCWSTR         TemplateReferenceString
    );

    NTSTATUS SetInterfaceFriendlyName
    (
        _In_ PUNICODE_STRING    SymbolicLinkName,
        _In_ PCWSTR             FriendlyName
    );
};

typedef struct _MINIPAIR_UNKNOWN
{
    LIST_ENTRY              ListEntry;
    WCHAR                   Name[MAX_PATH];
    PUNKNOWN                PortInterface;
    PUNKNOWN                MiniportInterface;
    PADAPTERPOWERMANAGEMENT PowerInterface;
} MINIPAIR_UNKNOWN;

#define MAX_DEVICE_REG_KEY_LENGTH 0x100

//-----------------------------------------------------------------------------
// Functions
//-----------------------------------------------------------------------------

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
NewAdapterCommon
(
    _Out_       PUNKNOWN *              Unknown,
    _In_        REFCLSID,
    _In_opt_    PUNKNOWN                UnknownOuter,
    _In_        POOL_FLAGS              PoolFlags
)
/*++

Routine Description:

  Creates a new CAdapterCommon

--*/
{
    PAGED_CODE();

    ASSERT(Unknown);

    //
    // Allocate an adapter object.
    //
    CAdapterCommon *p = new(PoolFlags, MINADAPTER_POOLTAG) CAdapterCommon(UnknownOuter);
    if (p == NULL)
    {
        DPF(D_ERROR, ("NewAdapterCommon failed, 0x%x", STATUS_INSUFFICIENT_RESOURCES));
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    p->m_pDeviceObject = NULL;
    p->m_pPhysicalDeviceObject = NULL;
    p->m_PowerState = PowerDeviceD0;
    p->m_CableCount = 0;
    p->m_Cables = NULL;
    InitializeListHead(&p->m_SubdeviceCache);

    *Unknown = PUNKNOWN((PADAPTERCOMMON)(p));
    (*Unknown)->AddRef();

    return STATUS_SUCCESS;
} // NewAdapterCommon

//=============================================================================
#pragma code_seg("PAGE")
CAdapterCommon::~CAdapterCommon
(
    void
)
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::~CAdapterCommon]"));

    EmptySubdeviceCache();
    FreeCables();
} // ~CAdapterCommon

//=============================================================================
#pragma code_seg("PAGE")
VOID
CAdapterCommon::FreeCables()
{
    PAGED_CODE();

    if (m_Cables != NULL)
    {
        for (ULONG i = 0; i < m_CableCount; i++)
        {
            if (m_Cables[i].CableInitialized)
            {
                m_Cables[i].Cable.Release();
                m_Cables[i].CableInitialized = FALSE;
            }
        }

        ExFreePoolWithTag(m_Cables, MINADAPTER_POOLTAG);
        m_Cables = NULL;
    }

    m_CableCount = 0;
}

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(PDEVICE_OBJECT)
CAdapterCommon::GetPhysicalDeviceObject
(
    void
)
{
    PAGED_CODE();

    return m_pPhysicalDeviceObject;
} // GetPhysicalDeviceObject

//=============================================================================
#pragma code_seg("PAGE")
ULONG
CAdapterCommon::ReadCableCount()
/*++

Routine Description:

  Reads Parameters\CableCount from the driver's service key. Missing,
  mistyped or out-of-range values fall back to the default or are clamped.

--*/
{
    PAGED_CODE();

    HANDLE          driverKey = NULL;
    UNICODE_STRING  valueName;
    UCHAR           buffer[sizeof(KEY_VALUE_PARTIAL_INFORMATION) + sizeof(ULONG)] = { 0 };
    ULONG           resultLength = 0;
    bool            hasValue = false;
    ULONG           value = 0;

    NTSTATUS ntStatus = IoOpenDriverRegistryKey(g_DriverObject,
                                                DriverRegKeyParameters,
                                                KEY_READ,
                                                0,
                                                &driverKey);
    if (NT_SUCCESS(ntStatus))
    {
        RtlInitUnicodeString(&valueName, CABLE_COUNT_VALUE_NAME);
        ntStatus = ZwQueryValueKey(driverKey,
                                   &valueName,
                                   KeyValuePartialInformation,
                                   buffer,
                                   sizeof(buffer),
                                   &resultLength);
        ZwClose(driverKey);

        PKEY_VALUE_PARTIAL_INFORMATION info = (PKEY_VALUE_PARTIAL_INFORMATION)buffer;
        if (NT_SUCCESS(ntStatus) && info->Type == REG_DWORD && info->DataLength == sizeof(ULONG))
        {
            hasValue = true;
            value = *(ULONG UNALIGNED *)info->Data;
        }
    }

    ULONG cableCount = CableCountFromSetting(hasValue, value);
    DPF(D_TERSE, ("CableCount: %u", cableCount));
    return cableCount;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::Init
(
    _In_  PDEVICE_OBJECT          DeviceObject
)
/*++

Routine Description:

    Initialize adapter common object: read the cable count and allocate the
    per-cable state. Endpoints are registered later by InstallCables.

--*/
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::Init]"));

    ASSERT(DeviceObject);

    NTSTATUS        ntStatus    = STATUS_SUCCESS;
    ULONG           cableCount  = 0;

    m_pDeviceObject         = DeviceObject;
    m_pPhysicalDeviceObject = NULL;
    m_PowerState            = PowerDeviceD0;

    //
    // Get the PDO.
    //
    ntStatus = PcGetPhysicalDeviceObject(DeviceObject, &m_pPhysicalDeviceObject);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("PcGetPhysicalDeviceObject failed, 0x%x", ntStatus)),
        Done);

    cableCount = ReadCableCount();

    // Pool memory is zeroed, which is the initial state CLoopbackCable expects.
    m_Cables = (PCABLE_ENDPOINTS)ExAllocatePool2(POOL_FLAG_NON_PAGED,
                                                 sizeof(CABLE_ENDPOINTS) * cableCount,
                                                 MINADAPTER_POOLTAG);
    IF_TRUE_ACTION_JUMP(m_Cables == NULL, ntStatus = STATUS_INSUFFICIENT_RESOURCES, Done);

    m_CableCount = cableCount;

    for (ULONG i = 0; i < m_CableCount; i++)
    {
        PCABLE_ENDPOINTS cable = &m_Cables[i];
        const ULONG number = i + 1;

        ntStatus = RtlStringCchPrintfW(cable->RenderTopoName, CABLE_REFSTRING_CCH, L"%s%02u", CABLE_TEMPLATE_RENDER_TOPO, number);
        IF_FAILED_JUMP(ntStatus, Done);
        ntStatus = RtlStringCchPrintfW(cable->RenderWaveName, CABLE_REFSTRING_CCH, L"%s%02u", CABLE_TEMPLATE_RENDER_WAVE, number);
        IF_FAILED_JUMP(ntStatus, Done);
        ntStatus = RtlStringCchPrintfW(cable->CaptureTopoName, CABLE_REFSTRING_CCH, L"%s%02u", CABLE_TEMPLATE_CAPTURE_TOPO, number);
        IF_FAILED_JUMP(ntStatus, Done);
        ntStatus = RtlStringCchPrintfW(cable->CaptureWaveName, CABLE_REFSTRING_CCH, L"%s%02u", CABLE_TEMPLATE_CAPTURE_WAVE, number);
        IF_FAILED_JUMP(ntStatus, Done);
        ntStatus = RtlStringCchPrintfW(cable->FriendlyName, CABLE_FRIENDLY_CCH, CABLE_FRIENDLY_NAME_FORMAT, number);
        IF_FAILED_JUMP(ntStatus, Done);

        cable->RenderPair = CableRenderMiniportsTemplate;
        cable->RenderPair.TopoName = cable->RenderTopoName;
        cable->RenderPair.WaveName = cable->RenderWaveName;
        cable->RenderPair.CableIndex = i;
        cable->RenderPair.FriendlyName = cable->FriendlyName;

        cable->CapturePair = CableCaptureMiniportsTemplate;
        cable->CapturePair.TopoName = cable->CaptureTopoName;
        cable->CapturePair.WaveName = cable->CaptureWaveName;
        cable->CapturePair.CableIndex = i;
        cable->CapturePair.FriendlyName = cable->FriendlyName;

        ntStatus = cable->Cable.Initialize();
        IF_FAILED_ACTION_JUMP(
            ntStatus,
            DPF(D_ERROR, ("Cable %u ring allocation failed, 0x%x", number, ntStatus)),
            Done);
        cable->CableInitialized = TRUE;
    }

Done:
    if (!NT_SUCCESS(ntStatus))
    {
        FreeCables();
    }

    return ntStatus;
} // Init

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(NTSTATUS)
CAdapterCommon::InstallCables
(
    _In_opt_    PIRP                Irp
)
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::InstallCables]"));

    NTSTATUS ntStatus = STATUS_SUCCESS;

    if (m_Cables == NULL || m_CableCount == 0)
    {
        return STATUS_INVALID_DEVICE_STATE;
    }

    for (ULONG i = 0; i < m_CableCount; i++)
    {
        ntStatus = InstallEndpointFilters(Irp, &m_Cables[i].RenderPair);
        IF_FAILED_ACTION_JUMP(
            ntStatus,
            DPF(D_ERROR, ("Cable %u render endpoint failed, 0x%x", i + 1, ntStatus)),
            Done);

        ntStatus = InstallEndpointFilters(Irp, &m_Cables[i].CapturePair);
        IF_FAILED_ACTION_JUMP(
            ntStatus,
            DPF(D_ERROR, ("Cable %u capture endpoint failed, 0x%x", i + 1, ntStatus)),
            Done);
    }

Done:
    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(CLoopbackCable*)
CAdapterCommon::GetCable
(
    _In_        ULONG               CableIndex
)
{
    PAGED_CODE();

    if (m_Cables == NULL || CableIndex >= m_CableCount || !m_Cables[CableIndex].CableInitialized)
    {
        return NULL;
    }

    return &m_Cables[CableIndex].Cable;
}

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP
CAdapterCommon::NonDelegatingQueryInterface
(
    _In_ REFIID                      Interface,
    _COM_Outptr_ PVOID *        Object
)
{
    PAGED_CODE();

    ASSERT(Object);

    if (IsEqualGUIDAligned(Interface, IID_IUnknown))
    {
        *Object = PVOID(PUNKNOWN(PADAPTERCOMMON(this)));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IAdapterCommon))
    {
        *Object = PVOID(PADAPTERCOMMON(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IAdapterPowerManagement))
    {
        *Object = PVOID(PADAPTERPOWERMANAGEMENT(this));
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
#pragma code_seg()
STDMETHODIMP_(void)
CAdapterCommon::PowerChangeState
(
    _In_  POWER_STATE             NewState
)
/*++

Routine Description:

  There is no hardware to power. PortCls pauses active streams before a
  sleep transition, so the cables just stop moving data.

--*/
{
    DPF_ENTER(("[CAdapterCommon::PowerChangeState]"));

    // Notify all registered miniports of a power state change
    PLIST_ENTRY le = NULL;
    for (le = m_SubdeviceCache.Flink; le != &m_SubdeviceCache; le = le->Flink)
    {
        MINIPAIR_UNKNOWN *pRecord = CONTAINING_RECORD(le, MINIPAIR_UNKNOWN, ListEntry);

        if (pRecord->PowerInterface)
        {
            pRecord->PowerInterface->PowerChangeState(NewState);
        }
    }

    if (NewState.DeviceState != m_PowerState)
    {
        switch (NewState.DeviceState)
        {
            case PowerDeviceD0:
            case PowerDeviceD1:
            case PowerDeviceD2:
            case PowerDeviceD3:
                m_PowerState = NewState.DeviceState;

                DPF
                (
                    D_VERBOSE,
                    ("Entering D%u", ULONG(m_PowerState) - ULONG(PowerDeviceD0))
                );

                break;

            default:

                DPF(D_VERBOSE, ("Unknown Device Power State"));
                break;
        }
    }
} // PowerStateChange

//=============================================================================
#pragma code_seg()
STDMETHODIMP_(NTSTATUS)
CAdapterCommon::QueryDeviceCapabilities
(
    _Inout_updates_bytes_(sizeof(DEVICE_CAPABILITIES)) PDEVICE_CAPABILITIES    PowerDeviceCaps
)
{
    UNREFERENCED_PARAMETER(PowerDeviceCaps);

    DPF_ENTER(("[CAdapterCommon::QueryDeviceCapabilities]"));

    return (STATUS_SUCCESS);
} // QueryDeviceCapabilities

//=============================================================================
#pragma code_seg()
STDMETHODIMP_(NTSTATUS)
CAdapterCommon::QueryPowerChangeState
(
    _In_  POWER_STATE             NewStateQuery
)
{
    NTSTATUS status = STATUS_SUCCESS;

    DPF_ENTER(("[CAdapterCommon::QueryPowerChangeState]"));

    // query each miniport for it's power state, we're finished if even one indicates
    // it cannot go to this power state.
    PLIST_ENTRY le = NULL;
    for (le = m_SubdeviceCache.Flink; le != &m_SubdeviceCache && NT_SUCCESS(status); le = le->Flink)
    {
        MINIPAIR_UNKNOWN *pRecord = CONTAINING_RECORD(le, MINIPAIR_UNKNOWN, ListEntry);

        if (pRecord->PowerInterface)
        {
            status = pRecord->PowerInterface->QueryPowerChangeState(NewStateQuery);
        }
    }

    return status;
} // QueryPowerChangeState

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::SetInterfaceFriendlyName
(
    _In_ PUNICODE_STRING    SymbolicLinkName,
    _In_ PCWSTR             FriendlyName
)
/*++

Routine Description:

  Gives a runtime interface its own friendly name. The audio stack reads
  the FriendlyName value from the interface's device parameters key, the
  same value an INF AddInterface section would write.

--*/
{
    PAGED_CODE();

    NTSTATUS        ntStatus;
    NTSTATUS        propStatus;
    HANDLE          interfaceKey = NULL;
    UNICODE_STRING  valueName;
    size_t          cbName = 0;

    ntStatus = RtlStringCbLengthW(FriendlyName, CABLE_FRIENDLY_CCH * sizeof(WCHAR), &cbName);
    IF_FAILED_JUMP(ntStatus, Exit);
    cbName += sizeof(WCHAR);

    ntStatus = IoOpenDeviceInterfaceRegistryKey(SymbolicLinkName, KEY_WRITE, &interfaceKey);
    IF_FAILED_JUMP(ntStatus, Exit);

    RtlInitUnicodeString(&valueName, L"FriendlyName");
    ntStatus = ZwSetValueKey(interfaceKey, &valueName, 0, REG_SZ, (PVOID)FriendlyName, (ULONG)cbName);
    IF_FAILED_JUMP(ntStatus, Exit);

    // Keep the PnP property in sync for tools that read it directly.
    propStatus = IoSetDeviceInterfacePropertyData(
        SymbolicLinkName,
        &DEVPKEY_DeviceInterface_FriendlyName,
        LOCALE_NEUTRAL,
        PLUGPLAY_PROPERTY_PERSISTENT,
        DEVPROP_TYPE_STRING,
        (ULONG)cbName,
        (PVOID)FriendlyName);
    if (!NT_SUCCESS(propStatus))
    {
        DPF(D_TERSE, ("SetInterfaceFriendlyName: property not set, 0x%x", propStatus));
    }

Exit:
    if (interfaceKey)
    {
        ZwClose(interfaceKey);
    }

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::CreateAudioInterface
(
    _In_ PCWSTR ReferenceString,
    _In_opt_ PCWSTR TemplateReferenceString,
    _In_opt_ PCWSTR FriendlyName,
    _Out_ _At_(AudioSymbolicLinkName->Buffer, __drv_allocatesMem(Mem)) PUNICODE_STRING AudioSymbolicLinkName
)
/*++

Routine Description:

Create the audio interface (in disabled mode).

--*/
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::CreateAudioInterface]"));

    NTSTATUS        ntStatus;
    UNICODE_STRING  referenceString;

    RtlInitUnicodeString(&referenceString, ReferenceString);

    //
    // Reset output value.
    //
    RtlZeroMemory(AudioSymbolicLinkName, sizeof(UNICODE_STRING));

    //
    // Register an audio interface if not already present.
    //
    ntStatus = IoRegisterDeviceInterface(
        GetPhysicalDeviceObject(),
        &KSCATEGORY_AUDIO,
        &referenceString,
        AudioSymbolicLinkName);

    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("CreateAudioInterface: IoRegisterDeviceInterface(KSCATEGORY_AUDIO): failed, 0x%x", ntStatus)),
        Done);

    //
    // Copy the interface parameters (CLSID, endpoint properties) from the
    // INF template first, so the friendly name overrides the template default.
    //
    if (NULL != TemplateReferenceString)
    {
        ntStatus = MigrateDeviceInterfaceTemplateParameters(AudioSymbolicLinkName, TemplateReferenceString);

        IF_FAILED_ACTION_JUMP(
            ntStatus,
            DPF(D_ERROR, ("CreateAudioInterface: MigrateDeviceInterfaceTemplateParameters(...): failed, 0x%x", ntStatus)),
            Done);
    }

    if (NULL != FriendlyName)
    {
        ntStatus = SetInterfaceFriendlyName(AudioSymbolicLinkName, FriendlyName);

        IF_FAILED_ACTION_JUMP(
            ntStatus,
            DPF(D_ERROR, ("CreateAudioInterface: SetInterfaceFriendlyName(...): failed, 0x%x", ntStatus)),
            Done);
    }

Done:
    if (!NT_SUCCESS(ntStatus))
    {
        RtlFreeUnicodeString(AudioSymbolicLinkName);
        RtlZeroMemory(AudioSymbolicLinkName, sizeof(UNICODE_STRING));
    }
    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::InstallSubdevice
(
    _In_opt_    PIRP                Irp,
    _In_        PWSTR               Name,
    _In_        PCWSTR              TemplateName,
    _In_        PCWSTR              FriendlyName,
    _In_        REFGUID             PortClassId,
    _In_        PFNCREATEMINIPORT   MiniportCreate,
    _In_        PENDPOINT_MINIPAIR  MiniportPair,
    _Out_       PUNKNOWN          * OutPortUnknown,
    _Out_       PUNKNOWN          * OutMiniportUnknown
)
/*++

Routine Description:

    Creates the audio interface, port and miniport for one filter and
    registers them as a PortCls subdevice.

--*/
{
    PAGED_CODE();
    DPF_ENTER(("[InstallSubDevice %S]", Name));

    ASSERT(Name != NULL);
    ASSERT(m_pDeviceObject != NULL);

    NTSTATUS                    ntStatus;
    PPORT                       port            = NULL;
    PUNKNOWN                    miniport        = NULL;
    PADAPTERCOMMON              adapterCommon   = PADAPTERCOMMON(this);
    UNICODE_STRING              symbolicLink    = { 0 };

    *OutPortUnknown = NULL;
    *OutMiniportUnknown = NULL;

    ntStatus = CreateAudioInterface(Name, TemplateName, FriendlyName, &symbolicLink);
    if (NT_SUCCESS(ntStatus))
    {
        RtlFreeUnicodeString(&symbolicLink);
        ntStatus = PcNewPort(&port, PortClassId);
    }

    if (NT_SUCCESS(ntStatus))
    {
        ntStatus = MiniportCreate(&miniport, PortClassId, NULL, POOL_FLAG_NON_PAGED, adapterCommon, NULL, MiniportPair);
    }

    if (NT_SUCCESS(ntStatus))
    {
#pragma warning(push)
        // IPort::Init's annotation requires a resource list; a virtual device has none.
#pragma warning(disable:6387)
        ntStatus = port->Init(m_pDeviceObject, Irp, miniport, adapterCommon, NULL);
#pragma warning (pop)
    }

    if (NT_SUCCESS(ntStatus))
    {
        ntStatus = PcRegisterSubdevice(m_pDeviceObject, Name, port);
    }

    if (NT_SUCCESS(ntStatus))
    {
        ntStatus = port->QueryInterface(IID_IUnknown, (PVOID *)OutPortUnknown);
    }

    if (NT_SUCCESS(ntStatus))
    {
        ntStatus = miniport->QueryInterface(IID_IUnknown, (PVOID *)OutMiniportUnknown);
    }

    SAFE_RELEASE(port);
    SAFE_RELEASE(miniport);

    return ntStatus;
} // InstallSubDevice

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::UnregisterSubdevice
(
    _In_opt_   PUNKNOWN     UnknownPort
)
/*++

Routine Description:

  Unregisters and releases the specified subdevice.

--*/
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::UnregisterSubdevice]"));

    ASSERT(m_pDeviceObject != NULL);

    NTSTATUS                ntStatus            = STATUS_SUCCESS;
    PUNREGISTERSUBDEVICE    unregisterSubdevice = NULL;

    if (NULL == UnknownPort)
    {
        return ntStatus;
    }

    //
    // Get the IUnregisterSubdevice interface.
    //
    ntStatus = UnknownPort->QueryInterface(
        IID_IUnregisterSubdevice,
        (PVOID *)&unregisterSubdevice);

    //
    // Unregister the port object.
    //
    if (NT_SUCCESS(ntStatus))
    {
        ntStatus = unregisterSubdevice->UnregisterSubdevice(
            m_pDeviceObject,
            UnknownPort);

        //
        // Release the IUnregisterSubdevice interface.
        //
        unregisterSubdevice->Release();
    }

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::ConnectTopologies
(
    _In_ PUNKNOWN                   UnknownTopology,
    _In_ PUNKNOWN                   UnknownWave,
    _In_ PHYSICALCONNECTIONTABLE*   PhysicalConnections,
    _In_ ULONG                      PhysicalConnectionCount
)
/*++

Routine Description:

  Connects the bridge pins between the wave and mixer topologies.

--*/
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::ConnectTopologies]"));

    ASSERT(m_pDeviceObject != NULL);

    NTSTATUS        ntStatus            = STATUS_SUCCESS;

    //
    // register wave <=> topology connections
    // This will connect bridge pins of wave and topology
    // miniports.
    //
    for (ULONG i = 0; i < PhysicalConnectionCount && NT_SUCCESS(ntStatus); i++)
    {

        switch(PhysicalConnections[i].eType)
        {
            case CONNECTIONTYPE_TOPOLOGY_OUTPUT:
                ntStatus =
                    PcRegisterPhysicalConnection
                    (
                        m_pDeviceObject,
                        UnknownTopology,
                        PhysicalConnections[i].ulTopology,
                        UnknownWave,
                        PhysicalConnections[i].ulWave
                    );
                if (!NT_SUCCESS(ntStatus))
                {
                    DPF(D_TERSE, ("ConnectTopologies: PcRegisterPhysicalConnection(capture) failed, 0x%x", ntStatus));
                }
                break;
            case CONNECTIONTYPE_WAVE_OUTPUT:
                ntStatus =
                    PcRegisterPhysicalConnection
                    (
                        m_pDeviceObject,
                        UnknownWave,
                        PhysicalConnections[i].ulWave,
                        UnknownTopology,
                        PhysicalConnections[i].ulTopology
                    );
                if (!NT_SUCCESS(ntStatus))
                {
                    DPF(D_TERSE, ("ConnectTopologies: PcRegisterPhysicalConnection(render) failed, 0x%x", ntStatus));
                }
                break;
        }
    }

    //
    // Cleanup in case of error.
    //
    if (!NT_SUCCESS(ntStatus))
    {
        // disconnect all connections on error, ignore error code because not all
        // connections may have been made
        DisconnectTopologies(UnknownTopology, UnknownWave, PhysicalConnections, PhysicalConnectionCount);
    }

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::DisconnectTopologies
(
    _In_ PUNKNOWN                   UnknownTopology,
    _In_ PUNKNOWN                   UnknownWave,
    _In_ PHYSICALCONNECTIONTABLE*   PhysicalConnections,
    _In_ ULONG                      PhysicalConnectionCount
)
/*++

Routine Description:

  Disconnects the bridge pins between the wave and mixer topologies.

--*/
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::DisconnectTopologies]"));

    ASSERT(m_pDeviceObject != NULL);

    NTSTATUS                        ntStatus                        = STATUS_SUCCESS;
    NTSTATUS                        ntStatus2                       = STATUS_SUCCESS;
    PUNREGISTERPHYSICALCONNECTION   unregisterPhysicalConnection    = NULL;

    //
    // Get the IUnregisterPhysicalConnection interface
    //
    ntStatus = UnknownTopology->QueryInterface(
        IID_IUnregisterPhysicalConnection,
        (PVOID *)&unregisterPhysicalConnection);

    if (NT_SUCCESS(ntStatus))
    {
        for (ULONG i = 0; i < PhysicalConnectionCount; i++)
        {
            ntStatus2 = STATUS_SUCCESS;

            switch(PhysicalConnections[i].eType)
            {
                case CONNECTIONTYPE_TOPOLOGY_OUTPUT:
                    ntStatus2 =
                        unregisterPhysicalConnection->UnregisterPhysicalConnection(
                            m_pDeviceObject,
                            UnknownTopology,
                            PhysicalConnections[i].ulTopology,
                            UnknownWave,
                            PhysicalConnections[i].ulWave
                        );
                    break;
                case CONNECTIONTYPE_WAVE_OUTPUT:
                    ntStatus2 =
                        unregisterPhysicalConnection->UnregisterPhysicalConnection(
                            m_pDeviceObject,
                            UnknownWave,
                            PhysicalConnections[i].ulWave,
                            UnknownTopology,
                            PhysicalConnections[i].ulTopology
                        );
                    break;
            }

            if (!NT_SUCCESS(ntStatus2))
            {
                DPF(D_TERSE, ("DisconnectTopologies: UnregisterPhysicalConnection failed, 0x%x", ntStatus2));
            }

            // cache and return the first error encountered, as it's likely the most relevent
            if (NT_SUCCESS(ntStatus))
            {
                ntStatus = ntStatus2;
            }
        }
    }

    //
    // Release the IUnregisterPhysicalConnection interface.
    //
    SAFE_RELEASE(unregisterPhysicalConnection);

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::CacheSubdevice
(
    _In_ PWSTR Name,
    _In_ PUNKNOWN UnknownPort,
    _In_ PUNKNOWN UnknownMiniport
)
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::CacheSubdevice]"));

    // add the item with this name/interface to the list
    NTSTATUS         ntStatus       = STATUS_SUCCESS;
    MINIPAIR_UNKNOWN *pNewSubdevice = NULL;

    pNewSubdevice = new(POOL_FLAG_NON_PAGED, MINADAPTER_POOLTAG) MINIPAIR_UNKNOWN;

    if (!pNewSubdevice)
    {
        DPF(D_TERSE, ("Insufficient memory to cache subdevice"));
        ntStatus = STATUS_INSUFFICIENT_RESOURCES;
    }

    if (NT_SUCCESS(ntStatus))
    {
        memset(pNewSubdevice, 0, sizeof(MINIPAIR_UNKNOWN));

        ntStatus = RtlStringCchCopyW(pNewSubdevice->Name, SIZEOF_ARRAY(pNewSubdevice->Name), Name);
    }

    if (NT_SUCCESS(ntStatus))
    {
        pNewSubdevice->PortInterface = UnknownPort;
        pNewSubdevice->PortInterface->AddRef();

        pNewSubdevice->MiniportInterface = UnknownMiniport;
        pNewSubdevice->MiniportInterface->AddRef();

        // Cache the IAdapterPowerManagement interface if the filter exposes
        // one. If this fails, the filter doesn't do power management.
        if (!NT_SUCCESS(UnknownMiniport->QueryInterface(IID_IAdapterPowerManagement, (PVOID *)&(pNewSubdevice->PowerInterface))))
        {
            pNewSubdevice->PowerInterface = NULL;
        }

        InsertTailList(&m_SubdeviceCache, &pNewSubdevice->ListEntry);
    }

    if (!NT_SUCCESS(ntStatus))
    {
        if (pNewSubdevice)
        {
            delete pNewSubdevice;
        }
    }

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::RemoveCachedSubdevice
(
    _In_ PWSTR Name
)
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::RemoveCachedSubdevice]"));

    // search list, remove the entry from the list

    PLIST_ENTRY le = NULL;
    BOOL bRemoved = FALSE;

    for (le = m_SubdeviceCache.Flink; le != &m_SubdeviceCache && !bRemoved; le = le->Flink)
    {
        MINIPAIR_UNKNOWN *pRecord = CONTAINING_RECORD(le, MINIPAIR_UNKNOWN, ListEntry);

        if (0 == wcscmp(Name, pRecord->Name))
        {
            SAFE_RELEASE(pRecord->PortInterface);
            SAFE_RELEASE(pRecord->MiniportInterface);
            SAFE_RELEASE(pRecord->PowerInterface);
            memset(pRecord->Name, 0, sizeof(pRecord->Name));
            RemoveEntryList(le);
            bRemoved = TRUE;
            delete pRecord;
            break;
        }
    }

    return bRemoved?STATUS_SUCCESS:STATUS_OBJECT_NAME_NOT_FOUND;
}

#pragma code_seg("PAGE")
VOID
CAdapterCommon::EmptySubdeviceCache()
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::EmptySubdeviceCache]"));

    while (!IsListEmpty(&m_SubdeviceCache))
    {
        PLIST_ENTRY le = RemoveHeadList(&m_SubdeviceCache);
        MINIPAIR_UNKNOWN *pRecord = CONTAINING_RECORD(le, MINIPAIR_UNKNOWN, ListEntry);

        SAFE_RELEASE(pRecord->PortInterface);
        SAFE_RELEASE(pRecord->MiniportInterface);
        SAFE_RELEASE(pRecord->PowerInterface);
        memset(pRecord->Name, 0, sizeof(pRecord->Name));

        delete pRecord;
    }
}

#pragma code_seg("PAGE")
VOID
CAdapterCommon::Cleanup()
/*++

Routine Description:

  Drops the cached port and miniport references. Wave miniports hold a
  reference to this object, so this breaks the cycle on stop/removal.

--*/
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::Cleanup]"));
    EmptySubdeviceCache();
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CAdapterCommon::InstallEndpointFilters
(
    _In_opt_    PIRP                Irp,
    _In_        PENDPOINT_MINIPAIR  MiniportPair
)
{
    PAGED_CODE();
    DPF_ENTER(("[CAdapterCommon::InstallEndpointFilters]"));

    NTSTATUS            ntStatus;
    PUNKNOWN            unknownTopology     = NULL;
    PUNKNOWN            unknownWave         = NULL;
    PUNKNOWN            unknownMiniTopo     = NULL;
    PUNKNOWN            unknownMiniWave     = NULL;

    ntStatus = InstallSubdevice(Irp,
                                MiniportPair->TopoName,
                                MiniportPair->TemplateTopoName,
                                MiniportPair->FriendlyName,
                                CLSID_PortTopology,
                                MiniportPair->TopoCreateCallback,
                                MiniportPair,
                                &unknownTopology,
                                &unknownMiniTopo);
    if (NT_SUCCESS(ntStatus))
    {
        ntStatus = CacheSubdevice(MiniportPair->TopoName, unknownTopology, unknownMiniTopo);
    }
    IF_FAILED_JUMP(ntStatus, Done);

    ntStatus = InstallSubdevice(Irp,
                                MiniportPair->WaveName,
                                MiniportPair->TemplateWaveName,
                                MiniportPair->FriendlyName,
                                CLSID_PortWaveRT,
                                MiniportPair->WaveCreateCallback,
                                MiniportPair,
                                &unknownWave,
                                &unknownMiniWave);
    if (NT_SUCCESS(ntStatus))
    {
        ntStatus = CacheSubdevice(MiniportPair->WaveName, unknownWave, unknownMiniWave);
    }
    IF_FAILED_JUMP(ntStatus, Done);

    // Connect the bridge pins of the wave and topology filters.
    ntStatus = ConnectTopologies(
        unknownTopology,
        unknownWave,
        MiniportPair->PhysicalConnections,
        MiniportPair->PhysicalConnectionCount);

Done:
    if (!NT_SUCCESS(ntStatus))
    {
        if (unknownTopology != NULL)
        {
            UnregisterSubdevice(unknownTopology);
            RemoveCachedSubdevice(MiniportPair->TopoName);
        }

        if (unknownWave != NULL)
        {
            UnregisterSubdevice(unknownWave);
            RemoveCachedSubdevice(MiniportPair->WaveName);
        }
    }

    SAFE_RELEASE(unknownMiniTopo);
    SAFE_RELEASE(unknownTopology);
    SAFE_RELEASE(unknownMiniWave);
    SAFE_RELEASE(unknownWave);

    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
CopyRegistryValues(HANDLE _hSourceKey, HANDLE _hDestinationKey)
/*++

Routine Description:

  This method copies the registry values in _hSourceKey to _hDestinationKey.

Return Value:

  NT status code.

--*/
{
    NTSTATUS                    ntStatus = STATUS_SUCCESS;
    PKEY_VALUE_FULL_INFORMATION kvFullInfo = NULL;
    ULONG                       ulFullInfoLength = 0;
    ULONG                       ulFullInfoResultLength = 0;
    PWSTR                       pwstrKeyValueName = NULL;
    UNICODE_STRING              strKeyValueName;
    PAGED_CODE();
    // Allocate the KEY_VALUE_FULL_INFORMATION structure
    ulFullInfoLength = sizeof(KEY_VALUE_FULL_INFORMATION) + MAX_DEVICE_REG_KEY_LENGTH;
    kvFullInfo = (PKEY_VALUE_FULL_INFORMATION)ExAllocatePool2(POOL_FLAG_NON_PAGED, ulFullInfoLength, MINADAPTER_POOLTAG);
    IF_TRUE_ACTION_JUMP(kvFullInfo == NULL, ntStatus = STATUS_INSUFFICIENT_RESOURCES, Exit);

    // Iterate over each value and copy it to the destination
    for (UINT i = 0; NT_SUCCESS(ntStatus); i++)
    {
        // Enumerate the next value
        ntStatus = ZwEnumerateValueKey(_hSourceKey, i, KeyValueFullInformation, kvFullInfo, ulFullInfoLength, &ulFullInfoResultLength);

        // Jump out of this loop if there are no more values
        IF_TRUE_ACTION_JUMP(ntStatus == STATUS_NO_MORE_ENTRIES, ntStatus = STATUS_SUCCESS, Exit);

        // Handle incorrect buffer size
        if (ntStatus == STATUS_BUFFER_TOO_SMALL || ntStatus == STATUS_BUFFER_OVERFLOW)
        {
            // Free and re-allocate the KEY_VALUE_FULL_INFORMATION structure with the correct size
            ExFreePoolWithTag(kvFullInfo, MINADAPTER_POOLTAG);

            ulFullInfoLength = ulFullInfoResultLength;

            kvFullInfo = (PKEY_VALUE_FULL_INFORMATION)ExAllocatePool2(POOL_FLAG_NON_PAGED, ulFullInfoLength, MINADAPTER_POOLTAG);
            IF_TRUE_ACTION_JUMP(kvFullInfo == NULL, ntStatus = STATUS_INSUFFICIENT_RESOURCES, loop_exit);

            // Try to enumerate the current value again
            ntStatus = ZwEnumerateValueKey(_hSourceKey, i, KeyValueFullInformation, kvFullInfo, ulFullInfoLength, &ulFullInfoResultLength);

            // Jump out of this loop if there are no more values
            IF_TRUE_ACTION_JUMP(ntStatus == STATUS_NO_MORE_ENTRIES, ntStatus = STATUS_SUCCESS, Exit);
            IF_FAILED_JUMP(ntStatus, loop_exit);
        }
        else
        {
            IF_FAILED_JUMP(ntStatus, loop_exit);
        }

        // Allocate the key value name string
        pwstrKeyValueName = (PWSTR)ExAllocatePool2(POOL_FLAG_NON_PAGED, kvFullInfo->NameLength + sizeof(WCHAR)*2, MINADAPTER_POOLTAG);
        IF_TRUE_ACTION_JUMP(pwstrKeyValueName == NULL, ntStatus = STATUS_INSUFFICIENT_RESOURCES, loop_exit);

        // Copy the key value name from the full information struct
        RtlStringCbCopyNW(pwstrKeyValueName, kvFullInfo->NameLength + sizeof(WCHAR)*2, kvFullInfo->Name, kvFullInfo->NameLength);

        // Make sure the string is null terminated
        pwstrKeyValueName[(kvFullInfo->NameLength) / sizeof(WCHAR)] = 0;

        // Copy the key value name string to a UNICODE string
        RtlInitUnicodeString(&strKeyValueName, pwstrKeyValueName);

        // Write the key value from the source into the destination
        ntStatus = ZwSetValueKey(_hDestinationKey, &strKeyValueName, 0, kvFullInfo->Type, (PVOID)((PUCHAR)kvFullInfo + kvFullInfo->DataOffset), kvFullInfo->DataLength);
        IF_FAILED_JUMP(ntStatus, loop_exit);

    loop_exit:
        // Free the key value name string
        if (pwstrKeyValueName)
        {
            ExFreePoolWithTag(pwstrKeyValueName, MINADAPTER_POOLTAG);
            pwstrKeyValueName = NULL;
        }

        // Bail if anything failed
        IF_FAILED_JUMP(ntStatus, Exit);
    }

Exit:
    // Free the KEY_VALUE_FULL_INFORMATION structure
    if (kvFullInfo)
    {
        ExFreePoolWithTag(kvFullInfo, MINADAPTER_POOLTAG);
    }

    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
CopyRegistryKey(HANDLE _hSourceKey, HANDLE _hDestinationKey, BOOL _bOverwrite = FALSE)
/*++

Routine Description:

  This method recursively copies the registry values in _hSourceKey to _hDestinationKey.
  Set _bOverwrite to indicate whether the first level values are copied or not.

Return Value:

  NT status code.

--*/
{
    NTSTATUS                ntStatus = STATUS_UNSUCCESSFUL;
    PKEY_BASIC_INFORMATION  kBasicInfo = NULL;
    ULONG                   ulBasicInfoLength = 0;
    ULONG                   ulBasicInfoResultLength = 0;
    ULONG                   ulDisposition = 0;
    PWSTR                   pwstrKeyName = NULL;
    UNICODE_STRING          strKeyName;
    OBJECT_ATTRIBUTES       hCurrentSourceKeyAttributes;
    OBJECT_ATTRIBUTES       hNewDestinationKeyAttributes;
    HANDLE                  hCurrentSourceKey = NULL;
    HANDLE                  hNewDestinationKey = NULL;
    PAGED_CODE();
    // Validate parameters
    IF_TRUE_ACTION_JUMP(_hSourceKey == nullptr, ntStatus = STATUS_INVALID_PARAMETER, Exit);
    IF_TRUE_ACTION_JUMP(_hDestinationKey == nullptr, ntStatus = STATUS_INVALID_PARAMETER, Exit);

    // Allocate the KEY_BASIC_INFORMATION structure
    ulBasicInfoLength = sizeof(KEY_BASIC_INFORMATION) + MAX_DEVICE_REG_KEY_LENGTH;
    kBasicInfo = (PKEY_BASIC_INFORMATION)ExAllocatePool2(POOL_FLAG_NON_PAGED, ulBasicInfoLength, MINADAPTER_POOLTAG);
    IF_TRUE_ACTION_JUMP(kBasicInfo == NULL, ntStatus = STATUS_INSUFFICIENT_RESOURCES, Exit);

    ntStatus = STATUS_SUCCESS;
    // Iterate over each key and copy it
    for (UINT i = 0; NT_SUCCESS(ntStatus); i++)
    {
        // Enumerate the next key
        ntStatus = ZwEnumerateKey(_hSourceKey, i, KeyBasicInformation, kBasicInfo, ulBasicInfoLength, &ulBasicInfoResultLength);

        // Jump out of this loop if there are no more keys
        IF_TRUE_ACTION_JUMP(ntStatus == STATUS_NO_MORE_ENTRIES, ntStatus = STATUS_SUCCESS, copy_values);

        // Handle incorrect buffer size
        if (ntStatus == STATUS_BUFFER_TOO_SMALL || ntStatus == STATUS_BUFFER_OVERFLOW)
        {
            // Free and re-allocate the KEY_BASIC_INFORMATION structure with the correct size.
            ExFreePoolWithTag(kBasicInfo, MINADAPTER_POOLTAG);
            ulBasicInfoLength = ulBasicInfoResultLength;
            kBasicInfo = (PKEY_BASIC_INFORMATION)ExAllocatePool2(POOL_FLAG_NON_PAGED, ulBasicInfoLength, MINADAPTER_POOLTAG);
            IF_TRUE_ACTION_JUMP(kBasicInfo == NULL, ntStatus = STATUS_INSUFFICIENT_RESOURCES, loop_exit);

            // Try to enumerate the current key again.
            ntStatus = ZwEnumerateKey(_hSourceKey, i, KeyBasicInformation, kBasicInfo, ulBasicInfoLength, &ulBasicInfoResultLength);

            // Jump out of this loop if there are no more keys
            IF_TRUE_ACTION_JUMP(ntStatus == STATUS_NO_MORE_ENTRIES, ntStatus = STATUS_SUCCESS, copy_values);
            IF_FAILED_JUMP(ntStatus, loop_exit);
        }
        else
        {
            IF_FAILED_JUMP(ntStatus, loop_exit);
        }

        // Allocate the key name string
        pwstrKeyName = (PWSTR)ExAllocatePool2(POOL_FLAG_NON_PAGED, kBasicInfo->NameLength + sizeof(WCHAR), MINADAPTER_POOLTAG);
        IF_TRUE_ACTION_JUMP(pwstrKeyName == NULL, ntStatus = STATUS_INSUFFICIENT_RESOURCES, loop_exit);

        // Copy the key name from the basic information struct
        RtlStringCbCopyNW(pwstrKeyName, kBasicInfo->NameLength + sizeof(WCHAR), kBasicInfo->Name, kBasicInfo->NameLength);

        // Make sure the string is null terminated
        pwstrKeyName[(kBasicInfo->NameLength) / sizeof(WCHAR)] = 0;

        // Copy the key name string to a UNICODE string
        RtlInitUnicodeString(&strKeyName, pwstrKeyName);

        // Initialize attributes to open the currently enumerated source key
        InitializeObjectAttributes(&hCurrentSourceKeyAttributes, &strKeyName, OBJ_CASE_INSENSITIVE | OBJ_KERNEL_HANDLE, _hSourceKey, NULL);

        // Open the currently enumerated source key
        ntStatus = ZwOpenKey(&hCurrentSourceKey, KEY_READ, &hCurrentSourceKeyAttributes);
        IF_FAILED_ACTION_JUMP(ntStatus, hCurrentSourceKey = NULL, loop_exit);

        // Initialize attributes to create the new destination key
        InitializeObjectAttributes(&hNewDestinationKeyAttributes, &strKeyName, OBJ_KERNEL_HANDLE, _hDestinationKey, NULL);

        // Create the key at the destination
        ntStatus = ZwCreateKey(&hNewDestinationKey, KEY_WRITE, &hNewDestinationKeyAttributes, 0, NULL, REG_OPTION_NON_VOLATILE, &ulDisposition);
        IF_FAILED_ACTION_JUMP(ntStatus, hNewDestinationKey = NULL, loop_exit);

        // Now copy the contents of the currently enumerated key to the destination
        ntStatus = CopyRegistryKey(hCurrentSourceKey, hNewDestinationKey, TRUE);
        IF_FAILED_JUMP(ntStatus, loop_exit);

    loop_exit:
        // Free the key name string
        if (pwstrKeyName)
        {
            ExFreePoolWithTag(pwstrKeyName, MINADAPTER_POOLTAG);
            pwstrKeyName = NULL;
        }

        // Close the current source key
        if (hCurrentSourceKey)
        {
            ZwClose(hCurrentSourceKey);
            hCurrentSourceKey = NULL;
        }

        // Close the new destination key
        if (hNewDestinationKey)
        {
            ZwClose(hNewDestinationKey);
            hNewDestinationKey = NULL;
        }

        // Bail if anything failed
        IF_FAILED_JUMP(ntStatus, Exit);
    }

copy_values:
    // Copy the values
    if (_bOverwrite)
    {
        ntStatus = CopyRegistryValues(_hSourceKey, _hDestinationKey);
        IF_FAILED_JUMP(ntStatus, Exit);
    }

Exit:
    // Free the basic information structure
    if (kBasicInfo)
    {
        ExFreePoolWithTag(kBasicInfo, MINADAPTER_POOLTAG);
    }
    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS CAdapterCommon::MigrateDeviceInterfaceTemplateParameters
(
    _In_ PUNICODE_STRING    SymbolicLinkName,
    _In_opt_ PCWSTR         TemplateReferenceString
)
/*++

Routine Description:

  Copies everything the INF wrote for the template interface (for example
  "WaveCableRender") to the runtime interface ("WaveCableRender03"). Unlike
  the sample, the first level values are copied too: the runtime interfaces
  have no INF section of their own, so they need the template's CLSID and
  FriendlyName values as well as its EP\0 subkey. The per-cable friendly
  name is written afterwards.

Return Value:

  NT status code.

--*/

{
    NTSTATUS            ntStatus = STATUS_SUCCESS;
    HANDLE              hDeviceInterfaceParametersKey(NULL);
    HANDLE              hTemplateDeviceInterfaceParametersKey(NULL);
    UNICODE_STRING      TemplateSymbolicLinkName;
    UNICODE_STRING      referenceString;

    PAGED_CODE();

    RtlInitUnicodeString(&TemplateSymbolicLinkName, NULL);
    RtlInitUnicodeString(&referenceString, TemplateReferenceString);

    //
    // Register an audio interface if not already present for the template interface, so we can access
    // the registry path. If it's already registered, this simply returns the symbolic link name.
    // No need to unregister it (there is no mechanism to), and we'll never make it active.
    //
    ntStatus = IoRegisterDeviceInterface(
        GetPhysicalDeviceObject(),
        &KSCATEGORY_AUDIO,
        &referenceString,
        &TemplateSymbolicLinkName);
    IF_FAILED_JUMP(ntStatus, Exit);

    // Open the template device interface's registry key path
    ntStatus = IoOpenDeviceInterfaceRegistryKey(&TemplateSymbolicLinkName, GENERIC_READ, &hTemplateDeviceInterfaceParametersKey);
    IF_FAILED_JUMP(ntStatus, Exit);

    // Open the new device interface's registry key path that we plan to activate
    ntStatus = IoOpenDeviceInterfaceRegistryKey(SymbolicLinkName, GENERIC_WRITE, &hDeviceInterfaceParametersKey);
    IF_FAILED_JUMP(ntStatus, Exit);

    // Copy the template device parameters key to the device interface key
    ntStatus = CopyRegistryKey(hTemplateDeviceInterfaceParametersKey, hDeviceInterfaceParametersKey, TRUE);
    IF_FAILED_JUMP(ntStatus, Exit);

Exit:
    RtlFreeUnicodeString(&TemplateSymbolicLinkName);

    if (hTemplateDeviceInterfaceParametersKey)
    {
        ZwClose(hTemplateDeviceInterfaceParametersKey);
    }

    if (hDeviceInterfaceParametersKey)
    {
        ZwClose(hDeviceInterfaceParametersKey);
    }

    return ntStatus;
}

#pragma code_seg()
