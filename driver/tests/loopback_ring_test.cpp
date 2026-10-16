// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Audio Program contributors
//
// Host unit tests for the cable ring buffer. Build with any C++17 compiler:
//   c++ -std=c++17 -Wall -Wextra -Werror -I../src loopback_ring_test.cpp

#include "loopback_ring.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <algorithm>
#include <vector>

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

// Matches the driver: 48 kHz, stereo, 32-bit.
constexpr unsigned int kFrame = 8;
constexpr unsigned int kBytesPerMs = 48 * kFrame;
constexpr unsigned int kCapacity = 250 * kBytesPerMs;
constexpr unsigned int kPrime = 20 * kBytesPerMs;
constexpr unsigned int kMaxLatency = 100 * kBytesPerMs;

struct Fixture
{
    std::vector<unsigned char> storage;
    LOOPBACK_RING ring;

    explicit Fixture(unsigned int capacity = kCapacity, unsigned int prime = kPrime,
                     unsigned int maxLatency = kMaxLatency)
        : storage(capacity)
    {
        bool ok = LoopbackRingInit(&ring, storage.data(), capacity, kFrame, prime, maxLatency);
        CHECK(ok);
    }
};

// Each frame carries its own running index so ordering is checkable.
std::vector<unsigned char> Frames(unsigned int firstFrame, unsigned int frameCount)
{
    std::vector<unsigned char> out(frameCount * kFrame);
    for (unsigned int i = 0; i < frameCount; ++i)
    {
        unsigned int value = firstFrame + i + 1;
        for (unsigned int b = 0; b < kFrame; b += 4)
        {
            std::memcpy(&out[i * kFrame + b], &value, 4);
        }
    }
    return out;
}

unsigned int FrameValue(const std::vector<unsigned char> & data, unsigned int frame)
{
    unsigned int value = 0;
    std::memcpy(&value, &data[frame * kFrame], 4);
    return value;
}

bool AllZero(const std::vector<unsigned char> & data)
{
    for (unsigned char c : data)
    {
        if (c != 0)
        {
            return false;
        }
    }
    return true;
}

void TestInitRejectsBadSizes()
{
    std::vector<unsigned char> storage(1024);
    LOOPBACK_RING ring;
    CHECK(!LoopbackRingInit(&ring, nullptr, 1024, kFrame, 0, 0));
    CHECK(!LoopbackRingInit(&ring, storage.data(), 1020, kFrame, 0, 0));
    CHECK(!LoopbackRingInit(&ring, storage.data(), 1024, kFrame, 12, 16));
    CHECK(!LoopbackRingInit(&ring, storage.data(), 1024, kFrame, 64, 32));
    CHECK(!LoopbackRingInit(&ring, storage.data(), 1024, kFrame, 64, 2048));
    CHECK(LoopbackRingInit(&ring, storage.data(), 1024, kFrame, 64, 512));
}

void TestSilentUntilPrimed()
{
    Fixture f;
    std::vector<unsigned char> out(kBytesPerMs, 0xAA);

    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(AllZero(out));
    CHECK(f.ring.Underruns == 0);

    auto in = Frames(0, 10 * 48);
    LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
    std::fill(out.begin(), out.end(), 0xAA);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(AllZero(out));
    CHECK(!f.ring.Primed);
    CHECK(f.ring.Fill == in.size());
}

void TestPassesAudioInOrderAfterPriming()
{
    Fixture f;
    unsigned int written = 0;
    for (int ms = 0; ms < 20; ++ms)
    {
        auto in = Frames(written, 48);
        LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
        written += 48;
    }

    std::vector<unsigned char> out(kBytesPerMs);
    unsigned int expected = 1;
    for (int ms = 0; ms < 20; ++ms)
    {
        auto in = Frames(written, 48);
        LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
        written += 48;

        LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
        for (unsigned int i = 0; i < 48; ++i)
        {
            CHECK(FrameValue(out, i) == expected);
            expected++;
        }
    }
    CHECK(f.ring.Primed);
    CHECK(f.ring.Underruns == 0);
    CHECK(f.ring.Overruns == 0);
    CHECK(f.ring.Fill == kPrime);
}

void TestWrapAroundIsSeamless()
{
    // Odd sizes against a small ring force reads and writes across the end.
    Fixture f(64 * kFrame, 8 * kFrame, 48 * kFrame);
    unsigned int written = 0;
    unsigned int expected = 1;
    std::vector<unsigned char> out;

    for (int round = 0; round < 500; ++round)
    {
        unsigned int writeFrames = 5 + (round % 7);
        auto in = Frames(written, writeFrames);
        LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
        written += writeFrames;

        if (!f.ring.Primed && f.ring.Fill < f.ring.PrimeBytes)
        {
            continue;
        }
        unsigned int readFrames = writeFrames;
        out.assign(readFrames * kFrame, 0);
        LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
        for (unsigned int i = 0; i < readFrames; ++i)
        {
            CHECK(FrameValue(out, i) == expected);
            expected++;
        }
    }
    CHECK(f.ring.Underruns == 0);
    CHECK(f.ring.Overruns == 0);
}

