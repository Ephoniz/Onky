// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Audio Program contributors
//
// Byte ring that carries one cable's audio from its render stream to its
// capture stream. It has no locking, no allocation and no OS dependencies so
// the same code runs in the driver (under a spin lock) and in host unit tests.
//
// Policy, in frames of the cable format:
//   - The reader stays silent until PrimeBytes are buffered, then plays.
//   - On underrun the missing tail is zero-filled and the reader re-primes.
//   - If the backlog grows past MaxLatencyBytes, the reader skips ahead so
//     only PrimeBytes remain.
//   - The writer never blocks: when the ring is full the oldest bytes go.

#ifndef AUDIO_PROGRAM_LOOPBACK_RING_H_
#define AUDIO_PROGRAM_LOOPBACK_RING_H_

#include <string.h>

struct LOOPBACK_RING
{
    unsigned char * Buffer;
    unsigned int    Capacity;         // bytes, multiple of FrameBytes
    unsigned int    FrameBytes;
    unsigned int    PrimeBytes;
    unsigned int    MaxLatencyBytes;

    unsigned int    ReadOffset;
    unsigned int    WriteOffset;
    unsigned int    Fill;
    bool            Primed;

    // Counters for diagnostics and tests.
    unsigned long long Underruns;
    unsigned long long Overruns;
    unsigned long long LatencyTrims;
};

inline unsigned int LoopbackRingFloorFrames(const LOOPBACK_RING * Ring, unsigned int Bytes)
{
    return Bytes - (Bytes % Ring->FrameBytes);
}

// Buffer must hold Capacity bytes. Returns false on inconsistent sizes.
inline bool LoopbackRingInit
(
    LOOPBACK_RING * Ring,
    unsigned char * Buffer,
    unsigned int    Capacity,
    unsigned int    FrameBytes,
    unsigned int    PrimeBytes,
    unsigned int    MaxLatencyBytes
)
{
    memset(Ring, 0, sizeof(*Ring));

    if (Buffer == nullptr || FrameBytes == 0 || Capacity < FrameBytes)
    {
        return false;
    }
    if ((Capacity % FrameBytes) != 0 || (PrimeBytes % FrameBytes) != 0 ||
        (MaxLatencyBytes % FrameBytes) != 0)
    {
        return false;
    }
    if (PrimeBytes > MaxLatencyBytes || MaxLatencyBytes > Capacity)
    {
        return false;
    }

    Ring->Buffer = Buffer;
    Ring->Capacity = Capacity;
    Ring->FrameBytes = FrameBytes;
    Ring->PrimeBytes = PrimeBytes;
    Ring->MaxLatencyBytes = MaxLatencyBytes;
    memset(Buffer, 0, Capacity);
    return true;
}

inline void LoopbackRingSkip(LOOPBACK_RING * Ring, unsigned int Bytes)
{
    if (Bytes > Ring->Fill)
    {
        Bytes = Ring->Fill;
    }
    Ring->ReadOffset = (Ring->ReadOffset + Bytes) % Ring->Capacity;
    Ring->Fill -= Bytes;
}

// Drops everything buffered. Used when a capture stream starts so it never
// replays audio that was written before it was listening.
inline void LoopbackRingResetReader(LOOPBACK_RING * Ring)
{
    Ring->ReadOffset = Ring->WriteOffset;
    Ring->Fill = 0;
    Ring->Primed = false;
}

// Source may be nullptr to write silence.
inline void LoopbackRingWrite(LOOPBACK_RING * Ring, const unsigned char * Source, unsigned int Bytes)
{
    if (Ring->Buffer == nullptr || Bytes == 0)
    {
        return;
    }

    bool lapped = false;
    if (Bytes > Ring->Capacity)
    {
        unsigned int excess = Bytes - Ring->Capacity;
        if (Source != nullptr)
        {
            Source += excess;
        }
        Bytes = Ring->Capacity;
        Ring->WriteOffset = (Ring->WriteOffset + excess) % Ring->Capacity;
        lapped = true;
    }

    unsigned int remaining = Bytes;
    while (remaining > 0)
    {
        unsigned int run = Ring->Capacity - Ring->WriteOffset;
        if (run > remaining)
        {
            run = remaining;
        }
        if (Source != nullptr)
        {
            memcpy(Ring->Buffer + Ring->WriteOffset, Source, run);
            Source += run;
        }
        else
        {
            memset(Ring->Buffer + Ring->WriteOffset, 0, run);
        }
        Ring->WriteOffset = (Ring->WriteOffset + run) % Ring->Capacity;
        remaining -= run;
    }

    unsigned long long fill = (unsigned long long)Ring->Fill + Bytes;
    if (lapped || fill > Ring->Capacity)
    {
        // The writer lapped the reader: the oldest bytes were overwritten.
        Ring->Overruns++;
        Ring->Fill = Ring->Capacity;
        Ring->ReadOffset = Ring->WriteOffset;
    }
    else
    {
        Ring->Fill = (unsigned int)fill;
    }
}

