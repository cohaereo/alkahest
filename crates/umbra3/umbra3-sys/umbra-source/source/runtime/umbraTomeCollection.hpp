// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef __UMBRATOMECOLLECTION_H
#define __UMBRATOMECOLLECTION_H

#include "umbraPrivateDefs.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraPlatform.hpp"
#include "umbraMemory.hpp"
#include "umbraArray.hpp"
#include "umbraRuntimeTomeGenerator.hpp"
#include "runtime/umbraTome.hpp"

namespace Umbra {

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Allocator that manages a single memory block
 *//*-------------------------------------------------------------------*/

class SingleBlockAllocator : public Allocator
{
public:
    SingleBlockAllocator(void* buf, size_t size) : m_buffer(buf), m_bufferSize(size), m_allocated(false) {}

    void* allocate(size_t size, const char*)
    {
        if (m_allocated)
            return NULL;
        if (size > m_bufferSize)
            return NULL;
        m_allocated = true;
        return m_buffer;
    }

    void deallocate(void* ptr)
    {
        UMBRA_UNREF(ptr);
        UMBRA_ASSERT(ptr == m_buffer);
        UMBRA_ASSERT(m_allocated);
        m_allocated = false;
    }

    void* getBlock() const { return m_buffer;  }
    size_t getBlockSize() const { return m_bufferSize; }

private:
    void* m_buffer;
    size_t m_bufferSize;
    bool m_allocated;
};


/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   TomeCollection implementation
 *//*-------------------------------------------------------------------*/

class ImpTomeCollection
{
public:

    ImpTomeCollection(Allocator* a)
    :   m_allocator(a),
        m_singleBlock(NULL, 0),
        m_ownsResult(false)
    {
        // must have allocator
        UMBRA_ASSERT(a);
    }

    ImpTomeCollection(void* buf, size_t bufSize)
    :   m_allocator(NULL),
        m_singleBlock(buf, bufSize),
        m_ownsResult(false)
    {
    }

    ~ImpTomeCollection(void)
    {
        clear();
    }

    TomeCollection::ErrorCode build (const ImpTome** tomes, int numTomes, const AABB& aabb, Allocator* scratch, const ImpTomeCollection* prev);

    const ImpTome* getResult (void) const
    {
        return m_result.m_result;
    };

    bool ownsResult (void) const
    {
        return m_ownsResult;
    }

    int getNumTomeContexts (void) const
    {
        return m_result.m_numContexts;
    }

    const TomeContext* getTomeContext (int i) const
    {
        UMBRA_ASSERT(i >= 0 && i < m_result.m_numContexts);
        UMBRA_ASSERT(!!m_result.m_contexts);
        return (const TomeContext*)DataPtr(m_result.m_contexts.getOffset() + i * sizeof(TomeContext)).getAddrNoCheck(m_result.m_result);
    }

    const ExtTile* getExtTile (int i) const
    {
        /* \todo [antti 28.1.2013]: ability to assert < getTileArraySize() here! (SPU) */
        if (!m_result.m_extTiles)
            return NULL;
        return (const ExtTile*)DataPtr(m_result.m_extTiles.getOffset() + i * sizeof(ExtTile)).getAddrNoCheck(m_result.m_result);
    }


    TomeCollection::ErrorCode serialize(OutputStream& stream) const;
    TomeCollection::ErrorCode deserialize(InputStream& stream, const ImpTome** tomes, int numTomes, Allocator* scratch);

private:

    void clear (void)
    {
        m_result.clear(m_ownsResult);
        m_ownsResult = false;
    }

    UserCallbackAllocator           m_allocator;
    SingleBlockAllocator            m_singleBlock;
    bool                            m_ownsResult;
    RuntimeTomeGenerator::Result    m_result;
};

} // namespace Umbra

#endif
