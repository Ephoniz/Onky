/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    endpoints.h

Abstract:

    Node and Pin numbers and other common definitions for the cable endpoints.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Inc/endpoints.h). Modifications Copyright (c) 2026 Audio Program
// contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_ENDPOINTS_H_
#define _AUDIOPROGRAMCABLE_ENDPOINTS_H_

// One stream per pin: the cable is a single playback-to-capture path.
#define CABLE_MAX_INPUT_SYSTEM_STREAMS  1
#define CABLE_MAX_CAPTURE_STREAMS       1

//----------------------------------------------------
// Render endpoint (the "playback" end of a cable).
//----------------------------------------------------

// Wave pins
enum
{
    KSPIN_WAVE_RENDER_SINK_SYSTEM = 0,
    KSPIN_WAVE_RENDER_SOURCE
};

// Topology pins.
enum
{
    KSPIN_TOPO_WAVEOUT_SOURCE = 0,
    KSPIN_TOPO_LINEOUT_DEST,
};

//----------------------------------------------------
// Capture endpoint (the "recording" end of a cable).
//----------------------------------------------------

// Wave pins
enum
{
    KSPIN_WAVE_BRIDGE = 0,
    KSPIN_WAVEIN_HOST,
};

// Topology pins.
enum
{
    KSPIN_TOPO_MIC_ELEMENTS = 0,
    KSPIN_TOPO_BRIDGE
};

// data format attribute range definitions.
static
KSATTRIBUTE PinDataRangeSignalProcessingModeAttribute =
{
    sizeof(KSATTRIBUTE),
    0,
    STATICGUIDOF(KSATTRIBUTEID_AUDIOSIGNALPROCESSING_MODE),
};

static
PKSATTRIBUTE PinDataRangeAttributes[] =
{
    &PinDataRangeSignalProcessingModeAttribute,
};

static
KSATTRIBUTE_LIST PinDataRangeAttributeList =
{
    ARRAYSIZE(PinDataRangeAttributes),
    PinDataRangeAttributes,
};

#endif // _AUDIOPROGRAMCABLE_ENDPOINTS_H_
