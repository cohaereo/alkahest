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
 * \brief   Runtime tome generator.
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraMemory.hpp"
#include "umbraArray.hpp"
#include "umbraQueryContext.hpp"
#include "umbraRuntimeTomeGenerator.hpp"
#include "umbraStructBuilder.hpp"
#include "umbraRect.hpp"
#include "umbraUnionFind.hpp"

using namespace Umbra;

#define UMBRA_EMPTY_SLOT_CELLS 1

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

namespace Umbra
{

typedef RuntimeTomeGenerator::ExternalPortal ExternalPortal;
typedef RuntimeTomeGenerator::GeneratorTree  GeneratorTree;

const UINT32 RuntimeTomeGenerator::m_defaultTreeData[] = {3};       // b11, one leaf
const UINT32 RuntimeTomeGenerator::m_defaultMap[2]     = {0, 0};

static const SIMDRegister   s_simdOneOver65535 = SIMDLoad(1.0f / 65535.0f);
static const SIMDRegister32 s_simd65535        = SIMDLoad32(65535);

/*-------------------------------------------------------------------*//*!
 * \brief   Generates portals between two tiles, A and B
 *//*-------------------------------------------------------------------*/

class TileMatcher
{
public:

    TileMatcher(Array<ExternalPortal>& portals, Array<int>& heads, GeneratorTree& treeA, GeneratorTree& treeB, int face, int slotB);

private:
    void matchTiles(int idxA, const Recti& rectA,
                    int idxB, const Recti& rectB);

    Array<ExternalPortal>&                      m_portals;  // Linked list of portals
    Array<int>&                                 m_heads;
    int                                         m_faceA;    // A's face, on which portals are generated
    AABB                                        m_boundsA;  // A's bounds

    KDTree&                                     m_treeA;    // A's matching tree
    const UINT32*                               m_mappingA;
    int                                         m_widthA;
    int                                         m_invalidA;

    KDTree&                                     m_treeB;    // B's matching tree
    const UINT32*                               m_mappingB;
    int                                         m_widthB;
    int                                         m_slotB;
    int                                         m_invalidB;

    float                                       m_portalExpand;
    int                                         m_faceAxes[3]; // world axis to face axis


    TileMatcher& operator=(const TileMatcher&);
};

/*-------------------------------------------------------------------*//*!
 * \brief   Generates portals at border of the scene for tile "A",
 *          when there's not other tile
 *//*-------------------------------------------------------------------*/

class BorderMatcher
{
public:

    BorderMatcher(Array<ExternalPortal>& portals, Array<int>& heads, GeneratorTree& treeA, int face);

private:
    void matchBorder(int idxA, const Recti& rectA);

    Array<ExternalPortal>&                      m_portals;  // Linked list of portals
    Array<int>&                                 m_heads;
    int                                         m_faceA;    // A's face, on which portals are generated
    AABB                                        m_boundsA;  // A's bounds

    KDTree&                                     m_treeA;    // A's matching tree
    const UINT32*                               m_mappingA;
    int                                         m_widthA;
    int                                         m_invalidA;

    float                                       m_portalExpand;
    int                                         m_faceAxes[3]; // world axis to face axis

