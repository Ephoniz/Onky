// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Audio Program contributors
//
// Cable count setting and cable names. No OS dependencies, so host tests can
// check them against the INF and the scripts.

#ifndef AUDIO_PROGRAM_CABLE_CONFIG_H_
#define AUDIO_PROGRAM_CABLE_CONFIG_H_

// Read from HKLM\SYSTEM\CurrentControlSet\Services\AudioProgramCable\Parameters
// when the device starts.
#define CABLE_COUNT_VALUE_NAME          L"CableCount"
#define CABLE_COUNT_DEFAULT             8
#define CABLE_COUNT_MIN                 1
#define CABLE_COUNT_MAX                 16

// Reference strings of the interface templates in the INF. Each cable's
// interfaces use these names followed by a two-digit cable number.
#define CABLE_TEMPLATE_RENDER_WAVE      L"WaveCableRender"
#define CABLE_TEMPLATE_RENDER_TOPO      L"TopologyCableRender"
#define CABLE_TEMPLATE_CAPTURE_WAVE     L"WaveCableCapture"
#define CABLE_TEMPLATE_CAPTURE_TOPO     L"TopologyCableCapture"

#define CABLE_FRIENDLY_NAME_FORMAT      L"Audio Program Cable %u"

// The default when no valid DWORD is set, otherwise Value clamped to the
// supported range.
inline unsigned int CableCountFromSetting(bool HasValue, unsigned int Value)
{
    if (!HasValue)
    {
        return CABLE_COUNT_DEFAULT;
    }
    if (Value < CABLE_COUNT_MIN)
    {
        return CABLE_COUNT_MIN;
    }
    if (Value > CABLE_COUNT_MAX)
    {
        return CABLE_COUNT_MAX;
    }
    return Value;
}

#endif // AUDIO_PROGRAM_CABLE_CONFIG_H_
