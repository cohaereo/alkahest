#pragma once
#ifndef __UMBRAQUERYCONTEXT_H
#define __UMBRAQUERYCONTEXT_H

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra query context
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraPlatform.hpp"
#include "umbraQueryArgs.hpp"
#include "umbraSIMD.hpp"
#include "umbraTomeCollection.hpp"
#if UMBRA_COMPILER == UMBRA_MSC
#include <new.h>
#else
#include <new>
#endif

#define UMBRA_MAX_MAPPED_TILES 2
#define UMBRA_FINDCELL_BATCH   32

namespace Umbra
{

// forward declarations
class DebugRenderer;
class DistanceLookup;
class ArrayMapper;
class Transformer;
class MappedTile;

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

static int roundUpMultiple(int value, int multiple)
{
    int remainder = value % multiple;
    if (!remainder)
        return value;
    return value + multiple - remainder;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Gets index range for a given bucket.
 *//*-------------------------------------------------------------------*/

static inline void bucketIndices(int currentBucketIdx, int totalBuckets, int totalIndices, int bucketSizeAlignment, int& startIndex, int& numIndices)
{
    UMBRA_ASSERT(totalIndices >= 0);
    UMBRA_ASSERT(totalBuckets >= 0);
    UMBRA_ASSERT(currentBucketIdx >= 0 && currentBucketIdx < totalBuckets);
    UMBRA_ASSERT(bucketSizeAlignment >= 1);

    if (totalIndices == 0 || totalBuckets == 0)
    {
        startIndex = 0;
        numIndices = 0;
        return;
    }

    int alignedRemainder = totalIndices % bucketSizeAlignment;
    int alignedTotal     = totalIndices - alignedRemainder;

    int baseBucketSize   = roundUpMultiple(alignedTotal / totalBuckets, bucketSizeAlignment);
    int remainder        = alignedTotal - baseBucketSize * totalBuckets;

    if (remainder >= 0)
    {
        // all buckets full, distribute remainder to first buckets

        if (currentBucketIdx < remainder / bucketSizeAlignment)
        {
            startIndex = currentBucketIdx * (baseBucketSize + bucketSizeAlignment);
            numIndices = baseBucketSize + bucketSizeAlignment;
        } else
        {
            startIndex = currentBucketIdx * baseBucketSize + remainder;
            numIndices = baseBucketSize;
        }
                
        // Remainder from the alignment must be in last bucket
        if (currentBucketIdx == totalBuckets - 1)
            numIndices += alignedRemainder;
    } else
    {
        // Not enough indices for full buckets, some buckets can be empty, 
        // last bucket not full

        startIndex = currentBucketIdx * baseBucketSize;
        numIndices = baseBucketSize;

        // empty buckets
        int lastBucket = totalBuckets + remainder / bucketSizeAlignment - 1;
        if (currentBucketIdx > lastBucket)
            numIndices = 0;

        if (currentBucketIdx == lastBucket)
            numIndices += (remainder + (totalBuckets - lastBucket - 1) * baseBucketSize) + alignedRemainder;
    }
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class Cell
{
public:
    Cell(int slot, int i): slotIdx(slot), index(i) {}
    Cell(void): slotIdx(-1), index(-1) {}
    bool operator==(const Cell& c) { return slotIdx == c.slotIdx && index == c.index; }
    bool valid(void) const { return (slotIdx != -1) && (index != -1); }

    int slotIdx;
    int index;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class StackAlloc
{
public:
    StackAlloc  (void* buf, size_t size);
    ~StackAlloc (void);

    explicit StackAlloc (StackAlloc& orig);

    void        reset       (void) { m_cur = m_start; m_min = m_end - m_start; }
    void        reset       (void* buf, size_t size);

    void*       allocate    (size_t size, const char* info = NULL);
    void        deallocate  (void* ptr);
    size_t      available   (void) const { return (m_end - m_cur - 16) & ~0xF; }
    size_t      allocated   (void) const { return m_cur - m_start; }

    UINT8*      getPtr      (void) const { return m_start; }
    size_t      getSize     (void) const { return (size_t)(m_end - m_start); }

private:

    StackAlloc*         m_parent;
    UINT8*              m_start;
    UINT8*              m_end;
    UINT8*              m_cur;
    size_t              m_min;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class StatsAlloc
{
public:
    StatsAlloc  (size_t size): m_size(size), m_current(0) {}
    ~StatsAlloc (void) {}

    void        reset       (void) { m_current = 0; }

    void*       allocate    (size_t size, const char* info = NULL);
    void        deallocate  (void* ptr);
    size_t      available   (void) const { return (m_size - m_current - 16) & ~0xF; }
    size_t      allocated   (void) const { return m_current; }

private:

    size_t              m_size;
    size_t              m_current;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class TagManager
{
public:
    TagManager (UINT32 head): m_head(head), m_val(head) {}
    TagManager (const TagManager& o): m_head(o.m_val), m_val(o.m_val) {}
    ~TagManager (void) { UMBRA_ASSERT(m_val == m_head); }

    UINT32      get     (void) { UMBRA_ASSERT(m_val < 32); return m_val++; }
    void        release (UINT32 tag) { UMBRA_ASSERT(tag == m_val - 1); m_val = tag; }

private:
    UINT32  m_head;
    UINT32  m_val;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class Tag
{
public:

    Tag (void): m_mgr(NULL), m_value(0xFFFFFFFF) {}

    Tag (TagManager& mgr): m_mgr(&mgr)
    {
        m_value = mgr.get();
    }

    ~Tag(void)
    {
        if (m_mgr)
            m_mgr->release(m_value);
    }

    void init (TagManager& mgr)
    {
        UMBRA_ASSERT(!m_mgr);
        m_mgr = &mgr;
        m_value = mgr.get();
    }

    void deinit (void)
    {
        UMBRA_ASSERT(m_mgr);
        m_mgr->release(m_value);
        m_mgr = NULL;
    }

    UINT32 getValue (void) const { return m_value; }

private:
    TagManager* m_mgr;
    UINT32      m_value;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Raw array accessors
 *//*-------------------------------------------------------------------*/

const void* mapArray    (StackAlloc* s, const DataArray& arr, int first = 0, int num = -1);
const void* mapArray    (StackAlloc* s, const BitDataArray& arr, int first = 0, int num = -1);
void        unmapArray  (StackAlloc* s, const void* data);

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Query statistics
 *//*-------------------------------------------------------------------*/

/*!
 * \brief   Internal statistics for queries
 */
enum QueryStat
{
    QUERYSTAT_TILES_VISITED = 0,
    QUERYSTAT_PORTALS_PROCESSED,
    QUERYSTAT_EXT_PORTALS_PROCESSED,
    QUERYSTAT_CELLS_PROCESSED,
    QUERYSTAT_CELL_SORT_FAILURES,
    QUERYSTAT_CELL_CYCLES,
    QUERYSTAT_CELL_REVISITS,
    QUERYSTAT_OBJECTS_TESTED,
    QUERYSTAT_OBJECTS_VISIBLE,
    QUERYSTAT_OBJECTS_FRUSTUMCULLED,
    QUERYSTAT_OBJECTS_DISTANCECULLED,
    QUERYSTAT_OBJECTS_STATICALLY_CULLED,
    QUERYSTAT_OBJECTS_CONTRIBUTIONCULLED,
    QUERYSTAT_LAST
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

union BasePtr
{
    void*   base;
    UINT32  pad[4];
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class MappedTome
{
public:

    int                 getIndex                (void) const { return m_idx; }
    const TomeContext*  getContext              (void) const { return m_ctx; }
    const ImpTome*      getTome                 (void) const { return m_tome; }
    const void*         getBase                 (void) const { return m_base; }

    inline int          mapLocalObject          (int localIdx) const;
    inline int          mapLocalTile            (int localIdx) const;
    inline int          mapLocalGate            (int localIdx) const;
    inline int          mapLocalCluster         (int localIdx) const;
    inline int          mapLocalClusterPortal   (int localIdx) const;
    inline int          mapGlobalCluster        (int globalIdx) const;
    inline int          mapGlobalClusterPortal  (int globalIdx) const;
    inline bool         containsCluster         (int globalIdx) const;
    inline bool         containsClusterPortal   (int globalIdx) const;

    bool                hasExternalPortals      (void) const { return m_ctx && m_ctx->hasExtClusters(); }
    inline DataArray    getExtClusterNodes      (void) const;
    inline DataArray    getExtPortals           (void) const;
    inline DataArray    getExtPortals           (const ExtClusterNode& node) const;

    bool                operator!               (void) const { return !m_tome; }

    MappedTome          (void)
        : m_idx(-1), m_ctx(NULL), m_tome(NULL), m_base(NULL)
    {}

private:

    MappedTome(int idx, const TomeContext* ctx, const ImpTome* tome, const void* base, int clusterStart, int clusterPortalStart, int numClusterPortals)
        : m_idx(idx), m_ctx(ctx), m_tome(tome), m_base(base),
        m_clusterStart(clusterStart), m_clusterPortalStart(clusterPortalStart),
        m_numClusterPortals(numClusterPortals)
    {
    }

    int                 m_idx;
    const TomeContext*  m_ctx;
    const ImpTome*      m_tome;
    const void*         m_base;
    int                 m_clusterStart;
    int                 m_clusterPortalStart;
    int                 m_numClusterPortals;

    friend class QueryContext;
    friend class QueryState;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class MappedTile
{
public:
    const ImpTile*      getTile             (void) const { return m_tile; }
    const ExtTile*      getExtTile          (void) const { return m_extTile; }
    const MappedTome&   getMappedTome       (void) const { return m_mappedTome; }

    bool                hasExternalPortals  (void) const { return m_extTile && m_extTile->hasExtCells(); }
    inline DataArray    getExtCellNodes     (void) const;
    inline DataArray    getExtPortals       (const ExtCellNode& node) const;
    Umbra::UINT32       getExitPortalMask   (void) const { return m_extTile ? m_extTile->getExitPortalMask() : ((1 << 6) - 1); }
    int                 getLocalSlot        (void) const { return m_localSlot; }

    bool                operator!           (void) const { return !m_tile; }

    MappedTile          (void)
        : m_tile(NULL), m_extTile(NULL)
    {}

private:

    MappedTile(int slot, const ImpTile* tile, const ExtTile* extTile, const MappedTome& mappedTome)
        : m_mappedTome(mappedTome), m_tile(tile), m_extTile(extTile)
    {
        UMBRA_ASSERT(m_tile && mappedTome.getTome());
        if (m_extTile)
            m_localSlot = m_extTile->getLocalSlot();
        else
            m_localSlot = slot;
    }

    MappedTome          m_mappedTome;
    const ImpTile*      m_tile;
    const ExtTile*      m_extTile;
    int                 m_localSlot;

    friend class QueryContext;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   The persistent query state
 *//*-------------------------------------------------------------------*/

class QueryState
{
public:
    QueryState (void* mem, size_t memSize);

    void setQueryData (const ImpTome* rootTome, const ImpTomeCollection* collection);

    bool getPortalState (int idx) const
    {
        // portals are always open if the state array is missing
        if (!m_portalStates || idx < 0)
            return true;
        return testBit(m_portalStates, idx);
    }

    const ImpTomeCollection* getCollection  (void) const { return m_collection; }
    const ImpTome*      getRootTome         (void) const { return m_tome; }
    const MappedTome&   getDefaultTome      (void) const { return m_mappedTome; }
    ArrayMapper*        getTileArray        (void) const { return m_tiles; }
    int                 getNumTomeContexts  (void) const { return m_collection ? m_collection->getNumTomeContexts() : 0; }
    bool                tilesArePointers    (void) const { return m_collection ? m_collection->ownsResult() : false; }
    const ExtTile*      getExtTile          (int idx) const { if (!m_collection) return NULL; else return m_collection->getExtTile(idx); }

    void                setGateStates       (const UINT32* p) { m_portalStates = p; }
    const UINT32*       getGateStates       (void) const { return m_portalStates; }
    void                setGateCosts        (const float* p, bool additive) { m_gateCosts = p; m_gateCostAdd = additive; }
    const float*        getGateCosts        (void) const { return m_gateCosts; }
    bool                isGateCostAdditive  (void) const { return m_gateCostAdd; }
    void                setDebugRenderer    (DebugRenderer* debug) { m_debug = debug; }
    DebugRenderer*      getDebugRenderer    (void) const { return m_debug; }
    StackAlloc*         getAllocator        (void) { return m_curStack; }
    StackAlloc*         getQueryAllocator   (void) { return &m_queryStack; }
    ArrayMapper*        getTilePaths        (void) const { return m_tilePaths; }
    int                 getBitsPerTilePath  (void) const { return m_bitsPerTilePath; }
    TagManager          getTagManager       (void) const { return m_tags; }
    void                setSpuUsage         (Query::SpuUsage usage) { m_spuUsage = usage; }
    Query::SpuUsage     getSpuUsage         (void) const { return m_spuUsage; }

    void                setWorkMem          (UINT8* workMem, size_t workMemSize);
    void                getWorkMem          (UINT8*& workMem, size_t& workMemSize);

    void                mapTome                 (MappedTome& mapped, int tomeIdx);
    int                 findTomeByCluster       (int globalCluster);
    int                 findTomeByClusterPortal (int globalCluster);
    
    inline int          mapLocalCluster         (int tomeIdx, int localIdx) const;

    static const void* importRemoteObj (StackAlloc& alloc, const void* remote, UINT32 size);
    static void freeRemoteObj (StackAlloc& alloc, void* local)
    {
#ifdef UMBRA_REMOTE_MEMORY
        alloc.deallocate(local);
#else
        UMBRA_UNREF(alloc);
        UMBRA_UNREF(local);
#endif
    }

private:

    struct TomeCache
    {
        TomeCache(void)
        {
            ctxBase.base   = NULL;
            tomeBase.base  = NULL;
        }

        TomeContext     ctx;
        BasePtr         ctxBase;

        ImpTome         tome;
        BasePtr         tomeBase;
    };

#ifdef UMBRA_REMOTE_MEMORY
    TomeCache                   m_tomeCache[UMBRA_MAX_MAPPED_TILES];
    int                         m_lastCache;
#endif
    const ImpTome*              m_tome;
    const ImpTomeCollection*    m_collection;
    ArrayMapper*                m_tiles;
    const UINT32*               m_portalStates;
    const float*                m_gateCosts;
    bool                        m_gateCostAdd;
    ArrayMapper*                m_tilePaths;
    int                         m_bitsPerTilePath;
    DebugRenderer*              m_debug;
    StackAlloc                  m_queryStack;
    StackAlloc                  m_workStack;
    StackAlloc*                 m_curStack;
    TagManager                  m_tags;
    Query::SpuUsage		        m_spuUsage;

    MappedTome                  m_mappedTome;
};

inline int QueryState::mapLocalCluster(int tomeIdx, int localIdx) const
{
    if (!getNumTomeContexts())
        return localIdx;
    int clusterStart = 0;
    getRootTome()->getClusterStarts().getElem(clusterStart, tomeIdx);
    return localIdx + clusterStart;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   The per-query state & data access helpers
 *//*-------------------------------------------------------------------*/

class QueryContext
{
public:

    QueryContext (QueryState* state, UINT32 flags = 0);
    ~QueryContext (void);

    const QueryState*   getState           (void) const { return m_state; }
    QueryState*         getState           (void) { return m_state; }
    bool                hasData            (void) const { return m_state->getRootTome() != NULL; }

    StackAlloc*         getAllocator       (void) { return &m_stack; }
    TagManager&         getTagManager      (void) { return m_tags; }

    void                setError           (UINT32 err) { m_error = err; }
    UINT32              getError           (void) const { return m_error; }
    bool                hasError           (void) const { return m_error != 0; }

    inline bool         hasTile            (int idx) const;
    inline float        getPortalExpand    (int idx);

    void                mapTile            (MappedTile& accessor, int idx);
    void                unmapTile          (MappedTile& accessor) { unmapTile(accessor.getTile()); }

    // Helpers

    int                 findSlot           (const Vector3& coord);
    Cell                findCell           (const Vector3& coord);
    void                findMultipleCells  (const Vector3* coord, Cell* results, int count);
    int                 findNodeInTile     (const ImpTile* tile, const Vector3& coord);
    int                 findCluster        (const Vector3& coord);

    bool                isGateOpen          (const MappedTile& tile, const Portal& portal);
    bool                isGateOpen          (const Portal& portal);
    float               getGateCost         (const MappedTile& tile, const Portal& portal);
    float               getGateCost         (const Portal& portal);

    inline UINT32*      bitVectorFromIndexList (int size, const IndexList& list);

    // Debug

    bool                debugEnabled       (UINT32 flag) const { return m_state->getDebugRenderer() && (m_debugFlags & flag); }
    void                addQueryDebugPoint (const Vector3& pt, const Vector4& color);
    void                addQueryDebugLine  (const Vector3& start, const Vector3& end, const Vector4& color);
    void                addQueryDebugAABB  (const Vector3& mn, const Vector3& mx, const Vector4& color, bool solid = false);
    void                addQueryDebugQuad  (const Vector3& x0y0, const Vector3& x0y1, const Vector3& x1y1, const Vector3& x1y0, const Vector4& color);
    void                addQueryDebugSphere(const Vector3& center, float radius, const Vector4& color);
    Vector4             clusterColor       (int idx);
    void                visualizeCell      (const ImpTile* tile, int cell);
    void                visualizeTiles     (void);
    void                visualizeFrustum   (const Transformer& transformer);
    void                setQueryStatInt    (QueryStat s, int val) { m_intStats[s] = val; }
    int                 getQueryStatInt    (QueryStat s) { return max2(m_intStats[s], 0); }

    const ImpTome*      getTome            (void) const { return m_state->getRootTome(); }
    const MappedTome&   getDefaultTome     (void) const { return m_state->getDefaultTome(); }

    // deprecated!

    void*               allocWorkMem       (size_t size, bool clear = false);
    void                freeWorkMem        (void* ptr) { m_stack.deallocate(ptr); }
    size_t              workMemAvailable   (void) const { return m_stack.available(); }

    static void estimateSize(StatsAlloc* allocator)
    {
        UMBRA_UNREF(allocator);
        UMBRA_HEAP_NEW_ARRAY_NOINIT(allocator, int, QUERYSTAT_LAST);
#ifdef UMBRA_REMOTE_MEMORY
        UMBRA_HEAP_NEW_ARRAY_NOINIT(allocator, TileCache, UMBRA_MAX_MAPPED_TILES);
#endif
    }

private:

    struct TileCache
    {
        TileCache(void)
        {
            tileBase.base  = NULL;
            extBase.base   = NULL;
        }

        ImpTile         tile;
        BasePtr         tileBase;

        ExtTile         extTile;
        BasePtr         extBase;
    };

    const ImpTile*      mapTile            (int idx);
    void                unmapTile          (const ImpTile* tile)
    {
        UMBRA_DEBUG_CODE(if (tile) { m_numMappedTiles--; });
#ifndef UMBRA_REMOTE_MEMORY
        UMBRA_UNREF(tile);
#else
        if (tile)
        {
            TileCache* m = (TileCache*)tile;
            m->tileBase.base = NULL;
            UMBRA_ASSERT(m >= &m_tileCache[0] && m < &m_tileCache[UMBRA_MAX_MAPPED_TILES]);
        }
#endif
    }

    QueryState*         m_state;
    StackAlloc          m_stack;
    UINT32              m_numMappedTiles;
    UINT32              m_error;
    UINT32              m_debugFlags;
    Tag                 m_tagHead;
    TagManager          m_tags;
    UINT32              m_fpState;
    TileCache*          m_tileCache;
    int*                m_intStats;
};

inline DataArray MappedTome::getExtClusterNodes(void) const
{
    if (!m_ctx || !m_ctx->hasExtClusters())
        return DataArray();
    return m_ctx->getExtClusters(getBase(), m_tome->getNumClusters());
}

inline DataArray MappedTome::getExtPortals(void) const
{
    if (!m_ctx)
        return DataArray();
    DataArray arr = m_ctx->getExtPortals(getBase());
    arr.m_count = m_numClusterPortals - m_tome->getNumClusterPortals();
    return arr;
}

inline DataArray MappedTome::getExtPortals(const ExtClusterNode& node) const
{
    if (!m_ctx)
        return DataArray();
    return m_ctx->getExtPortals(getBase(), node);
}

inline DataArray MappedTile::getExtCellNodes(void) const
{
    if (!m_extTile || !m_extTile->hasExtCells())
        return DataArray();
    return m_extTile->getExtCells(m_mappedTome.getBase(), m_tile->getNumCells());
}

inline DataArray MappedTile::getExtPortals(const ExtCellNode& node) const
{
    if (!m_extTile)
        return DataArray();
    return m_extTile->getExtPortals(m_mappedTome.getBase(), node);
}

inline int MappedTome::mapLocalObject (int localIdx) const
{
    if (!m_ctx)
        return localIdx;

    UMBRA_ASSERT(localIdx >= 0 && localIdx < m_tome->getNumObjects());
    int mapped = 0;
    m_ctx->getObjMap(m_base).getElem(mapped, localIdx);
    return mapped;
}

inline int MappedTome::mapLocalTile(int localIdx) const
{
    if (!m_ctx)
        return localIdx;
    UMBRA_ASSERT(localIdx >= 0 && localIdx < m_tome->getTileArraySize());
    int mapped = 0;
    m_ctx->getTileMap(m_base).getElem(mapped, localIdx);
    return mapped;
}

inline int MappedTome::mapLocalGate(int localIdx) const
{
    if (!m_ctx)
        return localIdx;

    UMBRA_ASSERT(localIdx >= 0 && localIdx < m_tome->getNumGates());
    int mapped = 0;
    m_ctx->getGateMap(m_base).getElem(mapped, localIdx);
    return mapped;
}

inline int MappedTome::mapLocalCluster(int localIdx) const
{
    if (m_ctx && (localIdx != -1))
        localIdx += m_clusterStart;
    return localIdx;
}

inline int MappedTome::mapLocalClusterPortal(int localIdx) const
{
    UMBRA_ASSERT(localIdx >= 0);
    if (m_ctx)
        localIdx += m_clusterPortalStart;
    return localIdx;
}

inline int MappedTome::mapGlobalCluster(int globalIdx) const
{
    UMBRA_ASSERT(globalIdx >= 0);
    if (m_ctx)
        globalIdx -= m_clusterStart;
    UMBRA_ASSERT(globalIdx >= 0);
    return globalIdx;
}

inline int MappedTome::mapGlobalClusterPortal(int globalIdx) const
{
    UMBRA_ASSERT(globalIdx >= 0);
    if (m_ctx)
        globalIdx -= m_clusterPortalStart;
    UMBRA_ASSERT(globalIdx >= 0);
    return globalIdx;
}

inline bool MappedTome::containsCluster(int globalIdx) const
{
    UMBRA_ASSERT(globalIdx >= 0);
    return globalIdx >= m_clusterStart && globalIdx - m_clusterStart < m_tome->getNumClusters();
}

inline bool MappedTome::containsClusterPortal(int globalIdx) const
{
    UMBRA_ASSERT(globalIdx >= 0);
    return globalIdx >= m_clusterPortalStart && globalIdx - m_clusterPortalStart < m_numClusterPortals;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 * \todo    protect resources from being used multiple times
 *//*-------------------------------------------------------------------*/

template <typename Element, int BatchSize = 16>
class ArrayIterator
{
public:
    UMBRA_CT_ASSERT(((sizeof(Element) * BatchSize) & 0xF) == 0);

#ifdef UMBRA_REMOTE_MEMORY
    struct Resources
    {
        Resources (StackAlloc* s, TagManager& tags): s(s), tag(tags)
        {
            mem = UMBRA_HEAP_NEW_ARRAY(s, Element, 2 * BatchSize);
        }
        ~Resources (void)
        {
            UMBRA_HEAP_DELETE_ARRAY(s, mem);
        }

        StackAlloc* s;
        Tag tag;
        Element* mem;
    };
#else
    struct Resources
    {
        Resources (StackAlloc*, TagManager&) {}
    };
#endif

    ArrayIterator (StackAlloc* s, TagManager& tags)
        : m_res(s, tags)
    {
    }

    ArrayIterator (StackAlloc* s, TagManager& tags, const DataArray& arr)
        : m_res(s, tags)
    {
        setArray(arr);
    }

    ArrayIterator (const Resources& res)
        : m_res(res)
    {
    }

    ~ArrayIterator (void)
    {
#ifdef UMBRA_REMOTE_MEMORY
        MemoryAccess::wait(m_res.tag.getValue());
#endif
    }

    UMBRA_FORCE_INLINE void clear (void)
    {
        m_left = 0;
#ifdef UMBRA_REMOTE_MEMORY
        m_remote = 0;
        m_end = 0;
        m_leftInBatch = 0;
        m_curSlice = 0;
        m_cur = 0;
#endif
    }

    UMBRA_FORCE_INLINE int setArray (const DataArray& arr)
    {
        UINTPTR addr = (UINTPTR)arr.m_ofs.getAddr(arr.m_base);
        UMBRA_ASSERT(!(addr & 0xF) || !((addr & 0xF) % sizeof(Element)));
        UMBRA_ASSERT(sizeof(Element) == arr.m_elemSize);
        UMBRA_ASSERT(arr.m_count != -1);
        clear();
        m_left = addr ? arr.m_count : 0;
#ifdef UMBRA_REMOTE_MEMORY
        if (m_left)
            initCache(addr);
#else
        m_cur = (Element*)addr;
        UMBRA_PREFETCH(m_cur);
#endif
        return m_left;
    }

    void skip (int num)
    {
        // todo: optimize
        while (num--)
            next();
    }

    UMBRA_FORCE_INLINE int hasMore (void) const
    {
        return m_left;
    }

    UMBRA_FORCE_INLINE const Element& next (void)
    {
        UMBRA_ASSERT(hasMore());
#ifdef UMBRA_REMOTE_MEMORY
        if (m_leftInBatch <= 0)
            nextBatch();
        m_leftInBatch--;
#endif
        UMBRA_ASSERT(m_cur);
        m_left--;
        Element* ret = m_cur++;
        UMBRA_PREFETCH(m_cur);
        return *ret;
    }

    UMBRA_FORCE_INLINE void nextN (Element* dst, int N)
    {
        UMBRA_ASSERT(hasMore() >= N);
#ifdef UMBRA_REMOTE_MEMORY
        Element* ptr = dst;
        int l = N;
        while (l)
        {
            if (m_leftInBatch <= 0)
                nextBatch();
            int num = min2(l, m_leftInBatch);
            UMBRA_ASSERT(num);
            memcpy(ptr, m_cur, num*sizeof(Element));
            m_leftInBatch -= num;
            l -= num;
            ptr += num;
            m_cur += num;
        }
        UMBRA_ASSERT(m_cur);
#else
        UMBRA_ASSERT(m_cur);
        memcpy(dst, m_cur, N*sizeof(Element));
        m_cur += N;
#endif
        m_left -= N;
    }

    static void estimateSize(StatsAlloc& stack)
    {
        UMBRA_UNREF(stack);
#ifdef UMBRA_REMOTE_MEMORY
        UMBRA_HEAP_NEW_ARRAY_NOINIT(&stack, Element, 2 * BatchSize);
#endif
    }

    bool isInitialized()
    {
#ifdef UMBRA_REMOTE_MEMORY
        return m_res.mem != NULL;
#else
        return true;
#endif
    }

private:

    // Avoid double deallocation with UMBRA_REMOTE_MEMORY
    ArrayIterator(const ArrayIterator&);
    ArrayIterator& operator=(const ArrayIterator&);

#ifdef UMBRA_REMOTE_MEMORY
    void readSlice (int sliceIdx)
    {
        // reads a batch of elements from remote memory to given slice,
        // takes care of not exceeding source array limits
        UMBRA_ASSERT(sliceIdx <= 2);
        UINTPTR readEnd = min2((UINTPTR)(m_remote + sizeof(Element) * BatchSize), m_end);
        size_t readSize = readEnd - m_remote;
        if (readSize)
        {
            UINT32 writeStart = sliceIdx * BatchSize;
            MemoryAccess::alignedReadAsync(m_res.mem + writeStart,
                (const void*)m_remote, readSize, m_res.tag.getValue());
            m_remote += readSize;
        }
    }

    void initCache (UINTPTR addr)
    {
        UMBRA_ASSERT(addr && m_left);
        int alignElems = (addr & 0xF) / sizeof(Element);
        UMBRA_ASSERT(alignElems < BatchSize);
        m_remote = (addr & ~0xF);
        m_end = m_remote + (m_left + alignElems) * sizeof(Element);
        m_end = (m_end + 0xF) & ~0xF;
        readSlice(0);
        // negative batch size value for element alignment
        m_leftInBatch = -alignElems;
        m_curSlice = 1;
    }

    void nextBatch (void)
    {
        UMBRA_ASSERT(m_leftInBatch <= 0);

        // read next slice
        MemoryAccess::wait(m_res.tag.getValue());
        readSlice(m_curSlice);
        m_curSlice ^= 1;

        // new elements to read
        m_cur = m_res.mem + (m_curSlice * BatchSize) - m_leftInBatch;
        m_leftInBatch += BatchSize;
    }
#endif

    Resources       m_res;
    Element*        m_cur;
    int             m_left;
#ifdef UMBRA_REMOTE_MEMORY
    UINTPTR         m_remote;
    UINTPTR         m_end;
    int             m_leftInBatch;
    int             m_curSlice;
#endif
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

template <typename Element, int Size>
class ArrayIndexedIterator
{
public:

    enum
    {
        ScratchSize = (sizeof(Element) & 0xF) ? 16 + (16 - (sizeof(Element) & 0xF)) : 0,
        TransferSize = sizeof(Element) + ScratchSize
    };

    ArrayIndexedIterator (void): m_remote(0) {}

    void init (QueryContext* q, const DataArray& arr)
    {
#ifdef UMBRA_REMOTE_MEMORY
        m_tag.init(q->getTagManager());
        m_indices = UMBRA_HEAP_NEW_ARRAY(q->getAllocator(), MemListElem, Size);
        m_elements = (UINT8*)UMBRA_HEAP_ALLOC_16(q->getAllocator(),
            Size*(sizeof(Element) + ScratchSize));
#else
        m_indices = NULL;
#endif
        UMBRA_UNREF(q);
        setArray(arr);
    }

    void deinit (QueryContext* q)
    {
#ifdef UMBRA_REMOTE_MEMORY
        UMBRA_HEAP_FREE_16(q->getAllocator(), m_elements);
        UMBRA_HEAP_DELETE_ARRAY(q->getAllocator(), m_indices);
        m_tag.deinit();
#endif
        UMBRA_UNREF(q);
    }

    void setArray (const DataArray& arr)
    {
        UINTPTR addr = (UINTPTR)arr.m_ofs.getAddr(arr.m_base);
        UMBRA_ASSERT((addr & 0x3) == 0);
        UMBRA_ASSERT(!arr.m_elemSize || (sizeof(Element) == arr.m_elemSize));
        UMBRA_ASSERT(arr.m_count != -1);
        m_remote = addr;
    }

    bool hasArray (void) const
    {
        return m_remote != 0;
    }

    void fetch (const UINT32* indices, int count)
    {
        UMBRA_ASSERT(count <= Size);
        UMBRA_ASSERT(count > 0);
        UMBRA_ASSERT(m_remote);
#ifdef UMBRA_REMOTE_MEMORY
        for (int i = 0; i < count; i++)
        {
            UINT32 src = indices[i] * sizeof(Element);
            UINT32 ofs = src & 0xF;
            m_offsets[i] = ofs;
            setMemListElem(m_indices[i], m_remote + (src & ~0xF), TransferSize);
        }
        MemoryAccess::alignedReadIndexedAsync(m_elements, m_indices, count, m_tag.getValue());
#else
        UMBRA_UNREF(count);
        m_indices = indices;
#endif
    }

    void process (void)
    {
#ifdef UMBRA_REMOTE_MEMORY
        MemoryAccess::wait(m_tag.getValue());
#else
        UMBRA_ASSERT(m_indices);
        UMBRA_PREFETCH((const Element*)m_remote + m_indices[0]);
        UMBRA_PREFETCH((const Element*)m_remote + m_indices[1]);
#endif
    }

    const Element* get (int idx) const
    {
        UMBRA_ASSERT(m_remote);
#ifdef UMBRA_REMOTE_MEMORY
        return (const Element*)(m_elements + idx * TransferSize + m_offsets[idx]);
#else
        UMBRA_ASSERT(m_indices);
        UMBRA_PREFETCH((const Element*)m_remote + m_indices[idx + 2]);
        return (const Element*)m_remote + m_indices[idx];
#endif
    }

private:

    UINTPTR         m_remote;
#ifdef UMBRA_REMOTE_MEMORY
    MemListElem*    m_indices;
    int             m_offsets[Size];
    UINT8*          m_elements;
    Tag             m_tag;
#else
    const UINT32*   m_indices;
#endif
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ArrayMapper
{
public:
    ArrayMapper (StackAlloc* s, TagManager& tags, size_t elemSize, size_t cacheSize = 512)
        : m_stack(s), m_tag(tags)
    {
        init(elemSize, cacheSize);
    }

    ArrayMapper (StackAlloc* s, TagManager& tags, const DataArray& arr, size_t cacheSize = 512)
        : m_stack(s), m_tag(tags)
    {
        init(arr.m_elemSize, cacheSize);
        setArray(arr);
    }

    ArrayMapper (QueryContext* ctx, size_t elemSize, size_t cacheSize = 512)
        : m_stack(ctx->getAllocator()), m_tag(ctx->getTagManager())
    {
        init(elemSize, cacheSize);
    }

    // deprecated
    ArrayMapper (QueryContext* ctx, const DataArray& arr, size_t cacheSize = 512)
        : m_stack(ctx->getAllocator()), m_tag(ctx->getTagManager())
    {
        init(arr.m_elemSize, cacheSize);
        setArray(arr);
    }

    bool isInitialized()
    {
#ifdef UMBRA_REMOTE_MEMORY
        return m_mem != NULL;
#else
        return true;
#endif
    }

    static void estimateSize(StatsAlloc* allocator, const DataArray& arr, size_t cacheSize = 512)
    {
        UMBRA_UNREF(allocator);
        UMBRA_UNREF(arr);
        UMBRA_UNREF(cacheSize);

#ifdef UMBRA_REMOTE_MEMORY
        size_t elemSize = arr.m_elemSize;
        int elemsPerSlice = elemSize ? (int)(cacheSize / elemSize) : 0;
        while (elemsPerSlice && (((elemsPerSlice * elemSize) & 0xF) != 0))
            elemsPerSlice--;
        UMBRA_ASSERT(elemsPerSlice);
        allocator->allocate(elemsPerSlice * elemSize);
#endif
    }

    ~ArrayMapper (void);

    void setArray (const void* addr, size_t elemSize, int count)
    {
        UMBRA_ASSERT(!elemSize || (elemSize == (size_t)m_elemSize));
        UMBRA_ASSERT(is128Aligned(addr));
        UMBRA_ASSERT(count >= 0);
        UMBRA_UNREF(elemSize);
        m_count = count;
#ifdef UMBRA_REMOTE_MEMORY
        m_curSlice = -1;
        m_remote = (UINTPTR)addr;
#else
        m_mem = (UINT8*)addr;
#endif
    }

    void setArray (const DataArray& arr)
    {
        setArray(arr.m_ofs.getAddr(arr.m_base), arr.m_elemSize, arr.m_count);
    }

    void prefetch (int idx)
    {
        UMBRA_UNREF(idx);
#ifndef UMBRA_REMOTE_MEMORY
        UMBRA_PREFETCH(m_mem + idx * m_elemSize);
#endif
    }

    template <class T> void get(T& elem, int idx)
    {
        UMBRA_ASSERT(sizeof(T) == m_elemSize);
        UMBRA_ASSERT(idx >= 0 && (idx < m_count || m_count == -1));

#ifdef UMBRA_REMOTE_MEMORY
        int slice = idx / m_elemsPerSlice;
        idx = idx % m_elemsPerSlice;
        if (slice != m_curSlice)
        {
            fetch(slice);
            MemoryAccess::wait(m_tag.getValue());
            m_curSlice = slice;
        }
#endif
        elem = *((const T*)(m_mem + idx * m_elemSize));
    }

    const void* getOriginal (void) const
    {
#ifdef UMBRA_REMOTE_MEMORY
        return (const void*)m_remote;
#else
        return m_mem;
#endif
    }

    int getCount (void) const { return m_count; }

private:

    // Avoid double deallocation
    ArrayMapper(const ArrayMapper&);
    ArrayMapper& operator=(const ArrayMapper&);

    void fetch (int slice);
    void init (size_t elemSize, size_t cacheSize);

    StackAlloc*     m_stack;
    UINT8*          m_mem;
    UINTPTR         m_remote;
    Tag             m_tag;
    int             m_elemSize;
    int             m_elemsPerSlice;
    int             m_curSlice;
    int             m_count;
};

#ifdef UMBRA_REMOTE_MEMORY

class BitStreamReader
{
public:
    BitStreamReader (StackAlloc* s, TagManager& tags)
        : m_dwords(s, tags),
        m_cur(0)
    {
    }

    int setArray (const DataArray& arr, int bitofs, int bitcount)
    {
        int dwordOfs = UMBRA_BIT_DWORD(bitofs);
        int numDwords = UMBRA_BIT_DWORD(bitofs + bitcount + 31) - dwordOfs;
        m_dwords.setArray(arr.slice(dwordOfs, numDwords));
        if (numDwords)
        {
            m_curBitsLeft = 32 - UMBRA_BIT_IDX(bitofs);
            m_cur = m_dwords.next();
            m_cur >>= (32 - m_curBitsLeft);
        } 
		else
        {
            m_curBitsLeft = 0;
            m_cur = 0;
        }

        m_left = bitcount;
        return bitcount;
    }

    UMBRA_FORCE_INLINE UINT32 next (int width)
    {
        UMBRA_ASSERT(width >= 0 && width <= 32);
        UMBRA_ASSERT(width <= m_left);
        UINT32 ret = 0;
        int left = width;
        int outIdx = 0;
        while (left)
        {
            if (!m_curBitsLeft)
            {
                m_cur = m_dwords.next();
                m_curBitsLeft = 32;
            }
            int bits = min2(left, m_curBitsLeft);
            UINT32 val = m_cur & UMBRA_BITFIELD_MASK(bits);
            ret |= (val << outIdx);
            m_curBitsLeft -= bits;
            left -= bits;
            outIdx += bits;
            m_cur >>= bits;
        }
        m_left -= width;
        return ret;
    }

    UMBRA_FORCE_INLINE UINT32 nextNarrow (int width)
    {
        return next(width);
    }

    int hasMore (void) const
    {
        return m_left;
    }

private:
    // Avoid double deallocation
    BitStreamReader(const BitStreamReader&);
    BitStreamReader& operator=(const BitStreamReader&);

    ArrayIterator<UINT32> m_dwords;
    int m_left;
    UINT32 m_cur;
    int m_curBitsLeft;
};

#else

class BitStreamReader
{
public:
    BitStreamReader (StackAlloc* s, TagManager& tags)
        : m_buffer(NULL),
        m_left(0),
        m_curPos(0)
    {
        UMBRA_UNREF(s);
        UMBRA_UNREF(tags);
    }

    int setArray (const DataArray& arr, int bitofs, int bitcount)
    {
        if (!bitcount)
            return 0;

        m_buffer        = (const UINT32*)arr.m_ofs.getAddrNoCheck(arr.m_base);
        m_left          = bitcount;
        m_curPos        = bitofs;

        return bitcount;
    }

    UMBRA_FORCE_INLINE UINT32 nextNarrow (int width)
    {
        UMBRA_ASSERT(width >= 0 && width < 32);
        UMBRA_ASSERT(width <= m_left);
        UMBRA_ASSERT(m_buffer != NULL);
        UINT32 ret = unpackElem(m_buffer, m_curPos, width);
        m_curPos += width;
        m_left -= width;
        return ret;
    }

    UMBRA_FORCE_INLINE UINT32 next (int width)
    {
        UMBRA_ASSERT(width >= 0 && width <= 32);
        UMBRA_ASSERT(width <= m_left);
        UMBRA_ASSERT(m_buffer != NULL);

        UINT32 ret = 0;
        if (width < 32)
            ret = unpackElem(m_buffer, m_curPos, width);
        else
            ret = unpackElem32(m_buffer, m_curPos); // rare
            
        m_curPos += width;
        m_left -= width;

        return ret;
    }

    int hasMore (void) const
    {
        return m_left;
    }

private:
    // Avoid double deallocation
    BitStreamReader(const BitStreamReader&);
    BitStreamReader& operator=(const BitStreamReader&);

    const UINT32* m_buffer;
    int           m_left;
    int           m_curPos;
};
#endif

class BitVectorIterator
{
public:

    BitVectorIterator (StackAlloc* s, TagManager& tags)
        : m_reader(s, tags), m_width(0)
    {
    }

    int setArray (const DataArray& arr, int width, int ofs, int count)
    {
        m_width = width;
        m_reader.setArray(arr, width * ofs, width * count);
        return count;
    }

    UMBRA_FORCE_INLINE UINT32 next ()
    {
        return m_reader.next(m_width);
    }

    void nextN (UINT32* dst, int N)
    {
        for (int i = 0; i < N; i++)
            dst[i] = next();
    }

    int hasMore (void) const
    {
        return m_reader.hasMore() / m_width;
    }

private:
    // Avoid double deallocation
    BitVectorIterator(const BitVectorIterator&);
    BitVectorIterator& operator=(const BitVectorIterator&);

    BitStreamReader   m_reader;
    int               m_width;
};

class RangeIterator
{
public:
    RangeIterator (StackAlloc* s, TagManager& tags)
        : m_reader(s, tags), m_bitwidthElem(0), m_bitwidthCount(0), m_left(0), m_value(0), m_count(0)
    {
    }

    UMBRA_FORCE_INLINE int setArray (const DataArray& arr, int bitwidthElem, int bitwidthCount, int ofs, int count)
    {
        int width = bitwidthElem + bitwidthCount;
        m_reader.setArray(arr, width * ofs, arr.m_count * 32 - width * ofs);
        m_isWide        = (bitwidthElem + bitwidthCount) > 31;
        m_bitwidthElem  = bitwidthElem;
        m_bitwidthCount = bitwidthCount;
        m_left          = count;
        m_count         = 0;
        return count;
    }

    UMBRA_FORCE_INLINE UINT32 next (void)
    {
        if (!m_count)
        {
            UMBRA_ASSERT(m_reader.hasMore() >= m_bitwidthElem + m_bitwidthCount);
            if (!UMBRA_OPT_AVOID_BRANCHES && !m_isWide)
            {
                UINT32 v = m_reader.nextNarrow(m_bitwidthElem + m_bitwidthCount);
                m_value = v & ((1 << m_bitwidthElem) - 1);
                m_count = v >> m_bitwidthElem;
            } 
			else
            {
                m_value = m_reader.next(m_bitwidthElem);
                m_count = m_reader.next(m_bitwidthCount);
            }
        }

        m_left--;
        m_count--;
        return m_value++;
    }

    void nextN (UINT32* dst, int N)
    {
        for (int i = 0; i < N; i++)
            dst[i] = next();
    }

    int hasMore (void) const
    {
        return m_left;
    }

private:
    // Avoid double deallocation
    RangeIterator(const RangeIterator&);
    RangeIterator& operator=(const RangeIterator&);

    BitStreamReader m_reader;
    int             m_bitwidthElem;
    int             m_bitwidthCount;
    int             m_left;
    int             m_value;
    int             m_count;
    bool            m_isWide;
};

template <bool SupportExternal>
class PortalIteratorT
{
public:

    UMBRA_FORCE_INLINE PortalIteratorT (StackAlloc* s, TagManager& tags)
    : m_iter(s, tags)
    {
    }

    UMBRA_FORCE_INLINE PortalIteratorT (const ArrayIterator<Portal>::Resources& res)
    : m_iter(res)
    {
    }

    inline int init(const MappedTile& tile, CellNode& node, ExtCellNode* extNode)
    {
        m_extPortals.m_count = 0;
        m_isExternal = false;
        if (extNode && tile.hasExternalPortals())
        {
            UMBRA_ASSERT(SupportExternal);
            m_extPortals = tile.getExtPortals(*extNode);
        }
        m_iter.setArray(tile.getTile()->getPortals(node));
        return hasMore();
    }

    inline int init(const MappedTome& tome, ClusterNode& node, ExtClusterNode& extNode)
    {
        m_extPortals.m_count = 0;
        m_isExternal = false;
        if (tome.hasExternalPortals())
        {
            UMBRA_ASSERT(SupportExternal);
            m_extPortals = tome.getExtPortals(extNode);
        }
        m_iter.setArray(tome.getTome()->getClusterPortals(node));
        return hasMore();
    }

    void skip (int num)
    {
        // todo: optimize
        while (num--)
            next();
    }

    UMBRA_FORCE_INLINE bool isExternal (void) const
    {
        return SupportExternal && m_isExternal;
    }

    UMBRA_FORCE_INLINE int hasMore (void) const
    {
        int n = m_iter.hasMore();
        if (SupportExternal)
            n += m_extPortals.getCount();
        return n;
    }

    UMBRA_FORCE_INLINE const Portal& next (void)
    {
        if (SupportExternal && !m_iter.hasMore())
        {
            UMBRA_ASSERT(m_extPortals.getCount());
            m_isExternal = true;
            m_iter.setArray(m_extPortals);
            m_extPortals = DataArray();
        }
        return m_iter.next();
    }

    UMBRA_FORCE_INLINE bool isInitialized()
    {
        return m_iter.isInitialized();
    }

    ArrayIterator<Portal> m_iter;
    DataArray m_extPortals;
    bool m_isExternal;
};

typedef PortalIteratorT<true> PortalIterator;

template <bool IterateBounds>
class ObjectIterator
{
public:
    ObjectIterator(QueryContext* ctx, bool iterateGlobal, int sliceIdx = 0, int numSlices = 1) 
        : m_sliceIdx(sliceIdx)
        , m_numSlices(numSlices)
        , m_bounds(ctx->getAllocator(), ctx->getTagManager())
        , m_distances(ctx->getAllocator(), ctx->getTagManager())
    {
        m_state = ctx->getState();
        if ((!ctx->getState()->getCollection() || !ctx->getState()->getCollection()->ownsResult()) || iterateGlobal)
        {
            m_totalTomes  = 1;
            m_tomeIdx     = 0;
            m_tomesLeft   = 1;
            m_left        = 0;
            m_tome        = ctx->getTome();
            m_iterateGlobal = true;
        } else
        {
            m_totalTomes    = m_state->getNumTomeContexts();
            bucketIndices(m_sliceIdx, m_numSlices, m_totalTomes, 1, m_tomeIdx, m_tomesLeft);
            m_left          = 0;
            m_iterateGlobal = false;
        }
    }

    UMBRA_FORCE_INLINE int hasMoreTomes (void)
    {
        return m_tomesLeft;
    }

    UMBRA_FORCE_INLINE int hasMoreObjects (void)
    {       
        return m_left;
    }

    UMBRA_FORCE_INLINE void nextTome (void)
    {
        UMBRA_ASSERT(m_tomesLeft);

        if (m_iterateGlobal)
        {
            bucketIndices(m_sliceIdx, m_numSlices, m_tome->getNumObjects(), 2, m_idx, m_left);
            m_tomesLeft--;
        } else
        {
            do
            {
                m_state->mapTome(m_mappedTome, m_tomeIdx);
                m_tome = m_mappedTome.getTome();
                m_idx = 0;
                m_left = m_tome->getNumObjects();
                m_tomeIdx++;
                m_tomesLeft--;
            } while (m_left == 0 && m_tomesLeft != 0);
        }

        initBounds(m_tome);
    }

    UMBRA_FORCE_INLINE void nextObject (void)
    {
        UMBRA_ASSERT(m_left);

        if (m_iterateGlobal)
        {
            m_localIdx = m_idx;
            m_globalIdx = m_idx;
            m_idx++;
            m_left--;
        } else
        {
            m_localIdx = m_idx;
            m_globalIdx = m_mappedTome.mapLocalObject(m_localIdx);
            m_idx++;
            m_left--;
        }
    }

    UMBRA_FORCE_INLINE void fetchBounds()
    {
        if (IterateBounds)
        {
            m_currentBounds = m_bounds.next();
            if (m_hasDistances)
                m_currentDistance = m_distances.next();
        }
    }

    UMBRA_INLINE const ImpTome*        getCurrentTome    (void) const { return m_tome; }
    UMBRA_INLINE int                   getLocalIdx       (void) const { return m_localIdx; }
    UMBRA_INLINE int                   getGlobalIdx      (void) const { return m_globalIdx; }
    UMBRA_INLINE const ObjectBounds&   getObjectBounds   (void) const { return m_currentBounds; }
    UMBRA_INLINE const ObjectDistance& getObjectDistance (void) const { return m_currentDistance; }
    UMBRA_INLINE bool                  hasDistances      (void) const { return m_hasDistances; }
    UMBRA_INLINE bool                  isGlobal          (void) const { return m_iterateGlobal; }

private:

    UMBRA_INLINE void initBounds(const ImpTome* tome)
    {
        if (IterateBounds)
        {
            DataArray boundArray = tome->getObjectBounds();
            DataArray distArray  = tome->getObjectDistances();
            if (!!boundArray)
                boundArray = boundArray.slice(m_idx, m_left);
            if (!!distArray)
                distArray = distArray.slice(m_idx, m_left);
                
            m_hasDistances = !!distArray;

            m_bounds.setArray(boundArray);
            m_distances.setArray(distArray);
        }
    }

    QueryState*     m_state;
    int             m_left;
    int             m_idx;

    int             m_sliceIdx;
    int             m_numSlices;

    MappedTome      m_mappedTome;
    int             m_totalTomes;
    int             m_tomesLeft;
    int             m_tomeIdx;
    const ImpTome*  m_tome;
    bool            m_iterateGlobal;

    int             m_localIdx;
    int             m_globalIdx;

    bool                          m_hasDistances;
    ArrayIterator<ObjectBounds>   m_bounds;
    ArrayIterator<ObjectDistance> m_distances;
    ObjectBounds                  m_currentBounds;
    ObjectDistance                m_currentDistance;
};

class QueryRunner
{
public:
    QueryRunner(QueryContext& ctx): m_query(&ctx) {}

    StackAlloc* getAllocator() const { return m_query->getAllocator(); }

protected:
    QueryContext* m_query;
};

bool QueryContext::hasTile (int tileIdx) const
{
    if (m_state->tilesArePointers())
    {
        AlignedPtr<void> ptr;
        m_state->getTileArray()->get(ptr, tileIdx);
        return !!ptr;
    }
    else
    {
        DataPtr ofs;
        m_state->getTileArray()->get(ofs, tileIdx);
        return !!ofs;
    }
}

float QueryContext::getPortalExpand (int tileIdx)
{
    UMBRA_ASSERT(hasTile(tileIdx));
    const ImpTile* t = mapTile(tileIdx);
    float ret = t->getPortalExpand();
    unmapTile(t);
    return ret;
}

Umbra::UINT32* QueryContext::bitVectorFromIndexList(int size, const IndexList& list)
{
    UINT32* bv = UMBRA_HEAP_NEW_ARRAY(getAllocator(), Umbra::UINT32, UMBRA_BITVECTOR_DWORDS(size));
    if (!bv)
        return NULL;
    memset(bv, 0x00, UMBRA_BITVECTOR_SIZE(size));
    for (int i = 0; i < list.getSize(); i++)
        setBit(bv, list.getPtr()[i]);
    return bv;
}

} // namespace Umbra


#endif
