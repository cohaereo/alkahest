// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include <standard/Memory.hpp>
#include <standard/BitwiseOps.hpp>
#include <umbraPlatform.hpp>
#include <stdlib.h>

// minimum alignment requirement for all memory allocations
#define UMBRA_MIN_ALLOCATION_ALIGNMENT_LOG2 0x2u
#define UMBRA_MIN_ALLOCATION_ALIGNMENT (1 << UMBRA_MIN_ALLOCATION_ALIGNMENT_LOG2)

namespace Umbra
{

static const uint32_t AllocGuard = 0xFCED1833u;
static const uint32_t AllocPadding = 0xFFF2FFE3u;

template <class Header>
static inline uint32_t getAlignedAllocSize(uint32_t size, uint32_t align)
{
    return size + align + sizeof(Header);
}

struct MallocImpl
{
    struct AllocHeader
    {
        uint32_t m_size;
        uint32_t m_guard;
    };


    void* alloc(uint32_t bytes, const char*) const { return malloc(bytes); }
    void dealloc(void* ptr) const { free(ptr); }
};

UMBRA_CT_ASSERT_MSG(sizeof(MallocImpl::AllocHeader) == 8, "Make sure that header is not padded");

struct UserAllocImpl
{
    struct AllocHeader
    {
        UserMemoryManager::AllocListNode m_node;
        uint32_t m_size;
        uint32_t m_guard;
    };

    UserAllocImpl(Allocator& userAlloc): m_userAlloc(userAlloc) {}

    void* alloc(uint32_t bytes, const char* info) const { return m_userAlloc.allocate(bytes, info); }
    void dealloc(void* ptr) const { m_userAlloc.deallocate(ptr); }

    Allocator& m_userAlloc;
private:
    UserAllocImpl& operator= (const UserAllocImpl&);
};

UMBRA_CT_ASSERT_MSG(sizeof(UserAllocImpl::AllocHeader) == (2*sizeof(void*) + 8), "Make sure that header is not padded");

template <class Impl>
static inline void* AlignedAlloc(const Impl& allocImpl, uint32_t numBytes, uint32_t requiredAlignment,
    const char* info, MemoryManager::AllocStats& stats)
{
    typedef typename Impl::AllocHeader Header;

    uint32_t alignLog2 = UMBRA_MIN_ALLOCATION_ALIGNMENT_LOG2;
    if (requiredAlignment > UMBRA_MIN_ALLOCATION_ALIGNMENT)
    {
        alignLog2 = log2Base(requiredAlignment - 1) + 1;
    }
    uint32_t alignedSize = getAlignedAllocSize<Header>(numBytes, 1 << alignLog2);

    Header* header = (Header*)allocImpl.alloc(alignedSize, info);
    if (!header)
    {
#if UMBRA_EXCEPTIONS_SUPPORTED
        throw OutOfMemoryException();
#else
        return NULL;
#endif
    }
    header->m_size = numBytes;
    header->m_guard = AllocGuard;
    uint32_t* ptr = (uint32_t*)(header + 1);
    while (!Mem::isAlignedLog2(ptr, alignLog2))
    {
        *ptr++ = AllocPadding;
    }
    UMBRA_ASSERT(header->m_guard == AllocGuard);
    stats.currentAllocCount++;
    stats.peakAllocCount = max2(stats.peakAllocCount, stats.currentAllocCount);
    stats.currentAllocatedSize += numBytes;
    stats.peakAllocatedSize = max2(stats.peakAllocatedSize, stats.currentAllocatedSize);
    return ptr;
}

template <class Impl>
static inline typename Impl::AllocHeader* GetHeader(void* ptr)
{
    UMBRA_ASSERT(ptr);
    uint32_t* p = (uint32_t*)ptr;
    while (*(p - 1) == AllocPadding)
        --p;
    typename Impl::AllocHeader* hdr = ((typename Impl::AllocHeader*)p) - 1;
    UMBRA_ASSERT(hdr->m_guard == AllocGuard);
    return hdr;
}

template <class Impl>
static inline void AlignedDealloc(const Impl& allocFuncs, typename Impl::AllocHeader* header, MemoryManager::AllocStats& stats)
{
    UMBRA_ASSERT(stats.currentAllocCount >= 1);
    UMBRA_ASSERT(stats.currentAllocatedSize >= header->m_size);
    stats.currentAllocCount--;
    stats.currentAllocatedSize -= header->m_size;
    allocFuncs.dealloc(header);
}

void* SystemMemoryManager::alloc(uint32_t numBytes, uint32_t requiredAlignment, const char* info)
{
    return AlignedAlloc(MallocImpl(), numBytes, requiredAlignment, info, m_stats);
}

void SystemMemoryManager::dealloc(void* ptr)
{
    if (ptr)
        AlignedDealloc(MallocImpl(), GetHeader<MallocImpl>(ptr), m_stats);
}

void* UserMemoryManager::alloc(uint32_t numBytes, uint32_t requiredAlignment, const char* info)
{
    // alloc
    void* ret = AlignedAlloc(UserAllocImpl(*m_allocator), numBytes, requiredAlignment, info, m_stats);
    // add to list
    UserAllocImpl::AllocHeader* header = GetHeader<UserAllocImpl>(ret);
    header->m_node.next = m_head.next;
    header->m_node.next->prev = &header->m_node;
    header->m_node.prev = &m_head;
    header->m_node.prev->next = &header->m_node;
    return ret;
}

void UserMemoryManager::dealloc(void* ptr)
{
    if (!ptr)
        return;
    // remove from list
    UserAllocImpl::AllocHeader* header = GetHeader<UserAllocImpl>(ptr);
    header->m_node.next->prev = header->m_node.prev;
    header->m_node.prev->next = header->m_node.next;
    // free
    AlignedDealloc(UserAllocImpl(*m_allocator), header, m_stats);
}

UserMemoryManager::~UserMemoryManager()
{
    // TODO: assert/report leaks here?
    AllocListNode* n = m_head.next;
    while (n != &m_head)
    {
        AllocListNode* next = n->next;
        UserAllocImpl::AllocHeader* header = (UserAllocImpl::AllocHeader*)((uintptr_t)n - offsetof(UserAllocImpl::AllocHeader, m_node));
        AlignedDealloc(UserAllocImpl(*m_allocator), header, m_stats);
        n = next;
    }
}

} // namespace Umbra