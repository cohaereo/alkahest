/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2007-2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Memory management code
 *
 */

#include "umbraMemory.hpp"
#ifndef UMBRA_DISABLE_DEFAULT_ALLOCATOR_CHECKS
#   include "umbraThread.hpp"   // for TLS
#endif

#include <stdlib.h>

namespace Umbra
{

class SystemAllocator : public Allocator
{
public:
    SystemAllocator (void)
    {
#if !defined(UMBRA_DISABLE_DEFAULT_ALLOCATOR_CHECKS)
        m_tlsIndex = Thread::allocTls();
#endif
    }

    ~SystemAllocator (void)
    {
#if !defined(UMBRA_DISABLE_DEFAULT_ALLOCATOR_CHECKS)
        Thread::freeTls(m_tlsIndex);
#endif
    }

    void* allocate (size_t size, const char* info)
    {
        UMBRA_UNREF(info);
#if defined(UMBRA_DEBUG) && !defined(UMBRA_DISABLE_DEFAULT_ALLOCATOR_CHECKS)
        if ((m_tlsIndex != -1) && (Thread::getTlsValue(m_tlsIndex) != 0))
        {
            UMBRA_ASSERT(!"Default allocator use not permitted!");
            return NULL;
        }
#endif
        return malloc(size);
    }

    void deallocate  (void* ptr)
    {
#if defined(UMBRA_DEBUG) && !defined(UMBRA_DISABLE_DEFAULT_ALLOCATOR_CHECKS)
        if ((m_tlsIndex != -1) && (Thread::getTlsValue(m_tlsIndex) != 0))
        {
            UMBRA_ASSERT(!"Default allocator use not permitted!");
            return;
        }
#endif
        free(ptr);
    }

    bool permit (bool allow)
    {
#if !defined(UMBRA_DISABLE_DEFAULT_ALLOCATOR_CHECKS)
        bool prev = (Thread::getTlsValue(m_tlsIndex) == 0);
        Thread::setTlsValue(m_tlsIndex, allow ? 0 : 1);
        return prev;
#else
        return allow;
#endif
    }

private:
    int m_tlsIndex;
};

struct DefaultAllocMem
{
    char mem[sizeof(SystemAllocator)];
    int initialized;
};

static DefaultAllocMem g_defaultAllocator;

Allocator* getAllocator (void)
{
    DefaultAllocMem* mem = &g_defaultAllocator;
    if (!mem->initialized)
    {
      new ((void*)mem->mem) SystemAllocator();
        mem->initialized = true;
    }
    return (SystemAllocator*)mem;
}

#ifdef UMBRA_DEBUG
bool allowDefaultAllocator (bool allow)
{
    SystemAllocator* alloc = (SystemAllocator*)getAllocator();
    return alloc->permit(allow);
}
#endif

}

//------------------------------------------------------------------------
