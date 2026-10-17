/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    kshelper.cpp

Abstract:

    Helper functions for the cable driver.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Utilities/kshelper.cpp). Modifications Copyright (c) 2026 Audio
// Program contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#include "definitions.h"

//-----------------------------------------------------------------------------
#pragma code_seg("PAGE")
PWAVEFORMATEX
GetWaveFormatEx
(
    _In_  PKSDATAFORMAT           pDataFormat
)
/*++

Routine Description:

  Returns the waveformatex for known formats.

Arguments:

  pDataFormat - data format.

Return Value:

    waveformatex in DataFormat.
    NULL for unknown data formats.

--*/
{
    PAGED_CODE();

    PWAVEFORMATEX           pWfx = NULL;

    // If this is a known dataformat extract the waveformat info.
    //
    if
    (
        pDataFormat &&
        pDataFormat->FormatSize >= sizeof(KSDATAFORMAT_WAVEFORMATEX) &&
        ( IsEqualGUIDAligned(pDataFormat->MajorFormat,
                KSDATAFORMAT_TYPE_AUDIO)             &&
          IsEqualGUIDAligned(pDataFormat->Specifier,
                KSDATAFORMAT_SPECIFIER_WAVEFORMATEX) )
    )
    {
        pWfx = PWAVEFORMATEX(pDataFormat + 1);
    }

    return pWfx;
} // GetWaveFormatEx

//-----------------------------------------------------------------------------
#pragma code_seg("PAGE")
NTSTATUS
PropertyHandler_BasicSupport
(
    _In_ PPCPROPERTY_REQUEST         PropertyRequest,
    _In_ ULONG                       Flags,
    _In_ DWORD                       PropTypeSetId
)
/*++

Routine Description:

  Default basic support handler. Basic processing depends on the size of data.
  For ULONG it only returns Flags. For KSPROPERTY_DESCRIPTION, the structure
  is filled.

Arguments:

  PropertyRequest -

  Flags - Support flags.

  PropTypeSetId - PropTypeSetId

Return Value:

    NT status code.

--*/
{
    PAGED_CODE();

    ASSERT(Flags & KSPROPERTY_TYPE_BASICSUPPORT);

    NTSTATUS                    ntStatus = STATUS_INVALID_PARAMETER;

    if (PropertyRequest->ValueSize >= sizeof(KSPROPERTY_DESCRIPTION))
    {
        // if return buffer can hold a KSPROPERTY_DESCRIPTION, return it
        //
        PKSPROPERTY_DESCRIPTION PropDesc =
            PKSPROPERTY_DESCRIPTION(PropertyRequest->Value);

        PropDesc->AccessFlags       = Flags;
        PropDesc->DescriptionSize   = sizeof(KSPROPERTY_DESCRIPTION);
        if  (VT_ILLEGAL != PropTypeSetId)
        {
            PropDesc->PropTypeSet.Set   = KSPROPTYPESETID_General;
            PropDesc->PropTypeSet.Id    = PropTypeSetId;
        }
        else
        {
            PropDesc->PropTypeSet.Set   = GUID_NULL;
            PropDesc->PropTypeSet.Id    = 0;
        }
        PropDesc->PropTypeSet.Flags = 0;
        PropDesc->MembersListCount  = 0;
        PropDesc->Reserved          = 0;

        PropertyRequest->ValueSize = sizeof(KSPROPERTY_DESCRIPTION);
        ntStatus = STATUS_SUCCESS;
    }
    else if (PropertyRequest->ValueSize >= sizeof(ULONG))
    {
        // if return buffer can hold a ULONG, return the access flags
        //
        *(PULONG(PropertyRequest->Value)) = Flags;

        PropertyRequest->ValueSize = sizeof(ULONG);
        ntStatus = STATUS_SUCCESS;
    }
    else
    {
        PropertyRequest->ValueSize = 0;
        ntStatus = STATUS_BUFFER_TOO_SMALL;
    }

    return ntStatus;
} // PropertyHandler_BasicSupport

#pragma code_seg()
