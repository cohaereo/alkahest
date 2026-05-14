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
 * \brief   Umbra Pool Allocator
 *
 */

#include "umbraPoolAllocator.hpp"

using namespace Umbra;

BasePoolAllocator::Page* BasePoolAllocator::allocatePage()
{
    //--------------------------------------------------------------------
    // Determine how much memory we want to allocate
    //--------------------------------------------------------------------

    size_t sz = m_numItems;

    if (sz < 32)
        sz = 32;

    if ((sz*getSizeOfItem()) > (8192-UMBRA_CACHE_LINE_SIZE))
    {
        sz = (8192-UMBRA_CACHE_LINE_SIZE) / getSizeOfItem();
        if (sz < 1)
            sz = 1;
    }

    size_t allocBytes = sizeof(Page) + getSizeOfItem()*sz + (UMBRA_CACHE_LINE_SIZE-1);

    //--------------------------------------------------------------------
    // Allocate memory for both the page structure and the elements
    //--------------------------------------------------------------------

    unsigned char* ptr = UMBRA_NEW_ARRAY(unsigned char, allocBytes);
    Page* p = reinterpret_cast<Page*>(ptr);


    p->m_freeEntries    = sz;
    p->m_firstItem      = reinterpret_cast<Item*>(alignCacheLine(p+1));
    p->m_next           = m_firstPage;

    m_firstPage = p;
    m_numItems += sz;
    m_memUsed  += allocBytes;

    return p;
}


void BasePoolAllocator::removeAll (void)
{
    Page* p = m_firstPage;
    while (p)
    {
        Page*           next = p->m_next;
        unsigned char*  d = reinterpret_cast<unsigned char*>(p);
        UMBRA_DELETE_ARRAY(d);
        p = next;
    }
    m_firstPage = 0;
    m_firstFree = 0;
    m_numItems  = 0;
    m_memUsed   = 0;
}


// This is extremely slow -- so use it only for debug build checks (!!!!)
bool BasePoolAllocator::isEmpty (void) const
{
    size_t cnt = 0;
    for (Item* i = m_firstFree; i; i = i->m_next)
        cnt++;
    for (Page* p = m_firstPage; p; p = p->m_next)
        cnt += p->m_freeEntries;

    return (cnt == m_numItems);
}

