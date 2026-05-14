#pragma once

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2012 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Object pool allocator
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraPlatform.hpp"
#include "umbraMemory.hpp"

namespace Umbra
{

/*!
 * \brief Object pool allocator that implements Allocator interface, suitable
 *        for multithreaded contexts.
 */
template <size_t DATA_SIZE>
class BaseObjectPool : public Allocator, public Base
{
public:
    BaseObjectPool(Allocator* allocator, size_t pageSize = 16384)
        : Base(allocator)
        , m_rootPage(NULL)
        , m_firstFree(NULL)
    {
        m_pageSize = pageSize + (align8(DATA_SIZE) - (pageSize % align8(DATA_SIZE)));
    }

    virtual ~BaseObjectPool();

    void* allocate(size_t size, const char* info = NULL);
    void deallocate(void* ptr);

private:
    BaseObjectPool(const BaseObjectPool&);
    BaseObjectPool& operator=(const BaseObjectPool&);

    static inline size_t align8(size_t x)
    {
        return (x + 7) & ~7;
    }

    void makeNewPage()
    {
        Page* newPage = (Page*)UMBRA_MALLOC(align8(sizeof(Page)) + m_pageSize);
        UMBRA_ASSERT(((UINTPTR)newPage & 7) == 0);
        newPage->m_size = m_pageSize;
        newPage->m_nextPage = m_rootPage;
        newPage->m_numUsed = 0;
        m_rootPage = newPage;
    }

    struct Page
    {
        // Return whether there are slots that have never been touched, will
        // not report slots that were allocated and later released.
        bool hasSlotsRemaining() const
        {
            return m_size - m_numUsed * align8(DATA_SIZE) >= align8(DATA_SIZE);
        }

        void* consumeSlot()
        {
            UMBRA_ASSERT(hasSlotsRemaining());
            void* result = (void*)((char*)this + align8(sizeof(Page)) + align8(DATA_SIZE) * m_numUsed);
            m_numUsed++;
            return result;
        }

        Page*  m_nextPage;
        size_t m_size;
        int    m_numUsed;
    };

    size_t m_pageSize;
    Page*  m_rootPage;
    void*  m_firstFree;
};

template <size_t DATA_SIZE>
BaseObjectPool<DATA_SIZE>::~BaseObjectPool()
{
    while (m_rootPage)
    {
        Page* next = m_rootPage->m_nextPage;
        UMBRA_FREE(m_rootPage);
        m_rootPage = next;
    }
}

template <size_t DATA_SIZE>
void* BaseObjectPool<DATA_SIZE>::allocate(size_t size, const char* info)
{
    UMBRA_UNREF(size);
    UMBRA_UNREF(info);

    // Even though this has Allocator interface, it can only be used to
    // allocate items up to the data block size. Smaller sizes work as
    // well of course.
    UMBRA_ASSERT(size <= DATA_SIZE);

    if (!m_rootPage)
        makeNewPage();

    // Recycle a previously used slot if possible.
    if (m_firstFree)
    {
        void* result = m_firstFree;
        m_firstFree = *((void**)m_firstFree);
        return result;
    }

    // If there are no new slots left on the latest page, allocate a new page.
    if (!m_rootPage->hasSlotsRemaining())
        makeNewPage();

    UMBRA_ASSERT(m_rootPage->hasSlotsRemaining());

    // Use a new slot from the latest page.
    return m_rootPage->consumeSlot();
}

template <size_t DATA_SIZE>
void BaseObjectPool<DATA_SIZE>::deallocate(void* ptr)
{
    // Write the next link in the reuse chain in the unused slot.
    *((void**)ptr) = m_firstFree;
    m_firstFree = ptr;
}

template <class C>
class ObjectPool : public BaseObjectPool<sizeof(C)>
{
public:
    ObjectPool(Allocator* allocator, size_t pageSize = 16384)
        : BaseObjectPool<sizeof(C)>(allocator, pageSize)
    {}
};

}