// Always fills Bytes of Destination, with silence where no audio is available.
inline void LoopbackRingRead(LOOPBACK_RING * Ring, unsigned char * Destination, unsigned int Bytes)
{
    if (Bytes == 0)
    {
        return;
    }
    if (Ring->Buffer == nullptr)
    {
        memset(Destination, 0, Bytes);
        return;
    }

    if (!Ring->Primed)
    {
        if (Ring->Fill < Ring->PrimeBytes || Ring->Fill == 0)
        {
            memset(Destination, 0, Bytes);
            return;
        }
        Ring->Primed = true;
    }

    if (Ring->Fill > Ring->MaxLatencyBytes)
    {
        Ring->LatencyTrims++;
        LoopbackRingSkip(Ring, LoopbackRingFloorFrames(Ring, Ring->Fill - Ring->PrimeBytes));
    }

    unsigned int available = Ring->Fill < Bytes ? LoopbackRingFloorFrames(Ring, Ring->Fill) : Bytes;
    unsigned int copied = 0;
    while (copied < available)
    {
        unsigned int run = Ring->Capacity - Ring->ReadOffset;
        if (run > available - copied)
        {
            run = available - copied;
        }
        memcpy(Destination + copied, Ring->Buffer + Ring->ReadOffset, run);
        Ring->ReadOffset = (Ring->ReadOffset + run) % Ring->Capacity;
        Ring->Fill -= run;
        copied += run;
    }

    if (copied < Bytes)
    {
        memset(Destination + copied, 0, Bytes - copied);
        Ring->Underruns++;
        // Discard any partial frame so the next read starts frame-aligned.
        LoopbackRingSkip(Ring, Ring->Fill);
        Ring->Primed = false;
    }
}

// Render: copies Bytes of a circular client buffer, starting at the linear
// byte Position, into the ring. Only the last BufferSize bytes can still be
// in the buffer, so any excess is written as silence first to keep timing.
inline void LoopbackRingWriteFromCircular
(
    LOOPBACK_RING *         Ring,
    const unsigned char *   Buffer,
    unsigned int            BufferSize,
    unsigned long long      Position,
    unsigned int            Bytes
)
{
    if (BufferSize == 0)
    {
        return;
    }
    if (Bytes > BufferSize)
    {
        unsigned int skipped = Bytes - BufferSize;
        LoopbackRingWrite(Ring, nullptr, skipped);
        Position += skipped;
        Bytes = BufferSize;
    }

    unsigned int offset = (unsigned int)(Position % BufferSize);
    while (Bytes > 0)
    {
        unsigned int run = BufferSize - offset;
        if (run > Bytes)
        {
            run = Bytes;
        }
        LoopbackRingWrite(Ring, Buffer + offset, run);
        offset = (offset + run) % BufferSize;
        Bytes -= run;
    }
}

// Capture: fills Bytes of a circular client buffer, starting at the linear
// byte Position, from the ring. If Bytes exceeds the buffer, the reader
// restarts and only the last BufferSize bytes are filled.
inline void LoopbackRingReadToCircular
(
    LOOPBACK_RING *         Ring,
    unsigned char *         Buffer,
    unsigned int            BufferSize,
    unsigned long long      Position,
    unsigned int            Bytes
)
{
    if (BufferSize == 0)
    {
        return;
    }
    if (Bytes > BufferSize)
    {
        LoopbackRingResetReader(Ring);
        Position += Bytes - BufferSize;
        Bytes = BufferSize;
    }

    unsigned int offset = (unsigned int)(Position % BufferSize);
    while (Bytes > 0)
    {
        unsigned int run = BufferSize - offset;
        if (run > Bytes)
        {
            run = Bytes;
        }
        LoopbackRingRead(Ring, Buffer + offset, run);
        offset = (offset + run) % BufferSize;
        Bytes -= run;
    }
}

#endif // AUDIO_PROGRAM_LOOPBACK_RING_H_
