/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    definitions.h

Abstract:

    Header file for common stuff.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Inc/Definitions.h). Modifications Copyright (c) 2026 Audio Program
// contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_DEFINITIONS_H_
#define _AUDIOPROGRAMCABLE_DEFINITIONS_H_

#include <portcls.h>
#include <stdunk.h>
#include <ksdebug.h>
#include <ntintsafe.h>
#include <ntstrsafe.h>
#include "newdelete.h"

//=============================================================================
// Defines
//=============================================================================

// Pool tag used for generic allocations.
#define AUDIOPROGRAMCABLE_POOLTAG   'CPdA'

// Debug module name
#define STR_MODULENAME              "AudioProgramCable: "

// Debug utility macros
#define D_FUNC                      4
#define D_BLAB                      DEBUGLVL_BLAB
#define D_VERBOSE                   DEBUGLVL_VERBOSE
#define D_TERSE                     DEBUGLVL_TERSE
#define D_ERROR                     DEBUGLVL_ERROR
#define DPF                         _DbgPrintF
#define DPF_ENTER(x)                DPF(D_FUNC, x)

//
// Cable configuration. The count is read from
// HKLM\SYSTEM\CurrentControlSet\Services\AudioProgramCable\Parameters\CableCount
// when the device starts.
//
#define CABLE_COUNT_VALUE_NAME      L"CableCount"
#define CABLE_COUNT_DEFAULT         8
#define CABLE_COUNT_MIN             1
#define CABLE_COUNT_MAX             16

//
// Every cable endpoint uses exactly one format, so the loopback is a byte copy:
// 48 kHz, 2 channels, 32-bit integer PCM. 384 bytes per millisecond keeps every
// millisecond of audio frame aligned.
//
#define CABLE_SAMPLE_RATE           48000
#define CABLE_CHANNELS              2
#define CABLE_BITS_PER_SAMPLE       32
#define CABLE_BLOCK_ALIGN           (CABLE_CHANNELS * CABLE_BITS_PER_SAMPLE / 8)
#define CABLE_BYTES_PER_SECOND      (CABLE_SAMPLE_RATE * CABLE_BLOCK_ALIGN)
#define CABLE_BYTES_PER_MS          (CABLE_BYTES_PER_SECOND / 1000)

// Loopback ring sizing, in milliseconds of audio.
#define CABLE_RING_MS               250
#define CABLE_PRIME_MS              20
#define CABLE_MAX_LATENCY_MS        100

#define KSPROPERTY_TYPE_ALL         KSPROPERTY_TYPE_BASICSUPPORT | \
                                    KSPROPERTY_TYPE_GET | \
                                    KSPROPERTY_TYPE_SET

#define HNSTIME_PER_MILLISECOND     10000

// Macros to assist with pin instance counting
#define VERIFY_PIN_INSTANCE_RESOURCES_AVAILABLE(status, allocated, max) \
    status = (allocated < max) ? STATUS_SUCCESS : STATUS_INSUFFICIENT_RESOURCES

#define ALLOCATE_PIN_INSTANCE_RESOURCES(allocated) \
    allocated++

#define FREE_PIN_INSTANCE_RESOURCES(allocated) \
    allocated--

//=============================================================================
// Typedefs
//=============================================================================

// Flags to identify stream processing mode
typedef enum {
    CONNECTIONTYPE_TOPOLOGY_OUTPUT = 0,
    CONNECTIONTYPE_WAVE_OUTPUT     = 1
} CONNECTIONTYPE;

// Connection table for registering topology/wave bridge connection
typedef struct _PHYSICALCONNECTIONTABLE
{
    ULONG            ulTopology;
    ULONG            ulWave;
    CONNECTIONTYPE   eType;
} PHYSICALCONNECTIONTABLE, *PPHYSICALCONNECTIONTABLE;

//
// This is the structure of the portclass FDO device extension Nt has created
// for us.  We keep the adapter common object here.
//
struct IAdapterCommon;
typedef struct _PortClassDeviceContext              // 32       64      Byte offsets for 32 and 64 bit architectures
{
    ULONG_PTR m_pulReserved1[2];                    // 0-7      0-15    First two pointers are reserved.
    PDEVICE_OBJECT m_DoNotUsePhysicalDeviceObject;  // 8-11     16-23   Reserved pointer to our Physical Device Object (PDO).
    PVOID m_pvReserved2;                            // 12-15    24-31   Reserved pointer to our Start Device function.
    PVOID m_pvReserved3;                            // 16-19    32-39   "Out Memory" according to DDK.
    IAdapterCommon* m_pCommon;                      // 20-23    40-47   Pointer to our adapter common object.
    PVOID m_pvUnused1;                              // 24-27    48-55   Unused space.
    PVOID m_pvUnused2;                              // 28-31    56-63   Unused space.

    // Anything after above line should not be used.
    // This actually goes on for (64*sizeof(ULONG_PTR)) but it is all opaque.
} PortClassDeviceContext;

//
// Global settings.
//
extern PDRIVER_OBJECT g_DriverObject;

//=============================================================================
// Function prototypes
//=============================================================================

// Default WaveFilter automation table.
NTSTATUS PropertyHandler_WaveFilter
(
    _In_ PPCPROPERTY_REQUEST PropertyRequest
);

// Topology filter automation table (jack description).
NTSTATUS PropertyHandler_CableTopoFilter
(
    _In_ PPCPROPERTY_REQUEST PropertyRequest
);

// common.h uses some of the above definitions.
#include "common.h"
#include "kshelper.h"

#endif // _AUDIOPROGRAMCABLE_DEFINITIONS_H_
