/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    mintopo.cpp

Abstract:

    Implementation of topology miniport.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Main/mintopo.cpp, Source/Main/basetopo.cpp and
// Source/Filters/speakertopo.cpp). Modifications Copyright (c) 2026 Audio
// Program contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#include "definitions.h"
#include "endpoints.h"
#include "mintopo.h"

//=============================================================================
// Jack descriptions. Only the physical-side pin of each topology filter
// reports one; there is no presence detection, so the jack is always connected.
//=============================================================================
static
KSJACK_DESCRIPTION CableRenderJackDescBridge =
{
    KSAUDIO_SPEAKER_STEREO,
    JACKDESC_RGB(0xB3,0xC9,0x8C),
    eConnTypeUnknown,
    eGeoLocFront,
    eGenLocPrimaryBox,
    ePortConnIntegratedDevice,
    TRUE
};

static
PKSJACK_DESCRIPTION CableRenderJackDescriptions[] =
{
    NULL,
    &CableRenderJackDescBridge
};

static
KSJACK_DESCRIPTION CableCaptureJackDescBridge =
{
    KSAUDIO_SPEAKER_STEREO,
    JACKDESC_RGB(0xFC,0xA5,0xB0),
    eConnTypeUnknown,
    eGeoLocFront,
    eGenLocPrimaryBox,
    ePortConnIntegratedDevice,
    TRUE
};

static
PKSJACK_DESCRIPTION CableCaptureJackDescriptions[] =
{
    &CableCaptureJackDescBridge,
    NULL
};

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CreateMiniportTopologyCable
(
    _Out_           PUNKNOWN *                              Unknown,
    _In_            REFCLSID,
    _In_opt_        PUNKNOWN                                UnknownOuter,
    _In_            POOL_FLAGS                              PoolFlags,
    _In_            PUNKNOWN                                UnknownAdapter,
    _In_opt_        PVOID                                   DeviceContext,
    _In_            PENDPOINT_MINIPAIR                      MiniportPair
)
/*++

Routine Description:

    Creates a new topology miniport.

--*/
{
    PAGED_CODE();

    UNREFERENCED_PARAMETER(UnknownAdapter);
    UNREFERENCED_PARAMETER(DeviceContext);

    ASSERT(Unknown);
    ASSERT(MiniportPair);

    CMiniportTopology *obj =
        new (PoolFlags, MINTOPORT_POOLTAG)
            CMiniportTopology( UnknownOuter,
                               MiniportPair->TopoDescriptor,
                               MiniportPair->DeviceType );
    if (NULL == obj)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    obj->AddRef();
    *Unknown = reinterpret_cast<IUnknown*>(obj);

    return STATUS_SUCCESS;
} // CreateMiniportTopologyCable

