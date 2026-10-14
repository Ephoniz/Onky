/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    cabletables.h

Abstract:

    Wave and topology filter tables shared by every cable. Each cable is a
    stereo render endpoint and a stereo capture endpoint.

--*/
// SPDX-License-Identifier: MS-PL
// Derived from Microsoft Windows-driver-samples audio/simpleaudiosample
// (Source/Filters/speakerwavtable.h, speakertoptable.h, micarraywavtable.h,
// micarray1toptable.h, minipairs.h). Modifications Copyright (c) 2026 Audio
// Program contributors. Licensed under the Microsoft Public License, see
// driver/LICENSES/MS-PL.txt.

#ifndef _AUDIOPROGRAMCABLE_CABLETABLES_H_
#define _AUDIOPROGRAMCABLE_CABLETABLES_H_

//=============================================================================
// The one supported device format: 48 kHz, stereo, 32-bit integer PCM.
//=============================================================================
static
KSDATAFORMAT_WAVEFORMATEXTENSIBLE CablePinSupportedDeviceFormats[] =
{
    {
        {
            sizeof(KSDATAFORMAT_WAVEFORMATEXTENSIBLE),
            0,
            0,
            0,
            STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
            STATICGUIDOF(KSDATAFORMAT_SUBTYPE_PCM),
            STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX)
        },
        {
            {
                WAVE_FORMAT_EXTENSIBLE,
                CABLE_CHANNELS,
                CABLE_SAMPLE_RATE,
                CABLE_BYTES_PER_SECOND,
                CABLE_BLOCK_ALIGN,
                CABLE_BITS_PER_SAMPLE,
                sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX)
            },
            CABLE_BITS_PER_SAMPLE,
            KSAUDIO_SPEAKER_STEREO,
            STATICGUIDOF(KSDATAFORMAT_SUBTYPE_PCM)
        }
    }
};

//
// Supported modes (only on streaming pins).
//
static
MODE_AND_DEFAULT_FORMAT CableRenderPinSupportedDeviceModes[] =
{
    {
        STATIC_AUDIO_SIGNALPROCESSINGMODE_DEFAULT,
        &CablePinSupportedDeviceFormats[0].DataFormat
    }
};

static
MODE_AND_DEFAULT_FORMAT CableCapturePinSupportedDeviceModes[] =
{
    {
        STATIC_AUDIO_SIGNALPROCESSINGMODE_DEFAULT,
        &CablePinSupportedDeviceFormats[0].DataFormat
    },
    {
        STATIC_AUDIO_SIGNALPROCESSINGMODE_RAW,
        &CablePinSupportedDeviceFormats[0].DataFormat
    }
};

//
// The entries here must follow the same order as the filter's pin
// descriptor array.
//
static
PIN_DEVICE_FORMATS_AND_MODES CableRenderPinDeviceFormatsAndModes[] =
{
    {
        SystemRenderPin,
        CablePinSupportedDeviceFormats,
        SIZEOF_ARRAY(CablePinSupportedDeviceFormats),
        CableRenderPinSupportedDeviceModes,
        SIZEOF_ARRAY(CableRenderPinSupportedDeviceModes)
    },
    {
        BridgePin,
        NULL,
        0,
        NULL,
        0
    }
};

static
PIN_DEVICE_FORMATS_AND_MODES CableCapturePinDeviceFormatsAndModes[] =
{
    {
        BridgePin,
        NULL,
        0,
        NULL,
        0
    },
    {
        SystemCapturePin,
        CablePinSupportedDeviceFormats,
        SIZEOF_ARRAY(CablePinSupportedDeviceFormats),
        CableCapturePinSupportedDeviceModes,
        SIZEOF_ARRAY(CableCapturePinSupportedDeviceModes)
    }
};

//=============================================================================
// Data ranges
//=============================================================================
static
KSDATARANGE_AUDIO CablePinDataRangesStream[] =
{
    {
        {
            sizeof(KSDATARANGE_AUDIO),
            KSDATARANGE_ATTRIBUTES,         // An attributes list follows this data range
            0,
            0,
            STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
            STATICGUIDOF(KSDATAFORMAT_SUBTYPE_PCM),
            STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX)
        },
        CABLE_CHANNELS,
        CABLE_BITS_PER_SAMPLE,
        CABLE_BITS_PER_SAMPLE,
        CABLE_SAMPLE_RATE,
        CABLE_SAMPLE_RATE
    }
};

static
PKSDATARANGE CablePinDataRangePointersStream[] =
{
    PKSDATARANGE(&CablePinDataRangesStream[0]),
    PKSDATARANGE(&PinDataRangeAttributeList),
};

static
KSDATARANGE CablePinDataRangesBridge[] =
{
    {
        sizeof(KSDATARANGE),
        0,
        0,
        0,
        STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
        STATICGUIDOF(KSDATAFORMAT_SUBTYPE_ANALOG),
        STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
    }
};

