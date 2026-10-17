/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    mintopo.h

Abstract:

    Declaration of topology miniport.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Inc/mintopo.h and Source/Inc/basetopo.h). Modifications Copyright
// (c) 2026 Audio Program contributors. Licensed under the Microsoft Public
// License, see driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_MINTOPO_H_
#define _AUDIOPROGRAMCABLE_MINTOPO_H_

//=============================================================================
// Classes
//=============================================================================

///////////////////////////////////////////////////////////////////////////////
// CMiniportTopology
//

class CMiniportTopology :
    public IMiniportTopology,
    public CUnknown
{
  private:
    PCFILTER_DESCRIPTOR    *m_FilterDescriptor;
    eDeviceType             m_DeviceType;

public:
    DECLARE_STD_UNKNOWN();
    CMiniportTopology
    (
        _In_opt_    PUNKNOWN                UnknownOuter,
        _In_        PCFILTER_DESCRIPTOR    *FilterDesc,
        _In_        eDeviceType             DeviceType
    )
    : CUnknown(UnknownOuter),
      m_FilterDescriptor(FilterDesc),
      m_DeviceType(DeviceType)
    {
    }

    ~CMiniportTopology();

    IMP_IMiniportTopology;

    NTSTATUS PropertyHandlerJackDescription
    (
        _In_        PPCPROPERTY_REQUEST                         PropertyRequest
    );

    NTSTATUS PropertyHandlerJackDescription2
    (
        _In_        PPCPROPERTY_REQUEST                         PropertyRequest
    );

private:
    ULONG GetJackDescriptions
    (
        _Outptr_result_buffer_(return) PKSJACK_DESCRIPTION **JackDescriptions
    );
};

typedef CMiniportTopology *PCMiniportTopology;

NTSTATUS
CreateMiniportTopologyCable
(
    _Out_       PUNKNOWN *,
    _In_        REFCLSID,
    _In_opt_    PUNKNOWN,
    _In_        POOL_FLAGS,
    _In_        PUNKNOWN,
    _In_opt_    PVOID,
    _In_        PENDPOINT_MINIPAIR
);

#endif // _AUDIOPROGRAMCABLE_MINTOPO_H_
