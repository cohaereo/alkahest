#ifndef UMBRAPOOLALLOCATOR_HPP
#define UMBRAPOOLALLOCATOR_HPP
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

#include "umbraPrivateDefs.hpp"
#include "umbraMemory.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief           Pool allocator class
 *
 * \note            The pool allocator class is a "zero overhead" allocator.
 *                  This means that there is no header section per allocation.
 *                  The downside is that the entire pool is released only
 *                  when the last allocation is released. Use the
 *                  "FixedAllocator" class if you want a more dynamic allocation
 *                  system.
 *//*-------------------------------------------------------------------*/


class BasePoolAllocator : public Base
{
private:
    struct Page;
    friend struct Page;

    struct Item
    {
        Item*   m_next;                 //!< pointer to next (if in free list)
    };
    Page*               allocatePage        (void);

                        BasePoolAllocator   (const BasePoolAllocator&); //!< not allowed
    BasePoolAllocator&  operator=           (const BasePoolAllocator&); //!< not allowed
    size_t              getSizeOfItem       (void)                       { return m_sizeOfItem;  }

    struct Page
    {
        Page*           m_next;         //!< next page in linked list
        Item*           m_firstItem;    //!< ptr to first item
        size_t          m_freeEntries;  //!< # of free items available on page
    };

    Page*   m_firstPage;                //!< first page (this is always the active one)
    Item*   m_firstFree;                //!< first free item
    size_t  m_numItems;                 //!< num items
    size_t  m_memUsed;                  //!< total amount of memory used
    int     m_sizeOfItem;
public:
                                BasePoolAllocator   (Allocator* a, size_t sizeOfItem) : Base(a), m_firstPage(0),m_firstFree(0),m_numItems(0),m_memUsed(0),m_sizeOfItem((int)(sizeOfItem<sizeof(Item*) ? sizeof(Item*) : sizeOfItem)) {}
                                ~BasePoolAllocator  (void)          { removeAll(); }
    UMBRA_FORCE_INLINE void*        allocate            (void)
    {
        Item* item = m_firstFree;
        if (!item)
        {
            Page* p = m_firstPage;
            if (!p || !p->m_freeEntries)
                p = allocatePage();
            p->m_freeEntries--;
            item = reinterpret_cast<Item*>(reinterpret_cast<char*>(p->m_firstItem) + p->m_freeEntries*getSizeOfItem());
        }
        else
            m_firstFree = item->m_next;
        return item;
    }
    UMBRA_FORCE_INLINE void     free                (void* t)       { Item* item = reinterpret_cast<Item*>(t); item->m_next = m_firstFree; m_firstFree = item; }
    bool                        isEmpty             (void) const;   //!< This is extremely slow -- so use it only for debug build checks (!!!!)
    UMBRA_FORCE_INLINE size_t   getMemUsed          (void) const    { return m_memUsed; }
    void                        removeAll           (void);
};

template <class T> class PoolAllocator : private BasePoolAllocator
{
public:
    UMBRA_FORCE_INLINE          PoolAllocator   (Allocator* a) : BasePoolAllocator(a, sizeof(T)) {}
    UMBRA_FORCE_INLINE          ~PoolAllocator  (void)          {}
    UMBRA_FORCE_INLINE T*       allocate        (void)          { return (T*)BasePoolAllocator::allocate(); }
    UMBRA_FORCE_INLINE void     removeAll       (void)          { BasePoolAllocator::removeAll(); }
    UMBRA_FORCE_INLINE size_t   getMemUsed      (void) const    { return BasePoolAllocator::getMemUsed(); }
    UMBRA_FORCE_INLINE void     free            (T* t)          { BasePoolAllocator::free(t); }
    UMBRA_FORCE_INLINE bool     isEmpty         (void)          { return BasePoolAllocator::isEmpty(); }
private:
                                PoolAllocator   (const PoolAllocator&); //!< not allowed
    PoolAllocator&              operator=       (const PoolAllocator&); //!< not allowed

};

} // Umbra

//------------------------------------------------------------------------
#endif // UMBRAPOOLALLOCATOR_HPP