static
PKSDATARANGE CablePinDataRangePointersBridge[] =
{
    &CablePinDataRangesBridge[0]
};

//=============================================================================
// Wave filter automation (shared by render and capture).
//=============================================================================
static
PCPROPERTY_ITEM PropertiesCableWaveFilter[] =
{
    {
        &KSPROPSETID_Pin,
        KSPROPERTY_PIN_PROPOSEDATAFORMAT,
        KSPROPERTY_TYPE_SET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_WaveFilter
    },
    {
        &KSPROPSETID_Pin,
        KSPROPERTY_PIN_PROPOSEDATAFORMAT2,
        KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_WaveFilter
    }
};

DEFINE_PCAUTOMATION_TABLE_PROP(AutomationCableWaveFilter, PropertiesCableWaveFilter);

//=============================================================================
// Render wave filter
//
//                   ----------------------------
//                   |                          |
//  Host Pin     0-->|                          |--> 1 KSPIN_WAVE_RENDER_SOURCE
//                   |                          |
//                   ----------------------------
//=============================================================================
static
PCPIN_DESCRIPTOR CableRenderWaveMiniportPins[] =
{
    // KSPIN_WAVE_RENDER_SINK_SYSTEM
    {
        CABLE_MAX_INPUT_SYSTEM_STREAMS,
        CABLE_MAX_INPUT_SYSTEM_STREAMS,
        0,
        NULL,        // AutomationTable
        {
            0,
            NULL,
            0,
            NULL,
            SIZEOF_ARRAY(CablePinDataRangePointersStream),
            CablePinDataRangePointersStream,
            KSPIN_DATAFLOW_IN,
            KSPIN_COMMUNICATION_SINK,
            &KSCATEGORY_AUDIO,
            NULL,
            0
        }
    },
    // KSPIN_WAVE_RENDER_SOURCE
    {
        0,
        0,
        0,
        NULL,
        {
            0,
            NULL,
            0,
            NULL,
            SIZEOF_ARRAY(CablePinDataRangePointersBridge),
            CablePinDataRangePointersBridge,
            KSPIN_DATAFLOW_OUT,
            KSPIN_COMMUNICATION_NONE,
            &KSCATEGORY_AUDIO,
            NULL,
            0
        }
    },
};

static
PCCONNECTION_DESCRIPTOR CableRenderWaveMiniportConnections[] =
{
    { PCFILTER_NODE,            KSPIN_WAVE_RENDER_SINK_SYSTEM,     PCFILTER_NODE,   KSPIN_WAVE_RENDER_SOURCE }
};

static
PCFILTER_DESCRIPTOR CableRenderWaveMiniportFilterDescriptor =
{
    0,                                                  // Version
    &AutomationCableWaveFilter,                         // AutomationTable
    sizeof(PCPIN_DESCRIPTOR),                           // PinSize
    SIZEOF_ARRAY(CableRenderWaveMiniportPins),          // PinCount
    CableRenderWaveMiniportPins,                        // Pins
    sizeof(PCNODE_DESCRIPTOR),                          // NodeSize
    0,                                                  // NodeCount
    NULL,                                               // Nodes
    SIZEOF_ARRAY(CableRenderWaveMiniportConnections),   // ConnectionCount
    CableRenderWaveMiniportConnections,                 // Connections
    0,                                                  // CategoryCount
    NULL                                                // Categories  - use defaults (audio, render, capture)
};

//=============================================================================
// Capture wave filter
//
//                   ----------------------------
//                   |                          |
//  Bridge Pin   0-->|                          |--> 1 KSPIN_WAVEIN_HOST
//                   |                          |
//                   ----------------------------
//=============================================================================
static
PCPIN_DESCRIPTOR CableCaptureWaveMiniportPins[] =
{
    // KSPIN_WAVE_BRIDGE
    {
        0,
        0,
        0,
        NULL,
        {
            0,
            NULL,
            0,
            NULL,
            SIZEOF_ARRAY(CablePinDataRangePointersBridge),
            CablePinDataRangePointersBridge,
            KSPIN_DATAFLOW_IN,
            KSPIN_COMMUNICATION_NONE,
            &KSCATEGORY_AUDIO,
            NULL,
            0
        }
    },
    // KSPIN_WAVEIN_HOST
    {
        CABLE_MAX_CAPTURE_STREAMS,
        CABLE_MAX_CAPTURE_STREAMS,
        0,
        NULL,
        {
            0,
            NULL,
            0,
            NULL,
            SIZEOF_ARRAY(CablePinDataRangePointersStream),
            CablePinDataRangePointersStream,
            KSPIN_DATAFLOW_OUT,
            KSPIN_COMMUNICATION_SINK,
            &KSCATEGORY_AUDIO,
            &KSAUDFNAME_RECORDING_CONTROL,
            0
        }
    }
};

