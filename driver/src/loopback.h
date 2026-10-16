// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Audio Program contributors
//
// One cable: a spin-lock protected LOOPBACK_RING shared by the cable's render
// stream (writer) and capture stream (reader). The adapter owns the storage;
// wave miniports keep the adapter alive while they hold a pointer to it.

#ifndef _AUDIOPROGRAMCABLE_LOOPBACK_H_
#define _AUDIOPROGRAMCABLE_LOOPBACK_H_

#include "loopback_ring.h"

class CLoopbackCable
{
public:
    // Storage is zeroed by the pool allocator before Initialize is called.
    NTSTATUS Initialize()
    {
        PAGED_CODE();

        const ULONG capacity = CABLE_RING_MS * CABLE_BYTES_PER_MS;

        KeInitializeSpinLock(&m_Lock);
        m_Buffer = (unsigned char *)ExAllocatePool2(POOL_FLAG_NON_PAGED, capacity, CABLE_POOLTAG);
        if (m_Buffer == NULL)
        {
            return STATUS_INSUFFICIENT_RESOURCES;
        }

        if (!LoopbackRingInit(&m_Ring,
                              m_Buffer,
                              capacity,
                              CABLE_BLOCK_ALIGN,
                              CABLE_PRIME_MS * CABLE_BYTES_PER_MS,
                              CABLE_MAX_LATENCY_MS * CABLE_BYTES_PER_MS))
        {
            ExFreePoolWithTag(m_Buffer, CABLE_POOLTAG);
            m_Buffer = NULL;
            return STATUS_INVALID_PARAMETER;
        }

        return STATUS_SUCCESS;
    }

    void Release()
    {
        PAGED_CODE();

        if (m_Buffer != NULL)
        {
            ExFreePoolWithTag(m_Buffer, CABLE_POOLTAG);
            m_Buffer = NULL;
        }
        RtlZeroMemory(&m_Ring, sizeof(m_Ring));
    }

    // Render side. Callable at IRQL <= DISPATCH_LEVEL.
    void Write(_In_reads_bytes_opt_(Bytes) const BYTE * Source, _In_ ULONG Bytes)
    {
        KIRQL irql;
        KeAcquireSpinLock(&m_Lock, &irql);
        LoopbackRingWrite(&m_Ring, Source, Bytes);
        KeReleaseSpinLock(&m_Lock, irql);
    }

    // Capture side. Always fills Bytes. Callable at IRQL <= DISPATCH_LEVEL.
    void Read(_Out_writes_bytes_(Bytes) BYTE * Destination, _In_ ULONG Bytes)
    {
        KIRQL irql;
        KeAcquireSpinLock(&m_Lock, &irql);
        LoopbackRingRead(&m_Ring, Destination, Bytes);
        KeReleaseSpinLock(&m_Lock, irql);
    }

    void ResetReader()
    {
        KIRQL irql;
        KeAcquireSpinLock(&m_Lock, &irql);
        LoopbackRingResetReader(&m_Ring);
        KeReleaseSpinLock(&m_Lock, irql);
    }

private:
    KSPIN_LOCK      m_Lock;
    LOOPBACK_RING   m_Ring;
    unsigned char * m_Buffer;
};

#endif // _AUDIOPROGRAMCABLE_LOOPBACK_H_