    BorderMatcher& operator=(const BorderMatcher&);
};

/*-------------------------------------------------------------------*//*!
 * \brief   Convert world-space rect to tile-local fixed point
 *//*-------------------------------------------------------------------*/

static inline Recti fixedPoint(const Vector4& rect, const AABB& bounds, int axisX, int axisY)
{
    // Convert input rect to tile relative values [0, 1]
    Vector3 dim = bounds.getDimensions();
    float xmn = (rect.x - bounds.getMin()[axisX]) / dim[axisX];
    float xmx = (rect.z - bounds.getMin()[axisX]) / dim[axisX];
    float ymn = (rect.y - bounds.getMin()[axisY]) / dim[axisY];
    float ymx = (rect.w - bounds.getMin()[axisY]) / dim[axisY];

    /*xmn = min2(1.f, max2(0.f, xmn));
    xmx = min2(1.f, max2(0.f, xmx));
    ymn = min2(1.f, max2(0.f, ymn));
    ymx = min2(1.f, max2(0.f, ymx)); */

    // Scale to 16 bit int
    return Recti(Vector2i((int)(xmn * 65535.f), (int)(ymn * 65535.f)),
                 Vector2i((int)(xmx * 65535.f), (int)(ymx * 65535.f)));
}

/*-------------------------------------------------------------------*//*!
 * \brief   Convert world-space rect to tile-local fixed point
 *//*-------------------------------------------------------------------*/

static inline Recti fixedPoint(const Vector4& rect, const AABB& bounds, int face)
{
    int axis = getFaceAxis(face);
    int axisX = (axis + 1) % 3;
    int axisY = (axis + 2) % 3;

    return fixedPoint(rect, bounds, axisX, axisY);
}

/*-------------------------------------------------------------------*//*!
 * \brief   load integer rect to SIMD registers
 *//*-------------------------------------------------------------------*/

inline static void loadSIMDRect(const Recti& rect, SIMDRegister32& mn, SIMDRegister32& mx)
{
    mn = SIMDLoad32(rect.getMin().i, rect.getMin().j, 0, 0);
    mx = SIMDLoad32(rect.getMax().i, rect.getMax().j, 0, 0);
}

/*-------------------------------------------------------------------*//*!
 * \brief   load SIMD-registers as Recti
 *//*-------------------------------------------------------------------*/

inline static void storeSIMDRect(Recti& rect, const SIMDRegister32& mn, const SIMDRegister32& mx)
{
    Vector4i UMBRA_ATTRIBUTE_ALIGNED(16, mn4);
    Vector4i UMBRA_ATTRIBUTE_ALIGNED(16, mx4);
    SIMDStoreAligned32(mn, (int*)&mn4);
    SIMDStoreAligned32(mx, (int*)&mx4);
    rect.set(Vector2i(mn4.i, mn4.j), Vector2i(mx4.i, mx4.j));
}

/*-------------------------------------------------------------------*//*!
 * \brief   Check for SIMD-register rect validity
 *//*-------------------------------------------------------------------*/

inline static bool isSIMDRectValid(const SIMDRegister32& mn, const SIMDRegister32& mx)
{
    return !SIMDCompareGTTestAny32(mn, mx);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Return true if SIMD-register rects intersect
 *//*-------------------------------------------------------------------*/

inline static bool testSIMDRectIntersection(const SIMDRegister32& mna, const SIMDRegister32& mxa, const SIMDRegister32& mnb, const SIMDRegister32& mxb)
{
    SIMDRegister32 mn = SIMDMax32(mna, mnb);
    SIMDRegister32 mx = SIMDMin32(mxa, mxb);
    return isSIMDRectValid(mn, mx);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Create a new portal, possibly combining the portal with
 *          an existing portal
 *//*-------------------------------------------------------------------*/

inline static void newPortal(Array<ExternalPortal>& portals, int& firstHead, int lastHead, UINT32 plink, UINT32 idx_z, SIMDRegister32& rectMn, SIMDRegister32& rectMx)
{
    // Traverse previously generated portals for this cell
    // to see if there's already matching portal
    int idx = firstHead;
    while(idx != lastHead)
    {
        if (portals[idx-1].link  == plink &&
            portals[idx-1].idx_z == idx_z)
        {
            SIMDRegister32 rectMnB, rectMxB;
            loadSIMDRect(portals[idx-1].rect, rectMnB, rectMxB);

            // Combine to existing portal only if it intersects it.

            if (testSIMDRectIntersection(rectMn, rectMx, rectMnB, rectMxB))
            {
                rectMn = SIMDMin32(rectMn, rectMnB);
                rectMx = SIMDMax32(rectMx, rectMxB);
                break;
            }
        }
        idx = portals[idx-1].next;
    }

    if (idx != lastHead)
    {
        // Combine with old portal
        storeSIMDRect(portals[idx-1].rect, rectMn, rectMx);

        // Attempt to combine this portal to previously generated portals.

        const int cur  = idx;

        bool done;
        do
        {
            done = true;

            int* node = &firstHead;

            while(*node != lastHead)
            {
                int idx = *node;

                if (cur != idx &&
                    portals[idx-1].link  == plink &&
                    portals[idx-1].idx_z == idx_z)
                {
                    SIMDRegister32 rectMnB, rectMxB;
                    loadSIMDRect(portals[idx-1].rect, rectMnB, rectMxB);

                    // Combine to existing portal only if it intersects it.

                    if (testSIMDRectIntersection(rectMn, rectMx, rectMnB, rectMxB))
                    {
                        rectMn = SIMDMin32(rectMn, rectMnB);
                        rectMx = SIMDMax32(rectMx, rectMxB);

                        storeSIMDRect(portals[cur-1].rect, rectMn, rectMx);

                        *node = portals[idx-1].next;

                        done = false;
                        break;
                    }
                }

                node = &portals[idx-1].next;
            }
        } while (!done);

        UMBRA_ASSERT(firstHead > 0);
    } else
    {
        // Create new portal
        ExternalPortal extPortal;
        extPortal.link  = plink;
        extPortal.idx_z = idx_z;
        //extPortal.rect  = rect;
        storeSIMDRect(extPortal.rect, rectMn, rectMx);
        extPortal.next  = firstHead;
        portals.pushBack(extPortal);

        firstHead = portals.getSize(); // note +1, 0 means end
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief   Make fixed-point rect of a subrect relative to bigger
 *          rect.
 *//*-------------------------------------------------------------------*/

static inline void unscaleRect(const Recti& subrect, const Recti& rect, SIMDRegister32& resultMn, SIMDRegister32& resultMx)
{
    SIMDRegister32 subrectMn32, subrectMx32;
    loadSIMDRect(rect,    resultMn,  resultMx);
    loadSIMDRect(subrect, subrectMn32, subrectMx32);

    SIMDRegister subrectMn = SIMDIntToFloat(subrectMn32);
    SIMDRegister scale = SIMDSub(SIMDIntToFloat(subrectMx32), subrectMn);
    scale = SIMDMultiply(scale, s_simdOneOver65535);
    resultMn = SIMDFloatToInt(SIMDMultiplyAdd(SIMDIntToFloat(resultMn), scale, subrectMn));
    resultMx = SIMDFloatToInt(SIMDMultiplyAdd(SIMDIntToFloat(resultMx), scale, subrectMn));

    /*
    Recti result = rect;
    Vector2 scale = subrect.getMax() - subrect.getMin();
    scale.x = scale.x / 65535.f;
    scale.y = scale.y / 65535.f;
    result.scaleFloat(scale);
    result.translate(subrect.getMin());
    return result; */
}

// todo
static inline const void* map(const DataArray& arr)
{
    return arr.m_ofs.getAddr(arr.m_base);
}

static inline int getCellFaceMask(const ImpTile* tile, int cellIdx)
{
    CellNode cell;
    tile->getCellNodes().getElem(cell, cellIdx);            
    const PackedAABB& aabb = cell.getBounds();

    int faceMask;
    faceMask  = UMBRA_SIGN_EXTEND(1-(int)aabb.getMnx()) & 1;
    faceMask |= UMBRA_SIGN_EXTEND(1-(int)aabb.getMny()) & (1 << 2);
    faceMask |= UMBRA_SIGN_EXTEND(1-(int)aabb.getMnz()) & (1 << 4);
    faceMask |= UMBRA_SIGN_EXTEND((int)aabb.getMxx()+1-0xffff) & (1 << 1);
    faceMask |= UMBRA_SIGN_EXTEND((int)aabb.getMxy()+1-0xffff) & (1 << 3);
    faceMask |= UMBRA_SIGN_EXTEND((int)aabb.getMxz()+1-0xffff) & (1 << 5);
    return faceMask;
}

static inline int getSharedFace(const AABB& aabbA, const AABB& aabbB)
{
    // Find which face is between us and the neighbor.
    int face = 0;
    for (; face < 6; face++)
    {
        if ((face & 1))
        {
            if (aabbA.getMax()[face/2] ==
                aabbB.getMin()[face/2])
                return face;
        } else
        {
            if (aabbA.getMin()[face/2] ==
                aabbB.getMax()[face/2])
                return face;
        }
    }

    return -1;
}

}

inline bool RuntimeStructBuilder::reserveHeap(size_t size, size_t slack)
{
    if (m_blocks.getSize() == 0 || m_blocks[m_blocks.getSize() - 1].available() < size)
    {
        // If assembling to output directly, this is an error
        if (m_reservedOutput)
            return false;

        // Grab new block. For large allocations no slack is added.
        size_t newBlockSize = size > slack ? size : size + slack;
        HeapBlock block;
        block.mem = (UINT8*)UMBRA_HEAP_ALLOC_16(m_allocator, newBlockSize);
        if (!block.mem)
            return false;
        block.cur = block.mem;
        block.size = (UINT32)newBlockSize;
        if (!m_blocks.pushBack(block))
            return false;
    }
    UMBRA_ASSERT(!m_inHeap);
    m_inHeap = true;
    return true;
}

inline void RuntimeStructBuilder::finishHeap(void)
{
    UMBRA_ASSERT(m_inHeap);
    m_inHeap = false;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

RuntimeStructBuilder::RuntimeStructBuilder(void)
{
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

TomeCollection::ErrorCode RuntimeStructBuilder::init(Allocator* a, Umbra::UINT32 flags)
{
    m_allocator = a;

    // streaming todo
    UMBRA_UNREF(flags);
    m_blocks.setAllocator(m_allocator);
    m_blocks.reset(16);
    m_blocks.clear();

    m_stack[0].cur = 0;
    m_stack[0].base = 0;
    m_stackPos = 0;

    m_inHeap = false;
    m_reservedOutput = false;
    return TomeCollection::SUCCESS;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

RuntimeStructBuilder::~RuntimeStructBuilder()
{
    clean();
}

/*-------------------------------------------------------------------*//*!
 * \brief   "Cancel" building a struct.
 *//*-------------------------------------------------------------------*/

void RuntimeStructBuilder::cancel()
{
    UMBRA_ASSERT(m_stackPos > 0);
    m_stackPos--;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void RuntimeStructBuilder::clean()
{
    if (!m_reservedOutput)
    {
        for (int i = 0; i < m_blocks.getSize(); ++i)
            UMBRA_HEAP_FREE_16(m_allocator, m_blocks[i].mem);
    }
    m_blocks.reset(0);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void* RuntimeStructBuilder::finalize(Allocator* a)
{
    if (m_reservedOutput)
    {
        UINT8* output = m_blocks[0].mem;
        clean();
        return output;
    }
    else
    {
        size_t total = 0;
        for (int i = 0; i < m_blocks.getSize(); ++i)
            total += m_blocks[i].used();
        if (!total)
            return NULL;
        UINT8* buf = (UINT8*)UMBRA_HEAP_ALLOC_16(a, total);
        if (!buf)
            return NULL;
        UINT8* output = buf;
        for (int i = 0; i < m_blocks.getSize(); ++i)
        {
            memcpy(output, m_blocks[i].mem, m_blocks[i].used());
            output += m_blocks[i].used();
            UMBRA_HEAP_FREE_16(m_allocator, m_blocks[i].mem);
        }
        m_blocks.reset(0);
        return buf;
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool RuntimeStructBuilder::reserveOutput(Allocator* a, size_t size)
{
    UMBRA_ASSERT(m_blocks.getSize() == 0);
    HeapBlock block;
    // leave room for alignment, allocated headers etc
    if (size < 64)
        return false;
    size_t adjustedSize = size - 64;
    block.mem = (UINT8*)UMBRA_HEAP_ALLOC_16(a, adjustedSize);
    if (!block.mem)
        return false;
    block.cur = block.mem;
    block.size = (UINT32)adjustedSize;
    if (!m_blocks.pushBack(block))
        return false;
    m_reservedOutput = true;
    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Alloc memory from tome (using preallocated buffer)
 *//*-------------------------------------------------------------------*/

inline Umbra::UINT8* RuntimeStructBuilder::allocOutput (DataPtr& dataPtr, size_t size)
{
    UMBRA_ASSERT(m_inHeap);

    UINT32 aligned = UMBRA_ALIGN(size, 16);
    HeapBlock& block = m_blocks[m_blocks.getSize() - 1];
    UMBRA_ASSERT(block.available() >= aligned);

    UINT8* mem = block.cur;
    memset(mem, 0, aligned);
    block.cur += aligned;

    dataPtr = m_stack[m_stackPos].offset();
    m_stack[m_stackPos].cur += aligned;

    return mem;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Alloc a struct in tome, so that following DataPtrs
 *          will be relative to this struct
 *//*-------------------------------------------------------------------*/

template<typename T>
inline T& RuntimeStructBuilder::beginStruct(DataPtr& dataPtr)
{
    UMBRA_ASSERT(m_inHeap);

    Umbra::UINT32 base = m_stack[m_stackPos].cur;
    UINT8* mem = allocOutput(dataPtr, sizeof(T));
    Umbra::UINT32 start = m_stack[m_stackPos].cur;
    m_stackPos++;
    UMBRA_ASSERT(m_stackPos < 16);
    m_stack[m_stackPos].base = base;
    m_stack[m_stackPos].cur = start;
    return *(T*)mem;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Finish generated a struct started with either beginStruct
 *          or allocHeapStruct.
 * \return  Struct's final size
 *//*-------------------------------------------------------------------*/

inline Umbra::UINT32 RuntimeStructBuilder::endStruct(void)
{
    UMBRA_ASSERT(m_stackPos > 0);
    m_stack[m_stackPos-1].cur = m_stack[m_stackPos].cur;
    UINT32 size = (UINT32)m_stack[m_stackPos].used();
    m_stackPos--;
    UMBRA_ASSERT(m_stackPos >= 0);
    return size;
}

RuntimeTomeGenerator::Result::Result()
    : m_allocator(0),
      m_result(0),
      m_numContexts(0)
{}

void RuntimeTomeGenerator::Result::clear (bool freeResult)
{
    if (freeResult)
        UMBRA_HEAP_FREE_16(m_allocator, (void*)m_result);
    m_numContexts = 0;
    m_result = NULL;
    m_extTiles = DataPtr();
    m_contexts = DataPtr();
    m_allocator = NULL;
}

RuntimeTomeGenerator::RuntimeTomeGenerator (
    Allocator* main, 
    Allocator* resultAlloc,
    Umbra::UINT32 flags, 
    const ImpTome** tomes, int numTomes, 
    const AABB& aabb)
: m_mainAllocator(main),
  m_resultAllocator(resultAlloc),
  m_flags(flags),
  m_tomes(tomes),
  m_numTomes(numTomes),
  m_minBounds(aabb),
  m_numSlots(0),
  m_missingMatchingData(false),
  m_connectedFaces(0),
  m_prematchedFaces(0),
  m_tomeContexts(NULL),
  m_extTiles(NULL),
  m_errorCode(TomeCollection::SUCCESS)
{
    UMBRA_ASSERT(KDTree::getDataDwords(1) <= m_defaultTreeDwords);
    m_defaultTree = KDTree(1, m_defaultTreeData, DataArray());
}

RuntimeTomeGenerator::~RuntimeTomeGenerator()
{
}

/*-------------------------------------------------------------------*//*!
 * \brief   Estimate memory required by tome.
 *//*-------------------------------------------------------------------*/

void RuntimeTomeGenerator::estimateTome(const ImpTome** inputs, int numInputs, const TileArray& tiles, Estimate& estimate)
{
    estimate.numTargets  = 0;
    estimate.numMaxCells = UMBRA_EMPTY_SLOT_CELLS;
    estimate.numMaxClusters = 1;
    estimate.numGates    = 0;

    AABB    bounds;
    Vector3 minDim(FLT_MAX, FLT_MAX, FLT_MAX);

    for (int i = 0; i < tiles.getSize(); i++)
    {
        if (!tiles[i].m_tile)
            continue;

        AABB aabb(tiles[i].m_tile->getTreeMin(), tiles[i].m_tile->getTreeMax());
        bounds.grow(aabb);

        estimate.numMaxCells = max2(estimate.numMaxCells, tiles[i].m_tile->getNumCells());
    }

    for (int i = 0; i < numInputs; i++)
    {
        estimate.numMaxClusters = max2(estimate.numMaxClusters, inputs[i]->getNumClusters());
        estimate.numTargets += inputs[i]->getNumObjects();
        estimate.numGates   += inputs[i]->getNumGates();
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief   Estimate upper limit for tome size
 *//*-------------------------------------------------------------------*/

size_t RuntimeTomeGenerator::estimateTomeSize (const ImpTome** inputs, int numInputs, const TileArray& tiles)
{
    Estimate estimate;
    estimateTome(inputs, numInputs, tiles, estimate);

    StatsAlloc tomeMem((size_t)0x7fffffff);
    // ImpTome
    UMBRA_HEAP_ALLOC(&tomeMem, sizeof(ImpTome));                                    // ImpTome

    return tomeMem.allocated() + 16;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Input sanity check
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::collectTiles(void)
{
    m_leafStarts.setAllocator(getAllocator());
    m_leafStarts.reset(0);
    m_tiles.setAllocator(getAllocator());
    if (!m_tiles.reset(m_numTomes))  // input subtiles
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }
    m_tiles.clear();
    
    // Enumerate tiles & subtiles
    for (int i = 0; i < m_numTomes; i++)
    {
        if (!m_tomes[i])
        {
            setError(TomeCollection::ERROR_INVALID_PARAM);
            return false;
        }

#ifdef UMBRA_DEBUG
        Tome::Status tomeStatus = checkStatus(m_tomes[i], true);
#else
        Tome::Status tomeStatus = checkStatus(m_tomes[i], false); // do not validate CRC32
#endif

        if (tomeStatus == Tome::STATUS_CORRUPT)
        {
            setError(TomeCollection::ERROR_CORRUPT_TOME);
            return false;
        }

        if (tomeStatus != Tome::STATUS_OK)
        {
            setError(TomeCollection::ERROR_INVALID_PARAM);
            return false;
        }

        DataArray subTiles = m_tomes[i]->getTileOffsets(false);
        UMBRA_ASSERT(subTiles.m_count >= 0);

        if (!m_leafStarts.pushBack(m_tiles.getSize()))
        {
            setError(TomeCollection::ERROR_OUT_OF_MEMORY);
            return false;
        }

        for (int j = 0; j < subTiles.m_count; j++)
        {
            DataPtr ptr;
            subTiles.getElem(ptr, j);

            const ImpTile* subTile = (const ImpTile*)ptr.getAddr(m_tomes[i]);
            GeneratorTile tile;
            tile.m_tile  = subTile;
            tile.m_local = j;
            if (!m_tiles.pushBack(tile))
            {
                setError(TomeCollection::ERROR_OUT_OF_MEMORY);
                return false;
            }
        }
    }

#ifdef UMBRA_DEBUG
    // n^2 overlap check
    for (int i = 0; i < m_numTomes; i++)
    {
        AABB a(m_tomes[i]->getTreeMin(), m_tomes[i]->getTreeMax());
        for (int j = i + 1; j < m_numTomes; j++)
        {
            AABB b(m_tomes[j]->getTreeMin(), m_tomes[j]->getTreeMax());

            if (a.intersectsWithVolume(b))
            {
                setError(TomeCollection::ERROR_OVERLAPPING_TOMES);
                return false;
            }
        }
    }
#endif

    estimateTome(m_tomes, m_numTomes, m_tiles, m_estimate);

    m_slotToIndex.setAllocator(getAllocator());
    m_slotToIndex.reset(m_tiles.getSize());
    m_slotToIndex.clear();
    m_emptyTiles.setAllocator(getAllocator());
    m_emptyTiles.clear();

    return true;
}

int RuntimeTomeGenerator::getCluster(int index, int cell)
{
    CellNode node;
    m_tiles[index].m_tile->getCellNodes().getElem(node, cell);
    UMBRA_ASSERT(node.getClusterCount() == 0);
    return node.getClusterIndex();
}

int RuntimeTomeGenerator::getParentCell(int index, int cell, int ancestorIdx)
{
    const ImpTome* input = m_tomes[m_extTiles[mapIndexToSlot(index)].m_tomeIdx];

    // Compute local leaf index
    KDTree toplevel(input->getTreeNodeCount(), (const UINT32*)map(input->getTreeData()), input->getTreeSplits());
    int leafIdx = toplevel.getLeafIdx(mapIndexToLocal(index));

    // Non-empty: get tree from StreamingTile
    const LeafTileMatchData& matchingData  = ((const LeafTileMatchData*)map(input->getMatchingData()))[leafIdx];
    int elemWidth = matchingData.m_cellLodElemWidth;
    int bitWidth  = matchingData.m_cellLodBitWidth;

    if (!bitWidth)
        return 0;

    const UINT32* cellMap = (const UINT32*)map(matchingData.getCellLodMap(input));
    if (bitWidth == 32)
        return unpackElem32(cellMap, (cell * elemWidth + ancestorIdx) * bitWidth);
    else
        return unpackElem  (cellMap, (cell * elemWidth + ancestorIdx) * bitWidth, bitWidth);
}

void RuntimeTomeGenerator::combineHierarchyPortals(int index)
{
    bool empty = isEmpty(index);
    int cellCount = UMBRA_EMPTY_SLOT_CELLS;

    if (!empty)
        cellCount = m_tiles[index].m_tile->getNumCells();

    for (int c = 0; c < cellCount; c++)
    {
	    int oldHead = m_cellPortalHeads[c];
        int newHead = oldHead;

        for (int p = oldHead; p != 0; p = m_extPortals[p - 1].next)
        {
            int idx = p - 1;

            Portal portal;
            portal.link = m_extPortals[idx].link;
            portal.idx_z = m_extPortals[idx].idx_z;
            if (portal.isOutside())
                continue;
            if (portal.isHierarchy())
                break;

            int targetIdx = m_slotToIndex[portal.getTargetTileIdx()];
            if (isEmpty(targetIdx))
                continue;

            UMBRA_ASSERT(m_tiles[targetIdx].m_tile->isLeaf());

            int parentIdx = 0;
            for (int parent = getParentTile(targetIdx); parent != -1; parent = getParentTile(parent))
            {
                UMBRA_ASSERT(empty || (m_extTiles[mapIndexToSlot(parent)].m_tomeIdx !=
                    m_extTiles[mapIndexToSlot(index)].m_tomeIdx));

                int parentCell = getParentCell(targetIdx, portal.getTargetIndex(), parentIdx);

                UINT32 plink = BUILD_PORTAL_LINK(portal.getFace(), 0, 0, 1, mapIndexToSlot(parent));
                UINT32 idx_z = (parentCell << 16) | (portal.idx_z & 0xffff);

                SIMDRegister32 rectMn, rectMx;
                loadSIMDRect(m_extPortals[idx].rect, rectMn, rectMx);
                newPortal(m_extPortals, newHead, oldHead,
                          plink, idx_z, rectMn, rectMx);

                parentIdx++;
            }
        }

        // append linked list
        m_cellPortalHeads[c] = newHead;
    }
}

void RuntimeTomeGenerator::combineClusterPortals(int input, int leafIndex)
{
    UMBRA_ASSERT(input >= 0 && input < m_numTomes);
    UMBRA_ASSERT(leafIndex >= 0 && leafIndex < m_tiles.getSize());

    if (!m_tiles[leafIndex].m_tile || !m_tiles[leafIndex].m_tile->isLeaf() || !m_tiles[leafIndex].m_borderMask)
        return;

    const ImpTile* leafInput = m_tiles[leafIndex].m_tile;

    ExtCellNode* cells = m_tiles[leafIndex].m_extCells;
    if (!cells)
        return;

    Recti UMBRA_ATTRIBUTE_ALIGNED16(rect);
    AABB aabbA = m_tomes[input]->getAABB();
    AABB aabbB = AABB(leafInput->getTreeMin(), leafInput->getTreeMax());

    for (int c = 0; c < leafInput->getNumCells(); c++)
    {
        int currentCluster = getCluster(leafIndex, c);
        Portal* portals = m_tiles[leafIndex].m_extPortals + cells[c].getPortalIndex();

        for (int p = 0; p < cells[c].getPortalCount(); p++)
        {
            if (portals[p].isHierarchy() || portals[p].isOutside())
                continue;

            rect.setMin(0, portals[p].xmn_xmx >> 16);
            rect.setMin(1, portals[p].ymn_ymx >> 16);
            rect.setMax(0, portals[p].xmn_xmx & 0xffff);
            rect.setMax(1, portals[p].ymn_ymx & 0xffff);

            Vector4 rectB  = aabbB.getFaceRect(portals[p].getFace());
            Recti   common = fixedPoint(rectB, aabbA, portals[p].getFace());
            SIMDRegister32 rectMn, rectMx;
            unscaleRect(common, rect, rectMn, rectMx);

            int targetSlot  = portals[p].getTargetTileIdx();
            int targetIndex = m_slotToIndex[targetSlot];

            if (isEmpty(targetIndex))
                continue;

            UMBRA_ASSERT(isEmpty(targetIndex) || (m_extTiles[targetSlot].m_tomeIdx != m_extTiles[mapIndexToSlot(leafIndex)].m_tomeIdx));

            int targetCluster = getCluster(targetIndex, portals[p].getTargetIndex());
            UINT32 link  = BUILD_PORTAL_LINK(portals[p].getFace(), 0, 0, 0, targetCluster);
            UINT32 idx_z = (m_extTiles[targetSlot].m_tomeIdx << 16) | (portals[p].idx_z & 0xffff);

            newPortal(m_extPortals, m_cellPortalHeads[currentCluster], 0,
                      link, idx_z, rectMn, rectMx);
        }
    }
}

void RuntimeTomeGenerator::copyLeafPortals(int innerIndex, int currentSlot, int level, Array<Umbra::UINT32>& bitmap, int& oldFaceMask)
{
    UMBRA_ASSERT(m_topLevelTree.getSplit(mapIndexToSlot(innerIndex)) != KDTree::LEAF);

    KDTree::Split split = m_topLevelTree.getSplit(currentSlot);

    if (split != KDTree::LEAF)
    {
        if (m_slotToIndex[currentSlot] >= 0)
            ++level;
        copyLeafPortals(innerIndex, m_topLevelTree.getLeftChildIdx(currentSlot),  level, bitmap, oldFaceMask);
        copyLeafPortals(innerIndex, m_topLevelTree.getRightChildIdx(currentSlot), level, bitmap, oldFaceMask);
        return;
    }

    AABB aabbA = getAABBByIndex(innerIndex);

    int   leafIndex = m_slotToIndex[currentSlot];
    ExtCellNode* cells = m_tiles[leafIndex].m_extCells;
    if (!cells)
        return;
    if (!m_tiles[leafIndex].m_borderMask)
        return;

    Recti UMBRA_ATTRIBUTE_ALIGNED16(rect);
    const ImpTile* leafInput = m_tiles[leafIndex].m_tile;
    int oldSlotIdx           = m_newOldTileMap.getSize() ? m_newOldTileMap[mapIndexToSlot(innerIndex)] : -1;
    AABB  aabbB              = AABB(leafInput->getTreeMin(), leafInput->getTreeMax());

    Vector4 rectB[6];
    for (int i = 0; i < 6; i++)
        rectB[i] = aabbB.getFaceRect(i);

    for (int c = 0; c < leafInput->getNumCells(); c++)
    {
        Portal* portals = m_tiles[leafIndex].m_extPortals + cells[c].getPortalIndex();

        for (int p = 0; p < cells[c].getPortalCount(); p++)
        {
            int targetSlot = portals[p].getTargetTileIdx();

            if (oldSlotIdx != -1)
            {
                int oldTargetSlotIdx = m_newOldTileMap[targetSlot];
                if (oldTargetSlotIdx != -1)
                {
                    oldFaceMask |= (1 << portals[p].getFace());
                    setBit(bitmap.getPtr(), oldTargetSlotIdx);
                    continue;
                }
            }

            Portal& leafPortal = portals[p];

            // Must be non-outside regular portal
            UMBRA_ASSERT(!leafPortal.isOutside() && leafPortal.hasTarget());
            // The target tile must either be empty or in a different tome
            UMBRA_ASSERT(isEmpty(m_slotToIndex[targetSlot]) || 
                         (m_extTiles[targetSlot].m_tomeIdx != m_extTiles[currentSlot].m_tomeIdx));

            rect.setMin(0, leafPortal.xmn_xmx >> 16);
            rect.setMin(1, leafPortal.ymn_ymx >> 16);
            rect.setMax(0, leafPortal.xmn_xmx & 0xffff);
            rect.setMax(1, leafPortal.ymn_ymx & 0xffff);

            UINT32 link  = BUILD_PORTAL_LINK(leafPortal.getFace(), 0, 0, 1, targetSlot);

            SIMDRegister32 rectMn, rectMx;
            Recti common = fixedPoint(rectB[leafPortal.getFace()], aabbA, leafPortal.getFace());
            unscaleRect(common, rect, rectMn, rectMx);

            int parentCell = getParentCell(leafIndex, c, level - 1);
            newPortal(m_extPortals, m_cellPortalHeads[parentCell],
                      0, link, leafPortal.idx_z, rectMn, rectMx);
            
        }
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief   Start generating the tome
 *//*-------------------------------------------------------------------*/

TomeCollection::ErrorCode RuntimeTomeGenerator::buildTome(Result& result, const Result* oldResult, size_t fixedResult)
{
    TomeCollection::ErrorCode status = m_builder.init(m_mainAllocator, m_flags);
    if (status != TomeCollection::SUCCESS)
        return status;
    if (fixedResult)
    {
        if (!m_builder.reserveOutput(m_resultAllocator, fixedResult))
            return TomeCollection::ERROR_OUT_OF_MEMORY;
    }

    result.clear();

    if (!collectTiles())
        return getError();

    if (!m_builder.reserveHeap(sizeof(ImpTome)))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return getError();
    }

    DataPtr root;
    ImpTome& tome = m_builder.beginStruct<ImpTome>(root);
    m_builder.finishHeap();

    // Tome global fields

    if (!generateHeader(tome, result, oldResult))
    {
        m_builder.cancel();
        result.clear();
        return getError();
    }

    // cell -> generated portals linked list heads
    m_cellPortalHeads.setAllocator(getAllocator());    
    // space for 512 portals initially, will grow if necessary
    m_extPortals.setAllocator(getAllocator());
    
    if (!m_cellPortalHeads.reset(m_estimate.numMaxCells) ||
        !m_extPortals.reset(512))
    {
        m_builder.cancel();
        result.clear();
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return getError();
    }

    m_extPortals.clear();

    // process tiles
    if (!m_builder.reserveHeap(sizeof(AlignedPtr<const ImpTile>) * m_numSlots + 16))
    {
        m_builder.cancel();
        result.clear();
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return getError();
    }
    AlignedPtr<const ImpTile>* tileOffsets = (AlignedPtr<const ImpTile>*)m_builder.allocOutput(tome.m_tiles, sizeof(AlignedPtr<const ImpTile>) * m_numSlots);
    m_builder.finishHeap();

    m_connectedFaces  = 0;
    m_prematchedFaces = 0;

    // Process leaf tiles
    //////////////////////

    m_emptySlotOfs.setAllocator(getAllocator());
    Array<UINT32>   oldNeighbors(getAllocator());
    Array<int>      neighbors(getAllocator());

    if (!m_emptySlotOfs.reset(m_emptyTiles.getSize()) || 
        !oldNeighbors.reset(4) || 
        !neighbors.reset(16))
    {
        m_builder.cancel();
        result.clear();
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return getError();
    }

    for (int slot = 0; slot < m_numSlots; slot++)
    {
        int index = m_slotToIndex[slot];

        tileOffsets[slot] = NULL;

        if (index < 0)
            continue;
        if (!isEmpty(index) && !(m_tiles[index].m_tile->getFlags() & ImpTile::TILEFLAG_ISLEAF))
            continue;

        memset(m_cellPortalHeads.getPtr(), 0, m_cellPortalHeads.getSize() * sizeof(int));
        m_extPortals.clear();

        neighbors.reset(0);

        // Generate new portals joining the new tile to others
        // (Only for empties or border tiles)
        if (isEmpty(index) || m_tiles[index].m_borderMask)
        {
            int oldFaceMask = 0;
            AABB tomeAABB = tome.getAABB();
            findNeighbors(neighbors, m_topLevelTree, 0, tomeAABB, index);
            generatePortals(index, neighbors, oldResult, oldFaceMask);

            // Makes outgoing hierarchy portals by combining leaf portals
            combineHierarchyPortals(index);

            makeNeighborBitvector(index, neighbors, true, oldNeighbors, oldResult);

            // Output generated portals
            if (!outputExternalPortals(m_extTiles[slot], 
                                       isEmpty(index) ? UMBRA_EMPTY_SLOT_CELLS : m_tiles[index].m_tile->getNumCells(), 
                                       index,
                                       oldResult, 
                                       oldNeighbors,
                                       oldFaceMask))
            {
                m_builder.cancel();
                result.clear();
                return getError();
            }
        }

        if (isEmpty(index))
            tileOffsets[slot] = generateEmptyTile(m_emptySlotOfs[getEmptyIndex(index)], getAABBByIndex(index), slot);
        else
            tileOffsets[slot] = m_tiles[index].m_tile;

        if (!tileOffsets[slot])
        {
            m_builder.cancel();
            result.clear();
            UMBRA_ASSERT(getError() != TomeCollection::SUCCESS);
            return getError();
        }
    }

    neighbors.reset(0);

    // Process hierarchy tiles
    ///////////////////////////

    for (int slot = 0; slot < m_numSlots; slot++)
    {
        int index = m_slotToIndex[slot];

        if (index < 0)
            continue;

        if (isEmpty(index) || (m_tiles[index].m_tile->getFlags() & ImpTile::TILEFLAG_ISLEAF))
            continue;

        int oldFaceMask = 0;
        memset(m_cellPortalHeads.getPtr(), 0, m_cellPortalHeads.getSize() * sizeof(int));
        m_extPortals.clear();

        if (oldResult && m_newOldTileMap.getSize() && m_newOldTileMap[slot] != -1)
        {
            if (!oldNeighbors.reset(UMBRA_BITVECTOR_DWORDS(oldResult->m_result->getTileArraySize())))
            {
                m_builder.cancel();
                result.clear();
                setError(TomeCollection::ERROR_OUT_OF_MEMORY);
                return getError();
            }
            memset(oldNeighbors.getPtr(), 0, oldNeighbors.getByteSize());
        } else
            oldNeighbors.reset(0);

        // Generate outgoing hierarchy portals by combining from leafs
        copyLeafPortals(index, slot, 0, oldNeighbors, oldFaceMask);

        // Output generated portals
        if (!outputExternalPortals(m_extTiles[slot], m_tiles[index].m_tile->getNumCells(), index, oldResult, oldNeighbors, oldFaceMask))
        {
            m_builder.cancel();
            result.clear();
            return getError();
        }

        tileOffsets[slot] = m_tiles[index].m_tile;
    }

    m_cellPortalHeads.reset(m_estimate.numMaxClusters);

    if (!m_builder.reserveHeap(sizeof(int) * (m_numTomes + 1) * 2 + 32))
    {
        m_builder.cancel();
        result.clear();
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return getError();
    }

    int* clusterStarts       = (int*)m_builder.allocOutput(tome.m_tomeClusterStarts,       sizeof(int) * (m_numTomes + 1));
    int* clusterPortalStarts = (int*)m_builder.allocOutput(tome.m_tomeClusterPortalStarts, sizeof(int) * (m_numTomes + 1));
    m_builder.finishHeap();

    int numClusters       = 0;
    int numClusterPortals = 0;
    for (int input = 0; input < m_numTomes; input++)
    {
        memset(m_cellPortalHeads.getPtr(), 0, m_cellPortalHeads.getSize() * sizeof(int));
        m_extPortals.clear();

        for (int i = m_leafStarts[input]; i < m_leafStarts[input] + m_tomes[input]->getNumTiles(); i++)
            combineClusterPortals(input, i);

        outputClusterPortals(input);

        clusterStarts[input]       = numClusters;
        clusterPortalStarts[input] = numClusterPortals;
        numClusters        += m_tomes[input]->getNumClusters();
        numClusterPortals  += m_tomes[input]->getNumClusterPortals() + m_extPortals.getSize();
    }
    clusterStarts[m_numTomes]       = numClusters;
    clusterPortalStarts[m_numTomes] = numClusterPortals;

    //printf("connected %d, prematched %d\n", m_connectedFaces, m_prematchedFaces);

    // Deallocate
    m_cellPortalHeads.reset(0);
    m_extPortals.reset(0);

    tome.m_size = m_builder.endStruct();

    ImpTome* finalTome = (ImpTome*)m_builder.finalize(m_resultAllocator);

    if (!finalTome)
    {
        result.clear();
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return getError();
    }

    /* \todo [antti 29.1.2013]: remove this hack */

    AlignedPtr<const ImpTile>* outTiles = (AlignedPtr<const ImpTile>*)DataPtr(finalTome->m_tiles).getAddr(finalTome);
    for (int slot = 0; slot < m_numSlots; slot++)
    {
        int index = m_slotToIndex[slot];
        if (!isEmpty(index))
            continue;
        DataPtr ofs = m_emptySlotOfs[getEmptyIndex(index)];
        outTiles[slot] = (ImpTile*)ofs.getAddr(finalTome);
    }

    result.m_allocator = m_resultAllocator;
    result.m_result = finalTome;
    result.m_numContexts = m_numTomes;

    if (m_missingMatchingData)
        setError(TomeCollection::ERROR_NO_MATCHING_DATA);
    return getError();
}

/*-------------------------------------------------------------------*//*!
 * \brief   Generate an ImpTile object
 *//*-------------------------------------------------------------------*/

ImpTile* RuntimeTomeGenerator::generateEmptyTile(DataPtr& ptr, const AABB& bounds, int slot)
{
    UINT32 viewTreeSize = KDTree::getDataDwords(1) * sizeof(UINT32);
    size_t estimate     =
        16 + sizeof(ImpTile) +
        16 + viewTreeSize    +      // view tree
        16 + sizeof(UINT32) * 2 +   // view tree map
        16 + sizeof(CellNode) * UMBRA_EMPTY_SLOT_CELLS;

    if (!m_builder.reserveHeap(estimate))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return NULL;
    }

    ImpTile& tile = m_builder.beginStruct<ImpTile>(ptr);

    UMBRA_UNREF(slot);
    //tile.m_slot                 = slot;
    tile.m_treeMin              = bounds.getMin();
    tile.m_treeMax              = bounds.getMax();
    tile.m_numCellsAndClusters  = UMBRA_EMPTY_SLOT_CELLS;
    tile.m_portalExpand         = 0.f;

    // streaming todo
    //tile.m_viewTree.m_kdCount   = 0;
    tile.m_viewTree.setNodeCount(1);
    tile.m_viewTree.setMapWidth(1);

    UINT32* viewtree    = (UINT32*)m_builder.allocOutput(tile.m_viewTree.m_treeData, viewTreeSize);
    memcpy(viewtree, m_defaultTree.getData(), viewTreeSize);

    UINT32* map         = (UINT32*)m_builder.allocOutput(tile.m_viewTree.m_map, sizeof(UINT32) * 2);
    map[0]              = m_defaultMap[0];

    PackedAABB cellBounds;
    cellBounds.mnx_mny = 0;
    cellBounds.mnz_mxx = 0xffff;
    cellBounds.mxy_mxz = 0xffffffff;

    CellNode* cells     = (CellNode*)m_builder.allocOutput(tile.m_cells, sizeof(CellNode) * UMBRA_EMPTY_SLOT_CELLS);

    cells[0].setObjects(0, 0);
    cells[0].setClusters(0, 0);
    cells[0].setPortalIdxAndCount(0, 0);
    cells[0].setBounds(cellBounds);

    for (int i = 1; i < UMBRA_EMPTY_SLOT_CELLS; i++)
        cells[i] = cells[0];

    UINT32 size = m_builder.endStruct();

    int sizeAndFlags = 0;
    sizeAndFlags |= (size << 8);
    sizeAndFlags |= ImpTile::TILEFLAG_ISLEAF;
    sizeAndFlags |= ImpTile::TILEFLAG_ISEMPTY;
    tile.m_sizeAndFlags = sizeAndFlags;

    m_builder.finishHeap();

    return &tile;
}

bool RuntimeTomeGenerator::outputClusterPortals(int input)
{
    if (!m_extPortals.getSize())
        return true;

    //ExtCellNode* extCells = (ExtCellNode*)m_builder.allocOutput(tile.m_extCells, sizeof(ExtCellNode) * numCells);
    //Portal* extPortals = (Portal*)m_builder.allocOutput(tile.m_extPortals, sizeof(Portal) * m_extPortals.getSize());

    size_t sizeClusterNodes = sizeof(ExtClusterNode) * m_tomes[input]->getNumClusters();
    size_t sizePortals      = sizeof(Portal) * m_extPortals.getSize();

    if (!m_builder.reserveHeap(sizeClusterNodes + sizePortals + 32))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    ExtClusterNode* extClusters = (ExtClusterNode*)m_builder.allocOutput(m_tomeContexts[input].m_extClusters, sizeClusterNodes);
    Portal*         extPortals  = (Portal*)m_builder.allocOutput(m_tomeContexts[input].m_extPortals,          sizePortals);
    m_builder.finishHeap();

    int extPortalPos = 0;

    // Iterate all cells
    for (int i = 0; i < m_tomes[input]->getNumClusters(); i++)
    {
        int offset = extPortalPos;

        // For (new) external portals (generated by the runtime matching code):
        // find this cell's new portals from the linked list
        int p = m_cellPortalHeads[i];
        while (p)
        {
            // 0 means linked list end
            int idx = p - 1;

            // Create the portal
            Portal portal;
            portal.link = m_extPortals[idx].link;

            Recti fpRect = m_extPortals[idx].rect;

            portal.idx_z   = m_extPortals[idx].idx_z;
            portal.xmn_xmx = (fpRect.getMin().i << 16) | (fpRect.getMax().i);
            portal.ymn_ymx = (fpRect.getMin().j << 16) | (fpRect.getMax().j);

            extPortals[extPortalPos++] = portal;
            p = m_extPortals[idx].next;
        }

        extClusters[i].setPortalIdxAndCount(offset, extPortalPos - offset);
    }

    return true;
}

void RuntimeTomeGenerator::makeNeighborBitvector(int index, const Array<int>& neighbors, bool doHierarchy, Array<Umbra::UINT32>& bitmap, const Result* oldResult)
{
    UMBRA_UNREF(index);
    bitmap.reset(0);

    if (neighbors.getSize() && m_newOldTileMap.getSize())
    {
        UMBRA_ASSERT(oldResult);

        const ImpTome*  oldTome     = NULL;
        oldTome     = oldResult->m_result;

        UMBRA_DEBUG_CODE(
            int             oldSlotIdx  = -1;
            const ExtTile*  oldExtTiles = NULL;
            oldSlotIdx  = m_newOldTileMap[mapIndexToSlot(index)];
            oldExtTiles = (const ExtTile*)oldResult->m_extTiles.getAddr(oldTome);
        )

        bitmap.resize(UMBRA_BITVECTOR_DWORDS(oldTome->getTileArraySize()));
        memset(bitmap.getPtr(), 0, bitmap.getByteSize());

        for (int i = 0; i < neighbors.getSize(); i++)
        {
            int nSlot  = neighbors[i];
            int nIndex = m_slotToIndex[neighbors[i]];

            if (nSlot == -1)
                continue;

            UMBRA_ASSERT(oldSlotIdx != -1 && m_newOldTileMap[nSlot] != -1);

            int oldNSlotIdx = m_newOldTileMap[nSlot];

            UMBRA_ASSERT(oldTome && oldExtTiles);
            UMBRA_ASSERT(isEmpty(nIndex) || oldTome->getTile(oldNSlotIdx, true) == m_tiles[nIndex].m_tile);

            setBit(bitmap.getPtr(), oldNSlotIdx);

            if (doHierarchy && !isEmpty(nIndex))
            {
                for (int parent = getParentTile(nIndex); parent != -1; parent = getParentTile(parent))
                {
                    int parentSlot = mapIndexToSlot(parent);
                    int oldParentSlot = m_newOldTileMap[parentSlot];
                    UMBRA_ASSERT(oldParentSlot != -1);
                    setBit(bitmap.getPtr(), oldParentSlot);
                }
            }
        }
    }
}

bool RuntimeTomeGenerator::outputExternalPortals(ExtTile& tile, int numCells, int tileIdx, const Result* oldResult, const Array<Umbra::UINT32>& bitmap, int bitmapFaces)
{
    int slot = mapIndexToSlot(tileIdx);
    Array<Portal> portals(32, getAllocator());
        
    int                 oldSlotIdx  = -1;
    const ImpTome*      oldTome     = NULL;
    const ExtTile*      oldExtTiles = NULL;
    const ExtCellNode*  oldExtCells = NULL;
    int                 oldExitPortalMask = 0;

    if (bitmap.getSize())
    {
        UMBRA_ASSERT(oldResult);

        oldSlotIdx  = m_newOldTileMap[slot];
        oldTome     = oldResult->m_result;
        oldExtTiles = (const ExtTile*)oldResult->m_extTiles.getAddr(oldTome);
        oldExtCells = (const ExtCellNode*)map(oldExtTiles[oldSlotIdx].getExtCells(oldTome, numCells));
        // We can copy exit portals for empty tiles that have common faces in old and new exit masks
        if (isEmpty(tileIdx))
            oldExitPortalMask = oldExtTiles[oldSlotIdx].getExitPortalMask() & tile.getExitPortalMask();
    }

    if (!m_extPortals.getSize() && !bitmap.getSize())
        return true;

    if (!m_builder.reserveHeap(sizeof(ExtCellNode) * numCells + 16))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    ExtCellNode* extCells = (ExtCellNode*)m_builder.allocOutput(tile.m_extCells, sizeof(ExtCellNode) * numCells);
    Array<Portal> extPortals(m_extPortals.getSize(), getAllocator());
    extPortals.reset(0);
        
    // Iterate all cells
    for (int i = 0; i < numCells; i++)
    {
        int offset = extPortals.getSize();

        // Check if we can copy old portals
        bool checkOldPortals = oldExtCells != NULL;
        if (checkOldPortals && !isEmpty(tileIdx) && bitmapFaces)
        {
            // Copy if cell shares common tile face with bitmapFaces
            int cellFaces = getCellFaceMask(m_tiles[tileIdx].m_tile, i);
            checkOldPortals = !!(bitmapFaces & cellFaces);
        }
        
        if (checkOldPortals)
        {
            Portal* portals = (Portal*)map(oldExtTiles[oldSlotIdx].getExtPortals(oldTome, oldExtCells[i]));
                
            int size = extPortals.getSize();
            if (!extPortals.resize(size + oldExtCells[i].getPortalCount()))
            {
                setError(TomeCollection::ERROR_OUT_OF_MEMORY);
                return false;
            }

            Portal* ptr = extPortals.getPtr()+size;

            for (int portal = 0; portal < oldExtCells[i].getPortalCount(); portal++)
            {
                if (!portals->hasTarget())
                {
                    if (oldExitPortalMask & (1 << portals->getFace()))
                    {
                        *ptr = *portals;
                        ptr++;
                        size++;
                        portals++;
                    }
                    continue;
                }
                    
                int oldTargetSlot = portals->getTargetTileIdx();
                if (!testBit(bitmap.getPtr(), oldTargetSlot))
                {
                    portals++;
                    continue;
                }

                *ptr = *portals;
                int newSlot = m_oldNewTileMap[oldTargetSlot];
                ptr->link &= ~0x3FFFFFF;
                ptr->link |= newSlot;  

                ptr++;
                size++;
                portals++;
            }

            extPortals.resize(size);
        }
        
        // For (new) external portals (generated by the runtime matching code):
        // find this cell's new portals from the linked list
        int p = m_cellPortalHeads[i];
        while (p)
        {
            // 0 means linked list end
            int idx = p - 1;

            // Create the portal
            Portal portal;
            portal.link = m_extPortals[idx].link;

            Recti fpRect = m_extPortals[idx].rect;

            portal.idx_z   = m_extPortals[idx].idx_z;
            portal.xmn_xmx = (fpRect.getMin().i << 16) | (fpRect.getMax().i);
            portal.ymn_ymx = (fpRect.getMin().j << 16) | (fpRect.getMax().j);

            extPortals.pushBack(portal);
            p = m_extPortals[idx].next;
        }

        extCells[i].setPortalIdxAndCount(offset, extPortals.getSize() - offset);
    }

    m_builder.finishHeap();

    if (!m_builder.reserveHeap(sizeof(Portal) * extPortals.getSize() + 16))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }
    Portal* outputPortals = (Portal*)m_builder.allocOutput(tile.m_extPortals, sizeof(Portal) * (extPortals.getSize()));
    memcpy(outputPortals, extPortals.getPtr(), extPortals.getByteSize());
    m_builder.finishHeap();

    if (!isEmpty(tileIdx))
    {
        m_tiles[tileIdx].m_extCells = extCells;
        m_tiles[tileIdx].m_extPortals = outputPortals;
    } else
    {
        m_emptyTiles[getEmptyIndex(tileIdx)].m_extCells = extCells;
        m_emptyTiles[getEmptyIndex(tileIdx)].m_extPortals = outputPortals;
    }
    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Collect objects from all inputs
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::outputObjects (ImpTome& tome, ObjectHash& objectHash)
{
    Array<UINT32> ids(16, getAllocator());

    // first pass through tomes to establish what to do

    bool haveDistances = false;
    bool haveGroups = false;
    int objectCountEstimate = 0;

    for (int i = 0; i < m_numTomes; i++)
    {
        const ImpTome* tome = m_tomes[i];
        if (tome->containsGroups())
            haveGroups = true;
        if (!!tome->getObjectDistances())
            haveDistances = true;

        // TODO: this could be very wrong with object grouping
        objectCountEstimate += tome->getNumObjects();

        if (haveGroups && haveDistances)
            break;
    }

    // union object groups

    UnionFind<UINT32> unionFind(getAllocator());
    if (haveGroups)
    {
        for (int i = 0; i < m_numTomes; i++)
        {
            const ImpTome* tome = m_tomes[i];
            for (int o = 0; o < tome->getNumObjects(); o++)
            {
                int numIds = ((const Tome*)tome)->getObjectUserIDs(o, NULL, 0);
                ids.reset(numIds);
                ((const Tome*)tome)->getObjectUserIDs(o, ids.getPtr(), ids.getSize());
                for (int userIdIndex = 0; userIdIndex < ids.getSize(); userIdIndex++)
                {
                    UINT32 userId = ids[userIdIndex];
                    if (userIdIndex > 0)
                        unionFind.unionSets(ids[0], userId);
                }
            }
        }
    }

    // new object map

    int currentObjectIndex = 0;
    {
        Hash<int, int> groupMap(getAllocator());

        for (int i = 0; i < m_numTomes; i++)
        {
            const ImpTome* tome = m_tomes[i];
            for (int o = 0; o < tome->getNumObjects(); o++)
            {
                int numIds = ((const Tome*)tome)->getObjectUserIDs(o, NULL, 0);
                ids.reset(numIds);
                ((const Tome*)tome)->getObjectUserIDs(o, ids.getPtr(), ids.getSize());
                for (int userIdIndex = 0; userIdIndex < ids.getSize(); userIdIndex++)
                {
                    UINT32 userId = ids[userIdIndex];
                    if (objectHash.get(userId) != NULL)
                        continue;
                    if (haveGroups)
                    {
                        int& groupIndex = groupMap.getDefault(unionFind.findSet(userId), currentObjectIndex);
                        if (groupIndex == currentObjectIndex)
                            ++currentObjectIndex;
                        if (!objectHash.insert(userId, groupIndex))
                        {
                            setError(TomeCollection::ERROR_OUT_OF_MEMORY);
                            return false;
                        }
                    }
                    else
                    {
                        if (!objectHash.insert(userId, currentObjectIndex++))
                        {
                            setError(TomeCollection::ERROR_OUT_OF_MEMORY);
                            return false;
                        }
                    }
                }
            }
        }
    }
    unionFind.clear();

    // allocate

    tome.m_numObjects = currentObjectIndex;
    int numIds = objectHash.getNumKeys();
    size_t objBoundsSize = tome.m_numObjects * sizeof(ObjectBounds);
    size_t objDistSize = haveDistances ? tome.m_numObjects * sizeof(ObjectDistance) : 0;
    size_t userIdMapSize =  numIds * sizeof(UINT32);
    size_t userIdStartArrSize = haveGroups ? (tome.m_numObjects + 1) * sizeof(int) : 0;

    if (!m_builder.reserveHeap(objBoundsSize + objDistSize + 
        userIdMapSize + userIdStartArrSize + 64))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    ObjectBounds* objBounds = (ObjectBounds*)m_builder.allocOutput(tome.m_objBounds, objBoundsSize);
    for (int i = 0; i < tome.m_numObjects; ++i)
        ((AABB&)objBounds[i]) = AABB();

    ObjectDistance* objDistances = NULL;
    if (haveDistances)
        objDistances = (ObjectDistance*)m_builder.allocOutput(tome.m_objDistances, objDistSize);

    UINT32* userIDs = (UINT32*)m_builder.allocOutput(tome.m_userIDs, userIdMapSize);
    int* userIDStarts = NULL;
    if (haveGroups)
        userIDStarts = (int*)m_builder.allocOutput(tome.m_userIDStarts, userIdStartArrSize);

    m_builder.finishHeap();

    // output

    // Populate id starts array by counting number of IDs per index first and then
    // accumulating over the array. Also write out one ID per group.

    for (ObjectHash::Iterator it = objectHash.iterate(); objectHash.isValid(it); objectHash.next(it))
    {
        UINT32 id = objectHash.getKey(it);
        int idx = objectHash.getValue(it);
        if (userIDStarts)
            userIDStarts[idx + 1]++;
        userIDs[idx] = id;
    }

    if (haveGroups)
    {
        // accumulate
        for (int i = 0; i < tome.m_numObjects; i++)
            userIDStarts[i + 1] += userIDStarts[i];

        // spread IDs
        for (int i = tome.m_numObjects - 1; i >= 0; --i)
        {
            int start = userIDStarts[i];
            int end = userIDStarts[i+1];
            for (int j = start; j < end; ++j)
                userIDs[j] = userIDs[i];
        }

        // add other IDs
        for (ObjectHash::Iterator it = objectHash.iterate(); objectHash.isValid(it); objectHash.next(it))
        {
            UINT32 id = objectHash.getKey(it);
            int idx = objectHash.getValue(it);
            int ofs = userIDStarts[idx];
            UINT32 first = userIDs[ofs++];
            if (id != first)
            {
                while (userIDs[ofs] != first)
                    ofs++;
                userIDs[ofs] = id;
            }
        }
    }

    // output

    for (int i = 0; i < m_numTomes; i++)
    {
        const ImpTome* tome = m_tomes[i];

        DataArray boundsArray = tome->getObjectBounds();
        DataArray distsArray = tome->getObjectDistances();
        const ObjectBounds* boundsIn = (const ObjectBounds*)map(boundsArray);
        const ObjectDistance* distsIn = (const ObjectDistance*)map(distsArray);

        for (int o = 0; o < tome->getNumObjects(); o++)
        {
            int numIds = ((const Tome*)tome)->getObjectUserIDs(o, NULL, 0);
            ids.reset(numIds);
            ((const Tome*)tome)->getObjectUserIDs(o, ids.getPtr(), ids.getSize());

            AABB aabb(boundsIn[o].mn, boundsIn[o].mx);
            AABB distanceBounds = aabb;
            Vector2 distanceLimits(0.f, FLT_MAX);
            if (distsIn)
            {
                distanceBounds = AABB(distsIn[o].boundMin, distsIn[o].boundMax);
                distanceLimits = Vector2(distsIn[o].nearLimit, distsIn[o].farLimit);
            }

            for (int userIdIndex = 0; userIdIndex < ids.getSize(); userIdIndex++)
            {
                UINT32 userId = ids[userIdIndex];
                int objIdx = *objectHash.get(userId);
                ObjectBounds& boundsOut = objBounds[objIdx];

                if (!((AABB&)boundsOut).isOK())
                {
                    boundsOut.mn = aabb.getMin();
                    boundsOut.mx = aabb.getMax();
                    if (haveDistances)
                    {
                        ObjectDistance& distOut = objDistances[objIdx];
                        distOut.boundMin = distanceBounds.getMin();
                        distOut.boundMax = distanceBounds.getMax();
                        distOut.nearLimit = distanceLimits[0];
                        distOut.farLimit = distanceLimits[1];
                    }
                }
                else
                {
                    ((AABB&)boundsOut).grow(aabb);
                    if (haveDistances)
                    {
                        ObjectDistance& distOut = objDistances[objIdx];
                        AABB curDistBounds(distOut.boundMin, distOut.boundMax);
                        curDistBounds.grow(distanceBounds);
                        distOut.boundMin = curDistBounds.getMin();
                        distOut.boundMax = curDistBounds.getMax();
                        distOut.nearLimit = min2(distOut.nearLimit, distanceLimits[0]);
                        distOut.farLimit = max2(distOut.farLimit, distanceLimits[1]);
                    }
                }
            }
        }
    }

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Checks if we can split given inputs along the given
 *          axis aligned plane
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::isGoodSplit(int axis, float splitPos, int* inputNodes, int n, int& left)
{
    left = 0;
    int right = 0;

    for (int i = 0; i < n; i++)
    {
        if (m_tomes[inputNodes[i]]->getTreeMax()[axis] <= splitPos)
            left++;
        else if (m_tomes[inputNodes[i]]->getTreeMin()[axis] >= splitPos)
            right++;
        else
            return false;
    }

    // Don't generate empty subtrees
    if (!left || !right)
        return false;

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Partition given list of inputs into left and right sets
 *          by axis aligned plane.
 * \return  Number of inputs in the left set.
 *//*-------------------------------------------------------------------*/

int RuntimeTomeGenerator::splitPartition(int axis, float splitPos, int* inputNodes, int n)
{
    int i = 0, j = 0;
    for (i = 0, j = 0; i < n; i++)
    {
        if (m_tomes[inputNodes[i]]->getTreeMax()[axis] <= splitPos)
        {
            if (i != j)
                swap2(inputNodes[i], inputNodes[j]);
            j++;
        }
        else if (m_tomes[inputNodes[i]]->getTreeMin()[axis] >= splitPos)
            ;
        else
        {
            // shouldn't happen: we checked against this using isGoodSplit
            UMBRA_ASSERT(false);
        }
    }

    return j;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Given a split, construct subtrees.
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::constructSubtrees(int* inputNodes, int n, RuntimeSpatialSubdivision<Allocator>::Constructor& c, const AABB& bounds, int axis, float splitPos, bool isMedian, Umbra::UINT32 exitPortalMask)
{
    if (!c.isOk())
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    // Partition into left and right subtree by split position
    int nLeft = splitPartition(axis, splitPos, inputNodes, n);

    if (isMedian)
        c.medianSplit(axis);            // split from middle
    else
    if (!c.kdSplit(axis, splitPos)) // split by arbitrary AA-plane
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    c.innerValue(-1);

    // Make bounding boxes for left and right subtree
    AABB boundsLeft (bounds);
    AABB boundsRight(bounds);
    boundsLeft.setMax (axis, splitPos);
    boundsRight.setMin(axis, splitPos);

    int leftMask   = exitPortalMask & ~(1 << (axis * 2 + 1));
    int rightMask  = exitPortalMask & ~(1 << (axis * 2));

    // Construct subtrees.

    // Note that whole left subtree must be constructed first
    // due to how RuntimeSpatialSubdivision works.

    RuntimeSpatialSubdivision<Allocator>::Constructor c2;
    c.constructLeft(c2);
    if (!constructTopLevel(c2, boundsLeft, inputNodes, nLeft, leftMask))
        return false;

    c.constructRight(c2);
    if (!constructTopLevel(c2, boundsRight, inputNodes + nLeft, n - nLeft, rightMask))
        return false;

    c.finish();
    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Decide split axis and position from given set, and whether
 *          it's a median split.
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::selectSplit(int* inputNodes, int n, const AABB& bounds, int axis, float& splitPos, bool& isMedian)
{
    int nLeft = 0;
    isMedian = true;
    splitPos = bounds.getCenter()[axis];

    // Try median split.
    // Todo: this violates the idea that all arbitrary splits (kd-splits)
    // are near root of the tree.
    if (!isGoodSplit(axis, splitPos, inputNodes, n, nLeft))
    {
        isMedian = false;

        bool ok = false;
        int minDiff = INT_MAX;

        // Select a split position from all the possible ones
        // if median split isn't possible
        for (int splitNode = 0; splitNode < n; splitNode++)
        {
            float testSplit = m_tomes[inputNodes[splitNode]]->getTreeMax()[axis];

            if (isGoodSplit(axis, testSplit, inputNodes, n, nLeft))
            {
                // Select a split position that divides the inputs most equally
                int diff = abs2(nLeft - (n - nLeft));
                if (nLeft > 0 && nLeft < n && diff < minDiff)
                {
                    ok = true;
                    minDiff  = diff;
                    splitPos = testSplit;
                }
            }
        }

        if (!ok)
            return false;
    }

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   "Paste" existing partial tile tree from given input
 *          (i.e. StreamingTile).
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::constructInputTree(RuntimeSpatialSubdivision<Allocator>::Constructor& c, int inputIndex, KDTree& tree, int nodeIndex, const AABB& aabb, int parent, Umbra::UINT32 borderMask, Umbra::UINT32 exitPortalMask)
{
    if (!c.isOk())
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }
    
    KDTree::Split s = tree.getSplit(nodeIndex);

    if (s == KDTree::LEAF)
    {
        // Compute global index
        /*int leafIndex = m_leafStarts[inputIndex] + tree.getLeafIdx(nodeIndex);
        AABB aabb2(m_leaves[leafIndex]->getTreeMin(), m_leaves[leafIndex]->getTreeMax());
        UMBRA_ASSERT(aabb == aabb2);
        c.leaf(leafIndex);*/

        int leafIndex = m_leafStarts[inputIndex] + nodeIndex;
        UMBRA_ASSERT(aabb == m_tiles[leafIndex].m_tile->getAABB());

        c.leaf(leafIndex);
        c.innerValue(leafIndex);

        m_tiles[leafIndex].m_parentTile = parent;
        m_tiles[leafIndex].m_borderMask = borderMask;
        m_tiles[leafIndex].m_exitPortalMask = exitPortalMask & borderMask;

        c.finish();
        return true;
    } else
    {
        // Straightforward tree copy

        AABB aabbLeft = aabb;
        AABB aabbRight = aabb;

        UINT32 borderMaskLeft  = borderMask & ~(1 << (s*2+1));
        UINT32 borderMaskRight = borderMask & ~(1 << (s*2));

        /*
        if (tree.isKDSplit(nodeIndex))
        {
            aabbLeft.setMax(s, tree.getKDSplit(nodeIndex));
            aabbRight.setMin(s, tree.getKDSplit(nodeIndex));
            c.kdSplit(s, tree.getKDSplit(nodeIndex));
        } else
        {
            aabbLeft.setMax(s,  aabb.getCenter()[s]);
            aabbRight.setMin(s, aabb.getCenter()[s]);
            c.medianSplit(s);
        } */

        if (tree.isNonMedianSplit(nodeIndex))
        {
            float split = tree.getNonMedianSplit(nodeIndex);
            aabbLeft.setMax(s,  split);
            aabbRight.setMin(s, split);
            if (!c.kdSplit(s,   split))
            {
                setError(TomeCollection::ERROR_OUT_OF_MEMORY);
                return false;
            }
        } else
        {
            aabbLeft.setMax(s,  aabb.getCenter()[s]);
            aabbRight.setMin(s, aabb.getCenter()[s]);
            c.medianSplit(s);
        }

        DataPtr ptr;
        DataArray subTiles = m_tomes[inputIndex]->getTileOffsets(false);
        subTiles.getElem(ptr, nodeIndex);

        if (!!ptr)
        {
            int leafIndex = m_leafStarts[inputIndex] + nodeIndex;
            c.innerValue(leafIndex);
            m_tiles[leafIndex].m_parentTile = parent;
            m_tiles[leafIndex].m_borderMask = borderMask;
            m_tiles[leafIndex].m_exitPortalMask = exitPortalMask & borderMask;
            parent = leafIndex;
        } else
            c.innerValue(-1);

        RuntimeSpatialSubdivision<Allocator>::Constructor c2;
        c.constructLeft(c2);
        if (!constructInputTree(c2, inputIndex, tree,
                                tree.getLeftChildIdx(nodeIndex), aabbLeft, parent, borderMaskLeft, exitPortalMask))
            return false;

        c.constructRight(c2);
        if (!constructInputTree(c2, inputIndex, tree,
                                tree.getRightChildIdx(nodeIndex), aabbRight, parent, borderMaskRight, exitPortalMask))
            return false;

        c.finish();
        return true;
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief   Construct a top level tree (tile tree) from given array
 *          of input indices.
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::constructTopLevel(RuntimeSpatialSubdivision<Allocator>::Constructor& c, const AABB& bounds, int* inputNodes, int n, Umbra::UINT32 exitPortalMask)
{
    if (!c.isOk())
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    if (n == 0)
    {
        // This is an empty slot with no corresponding StreamingTile.
        // We'll generate an empty tile here later.

        // Store AABB for this tile.
        EmptyTile empty;
        empty.m_aabb = bounds;
        empty.m_exitPortalMask = exitPortalMask;
        m_emptyTiles.pushBack(empty);

        // Index empty tiles beginning from m_tiles.getSize(),
        // we'll later know it's empty based on the index
        // (see RuntimeTomeGenerator::isEmpty).
        int emptyIndex = m_tiles.getSize() + m_emptyTiles.getSize() - 1;
        c.leaf(emptyIndex);
        c.innerValue(emptyIndex);
        c.finish();
        return true;
    }
    else if (n == 1)
    {
        // One StreamingTile left for this node, but we may need to split further

        AABB inputAABB(m_tomes[inputNodes[0]]->getTreeMin(), m_tomes[inputNodes[0]]->getTreeMax());

        // Manage empty space:
        // We'll split further if this node is bigger than the StreamingTile
        // we're about to assign into it.

        Vector3 diff = bounds.getDimensions() - inputAABB.getDimensions();
        if (diff.lengthSqr() > 0)
        {
            // Split longest axis with extra space
            int axis = getLongestAxis(diff);

            // Split after/before the only input
            float splitPos;
            if (bounds.getMax()[axis] > inputAABB.getMax()[axis])
                splitPos = inputAABB.getMax()[axis];
            else
                splitPos = inputAABB.getMin()[axis];

            if (!constructSubtrees(inputNodes, n, c, bounds, axis, splitPos, false, exitPortalMask))
                return false;

            return true;

        } else
        {
            // The StreamingTile fits this slot exactly.
            GeneratorTree tree = getInputTree(inputNodes[0]);
            if (!constructInputTree(c, inputNodes[0], tree.tree, 0, bounds, -1, g_faceMaskFull, exitPortalMask))
                return false;
        }
        return true;
    } else
    {
        // Inner node

        // Favor longest axis
        int firstAxis = bounds.getLongestAxis();

        for (int i = 0; i < 3; i++)
        {
            int axis = (i + firstAxis) % 3;

            bool  isMedian = true;
            float splitPos = 0.f;

            if (!selectSplit(inputNodes, n, bounds, axis, splitPos, isMedian))
                continue;

            if (!constructSubtrees(inputNodes, n, c, bounds, axis, splitPos, isMedian, exitPortalMask))
                // I guess we should always be able to find a split, since
                // arbitrary splits are allowed, and this shouldn't happen?
                continue;

            return true;
        }

        return false;
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief   Make the top level tree (i.e. tile tree)
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::makeTopLevel(RuntimeSpatialSubdivision<Allocator>& topLevel)
{
    int* nodes = NULL;

#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    try
    {
#endif

        for (int j = 0; j < m_numTomes; j++)
        {
            // Find scene dimensions
            AABB aabb(m_tomes[j]->getTreeMin(), m_tomes[j]->getTreeMax());
            m_bounds.grow(aabb);
        }

        if (m_minBounds.isOK())
        {
            m_minBounds.setMin(0, floorf(m_minBounds.getMin().x));
            m_minBounds.setMin(1, floorf(m_minBounds.getMin().y));
            m_minBounds.setMin(2, floorf(m_minBounds.getMin().z));
            m_minBounds.setMax(0, ceilf(m_minBounds.getMax().x));
            m_minBounds.setMax(1, ceilf(m_minBounds.getMax().y));
            m_minBounds.setMax(2, ceilf(m_minBounds.getMax().z));
            m_bounds.grow(m_minBounds);
        }

        RuntimeSpatialSubdivision<Allocator>::Constructor rootConstructor;
        topLevel.construct(rootConstructor,
                           // estimate node count based on integer dimensions
                           0,
                           m_mainAllocator);

        // Create array of input indices for constructTopLevel
        int  numInputs = 0;
        nodes = (int*)UMBRA_MALLOC(sizeof(int) * m_numTomes);
        if (!nodes)
        {
            setError(TomeCollection::ERROR_OUT_OF_MEMORY);
            return false;
        }

        for (int i = 0; i < m_numTomes; i++)
            nodes[numInputs++] = i;

        // Construct the tree recursively
        bool ok = constructTopLevel(rootConstructor, m_bounds, nodes, numInputs, g_faceMaskFull);
        UMBRA_FREE(nodes);
        if (!ok)
        {
            if (getError() == TomeCollection::SUCCESS)
                setError(TomeCollection::ERROR_OVERLAPPING_TOMES);
            return false;
        }


#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    } catch(OOMException)
    {
        if (nodes)
            UMBRA_FREE(nodes);
        throw;
    }
#endif 
    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Compress the top level tree (i.e. tile tree)
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::serializeTopLevel(RuntimeSpatialSubdivision<Allocator>& topLevel, ImpTome& tome)
{
    // Serialize into runtime tree representation.

    UINT32* toplevelTreeData = NULL;
    float*  toplevelKDData   = NULL;
    if (!topLevel.serialize(m_builder, tome.m_tileTree,
                      &m_slotToIndex, &toplevelTreeData, &toplevelKDData, AABB(m_bounds.getMin(), m_bounds.getMax())))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    m_numSlots = m_slotToIndex.getSize();

    for (int i = 0; i < m_tiles.getSize(); i++)
        m_tiles[i].m_slot = -1;

    for (int i = 0; i < m_numSlots; i++)
    {
        int index = m_slotToIndex[i];
        if (index >= 0)
        {
            if (isEmpty(index))
                m_emptyTiles[getEmptyIndex(index)].m_slot = i;
            else
                m_tiles[index].m_slot = i;
        }
    }

    m_topLevelTree = KDTree(tome.m_tileTree.getNodeCount(), toplevelTreeData, DataArray(toplevelKDData-1, DataPtr(4), sizeof(float), tome.m_tileTree.m_numSplitValues));

    // Generate slot paths

    int bitsPerPath = m_topLevelTree.getMaxDepth();
    int pathDataSize = UMBRA_BITVECTOR_SIZE(m_numSlots * bitsPerPath);
    if (!m_builder.reserveHeap(pathDataSize + 16))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }
    UINT32* slotPaths = (UINT32*)m_builder.allocOutput(tome.m_slotPaths, pathDataSize);
    memset(slotPaths, 0, UMBRA_BITVECTOR_SIZE(m_numSlots * bitsPerPath));
    m_topLevelTree.getPaths(slotPaths, bitsPerPath);
    m_builder.finishHeap();
    tome.m_bitsPerSlotPath = bitsPerPath;

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Generates global data (ImpTome)
 *//*-------------------------------------------------------------------*/

bool RuntimeTomeGenerator::generateHeader(ImpTome& tome, Result& result, const Result* oldResult)
{
    // Global tile tree
    ///////////////////////////

    {
        RuntimeSpatialSubdivision<Allocator> topLevel(m_mainAllocator);
        // Generate top level tree (i.e. the tile tree)
        if (!makeTopLevel(topLevel))
            return false;
        if (!serializeTopLevel(topLevel, tome))
            return false;
    }

    // Fields
    ///////////////////////////

    tome.m_versionMagic = (((UINT32)TOME_MAGIC << 16) | (UINT32)TOME_VERSION);
    tome.m_treeMin      = m_bounds.getMin();
    tome.m_treeMax      = m_bounds.getMax();
    tome.m_numTiles     = m_numSlots;
    tome.m_lodBaseDistance = FLT_MAX;
    tome.m_numClusters  = 0;
    tome.m_numTomes     = m_numTomes;

    // Contexts and ExtTiles
    ///////////////////////////

    if (!m_builder.reserveHeap(m_numTomes * sizeof(TomeContext) + m_numSlots * sizeof(ExtTile)))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    if (m_numTomes)
        m_tomeContexts = (TomeContext*)m_builder.allocOutput(result.m_contexts, m_numTomes * sizeof(TomeContext));
    m_extTiles = (ExtTile*)m_builder.allocOutput(result.m_extTiles, m_numSlots * sizeof(ExtTile));
    memset(m_extTiles, 0, sizeof(ExtTile) * m_numSlots);
    m_builder.finishHeap();

    // Objects
    ///////////////////////////

    ObjectHash objectHash(getAllocator());
    if (!outputObjects(tome, objectHash))
        return false;

    // Per-tome context object
    ///////////////////////////

    UINT32 perTomeSize = 0;

    for (int i = 0; i < m_numTomes; i++)
    {
        perTomeSize += sizeof(int) * 
            (UMBRA_ALIGN(m_tomes[i]->getNumObjects(),    4) +
             UMBRA_ALIGN(m_tomes[i]->getTileArraySize(), 4) + 
             UMBRA_ALIGN(m_tomes[i]->getNumGates(),      4));
    }

    if (!m_builder.reserveHeap(perTomeSize))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    Array<int*> tileMaps(getAllocator());
    if (!tileMaps.resize(m_numTomes))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    GateHash gateHash(getAllocator());

    for (int i = 0; i < m_numTomes; i++)
    {
        m_tomeContexts[i].m_tome = m_tomes[i];

        // object index to global object index
        int* objMap  = (int*)m_builder.allocOutput(m_tomeContexts[i].m_objGlobalIdx, sizeof(int)  * m_tomes[i]->getNumObjects());
        // slot index to global slot index
        int* tileMap = (int*)m_builder.allocOutput(m_tomeContexts[i].m_tileGlobalIdx, sizeof(int) * m_tomes[i]->getTileArraySize());
        // gate index to global gate index
        int* gateMap = (int*)m_builder.allocOutput(m_tomeContexts[i].m_gateGlobalIdx, sizeof(int) * m_tomes[i]->getNumGates());

        // fill object map
        for (int o = 0; o < m_tomes[i]->getNumObjects(); o++)
        {
            // Any of the group's ids should find us the global group
            UINT32 userId = 0;
            int numIds = ((const Tome*)m_tomes[i])->getObjectUserIDs(o, &userId, 1);

            if (numIds)
            {
                const int* idx  = objectHash.get(userId);
                UMBRA_ASSERT(idx);
                objMap[o] = *idx;
            }
        }

        // fill gate map
        for (int g = 0; g < m_tomes[i]->getNumGates(); g++)
        {
            UINT32 userId = m_tomes[i]->getGateIndexMap()[g];

            int* globalIdx = gateHash.get(userId);
            if (!globalIdx)
                globalIdx = gateHash.insert(userId, gateHash.getNumKeys());
            gateMap[g] = *globalIdx;
        }

        // store to be filled later
        tileMaps[i] = tileMap;

        tome.m_numClusters += m_tomes[i]->getNumClusters();
        tome.m_lodBaseDistance = min2(tome.m_lodBaseDistance, m_tomes[i]->getLodBaseDistance());
    }

    m_builder.finishHeap();
    objectHash.clear();

    tome.m_numGates = gateHash.getNumKeys();

    if (!m_builder.reserveHeap(gateHash.getNumKeys() * sizeof(UINT32) + 16 +
                               m_numSlots * sizeof(float) + 16 +
                              (m_numSlots + 1) * sizeof(int) + 16))
    {
        setError(TomeCollection::ERROR_OUT_OF_MEMORY);
        return false;
    }

    UINT32* gateMapping = (UINT32*)m_builder.allocOutput(tome.m_gateIndexMap, tome.m_numGates * sizeof(UINT32));
    float*  lodLevels   = (float*)m_builder.allocOutput(tome.m_tileLodLevels, m_numSlots * sizeof(float));
    int*    cellStarts  = (int*)m_builder.allocOutput(tome.m_cellStarts, (m_numSlots + 1) * sizeof(int));
    m_builder.finishHeap();

    // Cell starts array
    ///////////////////////////

    // Generate cell starts array
    // (i.e. global index of first cell for each tile)
    int numCells = 0;
    for (int i = 0; i < m_numSlots; i++)
    {
        if (m_slotToIndex[i] < 0)
            continue;

        cellStarts[i] = numCells;
        if (!isEmpty(m_slotToIndex[i]))
            numCells += m_tiles[m_slotToIndex[i]].m_tile->getNumCells();
        else
            numCells += UMBRA_EMPTY_SLOT_CELLS;
    }
    cellStarts[m_numSlots] = numCells;

    // Gate id mapping
    ///////////////////////////

    {
        // Gate global idx to id mapping
        GateHash::Iterator it = gateHash.iterate();
        while(gateHash.isValid(it))
        {
            UINT32  userId = gateHash.getKey(it);
            int     idx    = gateHash.getValue(it);

            UMBRA_ASSERT(idx >= 0 && idx < tome.m_numGates);
            gateMapping[idx] = userId;

            gateHash.next(it);
        }
        gateHash.clear();
    }

    // Lod levels, per-tile tomes
    ////////////////////////////////

    for (int i = 0; i < m_numSlots; i++)
    {
        lodLevels[i] = 1.f;
        m_extTiles[i].m_tomeIdx = -1;
        m_extTiles[i].m_exitPortalMask = 0;
        m_extTiles[i].m_localSlot = -1;
    }

    if (m_numTomes)
    {
        int*         starts    = m_leafStarts.getPtr();
        int          inputIdx  = -1;
        int          localIdx  = 0;
        const float* lodLocal = NULL;
        float        lodMultiplier = 0.f;

        for (int i = 0; i < m_tiles.getSize(); i++)
        {
            // map tiles to inputs, generating inputIdx (input tome index)
            // and localIdx (slot number in input tome)
            if (inputIdx < m_numTomes - 1 && i >= *starts)
            {
                inputIdx++;
                starts++;
                localIdx = 0;
                lodLocal = (const float*)map(m_tomes[inputIdx]->getTileLodLevels());
                lodMultiplier = m_tomes[inputIdx]->getLodBaseDistance() / tome.m_lodBaseDistance;
            }

            int slot = mapIndexToSlot(i);
            if (slot < 0)
            {
                tileMaps[inputIdx][localIdx] = 0;
                localIdx++;
                continue;
            }

            tileMaps[inputIdx][localIdx] = slot;
            lodLevels[slot] = lodMultiplier * lodLocal[localIdx];

            m_extTiles[slot].m_tomeIdx        = inputIdx;
            m_extTiles[slot].m_extCells       = 0;
            m_extTiles[slot].m_extPortals     = 0;
            m_extTiles[slot].m_exitPortalMask = m_tiles[i].m_exitPortalMask;
            m_extTiles[slot].m_localSlot      = m_tiles[i].m_local;

            localIdx++;
        }
    }
    
    for (int i = 0; i < m_emptyTiles.getSize(); i++)
    {
        int slot = m_emptyTiles[i].m_slot;
        m_extTiles[slot].m_exitPortalMask = m_emptyTiles[i].m_exitPortalMask;
    }

    // Mapping from old RuntimeTomeGenerator slot indices to new global indices (if exists)
    ////////////////////////////////

    if (oldResult && oldResult->m_result)
    {
        const ImpTome* oldTome = oldResult->m_result;
        KDTree oldTree(oldTome->getTreeNodeCount(), (const UINT32*)map(oldTome->getTreeData()), oldTome->getTreeSplits());

        m_oldNewTileMap.setAllocator(getAllocator());
        m_newOldTileMap.setAllocator(getAllocator());
        if (!m_oldNewTileMap.reset(oldTome->getTileArraySize()) ||
            !m_newOldTileMap.reset(m_numSlots))
        {
            setError(TomeCollection::ERROR_OUT_OF_MEMORY);
            return false;
        }

        // Initialize old->new map
        for (int i = 0; i < m_oldNewTileMap.getSize(); i++)
            m_oldNewTileMap[i] = -1;

        // Initialize new->old map
        for (int i = 0; i < m_newOldTileMap.getSize(); i++)
            m_newOldTileMap[i] = -1;
        
        // Initialize operation work mem
        Array<int> newTiles(getAllocator());
        if (!newTiles.resize(getNumTotalTiles()))
        {
            setError(TomeCollection::ERROR_OUT_OF_MEMORY);
            return false;
        }

        int j = 0;
        for (int i = 0; i < newTiles.getSize(); i++)
        {
            if ((isEmpty(i) || m_tiles[i].m_tile) &&
                oldTome->getAABB().contains(getAABBByIndex(i)))
            {
                    newTiles[j++] = i;
            }
        }
        if (!newTiles.resize(j))
        {
            setError(TomeCollection::ERROR_OUT_OF_MEMORY);
            return false;
        }

        makeOldNewMap(oldTome, oldTree, oldTree.getRoot(), newTiles.getPtr(), newTiles.getSize(), oldTome->getAABB());
    }

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Construct new->old and reverse slot idx maps.
 *//*-------------------------------------------------------------------*/
void RuntimeTomeGenerator::makeOldNewMap(const ImpTome* oldTome, KDTree& oldTree, int nodeIdx, int* indices, int N, const AABB& aabb)
{
    KDTree::Split split = oldTree.getSplit(nodeIdx);    

    const ImpTile* oldTilePtr = oldTome->getTile(nodeIdx, true);

    if (split == KDTree::LEAF)
    {
        if (N == 1 && isEmpty(indices[0]))
        {
            if (oldTilePtr->getFlags() & ImpTile::TILEFLAG_ISEMPTY &&
                getAABBByIndex(indices[0]) == oldTilePtr->getAABB())
            {
                int newSlot = mapIndexToSlot(indices[0]);
                m_newOldTileMap[newSlot] = nodeIdx;
                m_oldNewTileMap[nodeIdx] = newSlot;
            }
        } else
        if (N == 1 && oldTilePtr == m_tiles[indices[0]].m_tile)
        {
            int newSlot = m_tiles[indices[0]].m_slot;
            m_newOldTileMap[newSlot] = nodeIdx;
            m_oldNewTileMap[nodeIdx] = newSlot;
        }
        return;
    }

    float splitPos;
    if (oldTree.isNonMedianSplit(nodeIdx))
        splitPos = oldTree.getNonMedianSplit(nodeIdx);
    else
        splitPos = (aabb.getMax()[split] + aabb.getMin()[split]) / 2.f;

    AABB leftAABB = aabb;
    leftAABB.setMax(split, splitPos);
    AABB rightAABB = aabb;
    rightAABB.setMin(split, splitPos);

    int i = 0, j = 0;
    for (i = 0, j = 0; i < N; i++)
    {
        AABB aabb = getAABBByIndex(indices[i]);
        if (aabb.getMax()[split] <= splitPos)
        {
            if (i != j)
                swap2(indices[i], indices[j]);
            j++;
        }
        else if (aabb.getMin()[split] >= splitPos)
            ;
        // Intersects split, Hierarchy tile
        else
        {
            if (isEmpty(indices[i]))
            {
            } else
            if (oldTilePtr == m_tiles[indices[i]].m_tile)
            {
                int newSlot = m_tiles[indices[i]].m_slot;
                m_newOldTileMap[newSlot] = nodeIdx;
                m_oldNewTileMap[nodeIdx] = newSlot;
            }
            swap2(indices[i], indices[N-1]);
            N--;
            i--;
        }
    }

    makeOldNewMap(oldTome, oldTree, oldTree.getLeftChildIdx(nodeIdx),  indices,     j,     leftAABB);
    makeOldNewMap(oldTome, oldTree, oldTree.getRightChildIdx(nodeIdx), indices + j, N - j, rightAABB);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Match two tile faces together recursively.
 *          Generates new portals on the faces.
 *//*-------------------------------------------------------------------*/

void TileMatcher::matchTiles(
                int _idxA, const Recti& _rectA,
                int _idxB, const Recti& _rectB)
{

    struct StackEntry
    {
        StackEntry() : 
            rectA(Recti::NO_INIT),
            rectB(Recti::NO_INIT)
        {}

        void set(int pidxA, const Recti& prectA,
                 int pidxB, const Recti& prectB)
        {
            idxA   = pidxA;
            rectA  = prectA;
            idxB   = pidxB;
            rectB  = prectB;
        }

        Recti   rectA;
        Recti   rectB;
        int     idxA;
        int     idxB;
    };

    int        stackHead = 0;
    StackEntry stack[64];

    stack[stackHead++].set(_idxA, _rectA, _idxB, _rectB);

    while (stackHead > 0)
    {
        stackHead--;
        StackEntry UMBRA_ATTRIBUTE_ALIGNED16(current) = stack[stackHead];

        // Get splits for both nodes
        KDTree::Split splitA = m_treeA.getSplit(current.idxA);
        KDTree::Split splitB = m_treeB.getSplit(current.idxB);

        // Expect that this tree is on the face, and contains
        // no splits along the face's axis.
        UMBRA_ASSERT((int)splitA != getFaceAxis(m_faceA));
        UMBRA_ASSERT((int)splitB != getFaceAxis(m_faceA^1));

        // Both nodes are leaves, generate a portal
        if (splitA == KDTree::LEAF && splitB == KDTree::LEAF)
        {
            //int cellA = (int)m_mappingA.getElem(m_treeA.getLeafIdx(idxA) * m_widthA, m_widthA);
            //int cellB = (int)m_mappingB.getElem(m_treeB.getLeafIdx(idxB) * m_widthB, m_widthB);

            int cellA = unpackElem(m_mappingA, m_treeA.getLeafIdx(current.idxA) * m_widthA, m_widthA);
            int cellB = unpackElem(m_mappingB, m_treeB.getLeafIdx(current.idxB) * m_widthB, m_widthB);

            // Ignore invalid cell (outside cell)

            if (cellA == m_invalidA || cellB == m_invalidB)
                continue;

            // Get common rectangle
            SIMDRegister32 rectMn,  rectMx;
            SIMDRegister32 rectMnB, rectMxB;
            loadSIMDRect(current.rectA, rectMn, rectMx);
            loadSIMDRect(current.rectB, rectMnB, rectMxB);

            rectMn = SIMDMax32(rectMn, rectMnB);
            rectMx = SIMDMin32(rectMx, rectMxB);

            if (SIMDCompareGTTestAny32(rectMn, rectMx))
                continue;

            //Recti rect = rectA;
            //rect.clamp(rectB);
            //rect.inflate(m_portalExpand);

            // Figure out z
            //int axis     = getFaceAxis(m_faceA);
            //Vector3 dim  = m_boundsA.getDimensions();
            UINT32 z     = getFaceDirection(m_faceA) * 65535;
            //float z      = (m_boundsA.getFaceDist(m_faceA) - m_boundsA.getMin()[axis]) / dim[axis];
            //z            = min2(1.f, max2(0.f, z));

            // Compute plink and idx_z for the portal
            UINT32 plink = BUILD_PORTAL_LINK(m_faceA, 0, 0, 0, m_slotB);
            //UINT32 idx_z = (cellB << 16) | ((int)(z * 65535.f));
            UMBRA_ASSERT(z <= 65535);
            UINT32 idx_z = (cellB << 16) | z;

            // Traverse previously generated portals for this cell
            // to see if there's already matching portal
            int idx = m_heads[cellA];
            while(idx)
            {
                if (m_portals[idx-1].link  == plink &&
                    m_portals[idx-1].idx_z == idx_z)
                {
                    SIMDRegister32 rectMnB, rectMxB;
                    loadSIMDRect(m_portals[idx-1].rect, rectMnB, rectMxB);

                    rectMn = SIMDMin32(rectMn, rectMnB);
                    rectMx = SIMDMax32(rectMx, rectMxB);

                    //rect.grow(m_portals[idx-1].rect);
                    break;
                }
                idx = m_portals[idx-1].next;
            }

            if (idx)
            {
                // Combine with old portal
                storeSIMDRect(m_portals[idx-1].rect, rectMn, rectMx);
            } else
            {
                // Create new portal
                ExternalPortal extPortal;
                extPortal.link  = plink;
                extPortal.idx_z = idx_z;
                //extPortal.rect  = rect;
                storeSIMDRect(extPortal.rect, rectMn, rectMx);
                extPortal.next  = m_heads[cellA];
                m_portals.pushBack(extPortal);

                m_heads[cellA] = m_portals.getSize(); // note +1, 0 means end
            }

            continue;
        }

        // Expand both A and B as special case

        if ((current.rectA == current.rectB) &&
            !m_treeA.isNonMedianSplit(current.idxA) &&
            !m_treeB.isNonMedianSplit(current.idxB))
        {
            if (splitA == splitB)
            {
                int rightChildA = m_treeA.getRightChildIdx(current.idxA);
                int leftChildA  = rightChildA - 1;
                int rectAxisA   = m_faceAxes[splitA];

                int rightChildB = m_treeB.getRightChildIdx(current.idxB);
                int leftChildB  = rightChildB - 1;

                /*
                if (m_treeA.getSplit(rightChildA) == KDTree::LEAF &&
                    m_treeA.getSplit(leftChildA)  == KDTree::LEAF &&
                    m_treeB.getSplit(rightChildB) == KDTree::LEAF &&
                    m_treeB.getSplit(leftChildB)  == KDTree::LEAF)
                {
                }
                */

                int split = (current.rectA.getMin()[rectAxisA] + current.rectA.getMax()[rectAxisA]) / 2;

                Recti rectLeft, rectRight;
                current.rectA.split(rectAxisA, split, rectLeft, rectRight);

                stack[stackHead++].set(leftChildA,  rectLeft,  leftChildB,  rectLeft);
                stack[stackHead++].set(rightChildA, rectRight, rightChildB, rectRight);

                continue;
            }
            else if (splitA != KDTree::LEAF && splitB != KDTree::LEAF)
            {
                // unequal splits, practically never happens

                int rightChildA = m_treeA.getRightChildIdx(current.idxA);
                int leftChildA  = rightChildA - 1;
                int rectAxisA   = m_faceAxes[splitA];

                int rightChildB = m_treeB.getRightChildIdx(current.idxB);
                int leftChildB  = rightChildB - 1;
                int rectAxisB   = m_faceAxes[splitB];

                int splitA = (current.rectA.getMin()[rectAxisA] + current.rectA.getMax()[rectAxisA]) / 2;
                int splitB = (current.rectB.getMin()[rectAxisB] + current.rectB.getMax()[rectAxisB]) / 2;

                Recti rectLeftA, rectRightA;
                current.rectA.split(rectAxisA, splitA, rectLeftA, rectRightA);
                Recti rectLeftB, rectRightB;
                current.rectB.split(rectAxisB, splitB, rectLeftB, rectRightB);

                stack[stackHead++].set(leftChildA,  rectLeftA,
                                       leftChildB,  rectLeftB);
                stack[stackHead++].set(rightChildA, rectRightA,
                                       rightChildB, rectRightB);
                stack[stackHead++].set(leftChildA,  rectLeftA,
                                       rightChildB, rectRightB);
                stack[stackHead++].set(rightChildA, rectRightA,
                                       leftChildB,  rectLeftB);

                continue;
            }
        }

        // Node A, B or both are non-leaves.
        // Select which one to recurse into next.
        bool expandA = true;
        if (splitA != KDTree::LEAF && splitB != KDTree::LEAF)
        {
            // Both non-leaves: recurse into one with longer axis
            int aMax = current.rectA.getMaxAxisLength();
            int bMax = current.rectB.getMaxAxisLength();
            expandA = aMax >= bMax;
        } else
            // Only one of them is a leaf, choose the one that's not
            expandA = (splitA == KDTree::LEAF) ? false : true;

        if (expandA)
        {
            // If we chose to expand A
            int rightChild = m_treeA.getRightChildIdx(current.idxA);
            int leftChild  = rightChild - 1;
            int rectAxis   = m_faceAxes[splitA];

            // Get split position
            int split;
            if (m_treeA.isNonMedianSplit(current.idxA))
                split = (int)(((m_treeA.getNonMedianSplit(current.idxA) - m_boundsA.getMin()[splitA]) / m_boundsA.getDimensions()[splitA]) * 65535.f);
            else
                split = (current.rectA.getMin()[rectAxis] + current.rectA.getMax()[rectAxis]) / 2;

            // Get rectangles for left and right subtree
            Recti rectLeft, rectRight;
            current.rectA.split(rectAxis, split, rectLeft, rectRight);

            // Recurse into children
            if (rectLeft.intersectsWithArea(current.rectB))
                stack[stackHead++].set(leftChild,      rectLeft,
                                       current.idxB,   current.rectB);

            if (rectRight.intersectsWithArea(current.rectB))
                stack[stackHead++].set(rightChild,     rectRight,
                                       current.idxB,   current.rectB);
        }
        else
        {
            // If we chose to expand B

            int rightChild = m_treeB.getRightChildIdx(current.idxB);
            int leftChild  = rightChild - 1;
            int rectAxis   = m_faceAxes[splitB];

            int split;
            if (m_treeB.isNonMedianSplit(current.idxB))
                split = (int)(((m_treeB.getNonMedianSplit(current.idxB) - m_boundsA.getMin()[splitB]) / m_boundsA.getDimensions()[splitB]) * 65535.f);
            else
                split = (current.rectB.getMin()[rectAxis] + current.rectB.getMax()[rectAxis]) / 2;

            Recti rectLeft, rectRight;
            current.rectB.split(rectAxis, split, rectLeft, rectRight);

            if (current.rectA.intersectsWithArea(rectLeft))
                stack[stackHead++].set(current.idxA,   current.rectA,
                                       leftChild,      rectLeft);

            if (current.rectA.intersectsWithArea(rectRight))
                stack[stackHead++].set(current.idxA,   current.rectA,
                                       rightChild,     rectRight);
        }
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief   Match tile face with border, creating exit portals.
 *          Very similiar to TileMatcher::matchTiles, except that
 *          this uses only one tree.
 *//*-------------------------------------------------------------------*/

void BorderMatcher::matchBorder(int idxA, const Recti& rectA)
{
    KDTree::Split splitA = m_treeA.getSplit(idxA);

    if (splitA == KDTree::LEAF)
    {
        //int cellA = (int)m_mappingA.getElem(m_treeA.getLeafIdx(idxA) * m_widthA, m_widthA);
        int cellA = unpackElem(m_mappingA, m_treeA.getLeafIdx(idxA) * m_widthA, m_widthA);

        // Ignore invalid cell (outside cell)

        if (cellA == m_invalidA)
            return;

        Recti rect    = rectA;

        int axis     = getFaceAxis(m_faceA);
        Vector3 dim  = m_boundsA.getDimensions();
        float z      = (m_boundsA.getFaceDist(m_faceA) - m_boundsA.getMin()[axis]) / dim[axis];
        z            = min2(1.f, max2(0.f, z));

        // This is an exit portal
        UINT32 plink = BUILD_PORTAL_LINK(m_faceA, 1, 0, 0, Portal::getMaxSlotIdx());
        UINT32 idx_z = (0xffffffff << 16) | ((int)(z * 65535.f));

        int idx = m_heads[cellA];
        while(idx)
        {
            if (m_portals[idx-1].idx_z == idx_z &&
                m_portals[idx-1].link  == plink)
            {
                rect.grow(m_portals[idx-1].rect);
                break;
            }
            idx = m_portals[idx-1].next;
        }

        if (idx)
        {
            m_portals[idx-1].rect   = rect;
        } else
        {
            ExternalPortal extPortal;
            extPortal.link  = plink;
            extPortal.idx_z = idx_z;
            extPortal.rect  = rect;
            extPortal.next  = m_heads[cellA];
            m_portals.pushBack(extPortal);

            m_heads[cellA] = m_portals.getSize(); // note +1
        }

        return;
    }

    int rightChild = m_treeA.getRightChildIdx(idxA);
    int leftChild  = rightChild - 1;
    int rectAxis   = m_faceAxes[splitA];

    int split;
    if (m_treeA.isNonMedianSplit(idxA))
        split = (int)(((m_treeA.getNonMedianSplit(idxA) - m_boundsA.getMin()[splitA]) / m_boundsA.getDimensions()[splitA]) * 65535.f);
    else
        split = (rectA.getMin()[rectAxis] + rectA.getMax()[rectAxis]) / 2;

    Recti rectLeft, rectRight;
    rectA.split(rectAxis, split, rectLeft, rectRight);

    matchBorder(leftChild,  rectLeft);
    matchBorder(rightChild, rectRight);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Construct and execute TileMatcher.
 *//*-------------------------------------------------------------------*/

TileMatcher::TileMatcher(Array<ExternalPortal>& portals, Array<int>& heads, GeneratorTree& treeA, GeneratorTree& treeB, int face, int slotB)
: m_portals(portals),
  m_heads(heads),
  m_faceA(face),
  m_treeA(treeA.tree),
  m_treeB(treeB.tree),
  m_slotB(slotB)
{
    if (!m_treeA.getNumNodes() || !m_treeB.getNumNodes())
        return;

    // leaf mapping
    m_mappingA = (const UINT32*)map(treeA.map.m_array);
    m_mappingB = (const UINT32*)map(treeB.map.m_array);

    // mapping bit width
    m_widthA = treeA.mapWidth;
    m_widthB = treeB.mapWidth;

    // outside cell identifiers
    m_invalidA = (1 << m_widthA) - 1;
    m_invalidB = (1 << m_widthB) - 1;

    m_boundsA = treeA.aabb;

    int axis = getFaceAxis(m_faceA);
    int axisX = (axis + 1) % 3;
    int axisY = (axis + 2) % 3;

    // Generate mapping from (x, y, z) to face axes (x, y)
    m_faceAxes[axis]  = -1;
    m_faceAxes[axisX] = 0;
    m_faceAxes[axisY] = 1;

    Recti rectA = fixedPoint(treeA.aabb.getFaceRect(m_faceA),   m_boundsA, axisX, axisY);
    Recti rectB = fixedPoint(treeB.aabb.getFaceRect(m_faceA^1), m_boundsA, axisX, axisY);

    // With Umbra versions <= 3.3.4, all-solid faces can generate empty matching trees
    if (!m_widthA || !m_widthB)
        return;

    // execute
    matchTiles(0, rectA,
               0, rectB);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Construct and execute BorderMatcher.
 *//*-------------------------------------------------------------------*/

BorderMatcher::BorderMatcher(Array<ExternalPortal>& portals, Array<int>& heads, GeneratorTree& treeA, int face)
: m_portals(portals),
  m_heads(heads),
  m_faceA(face),
  m_treeA(treeA.tree)
{
    if (!m_treeA.getNumNodes())
        return;

    // leaf mapping and bitwidth
    m_mappingA  = (const UINT32*)map(treeA.map.m_array);
    m_widthA    = treeA.mapWidth;
    m_boundsA   = treeA.aabb;
    m_invalidA  = (1 << m_widthA) - 1;

    int axis = getFaceAxis(m_faceA);
    int axisX = (axis + 1) % 3;
    int axisY = (axis + 2) % 3;

    // Generate mapping from (x, y, z) to face axes (x, y)
    m_faceAxes[axis]  = -1;
    m_faceAxes[axisX] = 0;
    m_faceAxes[axisY] = 1;

    Recti rectA = fixedPoint(treeA.aabb.getFaceRect(m_faceA), m_boundsA, axisX, axisY);

    // With Umbra versions <= 3.3.4, all-solid faces can generate empty matching trees
    if (!m_widthA)
        return;

    // execute    
    matchBorder(0, rectA);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

GeneratorTree RuntimeTomeGenerator::getMatchingTree(int index, int face)
{
    if (isEmpty(index))
    {
        // Empty slot: use the default tree
        GeneratorTree tree;
        tree.tree     = m_defaultTree;
        tree.map      = BitDataArray(DataArray(((UINT32*)&m_defaultMap) - 1, DataPtr((UINT32)sizeof(UINT32)), sizeof(UINT32), -1), 0);
        tree.mapWidth = 1;
        tree.aabb     = getAABBByIndex(index);
        return tree;
    }

    const ImpTome* input = m_tomes[m_extTiles[mapIndexToSlot(index)].m_tomeIdx];

    // Non-empty: get tree from StreamingTile
    const LeafTileMatchData*  matchingData  = (const LeafTileMatchData*)map(input->getMatchingData());
    const SerializedTreeData* matchingTrees = (const SerializedTreeData*)map(input->getMatchingTrees());

    if (!matchingData || !m_tiles[index].m_borderMask)
    {
        m_missingMatchingData = true;
        return GeneratorTree();
    }

    // Requested face must be active in borderMask:
    // we're only matching faces between tomes
    UMBRA_ASSERT(m_tiles[index].m_borderMask & (1 << face));

    // Compute local leaf index
    KDTree toplevel(input->getTreeNodeCount(), (const UINT32*)map(input->getTreeData()), input->getTreeSplits());
    int leafIdx = toplevel.getLeafIdx(mapIndexToLocal(index));

    // Only faces with 1 in m_borderMask are expected to have tree entries in LeafTileMatchData,
    // compute (face number) - (number of preceding zeroes in m_borderMask) to figure out tree index
    Umbra::UINT32 bmask = m_tiles[index].m_borderMask;
    Umbra::UINT32 fmask = (1 << (face + 1)) - 1;
    int treeIdx = face - countOnes((~bmask) & fmask);
    const LeafTileMatchData&  leafMatchingData = matchingData[leafIdx];
    UMBRA_ASSERT(treeIdx < leafMatchingData.getMatchTreeCount());
    const SerializedTreeData& serialized = matchingTrees[leafMatchingData.getMatchTreeOfs() + treeIdx];

    GeneratorTree tree;
    tree.tree = KDTree(
        serialized.getNodeCount(),
        (const UINT32*)map(serialized.getTreeData(input)),
        DataArray((const float*)map(serialized.getSplitValues(input))-1, DataPtr(4), sizeof(float), serialized.m_numSplitValues));

    tree.map      = BitDataArray(serialized.getTreeMap(input), 0);
    tree.mapWidth = serialized.getMapWidth();
    tree.aabb     = AABB(m_tiles[index].m_tile->getTreeMin(), m_tiles[index].m_tile->getTreeMax());

    return tree;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

GeneratorTree RuntimeTomeGenerator::getInputTree(int index)
{
    if (isEmpty(index))
    {
        // Empty slot: use the default tree
        GeneratorTree tree;
        tree.tree     = m_defaultTree;
        tree.map      = BitDataArray(DataArray(((UINT32*)&m_defaultMap) - 1, DataPtr((UINT32)sizeof(UINT32)), sizeof(UINT32), -1), 0);
        tree.mapWidth = 1;
        tree.aabb     = getAABBByIndex(index);
        return tree;
    }

    // Non-empty: get tree from StreamingTile
    const ImpTome* input = m_tomes[index];
    SerializedTreeData serialized = input->getTileTree();

    GeneratorTree tree;

    tree.tree = KDTree(
        serialized.getNodeCount(),
        (const UINT32*)map(serialized.getTreeData(input)),
        DataArray((const float*)map(serialized.getSplitValues(input))-1, DataPtr(4), sizeof(float), serialized.m_numSplitValues));

    tree.map      = BitDataArray();
    tree.mapWidth = 0;
    tree.aabb     = AABB(input->getTreeMin(), input->getTreeMax());

    return tree;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Traverse the top level tree around the given input,
 *          finding neighbors and calling TileMatcher and BorderMatcher
 *          for them.
 *
 * \param   topLevel        The tree to traverse.
 * \param   nodeIdx         Current node index in tree.
 * \param   bounds          Bounds for current node.
 * \param   index           The input index which we're connecting.
 * \param   borderFaceMask  Mask of faces, that are borders (b111111
 *                          for root node)
 *//*-------------------------------------------------------------------*/

void RuntimeTomeGenerator::findNeighbors(Array<int>& outNeighbors, KDTree& topLevel, int nodeIdx, const AABB& bounds, int index)
{
    AABB tileBound = getAABBByIndex(index);

    // Must be a neighbor.
    if (!bounds.intersectsWithArea(tileBound))
        return;

    if (m_slotToIndex[nodeIdx] >= 0)
    {
        if (!isEmpty(m_slotToIndex[nodeIdx]) && !isEmpty(index) && m_extTiles[nodeIdx].m_tomeIdx == m_extTiles[mapIndexToSlot(index)].m_tomeIdx)
            return;
    }

    KDTree::Split split = topLevel.getSplit(nodeIdx);

    // Found a leaf, that's a neighbor to the given input
    if (split == KDTree::LEAF)
    {
        int slot   = nodeIdx;                      // slot number
        int nIndex = m_slotToIndex[slot];          // input index

        if (nIndex == index)
            return;

        if (!isEmpty(index) && !isEmpty(nIndex) && m_extTiles[slot].m_tomeIdx == m_extTiles[mapIndexToSlot(index)].m_tomeIdx)
            return;

        outNeighbors.pushBack(slot);

        return;
    }

    // Child indices
    int rightChild = topLevel.getRightChildIdx(nodeIdx);
    int leftChild  = rightChild - 1;

    float splitPos;
    if (topLevel.isNonMedianSplit(nodeIdx))
        splitPos = topLevel.getNonMedianSplit(nodeIdx);
    else
        splitPos = bounds.getCenter()[split];

    // Create AABBs for both subtrees.
    AABB    boundsLeft(bounds);
    AABB    boundsRight(bounds);
    boundsLeft.setMax (split, splitPos);
    boundsRight.setMin(split, splitPos);

    // Recurse into children.
    findNeighbors(outNeighbors, topLevel, leftChild,  boundsLeft,  index);
    findNeighbors(outNeighbors, topLevel, rightChild, boundsRight, index);
}

void RuntimeTomeGenerator::generatePortals(int index, Array<int>& neighbors, const Result* oldResult, int& oldFaceMask)
{
    oldFaceMask = 0;
    int slot = mapIndexToSlot(index);
    AABB tileBound = getAABBByIndex(index);

    int oldSlotIdx = -1;
    const ExtTile* oldExtTile = NULL;
    if (m_newOldTileMap.getSize())
    {
        UMBRA_ASSERT(oldResult);
        oldSlotIdx = m_newOldTileMap[slot];
        const ExtTile* oldExtTiles = (const ExtTile*)oldResult->m_extTiles.getAddr(oldResult->m_result);
        if (oldSlotIdx != -1 && !oldExtTiles[oldSlotIdx].hasExtCells())
            oldSlotIdx = -1;
        else
            oldExtTile = &oldExtTiles[oldSlotIdx];

        UMBRA_ASSERT(oldSlotIdx == -1 || isEmpty(index) || oldResult->m_result->getTile(oldSlotIdx, true) == m_tiles[index].m_tile);
    }

    // Generate exit portals for empties 
    // (all tomes have exit portals, extTile's m_exitPortalMask enables
    //  and disables them)
    if (isEmpty(index) && m_emptyTiles[getEmptyIndex(index)].m_exitPortalMask)
    {
        int oldMask = 0;
        if (oldExtTile)
            oldMask = (int)oldExtTile->getExitPortalMask();
        int mask = m_emptyTiles[getEmptyIndex(index)].m_exitPortalMask;
        // Faces both in new mask old old mask can be copied from old data
        oldFaceMask |= oldMask & mask;
        // Match only faces not in old exit portal mask
        mask &= ~oldMask;
        for (int face = 0; face < 6; face++)
        {
            // Connect to faces that are in exitPortalMask.
            if (mask & (1 << face))
            {
                GeneratorTree treeA = getMatchingTree(index, face);
                BorderMatcher matcher(m_extPortals, m_cellPortalHeads, treeA, face);
            }
        }
    }

    for (int i = 0; i < neighbors.getSize(); i++)
    {
        int nSlot  = neighbors[i];
        int nIndex = m_slotToIndex[nSlot];

        if (oldSlotIdx != -1 && m_newOldTileMap[nSlot] != -1)
        {
            AABB neighborBound = getAABBByIndex(nIndex);
            int face = getSharedFace(tileBound, neighborBound);
            if (face != -1)
                oldFaceMask |= (1 << face);                
            continue;
        }

        neighbors.removeSwap(i);
        i--;

        if (nSlot < slot)
        {
            m_prematchedFaces++;

            int numCells = 0;
            ExtCellNode* extCells = NULL;
            Portal* extPortals = NULL;
                
            if (isEmpty(nIndex))
            {
                numCells = UMBRA_EMPTY_SLOT_CELLS;
                extCells = m_emptyTiles[getEmptyIndex(nIndex)].m_extCells;
                extPortals = m_emptyTiles[getEmptyIndex(nIndex)].m_extPortals;
            } else
            {
                numCells = m_tiles[nIndex].m_tile->getNumCells();
                extCells = m_tiles[nIndex].m_extCells;
                extPortals = m_tiles[nIndex].m_extPortals;
            }

            if (!extCells || !extPortals)
                continue;

            AABB    aabbA  = getAABBByIndex(index);
            AABB    aabbB  = getAABBByIndex(nIndex);
            int     face   = getSharedFace(aabbA, aabbB);
            UINT32  z      = getFaceDirection(face) * 65535;

            int axisX = (getFaceAxis(face) + 1) % 3;
            int axisY = (axisX + 1) % 3;

            Recti faceOnB = fixedPoint(aabbA.getFaceRect(face), aabbB, axisX, axisY);
            Vector2i diff = faceOnB.getDimensions();
            
            SIMDRegister bias  = SIMDLoad(-(float)faceOnB.getMin().i, -(float)faceOnB.getMin().j, 0.f, 0.f);
            SIMDRegister scale = SIMDLoad(65535.f / diff.i, 65535.f / diff.j, 0.f, 0.f);
            bias = SIMDMultiply(bias, scale);
            
            for (int cell = 0; cell < numCells; cell++)
            {
                Portal* portals = (Portal*)&extPortals[extCells[cell].getPortalIndex()];
                
                for (int portal = 0; portal < extCells[cell].getPortalCount(); portal++)
                {
                    if (!portals[portal].hasTarget() || portals[portal].getTargetTileIdx() != slot)
                        continue;                    
                        
                    Portal& rPortal = portals[portal];
                    
                    SIMDRegister32 rectMn = SIMDLoad32(rPortal.xmn_xmx >> 16,    rPortal.ymn_ymx >> 16, 0, 0);
                    SIMDRegister32 rectMx = SIMDLoad32(rPortal.xmn_xmx & 0xffff, rPortal.ymn_ymx & 0xffff, 0, 0);

                    rectMn = SIMDFloatToInt(SIMDMultiplyAdd(SIMDIntToFloat(rectMn), scale, bias));
                    rectMx = SIMDFloatToInt(SIMDMultiplyAdd(SIMDIntToFloat(rectMx), scale, bias));
                    rectMn = SIMDMax32(rectMn, SIMDZero32());
                    rectMx = SIMDMin32(rectMx, s_simd65535);

                    UINT32 idx_z = (cell << 16) | z;                        
                    UINT32 link  = BUILD_PORTAL_LINK(face, 0, 0, 0, nSlot);

                    newPortal(m_extPortals, m_cellPortalHeads[rPortal.getTargetIndex()], 0,
                              link, idx_z, rectMn, rectMx);

                }
            }
        }
        else
        {
            AABB neighborBound = getAABBByIndex(nIndex);

            int face = getSharedFace(tileBound, neighborBound);

            // No face
            if (face == -1)
                continue;

            m_connectedFaces++;

            // Get the input's tree (A).
            GeneratorTree treeA = getMatchingTree(index, face);

            // Get the neighbor input's tree (B).
            GeneratorTree treeB = getMatchingTree(nIndex, face^1);

            // Match A and B.
            TileMatcher matcher(m_extPortals, m_cellPortalHeads,
                                treeA, treeB, face, nSlot);
        }
    }
}
