// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Audio Program contributors
//
// Host unit tests for the cable count setting and cable names. Build with any
// C++17 compiler:
//   c++ -std=c++17 -Wall -Wextra -Werror -I../src cable_config_test.cpp

#include "cable_config.h"

#include <cstdio>
#include <cstdlib>
#include <cwchar>

namespace
{

int g_failures = 0;

#define CHECK(cond)                                                            \
    do                                                                         \
    {                                                                          \
        if (!(cond))                                                           \
        {                                                                      \
            std::fprintf(stderr, "%s:%d: CHECK failed: %s\n", __FILE__,        \
                         __LINE__, #cond);                                     \
            g_failures++;                                                      \
        }                                                                      \
    } while (0)

void TestMissingSettingUsesDefault()
{
    CHECK(CableCountFromSetting(false, 0) == 8);
    CHECK(CableCountFromSetting(false, 3) == 8);
}

void TestSettingIsClamped()
{
    CHECK(CableCountFromSetting(true, 0) == 1);
    CHECK(CableCountFromSetting(true, 1) == 1);
    CHECK(CableCountFromSetting(true, 4) == 4);
    CHECK(CableCountFromSetting(true, 16) == 16);
    CHECK(CableCountFromSetting(true, 17) == 16);
    CHECK(CableCountFromSetting(true, 0xFFFFFFFFu) == 16);
}

void TestFriendlyName()
{
    wchar_t name[64];
    std::swprintf(name, 64, CABLE_FRIENDLY_NAME_FORMAT, 12u);
    CHECK(std::wcscmp(name, L"Audio Program Cable 12") == 0);
}

} // namespace

int main()
{
    TestMissingSettingUsesDefault();
    TestSettingIsClamped();
    TestFriendlyName();

    if (g_failures != 0)
    {
        std::fprintf(stderr, "%d check(s) failed\n", g_failures);
        return EXIT_FAILURE;
    }
    std::printf("cable config: all tests passed\n");
    return EXIT_SUCCESS;
}