static
PCCONNECTION_DESCRIPTOR CableCaptureWaveMiniportConnections[] =
{
    { PCFILTER_NODE,        KSPIN_WAVE_BRIDGE,      PCFILTER_NODE,       KSPIN_WAVEIN_HOST },
};

static
PCFILTER_DESCRIPTOR CableCaptureWaveMiniportFilterDescriptor =
{
    0,                                                  // Version
    &AutomationCableWaveFilter,                         // AutomationTable
    sizeof(PCPIN_DESCRIPTOR),                           // PinSize
    SIZEOF_ARRAY(CableCaptureWaveMiniportPins),         // PinCount
    CableCaptureWaveMiniportPins,                       // Pins
    sizeof(PCNODE_DESCRIPTOR),                          // NodeSize
    0,                                                  // NodeCount
    NULL,                                               // Nodes
    SIZEOF_ARRAY(CableCaptureWaveMiniportConnections),  // ConnectionCount
    CableCaptureWaveMiniportConnections,                // Connections
    0,                                                  // CategoryCount
    NULL                                                // Categories  - use defaults (audio, render, capture)
};

//=============================================================================
// Topology filters. There are no volume or mute nodes: the driver applies no
// gain, and Windows provides software volume for the endpoint.
//=============================================================================
static
KSDATARANGE CableTopoPinDataRangesBridge[] =
{
    {
        sizeof(KSDATARANGE),
        0,
        0,
        0,
        STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
        STATICGUIDOF(KSDATAFORMAT_SUBTYPE_ANALOG),
        STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
    }
};

static
PKSDATARANGE CableTopoPinDataRangePointersBridge[] =
{
    &CableTopoPinDataRangesBridge[0]
};

static
PCPROPERTY_ITEM PropertiesCableTopoFilter[] =
{
    {
        &KSPROPSETID_Jack,
        KSPROPERTY_JACK_DESCRIPTION,
        KSPROPERTY_TYPE_GET |
        KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_CableTopoFilter
    },
    {
        &KSPROPSETID_Jack,
        KSPROPERTY_JACK_DESCRIPTION2,
        KSPROPERTY_TYPE_GET |
        KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_CableTopoFilter
    }
};

DEFINE_PCAUTOMATION_TABLE_PROP(AutomationCableTopoFilter, PropertiesCableTopoFilter);

//-----------------------------------------------------------------------------
// Render topology:  wave bridge --> 0 [topo] 1 --> speaker
//-----------------------------------------------------------------------------
static
PCPIN_DESCRIPTOR CableRenderTopoMiniportPins[] =
{
    // KSPIN_TOPO_WAVEOUT_SOURCE
    {
        0,
        0,
        0,                                                  // InstanceCount
        NULL,                                               // AutomationTable
        {                                                   // KsPinDescriptor
            0,                                              // InterfacesCount
            NULL,                                           // Interfaces
            0,                                              // MediumsCount
            NULL,                                           // Mediums
            SIZEOF_ARRAY(CableTopoPinDataRangePointersBridge),// DataRangesCount
            CableTopoPinDataRangePointersBridge,            // DataRanges
            KSPIN_DATAFLOW_IN,                              // DataFlow
            KSPIN_COMMUNICATION_NONE,                       // Communication
            &KSCATEGORY_AUDIO,                              // Category
            NULL,                                           // Name
            0                                               // Reserved
        }
    },
    // KSPIN_TOPO_LINEOUT_DEST
    {
        0,
        0,
        0,                                                  // InstanceCount
        NULL,                                               // AutomationTable
        {                                                   // KsPinDescriptor
            0,                                              // InterfacesCount
            NULL,                                           // Interfaces
            0,                                              // MediumsCount
            NULL,                                           // Mediums
            SIZEOF_ARRAY(CableTopoPinDataRangePointersBridge),// DataRangesCount
            CableTopoPinDataRangePointersBridge,            // DataRanges
            KSPIN_DATAFLOW_OUT,                             // DataFlow
            KSPIN_COMMUNICATION_NONE,                       // Communication
            &KSNODETYPE_SPEAKER,                            // Category
            NULL,                                           // Name
            0                                               // Reserved
        }
    }
};

static
PCCONNECTION_DESCRIPTOR CableRenderTopoMiniportConnections[] =
{
    //  FromNode,                 FromPin,                      ToNode,                 ToPin
    {   PCFILTER_NODE,            KSPIN_TOPO_WAVEOUT_SOURCE,    PCFILTER_NODE,          KSPIN_TOPO_LINEOUT_DEST }
};

