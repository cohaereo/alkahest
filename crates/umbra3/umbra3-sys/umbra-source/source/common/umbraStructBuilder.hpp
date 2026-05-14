#pragma once

#include "umbraPlatform.hpp"

namespace Umbra
{

class Allocator;

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class BaseStructBuilder
{
public:
    BaseStructBuilder (Allocator* a): m_refList(NULL), m_allocator(a)
    {
        UMBRA_ASSERT(a);
    }

    ~BaseStructBuilder (void)
    {
        DataRef* ref = m_refList;
        while (ref)
        {
            DataRef* next = ref->next;
            UMBRA_HEAP_FREE(m_allocator, (void*)ref->block);
            m_allocator->deallocate(ref);
            ref = next;
        }
    }

    bool setDataRef (DataPtr& loc, const void* block, UINT32 size)
    {
        DataRef* ref = (DataRef*)m_allocator->allocate(sizeof(DataRef));
        if (!ref)
            return false;

        ref->loc    = &loc;
        ref->block  = block;
        ref->size   = size;
        ref->next   = NULL;

        // Add to end
        int listSize;
        if (m_refList)
        {
            // find last
            DataRef* last = m_refList;
            listSize = 1;
            while (last->next)
            {
                last = last->next;
                listSize++;
            }

            UMBRA_ASSERT(!last->next);
            last->next = ref;
        }
        else
        {
            m_refList = ref;
            listSize = 0;
        }

        loc = (listSize + 1);

        return true;
    }

    void* allocDataRef (DataPtr& loc, UINT32 size, bool zeroMem = false)
    {
        void* block = size ? UMBRA_HEAP_ALLOC(m_allocator, size) : NULL;
        setDataRef(loc, block, size);
        if (block && zeroMem)
            memset(block, 0, size);
        return block;
    }

protected:

    struct DataRef
    {
        DataPtr*    loc;
        const void* block;
        UINT32      size;
        DataRef*    next;   // linked list
    };

    DataRef* m_refList;
    Allocator* m_allocator;
};

template <class T> class StructBuilder : public T, public BaseStructBuilder
{
public:
    StructBuilder (Allocator* a) : BaseStructBuilder(a)
    {
        UMBRA_ASSERT(a);
        memset(this, 0, sizeof(T));
    }

    T* pack (void)
    {
        UINT32 size = getCurrentSize();
        UINT8* buf = (UINT8*)UMBRA_HEAP_ALLOC(m_allocator, size);

        return pack(buf);
    }

    T* pack (UINT8* buf)
    {
        UINT8* ptr = buf;
        if (!ptr)
            return NULL;

        ptr = write(ptr, this, sizeof(T));

        UINT32 i = 0;
        for (DataRef* ref = m_refList; ref; ref = ref->next, i++)
        {
            UMBRA_ASSERT(ref->loc->getOffset() == (i + 1));
            UINT32 ofs = mapLoc(ref->loc, i);
            if (!ofs)
                continue;
            DataPtr* loc = (DataPtr*)(buf + ofs);
            if (!ref->block)
                *loc = DataPtr(0);
            else
                *loc = DataPtr((UINT32)(ptr - buf));
            ptr = write(ptr, ref->block, ref->size);
        }

        return (T*)buf;
    }

    UINT32 getCurrentSize (void) const
    {
        UINT32 size = UMBRA_ALIGN(sizeof(T), 16);
        for (DataRef* ref = m_refList; ref; ref = ref->next)
            size += UMBRA_ALIGN(ref->size, 16);
        return size;
    }

    UINT32 getWastedSize (void) const
    {
        UINT32 size = UMBRA_ALIGN(sizeof(T), 16) - sizeof(T);
        for (DataRef* ref = m_refList; ref; ref = ref->next)
        {
            // alignment waste
            size += (UMBRA_ALIGN(ref->size, 16) - ref->size);
            // zero ptr waste
            if (!ref->block)
                size += 4;
        }
        return size;
    }

protected:

    static UINT8* write (UINT8* dst, const void* src, UINT32 size)
    {
        UINT32 aligned = UMBRA_ALIGN(size, 16);
        memcpy(dst, src, size);
        if (size != aligned)
            memset(dst + size, 0, aligned - size);
        return dst + aligned;
    }

    UINT32 mapLoc (DataPtr* loc, int maxRef)
    {
        UMBRA_ASSERT(loc);

        UINT8* addr = (UINT8*)loc;
        UINT32 base = 0;
        UINT8* start = (UINT8*)this;
        UINT8* end = start + sizeof(T);
        int ref = 0;
        DataRef* curRef = m_refList;

        for (;;)
        {
            if ((addr >= start) && (addr < end))
                return base + (UINT32)(addr - start);

            if (ref >= maxRef)
                break;

            base += UMBRA_ALIGN(end - start, 16);
            start = (UINT8*)curRef->block;
            end = start + curRef->size;
            curRef = curRef->next;
            ref++;
        }

        UMBRA_ASSERT(!"Data reference not found");
        return 0;
    }
};

} // namespace Umbra
