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

#include "umbraQueryContext.hpp"
#include "runtime/umbraQuery.hpp"
#include "umbraRandom.hpp"
#include "umbraQueryArgs.hpp"
#include "umbraTransformer.hpp"
#include "umbraTomeCollection.hpp"

#ifdef UMBRA_DEBUG
#   define UMBRA_DEBUG_ALLOC
#endif

#define TILEMAPPER_CACHESIZE (4 * 1024)

using namespace Umbra;

/*----------------------------------------------------------------------*//*!
 * \brief   Visualizes current cell
 *//*----------------------------------------------------------------------*/

class CellVisualizer
{
public:
    CellVisualizer(QueryContext* q, const ImpTile* tile, int cell)
        : m_tile(tile), m_query(q), m_cell(cell)
    {
        KDTree tree(tile->getTreeNodeCount(), (const Umbra::UINT32*)mapArray(q->getAllocator(), tile->getTreeData()), tile->getTreeSplits());
        m_traversal.init(tree, AABB(tile->getTreeMin(), tile->getTreeMax()));
    }

    ~CellVisualizer(void)
    {
        unmapArray(m_query->getAllocator(), m_traversal.getTree().getData());
    }

    bool containsCellRecursive(int ofs, int cellIdx)
    {
        TempBspNode cur;
        m_tile->getBSPTriangles().getElem(cur, ofs);

        bool found = false;

        {
            if (cur.isFrontLeaf())
            {
                UMBRA_ASSERT(cur.getFront() > -1);
                if (cur.getFront() == cellIdx)
                    found = true;
            }
            else
            {
                if (containsCellRecursive(cur.getFront(), cellIdx))
                    found = true;
            }
        }

        if (!found)
        {
           if (cur.isBackLeaf())
            {
                UMBRA_ASSERT(cur.getBack() > -1);
                if (cur.getBack() == cellIdx)
                    found = true;
            }
            else
            {
                if (containsCellRecursive(cur.getBack(), cellIdx))
                    found = true;
            }
        }

        return found;
    }

    void execute(void)
    {
        KDTree::Node node;
        while (m_traversal.next(node))
        {
            Umbra::UINT32 data = m_tile->getNodeData(m_traversal.getTree().getLeafIdx(node.getIndex()));
            if (data == 0xFFFFFFFF)
                continue;
            bool nodeInCell = false;
            if (data & 0x80000000)
            {
                // find from BSP
                int ofs = data & 0x7fffffff;
                nodeInCell = containsCellRecursive(ofs, m_cell);
            }
            else
            {
                // standard node
                nodeInCell = ((int)data == m_cell);
            }

            if (nodeInCell)
                m_query->addQueryDebugAABB(node.getAABBMin(), node.getAABBMax(), Vector4(0.5f, 0.5f, 0.5f, 0.5f));
        }
    }

private:

    KDTraversal<>   m_traversal;
    const ImpTile*  m_tile;
    QueryContext*   m_query;
    int             m_cell;
};

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