static
PCFILTER_DESCRIPTOR CableRenderTopoMiniportFilterDescriptor =
{
    0,                                                  // Version
    &AutomationCableTopoFilter,                         // AutomationTable
    sizeof(PCPIN_DESCRIPTOR),                           // PinSize
    SIZEOF_ARRAY(CableRenderTopoMiniportPins),          // PinCount
    CableRenderTopoMiniportPins,                        // Pins
    sizeof(PCNODE_DESCRIPTOR),                          // NodeSize
    0,                                                  // NodeCount
    NULL,                                               // Nodes
    SIZEOF_ARRAY(CableRenderTopoMiniportConnections),   // ConnectionCount
    CableRenderTopoMiniportConnections,                 // Connections
    0,                                                  // CategoryCount
    NULL                                                // Categories
};

//-----------------------------------------------------------------------------
// Capture topology:  microphone --> 0 [topo] 1 --> wave bridge
//-----------------------------------------------------------------------------
static
PCPIN_DESCRIPTOR CableCaptureTopoMiniportPins[] =
{
    // KSPIN_TOPO_MIC_ELEMENTS
    {
        0,
        0,
        0,                                                  // InstanceCount
        NULL,                                               // AutomationTable
        {                                                   // KsPinDescriptor
            0,                                              // InterfacesCount
            NULL,                                           // Interfaces
            0,                                              // MediumsCount
            NULL,                                           // Mediums
            SIZEOF_ARRAY(CableTopoPinDataRangePointersBridge),// DataRangesCount
            CableTopoPinDataRangePointersBridge,            // DataRanges
            KSPIN_DATAFLOW_IN,                              // DataFlow
            KSPIN_COMMUNICATION_NONE,                       // Communication
            &KSNODETYPE_MICROPHONE,                         // Category
            NULL,                                           // Name
            0                                               // Reserved
        }
    },
    // KSPIN_TOPO_BRIDGE
    {
        0,
        0,
        0,                                                  // InstanceCount
        NULL,                                               // AutomationTable
        {                                                   // KsPinDescriptor
            0,                                              // InterfacesCount
            NULL,                                           // Interfaces
            0,                                              // MediumsCount
            NULL,                                           // Mediums
            SIZEOF_ARRAY(CableTopoPinDataRangePointersBridge),// DataRangesCount
            CableTopoPinDataRangePointersBridge,            // DataRanges
            KSPIN_DATAFLOW_OUT,                             // DataFlow
            KSPIN_COMMUNICATION_NONE,                       // Communication
            &KSCATEGORY_AUDIO,                              // Category
            NULL,                                           // Name
            0                                               // Reserved
        }
    }
};

static
PCCONNECTION_DESCRIPTOR CableCaptureTopoMiniportConnections[] =
{
    //  FromNode,                 FromPin,                      ToNode,                 ToPin
    {   PCFILTER_NODE,            KSPIN_TOPO_MIC_ELEMENTS,      PCFILTER_NODE,          KSPIN_TOPO_BRIDGE }
};

static
PCFILTER_DESCRIPTOR CableCaptureTopoMiniportFilterDescriptor =
{
    0,                                                  // Version
    &AutomationCableTopoFilter,                         // AutomationTable
    sizeof(PCPIN_DESCRIPTOR),                           // PinSize
    SIZEOF_ARRAY(CableCaptureTopoMiniportPins),         // PinCount
    CableCaptureTopoMiniportPins,                       // Pins
    sizeof(PCNODE_DESCRIPTOR),                          // NodeSize
    0,                                                  // NodeCount
    NULL,                                               // Nodes
    SIZEOF_ARRAY(CableCaptureTopoMiniportConnections),  // ConnectionCount
    CableCaptureTopoMiniportConnections,                // Connections
    0,                                                  // CategoryCount
    NULL                                                // Categories
};

//=============================================================================
// Wave/topology bridge connections.
//
//  Render:   System --->|0 Wave 1|--->|0 Topo 1|---> Speaker
//  Capture:  Mic    --->|0 Topo 1|--->|0 Wave 1|---> Capture host pin
//=============================================================================
static
PHYSICALCONNECTIONTABLE CableRenderTopologyPhysicalConnections[] =
{
    {
        KSPIN_TOPO_WAVEOUT_SOURCE,  // TopologyIn
        KSPIN_WAVE_RENDER_SOURCE,   // WaveOut
        CONNECTIONTYPE_WAVE_OUTPUT
    }
};

static
PHYSICALCONNECTIONTABLE CableCaptureTopologyPhysicalConnections[] =
{
    {
        KSPIN_TOPO_BRIDGE,          // TopologyOut
        KSPIN_WAVE_BRIDGE,          // WaveIn
        CONNECTIONTYPE_TOPOLOGY_OUTPUT
    }
};

#endif // _AUDIOPROGRAMCABLE_CABLETABLES_H_
