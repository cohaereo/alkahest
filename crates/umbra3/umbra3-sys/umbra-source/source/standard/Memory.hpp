// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Portability.hpp>
#include <standard/IntTypes.hpp>
#include <standard/Assert.hpp>
#include <string.h>
#include <new>

namespace Umbra
{

// standard operations on raw memory

class Mem
{
public:
    static UMBRA_FORCE_INLINE void* copy(void* dst, void* src, uint32_t numBytes)
    {
        return memcpy(dst, src, numBytes);
    }

    static UMBRA_FORCE_INLINE void* move(void* dst, void* src, uint32_t numBytes)
    {
        return memmove(dst, src, numBytes);
    }

    static UMBRA_FORCE_INLINE void* set(void* dst, char val, uint32_t numBytes)
    {
        return memset(dst, val, numBytes);
    }

    static UMBRA_FORCE_INLINE void* zero(void* dst, uint32_t numBytes)
    {
        return memset(dst, 0, numBytes);
    }

    static UMBRA_FORCE_INLINE bool isAlignedLog2(void* ptr, uint32_t log2Align)
    {
        return !(reinterpret_cast<uintptr_t>(ptr) & ((1 << log2Align) - 1));
    }

    static UMBRA_FORCE_INLINE bool isAligned(void* ptr, uint32_t align)
    {
        return (reinterpret_cast<uintptr_t>(ptr) % align) == 0;
    }
};

// Out of memory exception type, thrown when NULL pointer returned from system/user

#if UMBRA_EXCEPTIONS_SUPPORTED
class OutOfMemoryException {};
#endif


// c++ object creation and deletion with Umbra Memory managers

#define UMBRA_NEW_ALLOCINFO(C) __FILE__ ":" UMBRA_STRINGIFY(__LINE__) #C
#define M_NEW(M, C, ...) (new ((M).alloc(sizeof(C), UMBRA_ALIGNOF(C), UMBRA_NEW_ALLOCINFO(C))) C (__VA_ARGS__))
#define M_DELETE(M, C, p) do { if (p) {(p)->~C(); (M).dealloc(p);} } while ((void)0, 0)


// memory manager interface

class MemoryManager
{
public:
    MemoryManager()
    {
        Mem::set(&m_stats, 0, sizeof(AllocStats));
    }
    virtual ~MemoryManager()
    {
        UMBRA_ASSERT(m_stats.currentAllocCount == 0);
    }

    struct AllocStats
    {
        uint32_t currentAllocCount;
        uint32_t peakAllocCount;
        uint32_t currentAllocatedSize;
        uint32_t peakAllocatedSize;
    };

    virtual void* alloc(uint32_t numBytes, uint32_t requiredAlignment = 0, const char* info = NULL) = 0;
    virtual void dealloc(void* current) = 0;
    const AllocStats& getStats(void) const { return m_stats; }

protected:
    AllocStats m_stats;
};

// standard memory managers and wrappers

// thin wrapper around std malloc/free
class SystemMemoryManager: public MemoryManager
{
public:
    virtual ~SystemMemoryManager() {}
    void* alloc(uint32_t numBytes, uint32_t requiredAlignment = 0, const char* info = NULL);
    void dealloc(void* current);
};

class Allocator;

// Wrapper that forwards to public interface Allocator
// Takes care of alignment (user alloc alignment can be arbitrary).
// Tracks allocations so that they can be freed in the destructor.
class UserMemoryManager: public MemoryManager
{
public:
    UserMemoryManager(Allocator* userAllocator): m_allocator(userAllocator)
    {
        m_head.next = m_head.prev = &m_head;
    }
    virtual ~UserMemoryManager();

    void* alloc(uint32_t numBytes, uint32_t requiredAlignment = 0, const char* info = NULL);
    void dealloc(void* current);

    struct AllocListNode
    {
        AllocListNode* prev;
        AllocListNode* next;
    };

private:

    Allocator* m_allocator;
    AllocListNode m_head;
};

} // namespace Umbra