void TestUnderrunZeroFillsAndReprimes()
{
    Fixture f;
    auto in = Frames(0, 25 * 48);
    LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());

    std::vector<unsigned char> out(30 * kBytesPerMs, 0xAA);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());

    CHECK(FrameValue(out, 0) == 1);
    CHECK(FrameValue(out, 25 * 48 - 1) == 25 * 48);
    std::vector<unsigned char> tail(out.begin() + 25 * kBytesPerMs, out.end());
    CHECK(AllZero(tail));
    CHECK(f.ring.Underruns == 1);
    CHECK(!f.ring.Primed);
    CHECK(f.ring.Fill == 0);

    // Not enough to re-prime yet: silence again, no extra underrun counted.
    auto more = Frames(0, 5 * 48);
    LoopbackRingWrite(&f.ring, more.data(), (unsigned int)more.size());
    std::vector<unsigned char> small(kBytesPerMs, 0xAA);
    LoopbackRingRead(&f.ring, small.data(), (unsigned int)small.size());
    CHECK(AllZero(small));
    CHECK(f.ring.Underruns == 1);
}

void TestOverrunKeepsNewestAudio()
{
    Fixture f(32 * kFrame, 0, 32 * kFrame);
    auto in = Frames(0, 40);
    LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
    CHECK(f.ring.Overruns == 1);
    CHECK(f.ring.Fill == 32 * kFrame);

    std::vector<unsigned char> out(32 * kFrame);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(FrameValue(out, 0) == 9);
    CHECK(FrameValue(out, 31) == 40);
}

void TestOverrunAcrossManyWrites()
{
    Fixture f(32 * kFrame, 0, 32 * kFrame);
    unsigned int written = 0;
    for (int i = 0; i < 9; ++i)
    {
        auto in = Frames(written, 5);
        LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
        written += 5;
    }
    CHECK(f.ring.Fill == 32 * kFrame);
    std::vector<unsigned char> out(32 * kFrame);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(FrameValue(out, 0) == written - 32 + 1);
    CHECK(FrameValue(out, 31) == written);
}

void TestLatencyIsTrimmedToPrime()
{
    Fixture f;
    auto in = Frames(0, 150 * 48);
    LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());

    std::vector<unsigned char> out(kBytesPerMs);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(f.ring.LatencyTrims == 1);
    CHECK(FrameValue(out, 0) == 130 * 48 + 1);
    CHECK(f.ring.Fill == kPrime - kBytesPerMs);
}

void TestResetReaderDropsStaleAudio()
{
    Fixture f;
    auto stale = Frames(1000, 50 * 48);
    LoopbackRingWrite(&f.ring, stale.data(), (unsigned int)stale.size());
    LoopbackRingResetReader(&f.ring);
    CHECK(f.ring.Fill == 0);
    CHECK(!f.ring.Primed);

    auto fresh = Frames(0, 20 * 48);
    LoopbackRingWrite(&f.ring, fresh.data(), (unsigned int)fresh.size());
    std::vector<unsigned char> out(kBytesPerMs);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(FrameValue(out, 0) == 1);
}

void TestNullSourceWritesSilence()
{
    Fixture f(64 * kFrame, 0, 64 * kFrame);
    auto in = Frames(0, 8);
    LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
    LoopbackRingWrite(&f.ring, nullptr, 8 * kFrame);
    std::vector<unsigned char> out(16 * kFrame);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(FrameValue(out, 7) == 8);
    std::vector<unsigned char> tail(out.begin() + 8 * kFrame, out.end());
    CHECK(AllZero(tail));
    CHECK(f.ring.Underruns == 0);
}

void TestUnalignedFillIsDiscardedOnUnderrun()
{
    Fixture f(64 * kFrame, 0, 64 * kFrame);
    auto in = Frames(0, 3);
    LoopbackRingWrite(&f.ring, in.data(), 3 * kFrame - 4);
    std::vector<unsigned char> out(4 * kFrame, 0xAA);
    LoopbackRingRead(&f.ring, out.data(), (unsigned int)out.size());
    CHECK(FrameValue(out, 0) == 1);
    CHECK(FrameValue(out, 1) == 2);
    std::vector<unsigned char> tail(out.begin() + 2 * kFrame, out.end());
    CHECK(AllZero(tail));
    CHECK(f.ring.Fill == 0);
    CHECK(f.ring.ReadOffset == f.ring.WriteOffset);
}

void TestUninitializedRingIsSilent()
{
    LOOPBACK_RING ring;
    std::memset(&ring, 0, sizeof(ring));
    std::vector<unsigned char> out(64, 0xAA);
    LoopbackRingWrite(&ring, out.data(), 64);
    LoopbackRingRead(&ring, out.data(), (unsigned int)out.size());
    CHECK(AllZero(out));
}

// A small ring with no priming, so reads return exactly what was written.
constexpr unsigned int kClientFrames = 16;
constexpr unsigned int kClientBytes = kClientFrames * kFrame;

Fixture DirectFixture()
{
    return Fixture(64 * kFrame, 0, 64 * kFrame);
}

