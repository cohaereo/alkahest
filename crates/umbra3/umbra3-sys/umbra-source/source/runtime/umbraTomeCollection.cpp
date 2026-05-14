// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraTomeCollection.hpp"
#include "umbraRuntimeTomeGenerator.hpp"
#include "umbraPatcher.hpp"
#include "umbraBinStream.hpp"

using namespace Umbra;

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

TomeCollection::ErrorCode ImpTomeCollection::build(
    const ImpTome** tomes, int numTomes, const AABB& aabb, Allocator* scratch, const ImpTomeCollection* prevCollection)
{
    RuntimeTomeGenerator* generator = NULL;

    UserCallbackAllocator scratchWrap(scratch);
    if (!scratch)
    {
        // no scratch given => must have result allocator
        if (!m_allocator.getWrappedAllocator())
            return TomeCollection::ERROR_INVALID_PARAM;
        scratch = &m_allocator;
    }
    else
    {
        scratch = &scratchWrap;
    }

    AllowDefaultAllocatorForScope allow(false);

#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    try
    {
#endif

        clear();

        if ((numTomes <= 0))
            return TomeCollection::SUCCESS;

        // fast path for single tome, when no empty size requested
        if (numTomes == 1)
        {
            if (!tomes[0])
                return TomeCollection::ERROR_INVALID_PARAM;

            Tome::Status status = ((const Tome*)tomes[0])->getStatus();
            if (status == Tome::STATUS_CORRUPT)
                return TomeCollection::ERROR_CORRUPT_TOME;
            if (status != Tome::STATUS_OK)
                return TomeCollection::ERROR_INVALID_PARAM;

            Vector3 tomeMn = tomes[0]->getTreeMin();
            Vector3 tomeMx = tomes[0]->getTreeMax();

            if (!aabb.isOK() || AABB(tomeMn, tomeMx).contains(aabb))
            {
                m_result.m_result = tomes[0];
                return TomeCollection::SUCCESS;
            }
        }

        const RuntimeTomeGenerator::Result* oldResult = NULL;

        if (prevCollection && prevCollection->getResult() && prevCollection->ownsResult())
        {
            oldResult = &prevCollection->m_result;
        }

        m_ownsResult = true;
        Allocator* resultAlloc = &m_allocator;
        if (m_singleBlock.getBlockSize())
        {
            resultAlloc = &m_singleBlock;
            if (prevCollection != NULL &&
                prevCollection->m_singleBlock.getBlock() == m_singleBlock.getBlock())
            {
                // protect user from obvious error of reusing same buffer
                // TODO: could also copy old result to scratch here
                return TomeCollection::ERROR_INVALID_PARAM;
            }
        }

        UMBRA_HEAP_NEW_NOTHROW(generator, scratch,
            RuntimeTomeGenerator, scratch, resultAlloc,
            0, tomes, numTomes, aabb);
        if (!generator)
            return TomeCollection::ERROR_OUT_OF_MEMORY;
        TomeCollection::ErrorCode ret = generator->buildTome(m_result, oldResult, m_singleBlock.getBlockSize());
        UMBRA_HEAP_DELETE(scratch, generator);
        return ret;
#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    } catch(OOMException)
    {
        UMBRA_HEAP_DELETE(scratch, generator);
        return TomeCollection::ERROR_OUT_OF_MEMORY;
    }
#endif
}

TomeCollection::ErrorCode ImpTomeCollection::serialize(OutputStream& stream) const
{
    if (!m_result.m_result)
        return TomeCollection::ERROR_UNBUILT;

    const ImpTome* outputData =  m_result.m_result;

    ImpTomeCollectionSerialized serialized;
    serialized.m_versionMagic = (((UINT32)TOMECOLLECTION_MAGIC << 16) | (UINT32)TOMECOLLECTION_VERSION);
    serialized.m_dataSize     = outputData->getSize();

    if (!m_ownsResult)
    {
        serialized.m_size           = sizeof(ImpTomeCollectionSerialized);
        serialized.m_data           = DataPtr();
        serialized.m_numContexts    = 0;
        serialized.m_contexts       = DataPtr();
        serialized.m_extTiles       = DataPtr();

        if (stream.write(&serialized, sizeof(ImpTomeCollectionSerialized)) != sizeof(ImpTomeCollectionSerialized))
            return TomeCollection::ERROR_IO;

        return TomeCollection::SUCCESS;
    }

    serialized.m_size         = sizeof(ImpTomeCollectionSerialized) + serialized.m_dataSize + (int)PatchWriter::computeSize(m_result.m_numContexts + outputData->getTileArraySize());
    serialized.m_data         = DataPtr(sizeof(ImpTomeCollectionSerialized));
    serialized.m_numContexts  = m_result.m_numContexts;
    serialized.m_contexts     = m_result.m_contexts;
    serialized.m_extTiles     = m_result.m_extTiles;

    // Write header
    if (stream.write(&serialized, sizeof(ImpTomeCollectionSerialized)) != sizeof(ImpTomeCollectionSerialized))
        return TomeCollection::ERROR_IO;

    // Write data itself
    if (stream.write(outputData, serialized.m_dataSize) != serialized.m_dataSize)
        return TomeCollection::ERROR_IO;
    
    PatchWriter writer;
    writer.init(stream, (const Umbra::UINT8*)outputData);

    // Patch ImpTome pointers
    for (int i = 0; i < m_result.m_numContexts; i++)
    {
        const TomeContext* ptr = getTomeContext(i);

        // Patch tome pointer into TomeContext.
        if (!writer.patch(ptr->m_tome, i, ptr->m_tome, ptr->m_tome.getPtrAddr()))
            return TomeCollection::ERROR_IO;
    }
    
    DataArray tilePtrs = outputData->getTileOffsets(true);
    AlignedPtr<const ImpTile>* tilePtrArray = (AlignedPtr<const ImpTile>*)tilePtrs.m_ofs.getAddr(tilePtrs.m_base);

    // Patch ImpTile pointers
    for (int i = 0; i < outputData->getTileArraySize(); i++)
    {
        AlignedPtr<const ImpTile> ptr     = tilePtrArray[i];
        const ExtTile*            extTile = getExtTile(i);

        if (!ptr)
            continue;

        if (ptr->getFlags() & ImpTile::TILEFLAG_ISEMPTY)
        {
            // Empty tiles are generated at runtime and stored in outputData instead of any of the input tomes.
            // Patch empty tile ptr into tilePtrArray.
            if (!writer.patch(ptr, tilePtrArray[i].getPtrAddr()))
                return TomeCollection::ERROR_IO;
            continue;
        }

        // Find out which tome this tile originally belongs in
        int tomeIdx  = extTile->getTomeIdx();        
        if (tomeIdx == -1)
            continue;

        // Get tome ptr
        const ImpTome* tome = getTomeContext(tomeIdx)->m_tome;

        // Patch the tile ptr, belonging to the tome, into tilePtrArray.
        if (!writer.patch(tome, tomeIdx, ptr, tilePtrArray[i].getPtrAddr()))
            return TomeCollection::ERROR_IO;
    }

    if  (!writer.finish())
        return TomeCollection::ERROR_IO;

    return TomeCollection::SUCCESS;
}

TomeCollection::ErrorCode ImpTomeCollection::deserialize(InputStream& stream, const ImpTome** tomes, int N, Allocator* scratch)
{
    clear();
    
    UserCallbackAllocator scratchWrap(scratch);
    if (!scratch)
        scratch = &m_allocator;
    else
        scratch = &scratchWrap;
    
    AllowDefaultAllocatorForScope allow(false);
    UMBRA_UNREF(scratch);

#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    try
    {
#endif
        
        ImpTomeCollectionSerialized serialized;
        if (stream.read(&serialized, sizeof(ImpTomeCollectionSerialized)) != sizeof(ImpTomeCollectionSerialized))
            return TomeCollection::ERROR_IO;

        if (serialized.getMagic() != TOMECOLLECTION_MAGIC)
        {
            UINT32 versionMagic = serialized.getVersionMagic();
            UINT32 swapped = swapBytes_4(&versionMagic);
            if ((swapped >> 16) == TOMECOLLECTION_MAGIC)
                return TomeCollection::ERROR_BAD_ENDIAN;

            return TomeCollection::ERROR_CORRUPTED;
        }

        if (serialized.getVersion() < TOMECOLLECTION_VERSION)
            return TomeCollection::ERROR_OLDER_VERSION;
        
        if (serialized.getVersion() > TOMECOLLECTION_VERSION)
            return TomeCollection::ERROR_NEWER_VERSION;

        if (!(serialized.getNumContexts() == 0 && N == 1) && serialized.getNumContexts() != N)
            return TomeCollection::ERROR_INVALID_INPUT_TOMES;

        for (int i = 0; i < N; i++)
        {
            if (!tomes[i])
                return TomeCollection::ERROR_INVALID_INPUT_TOMES;

            Tome::Status status = ((const Tome*)tomes[i])->getStatus();
            if (status == Tome::STATUS_CORRUPT)
                return TomeCollection::ERROR_CORRUPT_TOME;
            if (status != Tome::STATUS_OK)
                return TomeCollection::ERROR_INVALID_INPUT_TOMES;
        }

        if (serialized.getNumContexts() == 0)
        {
            m_result.m_result = tomes[0];
            m_ownsResult = false;
            return TomeCollection::SUCCESS;
        }

        m_ownsResult = true;
        m_result.m_allocator   = &m_allocator;
        m_result.m_result      = (const ImpTome*)UMBRA_HEAP_ALLOC_16(&m_allocator, serialized.m_dataSize);
        m_result.m_numContexts = serialized.getNumContexts();
        m_result.m_contexts    = serialized.getContexts();
        m_result.m_extTiles    = serialized.getExtTiles();
        
        if (!m_result.m_result)
        {
            clear();
            return TomeCollection::ERROR_OUT_OF_MEMORY;
        }

        UMBRA_ASSERT((serialized.getData().getOffset() & 3) == 0);

        for (UINT32 offset = (UINT32)sizeof(ImpTomeCollectionSerialized); offset < serialized.getData().getOffset(); offset += sizeof(UINT32))
        {
            UINT32 data;
            stream.read(&data, sizeof(UINT32));
        }

        if (!stream.read((ImpTome*)m_result.m_result, serialized.m_dataSize))
        {
            clear();
            UMBRA_HEAP_FREE_16(&m_allocator, (void*)m_result.m_result);
            return TomeCollection::ERROR_IO;
        }
       
        PatchReader reader;
        if (!reader.init(stream, (Umbra::UINT8*)m_result.m_result, (const void**)tomes, N))
        {
            clear();
            UMBRA_HEAP_FREE_16(&m_allocator, (void*)m_result.m_result);
            return TomeCollection::ERROR_IO;
        }

#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    } catch(OOMException)
    {
        return TomeCollection::ERROR_OUT_OF_MEMORY;
    }
#endif 

    return TomeCollection::SUCCESS;
}
