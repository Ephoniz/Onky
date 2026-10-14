/*****************************************************************************
* newdelete.cpp -  CPP placement new and delete operators implementation
*****************************************************************************
* Copyright (c) Microsoft Corporation All Rights Reserved
*
* Module Name:
*
* newdelete.cpp
*
* Abstract:
*
*   Definition of placement new and delete operators.
*
*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Main/NewDelete.cpp). Modifications Copyright (c) 2026 Audio Program
// contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifdef _NEW_DELETE_OPERATORS_

#include "definitions.h"

#pragma code_seg()
/*****************************************************************************
* Functions
*/

/*****************************************************************************
* ::new()
*****************************************************************************
* New function for creating objects with a specified allocation tag.
*/
PVOID operator new
(
    size_t      iSize,
    POOL_FLAGS  poolFlags,
    ULONG       tag
)
{
    PVOID result = ExAllocatePool2(poolFlags & ~POOL_FLAG_NON_PAGED_EXECUTE, iSize, tag);

    return result;
}


/*****************************************************************************
* ::new()
*****************************************************************************
* New function for creating objects with a specified allocation tag.
*/
PVOID operator new
(
    size_t      iSize,
    POOL_FLAGS  poolFlags
)
{
    PVOID result = ExAllocatePool2(poolFlags & ~POOL_FLAG_NON_PAGED_EXECUTE, iSize, AUDIOPROGRAMCABLE_POOLTAG);

    return result;
}


/*****************************************************************************
* ::delete()
*****************************************************************************
* Delete with tag function.
*/
void __cdecl operator delete
(
    PVOID pVoid,
    ULONG tag
)
{
    if (pVoid)
    {
        ExFreePoolWithTag(pVoid, tag);
    }
}


/*****************************************************************************
* ::delete()
*****************************************************************************
* Sized Delete function.
*/
void __cdecl operator delete
(
    _Pre_maybenull_ __drv_freesMem(Mem) PVOID pVoid,
    _In_ size_t cbSize
)
{
    UNREFERENCED_PARAMETER(cbSize);

    if (pVoid)
    {
        ExFreePool(pVoid);
    }
}


/*****************************************************************************
* ::delete()
*****************************************************************************
* Sized Array Delete function.
*/
void __cdecl operator delete[]
(
    _Pre_maybenull_ __drv_freesMem(Mem) PVOID pVoid,
    _In_ size_t cbSize
)
{
    UNREFERENCED_PARAMETER(cbSize);

    if (pVoid)
    {
        ExFreePool(pVoid);
    }
}


/*****************************************************************************
* ::delete()
*****************************************************************************
* Array Delete function.
*/
void __cdecl operator delete[]
(
    _Pre_maybenull_ __drv_freesMem(Mem) PVOID pVoid
)
{
    if (pVoid)
    {
        ExFreePool(pVoid);
    }
}
#endif//_NEW_DELETE_OPERATORS_