const void* Umbra::mapArray (StackAlloc* s, const DataArray& arr, int first, int num)
{
    UMBRA_UNREF(s);
    UMBRA_ASSERT(first + num <= arr.m_count || arr.m_count == -1 || num == -1);
    UMBRA_ASSERT(!!arr);

    const void* addr = (UINT8*)arr.m_ofs.getAddr(arr.m_base) + first * arr.m_elemSize;

#ifndef UMBRA_REMOTE_MEMORY
    UMBRA_UNREF(num);
    return addr;
#else
    if (num < 0)
    {
        UMBRA_ASSERT(arr.m_count > 0);
        num = arr.m_count - first;
        UMBRA_ASSERT(num >= 0);
    }
    UINTPTR remote = (uintptr_t)addr;
    size_t size = num * arr.m_elemSize;
    size += (remote & 0xF);         // start from qword boundary
    size = (size + 0xF) & ~0xF;     // full qwords

    UINT8* local = (UINT8*)s->allocate(size);
    if (!local)
        return NULL;

    UINT8* dst = local;
    UINTPTR src = remote & ~0xF;

    while (size)
    {
        size_t batch = min2((size_t)(16 << 10), size);
        MemoryAccess::alignedRead(dst, (const void*)src, batch);
        dst += batch;
        src += batch;
        size -= batch;
    }

    return local + (remote & 0xF);
#endif
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

const void* Umbra::mapArray (StackAlloc* s, const BitDataArray& arr, int first, int num)
{
    int firstword = UMBRA_BIT_DWORD(first + arr.m_bitOffset);
    int numwords = (num == -1) ? -1 : (int)(UMBRA_BIT_DWORD(first + arr.m_bitOffset + num) - firstword + 1);
    return mapArray(s, arr.m_array, firstword, numwords);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void Umbra::unmapArray (StackAlloc* s, const void* data)
{
#ifndef UMBRA_REMOTE_MEMORY
    UMBRA_UNREF(s);
    UMBRA_UNREF(data);
#else
    data = (const void*)((size_t)data & ~0xF);
    s->deallocate((void*)data);
#endif
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void* StatsAlloc::allocate    (size_t size, const char* info)
{
    UMBRA_UNREF(info);

    size += 16;
    //size = UMBRA_ALIGN(size, 16);
    size += 16;

    size_t ptr = m_current;
    m_current += size;

#ifdef UMBRA_DEBUG_ALLOC
    ptr += 16;
#endif

    return (void*)(ptr + 16);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void StatsAlloc::deallocate  (void* ptr_)
{
    size_t ptr = (size_t)ptr_ - 16;

    if (!ptr)
        return;

#ifdef UMBRA_DEBUG_ALLOC
    ptr -= 16;
#endif

    UMBRA_ASSERT(ptr < m_current || !"Bad allocation pattern");
    m_current = ptr;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

StackAlloc::StackAlloc(void* buf, size_t size)
{
    m_start = (UINT8*)UMBRA_ALIGN(buf, 16);
    size -= (m_start - (UINT8*)buf);
    m_end = m_start + size;
    m_parent = NULL;
    reset();
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

StackAlloc::StackAlloc(StackAlloc& orig)
{
    m_parent = &orig;
    size_t size = orig.available();
    m_start = (UINT8*)orig.allocate(size);
    m_end = m_start + size;
    reset();
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

StackAlloc::~StackAlloc(void)
{
    UMBRA_ASSERT(m_start == m_cur);
    if (m_parent)
        m_parent->deallocate(m_start);
}

void StackAlloc::reset(void* buf, size_t size)
{
    m_start = (UINT8*)UMBRA_ALIGN(buf, 16);
    size -= (m_start - (UINT8*)buf);
    m_end = m_start + size;
    m_parent = NULL;
    reset();
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void* StackAlloc::allocate (size_t size, const char* info)
{
    UMBRA_UNREF(info);

#ifdef UMBRA_DEBUG_ALLOC
    size += 16;
#endif
    size = UMBRA_ALIGN(size, 16);

    if (m_cur + size > m_end)
    {
        UMBRA_ASSERT(!"Out of memory");
        return NULL;
    }

    UINT8* a = m_cur;
    m_cur += size;

#ifdef UMBRA_DEBUG_ALLOC
    *(size_t*)a = size;
    a += 16;
    m_min = min2(m_min, available());
#endif

    return a;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void StackAlloc::deallocate(void* ptr_)
{
    UINT8* ptr = (UINT8*)ptr_;

    if (!ptr)
        return;

#ifdef UMBRA_DEBUG_ALLOC
    ptr -= 16;
    size_t size = *((size_t*)ptr);
    UMBRA_ASSERT(ptr + size == m_cur || !"Bad allocation pattern");
#endif

    UMBRA_ASSERT(ptr < m_cur || !"Bad allocation pattern");
    UMBRA_ASSERT(ptr >= m_start);
    m_cur = (UINT8*)ptr;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

QueryState::QueryState (void* mem, size_t size)
:   
#ifdef UMBRA_REMOTE_MEMORY
    m_lastCache(UMBRA_MAX_MAPPED_TILES - 1),
#endif
    m_tome(NULL),
    m_collection(NULL),
    m_tiles(NULL),
    m_portalStates(NULL),
    m_gateCosts(NULL),
    m_tilePaths(NULL),
    m_bitsPerTilePath(0),
    m_debug(NULL),
    m_queryStack(mem, size),
    m_workStack(NULL, 0),
    m_curStack(&m_queryStack),
    m_tags(MemoryAccess::tagHead()),
    m_spuUsage(Query::SPU_USAGE_SPURS_THREAD0)
{
#ifdef UMBRA_DEBUG
    bitOpsTest();
#endif
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryState::setWorkMem(Umbra::UINT8* workMem, size_t workMemSize)
{
    m_workStack.reset(workMem, workMemSize);
    if (workMem)
        m_curStack = &m_workStack;
    else
        m_curStack = &m_queryStack;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryState::getWorkMem(Umbra::UINT8*& workMem, size_t& workMemSize)
{
    workMem     = m_workStack.getPtr();
    workMemSize = m_workStack.getSize();
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

const void* QueryState::importRemoteObj (StackAlloc& alloc, const void* remote, Umbra::UINT32 size)
{
#ifndef UMBRA_REMOTE_MEMORY
    UMBRA_UNREF(alloc);
    UMBRA_UNREF(size);
    return remote;
#else
    UINT32 aligned = UMBRA_ALIGN(size, 16);
    void* local = alloc.allocate(aligned + sizeof(remote));
    MemoryAccess::alignedRead(local, remote, aligned);
    const void** base = (const void**)((UINT8*)local + size);
    *base = remote;
    return local;
#endif
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryState::setQueryData (const ImpTome* t, const ImpTomeCollection* c)
{
    if (m_tilePaths)
        UMBRA_HEAP_DELETE(&m_queryStack, m_tilePaths);
    if (m_tiles)
        UMBRA_HEAP_DELETE(&m_queryStack, m_tiles);

    // force free rest of allocations
    m_queryStack.reset();

    m_tome = NULL;
    m_collection = NULL;
    m_tilePaths = NULL;
    m_tiles = NULL;

    if (c)
    {
        UMBRA_ASSERT(!t);
        m_collection = (ImpTomeCollection*)importRemoteObj(m_queryStack, c, sizeof(ImpTomeCollection));
        t = m_collection->getResult();
    }

    if (t)
    {
        // get tome
        ImpTome* localTome = (ImpTome*)importRemoteObj(m_queryStack, t, sizeof(ImpTome));
        if ((localTome->getVersionMagic() & 0xFFFF) != TOME_VERSION)
            return;
        m_tome = localTome;

        // create tile mapper
        m_tiles = UMBRA_HEAP_NEW(&m_queryStack, ArrayMapper, &m_queryStack, m_tags,
            tilesArePointers() ? sizeof(AlignedPtr<const ImpTile*>) : sizeof(DataPtr), TILEMAPPER_CACHESIZE);
        DataArray tileArr(m_tome->getTileOffsets(tilesArePointers()));
        tileArr.m_count = m_tome->getTreeNodeCount();
        m_tiles->setArray(tileArr);

        // get tile path array, as a fallback generate it here
        m_tilePaths = UMBRA_HEAP_NEW(&m_queryStack, ArrayMapper, &m_queryStack, m_tags, sizeof(UINT32));
        m_tilePaths->setArray(m_tome->getTilePaths());
        m_bitsPerTilePath = m_tome->getBitsPerSlotPath();

        m_mappedTome = MappedTome(0, NULL, getRootTome(), getRootTome()->getTileBase(), 0, 0, 0);
    }
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

QueryContext::QueryContext (QueryState* state, Umbra::UINT32 flags)
:   m_state(state),
    m_stack(*state->getAllocator()),
    m_numMappedTiles(0),
    m_error(0),
    m_debugFlags(flags),
    m_tags(state->getTagManager()),
    m_fpState(SIMDSaveState()),
    m_tileCache(NULL)
{
    m_intStats = UMBRA_NEW_ARRAY(int, QUERYSTAT_LAST);
    for (int i = 0; i < QUERYSTAT_LAST; i++)
        m_intStats[i] = -1;
#ifdef UMBRA_REMOTE_MEMORY
    m_tileCache = UMBRA_NEW_ARRAY(TileCache, UMBRA_MAX_MAPPED_TILES);
#endif
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

#if UMBRA_ARCH != UMBRA_SPU
#   define REPORT_STAT(n) \
    if (m_intStats[QUERYSTAT_ ## n] >= 0) \
    m_state->getDebugRenderer()->addStat(#n, m_intStats[QUERYSTAT_ ## n]);
#else
#   define REPORT_STAT(n)
#endif

QueryContext::~QueryContext (void)
{
    UMBRA_ASSERT(m_numMappedTiles == 0);
#ifdef UMBRA_REMOTE_MEMORY
    UMBRA_DELETE_ARRAY(m_tileCache);
#endif
    if (debugEnabled(Query::DEBUGFLAG_STATISTICS))
    {
        REPORT_STAT(TILES_VISITED);
        REPORT_STAT(PORTALS_PROCESSED);
        REPORT_STAT(EXT_PORTALS_PROCESSED);
        REPORT_STAT(CELLS_PROCESSED);
        REPORT_STAT(CELL_REVISITS);
        REPORT_STAT(CELL_SORT_FAILURES);
        REPORT_STAT(OBJECTS_STATICALLY_CULLED);
        /* \todo [antti 19.11.2012]: add stats to report here */
    }
    UMBRA_DELETE_ARRAY(m_intStats);
    SIMDRestoreState(m_fpState);
}

#undef REPORT_STAT

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void* QueryContext::allocWorkMem (size_t size, bool clear)
{
    void* ptr = UMBRA_HEAP_ALLOC(&m_stack, size);
    if (ptr && clear)
        memset(ptr, 0, size);
    return ptr;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryState::mapTome (MappedTome& mapped, int tomeIdx)
{
    const ImpTome* tome = NULL;
    const TomeContext* ctx = (m_collection && (tomeIdx != -1)) ?
        m_collection->getTomeContext(tomeIdx) : NULL;

    if (ctx)
    {
#ifdef UMBRA_REMOTE_MEMORY
        for (int i = 0; i < UMBRA_MAX_MAPPED_TILES; i++)
        {
            if (m_tomeCache[i].ctxBase.base == ctx)
            {
                ctx  = &m_tomeCache[i].ctx;
                tome = &m_tomeCache[i].tome;
                break;
            }
        }

        if (!tome)
        {
            m_lastCache = (m_lastCache + 1) % UMBRA_MAX_MAPPED_TILES;
            TomeCache& tomeCache = m_tomeCache[m_lastCache];

            void* local = &tomeCache.ctx;
            MemoryAccess::alignedRead(local, ctx, sizeof(TomeContext));
            tomeCache.ctxBase.base = (void*)ctx;
            ctx   = (const TomeContext*)local;

            tome  = ctx->m_tome;
            local = &tomeCache.tome;
            MemoryAccess::alignedRead(local, tome, sizeof(ImpTome));
            tomeCache.tomeBase.base = (void*)tome;
            tome  = (const ImpTome*)local;
        }
#else
        if (ctx)
            tome = ctx->m_tome;
#endif

        UMBRA_ASSERT(!!getRootTome()->getClusterStarts().m_ofs &&
                     !!getRootTome()->getClusterPortalStarts().m_ofs);

        int clusterStart = 0, clusterPortalStart = 0, clusterPortalEnd = 0;
        getRootTome()->getClusterStarts().getElem(clusterStart, tomeIdx);
        getRootTome()->getClusterPortalStarts().getElem(clusterPortalStart, tomeIdx);
        getRootTome()->getClusterPortalStarts().getElem(clusterPortalEnd,   tomeIdx + 1);

        mapped = MappedTome(tomeIdx, ctx, tome, getRootTome()->getTileBase(), clusterStart, clusterPortalStart, clusterPortalEnd - clusterPortalStart);
        m_mappedTome = mapped;
    }
    else
    {
        mapped = MappedTome(0, NULL, getRootTome(), getRootTome()->getTileBase(), 0, 0, 0);
        m_mappedTome = mapped;
    }
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

int QueryState::findTomeByCluster(int globalCluster)
{
    if (globalCluster < 0 || globalCluster >= getRootTome()->getNumClusters())
        return -1;

    if (!getNumTomeContexts())
        return 0;

    if (getDefaultTome().containsCluster(globalCluster))
        return getDefaultTome().getIndex();

    // todo: check currently mapped tome

    ArrayMapper starts(getAllocator(), m_tags, getRootTome()->getClusterStarts());

    // binary search
    int start = 0;
    int end   = getNumTomeContexts() - 1;
    while (end - start > 0)
    {
        int mid   = (start + end + 1) / 2;
        int value = 0;
        starts.get(value, mid);

        if (globalCluster >= value)
            start = mid;
        else
            end = mid - 1;
    }

    return start;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

int QueryState::findTomeByClusterPortal(int globalPortal)
{
    if (globalPortal < 0 || globalPortal >= getRootTome()->getNumClusterPortals())
        return -1;

    if (!getNumTomeContexts())
        return 0;

    if (getDefaultTome().containsClusterPortal(globalPortal))
        return getDefaultTome().getIndex();

    // todo: check currently mapped tome

    ArrayMapper starts(getAllocator(), m_tags, getRootTome()->getClusterPortalStarts());

    // binary search
    int start = 0;
    int end   = getNumTomeContexts() - 1;
    while (end - start > 0)
    {
        int mid   = (start + end + 1) / 2;
        int value = 0;
        starts.get(value, mid);

        if (globalPortal >= value)
            start = mid;
        else
            end = mid - 1;
    }

    return start;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::mapTile (MappedTile& mapped, int idx)
{
    const ImpTile* tile = mapTile(idx);
    if (tile)
    {
        const ExtTile* extTile = m_state->getExtTile(idx);
        MappedTome mappedTome;

        if (extTile)
        {
#ifdef UMBRA_REMOTE_MEMORY
            TileCache& cache = *(TileCache*)tile;

            void* local = &cache.extTile;
            MemoryAccess::alignedRead(local, extTile, sizeof(ExtTile));
            cache.extBase.base = (void*)extTile;
            extTile = (const ExtTile*)local;
#endif
            m_state->mapTome(mappedTome, extTile->getTomeIdx());
        }
        else
        {
            m_state->mapTome(mappedTome, -1);
        }
        mapped = MappedTile(idx, tile, extTile, mappedTome);
    }
    else
    {
        mapped = MappedTile();
    }
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

const ImpTile* QueryContext::mapTile (int tileIdx)
{
    AlignedPtr<const ImpTile> addr;
    addr = (const ImpTile*)NULL;
    UMBRA_ASSERT(tileIdx >= 0 && tileIdx < getTome()->getTreeNodeCount());
    UMBRA_ASSERT(m_numMappedTiles < UMBRA_MAX_MAPPED_TILES);

    if (m_state->tilesArePointers())
    {
        m_state->getTileArray()->get(addr, tileIdx);
    }
    else
    {
        DataPtr ofs;
        m_state->getTileArray()->get(ofs, tileIdx);
        addr = (const ImpTile*)ofs.getAddr(getTome()->getTileBase());
    }

    if (!!addr)
    {
        UMBRA_DEBUG_CODE(m_numMappedTiles++);
#ifdef UMBRA_REMOTE_MEMORY
        int idx = 0;
        for (; idx < UMBRA_MAX_MAPPED_TILES; idx++)
        {
            if (!m_tileCache[idx].tileBase.base)
                break;
        }
        if (idx == UMBRA_MAX_MAPPED_TILES)
        {
            UMBRA_ASSERT(!"Too many tiles mapped");
            return NULL;
        }
        void* local = &m_tileCache[idx].tile;
        MemoryAccess::alignedRead(local, addr, sizeof(ImpTile));
        m_tileCache[idx].tileBase.base = (void*)(const ImpTile*)addr;
        addr = (const ImpTile*)local;
#endif
    }

    return addr;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

int QueryContext::findSlot (const Vector3& coord)
{
    int idx = -1;
    NodeLocator* locator = UMBRA_NEW(NodeLocator);
    if (!locator)
        return -1;

    const UINT32* treeData   = (const UINT32*)mapArray(getAllocator(), getTome()->getTreeData());
    if (!treeData)
    {
        UMBRA_DELETE(locator);
        return -1;
    }

    KDTree tree(getTome()->getTreeNodeCount(), treeData, getTome()->getTreeSplits());
    KDTree::Node node;
    if (locator->findNode(tree, getTome()->getAABB(), coord, node))
        idx = node.getIndex();
    unmapArray(getAllocator(), tree.getData());
    UMBRA_DELETE(locator);
    return idx;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

int QueryContext::findNodeInTile (const ImpTile* tile, const Vector3& coord)
{
    UMBRA_ASSERT(coord.x >= tile->getTreeMin().x && coord.y >= tile->getTreeMin().y && coord.z >= tile->getTreeMin().z);
    UMBRA_ASSERT(coord.x <= tile->getTreeMax().x && coord.y <= tile->getTreeMax().y && coord.z <= tile->getTreeMax().z);
    
    // mapArray asserts with UMBRA_REMOTE_MEMORY otherwise
    if (!tile->getTreeNodeCount())
        return -1;

    KDTree tree(tile->getTreeNodeCount(),
        (const UINT32*)mapArray(getAllocator(), tile->getTreeData()), tile->getTreeSplits());
    KDTree::Node node;
    int idx = -1;

    NodeLocator* locator = UMBRA_NEW(NodeLocator);
    if (locator && locator->findNode(tree, tile->getAABB(), coord, node))
        idx = tree.getLeafIdx(node.getIndex());
    UMBRA_DELETE(locator);
    unmapArray(getAllocator(), tree.getData());
    return idx;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

Umbra::Cell QueryContext::findCell (const Vector3& coord)
{
    int slotIdx = findSlot(coord);
    if (slotIdx == -1)
        return Cell(-1, -1);
    MappedTile mappedTile;
    mapTile(mappedTile, slotIdx);
    const ImpTile* tile = mappedTile.getTile();
    if (!tile)
        return Cell(-1, -1);
    int node = findNodeInTile(tile, coord);
    if (node == -1)
    {
        unmapTile(mappedTile);
        return Cell(-1, -1);
    }
    int cellIdx = tile->getCellIndex(node, coord);
    /* \todo [antti 2.4.2013]: refactor this */
    if (cellIdx >= tile->getNumCells())
        cellIdx = -1;
    unmapTile(mappedTile);
    return Cell(slotIdx, cellIdx);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool QueryContext::isGateOpen(const MappedTile& tile, const Portal& portal)
{
    UMBRA_ASSERT(portal.isUser());

    int gateCount = portal.getUserObjCount();
    for (int i = 0; i < gateCount; i++)
    {
        int gateIdx;
        tile.getMappedTome().getTome()->getGateIndices().getElem(gateIdx, portal.getUserObjOfs()+i);

        gateIdx = tile.getMappedTome().mapLocalGate(gateIdx);
        UMBRA_ASSERT(gateIdx >= 0 && gateIdx < getTome()->getNumGates());

        if (!getState()->getPortalState(gateIdx))
            return false;
    }

    return true;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

float QueryContext::getGateCost(const MappedTile& tile, const Portal& portal)
{
    UMBRA_ASSERT(portal.isUser());
    const float* gateCosts = getState()->getGateCosts();
    if (!gateCosts)
        return 0.f;

    float cost = 0.f;

    int gateCount = portal.getUserObjCount();
    for (int i = 0; i < gateCount; i++)
    {
        int gateIdx;
        tile.getMappedTome().getTome()->getGateIndices().getElem(gateIdx, portal.getUserObjOfs()+i);

        gateIdx = tile.getMappedTome().mapLocalGate(gateIdx);
        UMBRA_ASSERT(gateIdx >= 0 && gateIdx < getTome()->getNumGates());

        cost += gateCosts[gateIdx];
    }

    return cost;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool QueryContext::isGateOpen(const Portal& portal)
{
    UMBRA_ASSERT(portal.isUser());

    int gateCount = portal.getUserObjCount();
    for (int i = 0; i < gateCount; i++)
    {
        int gateIdx;
        getDefaultTome().getTome()->getGateIndices().getElem(gateIdx, portal.getUserObjOfs()+i);

        gateIdx = getDefaultTome().mapLocalGate(gateIdx);
        UMBRA_ASSERT(gateIdx >= 0 && gateIdx < getTome()->getNumGates());

        if (!getState()->getPortalState(gateIdx))
            return false;
    }

    return true;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

float QueryContext::getGateCost(const Portal& portal)
{
    UMBRA_ASSERT(portal.isUser());
    const float* gateCosts = getState()->getGateCosts();
    if (!gateCosts)
        return 0.f;

    float cost = 0.f;

    int gateCount = portal.getUserObjCount();
    for (int i = 0; i < gateCount; i++)
    {
        int gateIdx;
        getDefaultTome().getTome()->getGateIndices().getElem(gateIdx, portal.getUserObjOfs()+i);

        gateIdx = getDefaultTome().mapLocalGate(gateIdx);
        UMBRA_ASSERT(gateIdx >= 0 && gateIdx < getTome()->getNumGates());

        cost += gateCosts[gateIdx];
    }

    return cost;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

class MultiPointTraverse: public TraverseFilter<Umbra::UINT32>
{
public:
    MultiPointTraverse(void) {}
    MultiPointTraverse(const Vector3* points, int numPoints, Umbra::UINT32 rootMask)
        : points(points), numPoints(numPoints), rootMask(rootMask)
    {
        UMBRA_ASSERT(rootMask);
    }

    /* intentionally not inlined */
    bool filterPoints (const NodeType& n, Umbra::UINT32 inmask) const;

    bool initialNode (const NodeType& n) const
    {
        return filterPoints(n, rootMask);
    }

    bool pushNode (const NodeType& n) const
    {
        return filterPoints(n, n.userData());
    }

    const Vector3* points;
    int numPoints;
    Umbra::UINT32 rootMask;
};

bool MultiPointTraverse::filterPoints (const NodeType& n, Umbra::UINT32 inmask) const
{
    UMBRA_ASSERT(inmask);
    AABB bounds(n.treeNode().getAABB());
    Umbra::UINT32 outmask = 0;
    Umbra::UINT32 curmask = inmask;
    int pos = 0;
    while (curmask && (pos < 32))
    {
        int next = lowestBitSet(curmask);
        pos += next;
        UMBRA_ASSERT(pos < numPoints);
        if (bounds.contains(points[pos]))
            outmask |= (1 << pos);
        curmask >>= (next + 1);
        pos++;
    }
    if (outmask)
    {
        n.userData() = outmask;
        return true;
    }
    return false;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::findMultipleCells  (const Vector3* coord, Cell* results, int count)
{
    KDTraversal<MultiPointTraverse> tiletraverse;
    KDTraversal<MultiPointTraverse> celltraverse;

    const UINT32* treeData   = (const UINT32*)mapArray(getAllocator(), getTome()->getTreeData());
    UINT32 rootMask = ((UINT32)(1 << count) & (~UMBRA_SIGN_EXTEND(count - 31))) - 1;
    KDTree tileTree(getTome()->getTreeNodeCount(), treeData, getTome()->getTreeSplits());
    tiletraverse.init(tileTree, AABB(getTome()->getTreeMin(), getTome()->getTreeMax()), MultiPointTraverse(coord, count, rootMask));

    KDTree::Node tileNode;
    UINT32 tileMask;
    while (tiletraverse.next(tileNode, tileMask))
    {
        int tileIdx = tileNode.getIndex();
        MappedTile mappedTile;
        mapTile(mappedTile, tileIdx);
        const ImpTile* tile = mappedTile.getTile();
        KDTree cellTree(tile->getTreeNodeCount(), (const UINT32*)mapArray(getAllocator(), tile->getTreeData()), tile->getTreeSplits());
        celltraverse.init(cellTree, AABB(tile->getTreeMin(), tile->getTreeMax()), MultiPointTraverse(coord, count, tileMask));
        KDTree::Node node;
        UINT32 mask;
        while (celltraverse.next(node, mask))
        {
            int pos = 0;
            int leaf = cellTree.getLeafIdx(node.getIndex());
            while (mask && (pos < 32))
            {
                int next = lowestBitSet(mask);
                pos += next;
                UMBRA_ASSERT(pos < count);
                results[pos].slotIdx = tileIdx;
                results[pos].index = tile->getCellIndex(leaf, coord[pos]);
                mask >>= (next + 1);
                pos++;
            }
        }
        unmapArray(getAllocator(), cellTree.getData());
        unmapTile(mappedTile);
    }

    unmapArray(getAllocator(), tileTree.getData());
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

int QueryContext::findCluster (const Vector3& coord)
{
    Cell cell = findCell(coord);
    if (!cell.valid())
        return -1;
    MappedTile mappedTile;
    mapTile(mappedTile, cell.slotIdx);
    const ImpTile* tile = mappedTile.getTile();
    if (!tile)
        return -1;
    int cluster = tile->getClusterIndex(cell.index);
    unmapTile(mappedTile);
    return cluster;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::addQueryDebugPoint (const Vector3& pt, const Vector4& color)
{
    if (m_state->getDebugRenderer())
        m_state->getDebugRenderer()->addPoint(pt, color);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::addQueryDebugLine (const Vector3& start, const Vector3& end, const Vector4& color)
{
    if (m_state->getDebugRenderer())
        m_state->getDebugRenderer()->addLine(start, end, color);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::addQueryDebugAABB  (const Vector3& aabbMin, const Vector3& aabbMax, const Vector4& color, bool solid)
{
    DebugRenderer* d = m_state->getDebugRenderer();
    if (!d)
        return;

    if (solid)
    {
        const int steps = 10;
        AABB aabb(aabbMin, aabbMax);
        Vector3 quad[4];

        for (int i = 0; i < 6; i++)
        {
            aabb.getSideQuad(i, quad);

            d->addLine(quad[0], quad[1], color);
            d->addLine(quad[1], quad[2], color);
            d->addLine(quad[2], quad[3], color);
            d->addLine(quad[3], quad[0], color);

            for (float a = 0; a <= 1.f; a += 1.f / (float)steps)
            {
                Vector3 pos1 = (quad[1] - quad[0]) * a + quad[0];
                Vector3 pos2 = (quad[3] - quad[0]) * a + quad[0];
                d->addLine(pos1, pos2, color);
            }

            for (float a = 0; a <= 1.f; a += 1.f / (float)steps)
            {
                Vector3 pos1 = (quad[2] - quad[1]) * a + quad[1];
                Vector3 pos2 = (quad[2] - quad[3]) * a + quad[3];
                d->addLine(pos1, pos2, color);
            }
        }
    }
    else
    {
        d->addAABB(aabbMin, aabbMax, color);
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::addQueryDebugQuad (const Vector3& x0y0, const Vector3& x0y1, const Vector3& x1y1, const Vector3& x1y0, const Vector4& color)
{
    if (m_state->getDebugRenderer())
        m_state->getDebugRenderer()->addQuad(x0y0, x0y1, x1y1, x1y0, color);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/
void QueryContext::addQueryDebugSphere(const Vector3& center, float radius, const Vector4& color)
{
    const int thetaSubdivisionCount = 16;
    const int phiSubdivisionCount = 32;
    const float PI = 3.14159265358f;
    const float deltaTheta = PI/float(thetaSubdivisionCount);
    const float deltaPhi = 2.0f*PI/float(phiSubdivisionCount);

    for (int I = 0; I < phiSubdivisionCount; I++)
    {
        float phi1 = I*deltaPhi;
        float phi2 = (I+1)*deltaPhi;

        for (int J = 0; J < thetaSubdivisionCount; J++)
        {
            float theta1 = J*deltaTheta;
            float theta2 = (J+1)*deltaTheta;

            Vector3 V0 = center + radius*Vector3(sinf(theta1)*cosf(phi1), sinf(theta1)*sinf(phi1), cosf(theta1));
            Vector3 V1 = center + radius*Vector3(sinf(theta1)*cosf(phi2), sinf(theta1)*sinf(phi2), cosf(theta1));
            Vector3 V2 = center + radius*Vector3(sinf(theta2)*cosf(phi2), sinf(theta2)*sinf(phi2), cosf(theta2));
            Vector3 V3 = center + radius*Vector3(sinf(theta2)*cosf(phi1), sinf(theta2)*sinf(phi1), cosf(theta2));

            addQueryDebugLine(V0, V1, color);
            addQueryDebugLine(V1, V2, color);
            addQueryDebugLine(V2, V3, color);
            addQueryDebugLine(V3, V0, color);
        }
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Vector4 QueryContext::clusterColor (int idx)
{
#if UMBRA_ARCH == UMBRA_SPU
    return Vector4(1.f, 1.f, 1.f, 1.f);
#else
    Random rnd;
    rnd.reset(idx * 444 + 555);
    return Vector4(rnd.get(), rnd.get(), rnd.get(), 1.f);
#endif
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::visualizeCell(const ImpTile* tile, int cell)
{
    CellVisualizer visualizer(this, tile, cell);
    visualizer.execute();
    addQueryDebugAABB(tile->getTreeMin(), tile->getTreeMax(), Vector4(1.f, 1.f, 0.f, 1.f));
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::visualizeTiles()
{
    const UINT32* treeData = (const UINT32*)mapArray(getAllocator(), getTome()->getTreeData());
    KDTree tree(getTome()->getTreeNodeCount(), treeData, getTome()->getTreeSplits());
    KDTraversal<> traverse;
    traverse.init(tree, getTome()->getAABB());
    KDTree::Node n;
    while (traverse.next(n))
    {
        addQueryDebugAABB(n.getAABBMin(), n.getAABBMax(), Vector4(1.f, 1.f, 0.f, 1.f));
    }
    unmapArray(getAllocator(), treeData);
}


/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryContext::visualizeFrustum(const Transformer& transformer)
{
    Vector3 v[8];
    for (int i = 0; i < 4; i++)
        v[i] = transformer.getFrustumCorner(i);

    addQueryDebugLine(v[0], v[1], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[1], v[3], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[3], v[2], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[2], v[0], Vector4(1.f, 1.f, 0.f, 1.f));

    if (!transformer.hasFarPlane())
        return;

    for (int i = 4; i < 8; i++)
        v[i] = transformer.getFrustumCorner(i);

    addQueryDebugLine(v[4], v[5], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[5], v[7], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[7], v[6], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[6], v[4], Vector4(1.f, 1.f, 0.f, 1.f));

    addQueryDebugLine(v[0], v[4], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[1], v[5], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[2], v[6], Vector4(1.f, 1.f, 0.f, 1.f));
    addQueryDebugLine(v[3], v[7], Vector4(1.f, 1.f, 0.f, 1.f));
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ArrayMapper::~ArrayMapper (void)
{
#ifdef UMBRA_REMOTE_MEMORY
    m_stack->deallocate(m_mem);
#endif
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ArrayMapper::init (size_t elemSize, size_t cacheSize)
{
    UMBRA_ASSERT((cacheSize & 0xF) == 0);
    m_elemSize = (int)elemSize;
    UMBRA_UNREF(cacheSize);
#ifdef UMBRA_REMOTE_MEMORY
    m_elemsPerSlice = m_elemSize ? (int)(cacheSize / elemSize) : 0;
    while (m_elemsPerSlice && (((m_elemsPerSlice * elemSize) & 0xF) != 0))
        m_elemsPerSlice--;
    m_mem = (UINT8*)m_stack->allocate(m_elemsPerSlice * elemSize);
#endif
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ArrayMapper::fetch (int slice)
{
    UMBRA_ASSERT(slice >= 0);
    UINT32 batch = m_elemsPerSlice * m_elemSize;
    UINT32 ofs = slice * batch;
    UINT32 bound = ((m_count * m_elemSize) + 0xF) & ~0xF;
    UINT32 end = min2(bound, ofs + batch);
    UINTPTR remote = m_remote + ofs;
    UINT32 size = end - ofs;
    MemoryAccess::alignedReadAsync(m_mem, (const void*)remote, size, m_tag.getValue());
    m_curSlice = slice;
}