std::vector<unsigned char> ReadFrames(LOOPBACK_RING * ring, unsigned int frameCount)
{
    std::vector<unsigned char> out(frameCount * kFrame);
    LoopbackRingRead(ring, out.data(), (unsigned int)out.size());
    return out;
}

void TestWriteFromCircularWraps()
{
    Fixture f = DirectFixture();
    auto client = Frames(0, kClientFrames);

    LoopbackRingWriteFromCircular(&f.ring, client.data(), kClientBytes, 3 * kClientBytes + 12 * kFrame, 8 * kFrame);

    auto out = ReadFrames(&f.ring, 8);
    CHECK(FrameValue(out, 0) == 13);
    CHECK(FrameValue(out, 3) == 16);
    CHECK(FrameValue(out, 4) == 1);
    CHECK(FrameValue(out, 7) == 4);
}

void TestWriteFromCircularPadsExcessWithSilence()
{
    Fixture f = DirectFixture();
    auto client = Frames(0, kClientFrames);

    LoopbackRingWriteFromCircular(&f.ring, client.data(), kClientBytes, 0, kClientBytes + 4 * kFrame);

    CHECK(f.ring.Fill == kClientBytes + 4 * kFrame);
    auto out = ReadFrames(&f.ring, kClientFrames + 4);
    for (unsigned int i = 0; i < 4; ++i)
    {
        CHECK(FrameValue(out, i) == 0);
    }
    CHECK(FrameValue(out, 4) == 5);
    CHECK(FrameValue(out, 15) == 16);
    CHECK(FrameValue(out, 16) == 1);
    CHECK(FrameValue(out, 19) == 4);
}

void TestWriteFromCircularUses64BitPosition()
{
    Fixture f = DirectFixture();
    auto client = Frames(0, kClientFrames);

    LoopbackRingWriteFromCircular(&f.ring, client.data(), kClientBytes, (1ull << 33) + 3 * kFrame, 2 * kFrame);

    auto out = ReadFrames(&f.ring, 2);
    CHECK(FrameValue(out, 0) == 4);
    CHECK(FrameValue(out, 1) == 5);
}

void TestReadToCircularWraps()
{
    Fixture f = DirectFixture();
    auto in = Frames(0, 8);
    LoopbackRingWrite(&f.ring, in.data(), (unsigned int)in.size());
    std::vector<unsigned char> client(kClientBytes, 0xAA);

    LoopbackRingReadToCircular(&f.ring, client.data(), kClientBytes, 12 * kFrame, 8 * kFrame);

    CHECK(FrameValue(client, 12) == 1);
    CHECK(FrameValue(client, 15) == 4);
    CHECK(FrameValue(client, 0) == 5);
    CHECK(FrameValue(client, 3) == 8);
    CHECK(client[4 * kFrame] == 0xAA);
    CHECK(client[12 * kFrame - 1] == 0xAA);
    CHECK(f.ring.Fill == 0);
}

void TestReadToCircularExcessDropsStaleAudio()
{
    Fixture f = DirectFixture();
    auto stale = Frames(0, 8);
    LoopbackRingWrite(&f.ring, stale.data(), (unsigned int)stale.size());
    std::vector<unsigned char> client(kClientBytes, 0xAA);

    LoopbackRingReadToCircular(&f.ring, client.data(), kClientBytes, 0, kClientBytes + 4 * kFrame);

    CHECK(AllZero(client));
    CHECK(f.ring.Fill == 0);
}

void TestCircularCopiesIgnoreEmptyBuffer()
{
    Fixture f = DirectFixture();
    unsigned char byte = 0xAA;

    LoopbackRingWriteFromCircular(&f.ring, &byte, 0, 0, kFrame);
    LoopbackRingReadToCircular(&f.ring, &byte, 0, 0, kFrame);

    CHECK(f.ring.Fill == 0);
    CHECK(byte == 0xAA);
}

} // namespace

int main()
{
    TestInitRejectsBadSizes();
    TestSilentUntilPrimed();
    TestPassesAudioInOrderAfterPriming();
    TestWrapAroundIsSeamless();
    TestUnderrunZeroFillsAndReprimes();
    TestOverrunKeepsNewestAudio();
    TestOverrunAcrossManyWrites();
    TestLatencyIsTrimmedToPrime();
    TestResetReaderDropsStaleAudio();
    TestNullSourceWritesSilence();
    TestUnalignedFillIsDiscardedOnUnderrun();
    TestUninitializedRingIsSilent();
    TestWriteFromCircularWraps();
    TestWriteFromCircularPadsExcessWithSilence();
    TestWriteFromCircularUses64BitPosition();
    TestReadToCircularWraps();
    TestReadToCircularExcessDropsStaleAudio();
    TestCircularCopiesIgnoreEmptyBuffer();

    if (g_failures != 0)
    {
        std::fprintf(stderr, "%d check(s) failed\n", g_failures);
        return EXIT_FAILURE;
    }
    std::printf("loopback ring: all tests passed\n");
    return EXIT_SUCCESS;
}
