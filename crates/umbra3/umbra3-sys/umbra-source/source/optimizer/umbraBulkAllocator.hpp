// Copyright (c) 2010-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRABULKALLOCATOR_HPP
#define UMBRABULKALLOCATOR_HPP

#include "umbraPrivateDefs.hpp"
#include "umbraMemory.hpp" 

namespace Umbra
{

//------------------------------------------------------------------------

template <typename T, int POOL_SIZE = 4096> 
class BulkAllocator 
{
    //------------------------------------------------------------------------

	struct Page
	{	
		Page(Allocator* allocator) 
        { 
            UMBRA_ASSERT(allocator); 

            m_pool = (T*)allocator->allocate(sizeof(T)*POOL_SIZE); 
            m_release = POOL_SIZE; 
        }

		T*			m_pool; 
		int			m_release; 
		Page*		m_next; 
	};

    //------------------------------------------------------------------------

    struct Item 
    { 
        Item(void) 
        :   m_next  (0) 
        {
        } 

        Item* m_next; 
    }; 
    
    //------------------------------------------------------------------------

public: 

	BulkAllocator(Allocator* allocator = NULL)
    :   m_head      (NULL)
    ,   m_freeList  (NULL)
    ,   m_allocator (allocator)
    {
        UMBRA_ASSERT(sizeof(T) >= sizeof(Item)); 

        if (!allocator) 
            m_allocator = getAllocator(); 
    } 

	~BulkAllocator(void)
    {
        releaseAll(); 
    } 

	T* allocate(void)
    {
        if (m_freeList) 
        { 
            T* res = (T*)m_freeList; 
            m_freeList = m_freeList->m_next; 
            return res; 
        } 

        Page* page = m_head; 	

        while (page && !page->m_release)
            page = page->m_next; 

        if (!page)
        {
            page = new (m_allocator->allocate(sizeof(Page))) Page(m_allocator); 
            page->m_next = m_head; 
            m_head = page; 
        }

        T* res = &page->m_pool[--page->m_release]; 
        return res;
    } 
    
    void release(T* P)
    {
        P->~T(); 
        Item* item = (Item*)P; 
        item->m_next = m_freeList; 
        m_freeList = item; 
    } 

	void releaseAll(void)
    {
        Page* page = m_head; 
        while (page) 
        {
            Page* next = page->m_next; 
            m_allocator->deallocate(page->m_pool); 
            m_allocator->deallocate(page); 
            page = next; 
        }
        m_head = 0; 
    } 

private: 

	Page*			m_head; 
    Item*           m_freeList; 
    Allocator*      m_allocator;
}; 

//------------------------------------------------------------------------

} // namespace Umbra

#endif // UMBRABULKALLOCATOR_HPP