//=============================================================================
#pragma code_seg("PAGE")
CMiniportTopology::~CMiniportTopology
(
    void
)
{
    PAGED_CODE();

    DPF_ENTER(("[CMiniportTopology::~CMiniportTopology]"));
} // ~CMiniportTopology

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(NTSTATUS)
CMiniportTopology::DataRangeIntersection
(
    _In_        ULONG                   PinId,
    _In_        PKSDATARANGE            ClientDataRange,
    _In_        PKSDATARANGE            MyDataRange,
    _In_        ULONG                   OutputBufferLength,
    _Out_writes_bytes_to_opt_(OutputBufferLength, *ResultantFormatLength)
                PVOID                   ResultantFormat,
    _Out_       PULONG                  ResultantFormatLength
)
/*++

Routine Description:

  The topology filter only has analog bridge pins; let the class handler
  do the intersection.

--*/
{
    UNREFERENCED_PARAMETER(PinId);
    UNREFERENCED_PARAMETER(ClientDataRange);
    UNREFERENCED_PARAMETER(MyDataRange);
    UNREFERENCED_PARAMETER(OutputBufferLength);
    UNREFERENCED_PARAMETER(ResultantFormat);
    UNREFERENCED_PARAMETER(ResultantFormatLength);

    PAGED_CODE();

    return STATUS_NOT_IMPLEMENTED;
} // DataRangeIntersection

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(NTSTATUS)
CMiniportTopology::GetDescription
(
    _Out_ PPCFILTER_DESCRIPTOR *  OutFilterDescriptor
)
{
    PAGED_CODE();

    ASSERT(OutFilterDescriptor);

    *OutFilterDescriptor = m_FilterDescriptor;

    return STATUS_SUCCESS;
} // GetDescription

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(NTSTATUS)
CMiniportTopology::Init
(
    _In_ PUNKNOWN                 UnknownAdapter,
    _In_ PRESOURCELIST            ResourceList,
    _In_ PPORTTOPOLOGY            Port_
)
{
    UNREFERENCED_PARAMETER(UnknownAdapter);
    UNREFERENCED_PARAMETER(ResourceList);
    UNREFERENCED_PARAMETER(Port_);

    PAGED_CODE();

    DPF_ENTER(("[CMiniportTopology::Init]"));

    return STATUS_SUCCESS;
} // Init

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP
CMiniportTopology::NonDelegatingQueryInterface
(
    _In_ REFIID                  Interface,
    _COM_Outptr_ PVOID      * Object
)
{
    PAGED_CODE();

    ASSERT(Object);

    if (IsEqualGUIDAligned(Interface, IID_IUnknown))
    {
        *Object = PVOID(PUNKNOWN(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniport))
    {
        *Object = PVOID(PMINIPORT(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportTopology))
    {
        *Object = PVOID(PMINIPORTTOPOLOGY(this));
    }
    else
    {
        *Object = NULL;
    }

    if (*Object)
    {
        // We reference the interface for the caller.
        PUNKNOWN(*Object)->AddRef();
        return(STATUS_SUCCESS);
    }

    return(STATUS_INVALID_PARAMETER);
} // NonDelegatingQueryInterface

//=============================================================================
#pragma code_seg("PAGE")
ULONG
CMiniportTopology::GetJackDescriptions
(
    _Outptr_result_buffer_(return) PKSJACK_DESCRIPTION **JackDescriptions
)
{
    PAGED_CODE();

    if (m_DeviceType == eCableRenderDevice)
    {
        *JackDescriptions = CableRenderJackDescriptions;
        return ARRAYSIZE(CableRenderJackDescriptions);
    }

    *JackDescriptions = CableCaptureJackDescriptions;
    return ARRAYSIZE(CableCaptureJackDescriptions);
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CMiniportTopology::PropertyHandlerJackDescription
(
    _In_        PPCPROPERTY_REQUEST                      PropertyRequest
)
/*++

Routine Description:

  Handles ( KSPROPSETID_Jack, KSPROPERTY_JACK_DESCRIPTION )

--*/
{
    PAGED_CODE();

    ASSERT(PropertyRequest);

    DPF_ENTER(("[PropertyHandlerJackDescription]"));

    NTSTATUS                ntStatus = STATUS_INVALID_DEVICE_REQUEST;
    ULONG                   nPinId = (ULONG)-1;
    PKSJACK_DESCRIPTION *   JackDescriptions = NULL;
    ULONG                   cJackDescriptions = GetJackDescriptions(&JackDescriptions);

    if (PropertyRequest->InstanceSize >= sizeof(ULONG))
    {
        nPinId = *(PULONG(PropertyRequest->Instance));

        if ((nPinId < cJackDescriptions) && (JackDescriptions[nPinId] != NULL))
        {
            if (PropertyRequest->Verb & KSPROPERTY_TYPE_BASICSUPPORT)
            {
                ntStatus =
                    PropertyHandler_BasicSupport
                    (
                        PropertyRequest,
                        KSPROPERTY_TYPE_BASICSUPPORT | KSPROPERTY_TYPE_GET,
                        VT_ILLEGAL
                    );
            }
            else
            {
                ULONG cbNeeded = sizeof(KSMULTIPLE_ITEM) + sizeof(KSJACK_DESCRIPTION);

                if (PropertyRequest->ValueSize == 0)
                {
                    PropertyRequest->ValueSize = cbNeeded;
                    ntStatus = STATUS_BUFFER_OVERFLOW;
                }
                else if (PropertyRequest->ValueSize < cbNeeded)
                {
                    ntStatus = STATUS_BUFFER_TOO_SMALL;
                }
                else
                {
                    if (PropertyRequest->Verb & KSPROPERTY_TYPE_GET)
                    {
                        PKSMULTIPLE_ITEM pMI = (PKSMULTIPLE_ITEM)PropertyRequest->Value;
                        PKSJACK_DESCRIPTION pDesc = (PKSJACK_DESCRIPTION)(pMI+1);

                        pMI->Size = cbNeeded;
                        pMI->Count = 1;

                        RtlCopyMemory(pDesc, JackDescriptions[nPinId], sizeof(KSJACK_DESCRIPTION));
                        PropertyRequest->ValueSize = cbNeeded;
                        ntStatus = STATUS_SUCCESS;
                    }
                }
            }
        }
    }

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
CMiniportTopology::PropertyHandlerJackDescription2
(
    _In_        PPCPROPERTY_REQUEST                      PropertyRequest
)
/*++

Routine Description:

  Handles ( KSPROPSETID_Jack, KSPROPERTY_JACK_DESCRIPTION2 )

--*/
{
    PAGED_CODE();

    ASSERT(PropertyRequest);

    DPF_ENTER(("[PropertyHandlerJackDescription2]"));

    NTSTATUS                ntStatus = STATUS_INVALID_DEVICE_REQUEST;
    ULONG                   nPinId = (ULONG)-1;
    PKSJACK_DESCRIPTION *   JackDescriptions = NULL;
    ULONG                   cJackDescriptions = GetJackDescriptions(&JackDescriptions);

    if (PropertyRequest->InstanceSize >= sizeof(ULONG))
    {
        nPinId = *(PULONG(PropertyRequest->Instance));

        if ((nPinId < cJackDescriptions) && (JackDescriptions[nPinId] != NULL))
        {
            if (PropertyRequest->Verb & KSPROPERTY_TYPE_BASICSUPPORT)
            {
                ntStatus =
                    PropertyHandler_BasicSupport
                    (
                        PropertyRequest,
                        KSPROPERTY_TYPE_BASICSUPPORT | KSPROPERTY_TYPE_GET,
                        VT_ILLEGAL
                    );
            }
            else
            {
                ULONG cbNeeded = sizeof(KSMULTIPLE_ITEM) + sizeof(KSJACK_DESCRIPTION2);

                if (PropertyRequest->ValueSize == 0)
                {
                    PropertyRequest->ValueSize = cbNeeded;
                    ntStatus = STATUS_BUFFER_OVERFLOW;
                }
                else if (PropertyRequest->ValueSize < cbNeeded)
                {
                    ntStatus = STATUS_BUFFER_TOO_SMALL;
                }
                else
                {
                    if (PropertyRequest->Verb & KSPROPERTY_TYPE_GET)
                    {
                        PKSMULTIPLE_ITEM pMI = (PKSMULTIPLE_ITEM)PropertyRequest->Value;
                        PKSJACK_DESCRIPTION2 pDesc = (PKSJACK_DESCRIPTION2)(pMI+1);

                        pMI->Size = cbNeeded;
                        pMI->Count = 1;

                        RtlZeroMemory(pDesc, sizeof(KSJACK_DESCRIPTION2));

                        //
                        // No presence detection and no dynamic format change:
                        // JackCapabilities stays 0, so IsConnected == TRUE means
                        // "always present".
                        //
                        pDesc->DeviceStateInfo = 0;
                        pDesc->JackCapabilities = 0;

                        PropertyRequest->ValueSize = cbNeeded;
                        ntStatus = STATUS_SUCCESS;
                    }
                }
            }
        }
    }

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
PropertyHandler_CableTopoFilter
(
    _In_ PPCPROPERTY_REQUEST      PropertyRequest
)
/*++

Routine Description:

  Redirects property request to miniport object

--*/
{
    PAGED_CODE();

    ASSERT(PropertyRequest);

    DPF_ENTER(("[PropertyHandler_CableTopoFilter]"));

    // PropertryRequest structure is filled by portcls.
    // MajorTarget is a pointer to miniport object for miniports.
    //
    NTSTATUS            ntStatus = STATUS_INVALID_DEVICE_REQUEST;
    PCMiniportTopology  pMiniport = (PCMiniportTopology)PropertyRequest->MajorTarget;

    if (IsEqualGUIDAligned(*PropertyRequest->PropertyItem->Set, KSPROPSETID_Jack))
    {
        if (PropertyRequest->PropertyItem->Id == KSPROPERTY_JACK_DESCRIPTION)
        {
            ntStatus = pMiniport->PropertyHandlerJackDescription(PropertyRequest);
        }
        else if (PropertyRequest->PropertyItem->Id == KSPROPERTY_JACK_DESCRIPTION2)
        {
            ntStatus = pMiniport->PropertyHandlerJackDescription2(PropertyRequest);
        }
    }

    return ntStatus;
} // PropertyHandler_CableTopoFilter

#pragma code_seg()
