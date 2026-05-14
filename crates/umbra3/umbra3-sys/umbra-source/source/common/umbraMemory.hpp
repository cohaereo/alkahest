#pragma once

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
 * \brief   Memory management
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraPlatform.hpp"
#include <standard/Memory.hpp>
#if UMBRA_COMPILER == UMBRA_MSC
#include <new.h>
#else
#include <new>
#endif

/*
Enable the printCallStack function.
Warning: needs dbghelp.dll on windows, which might not be present on
non-development systems or some windows versions.
*/
#define UMBRA_ENABLE_CALLSTACK  0

namespace Umbra
{

void printCallStack(Logger* logger = NULL);

#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
typedef OutOfMemoryException OOMException;
#endif

template<>
inline void* allocThrow (Allocator* a, size_t s, const char* info)
{
    UMBRA_ASSERT(a);

    void* ptr = a->allocate(s, info);
#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    if (!ptr)
    {
#if UMBRA_ENABLE_CALLSTACK
        printf("allocation failed: size %u\n", s);
        printCallStack();
#endif
        throw OOMException();
    }
#endif

    return ptr;
}

// The default allocator implementation, used when there's nothing better
// available. The intention is to always use a user-supplied allocator so
// in the future we might remove this altogether.

Allocator* getAllocator (void);

// Control whether allocations from the default allocator are permitted for
// _this_thread_, defaults to yes. Returns old value. Only supported in
// debug builds, does nothing in release

#ifdef UMBRA_DEBUG
bool allowDefaultAllocator (bool allow);
#else
static inline bool allowDefaultAllocator (bool a)
{
    UMBRA_UNREF(a);
    return false;
}
#endif

class AllowDefaultAllocatorForScope
{
public:
    AllowDefaultAllocatorForScope(bool state)
    {
        m_prevState = allowDefaultAllocator(state);
    }
    ~AllowDefaultAllocatorForScope()
    {
        allowDefaultAllocator(m_prevState);
    }
private:
    bool m_prevState;
};

// Override elem to propagate heap

static inline void copyHeap (void* elem, Allocator* heap)
{
    UMBRA_UNREF(elem);
    UMBRA_UNREF(heap);
}

// Umbra base class, for elements allocated through Umbra::Allocator.

class Base
{
public:
    Allocator*  getAllocator (void) const { return m_heap; }
    void        setAllocator (Allocator* alloc)
    {
        m_heap = alloc;
        if (!m_heap)
            m_heap = Umbra::getAllocator();
    }

protected:
    Base (Allocator* heap = NULL)
    {
        setAllocator(heap);
    }

    template<typename T>
    inline T* newArray(void* ptr, int n)
    {
#if defined(UMBRA_COMP_NO_EXCEPTIONS)
        if (!ptr)
            return NULL;
#endif
        int* iptr = (int*)ptr;
        *iptr = n;
        T* t = (T*)(iptr+4);
        for (int i = 0; i < n; i++)
        {
            new (&t[i]) T;
            copyHeap(&t[i], m_heap);
        }
        return t;
    }

private:
    Allocator*    m_heap;
};

static inline void copyHeap (Base* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

class UserCallbackAllocator: public Allocator
{
public:
    UserCallbackAllocator(Allocator* a): m_forward(a)
    {
    }

    void*   allocate    (size_t size, const char* info)
    {
        bool prev = allowDefaultAllocator(true);
        void* ret = m_forward->allocate(size, info);
        allowDefaultAllocator(prev);
        return ret;
    }
    void    deallocate  (void* ptr)
    {
        bool prev = allowDefaultAllocator(true);
        m_forward->deallocate(ptr);
        allowDefaultAllocator(prev);
    }

    Allocator* getWrappedAllocator() const { return m_forward; }

private:
    Allocator* m_forward;
};

#if UMBRA_ENABLE_CALLSTACK && (UMBRA_OS == UMBRA_WINDOWS)
#pragma comment( lib, "dbghelp" )
#endif

}
