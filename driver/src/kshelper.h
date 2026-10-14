/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    kshelper.h

Abstract:

    Helper functions for the cable driver.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Inc/kshelper.h). Modifications Copyright (c) 2026 Audio Program
// contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_KSHELPER_H_
#define _AUDIOPROGRAMCABLE_KSHELPER_H_

#include <portcls.h>
#include <ksdebug.h>

PWAVEFORMATEX
GetWaveFormatEx
(
    _In_  PKSDATAFORMAT           pDataFormat
);

NTSTATUS
PropertyHandler_BasicSupport
(
    _In_  PPCPROPERTY_REQUEST     PropertyRequest,
    _In_  ULONG                   Flags,
    _In_  DWORD                   PropTypeSetId
);

#endif  // _AUDIOPROGRAMCABLE_KSHELPER_H_
