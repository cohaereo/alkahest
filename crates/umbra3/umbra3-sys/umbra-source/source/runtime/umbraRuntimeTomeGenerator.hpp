#pragma once

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

#ifndef UMBRARUNTIMETOMGENERATOR_HPP
#define UMBRARUNTIMETOMGENERATOR_HPP

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "runtime/umbraTome.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraRect.hpp"
#include "umbraHash.hpp"
#include "umbraArray.hpp"

namespace Umbra
{

const Umbra::UINT32 g_faceMaskFull = 63;

class RuntimeStructBuilder
{
public:
    enum
    {
        DefaultSlackSize = 64 * 1024
    };

    RuntimeStructBuilder();
    ~RuntimeStructBuilder();

    TomeCollection::ErrorCode   init                (Allocator* a, UINT32 flags);

    template <typename T>
    inline T&                   beginStruct         (DataPtr& ptr);
    inline UINT32               endStruct           (void);
    inline void                 cancel              (void);

    inline bool                 reserveHeap         (size_t size, size_t slack = DefaultSlackSize);
    inline void                 finishHeap          (void);

    inline UINT8*               allocOutput(DataPtr& ptr, size_t size);

    void*                       finalize            (Allocator* a);
    bool                        reserveOutput       (Allocator* a, size_t maxSize);

private:

    struct HeapBlock
    {
        UINT8*          mem;
        UINT8*          cur;
        Umbra::UINT32   size;

        Umbra::UINT32   used      (void) { return (UINT32)(cur  - mem); }
        Umbra::UINT32   available (void) { return size - used(); }
    };

    struct MemEntry
    {
        Umbra::UINT32   base;
        Umbra::UINT32   cur;

        DataPtr         offset    (void) { return DataPtr((UINT32)(cur - base)); }
        Umbra::UINT32   used      (void) { return cur  - base; }
    };

    Umbra::UINT8*           finalize            (UINT8** dst, Allocator* a);
    void                    clean               (void);

    Allocator*              getAllocator        (void) { return m_allocator; }
    Allocator*              m_allocator;

    MemEntry                m_stack[16];
    int                     m_stackPos;
    bool                    m_inHeap;
    bool                    m_reservedOutput;

    Array<HeapBlock>        m_blocks;
};

template<class Allocator>
class RuntimeSpatialSubdivision
{
public:
    enum NodeType
    {
        KD_SPLIT,
        MEDIAN_SPLIT,
        LEAF
    };

    RuntimeSpatialSubdivision(Allocator* a) : m_allocator(a), m_maxAssigned(-1) {}

    int      getRoot           ()        const { return 0; }
    NodeType getType           (int idx) const { return (NodeType)(m_nodes[idx].data & 3); }
    bool     isKDSplit         (int idx) const { return getType(idx) == KD_SPLIT; }
    bool     isMedianSplit     (int idx) const { return getType(idx) == MEDIAN_SPLIT; }
    bool     isLeaf            (int idx) const { return getType(idx) == LEAF; }
    int      getKDSplitAxis    (int idx) const { UMBRA_ASSERT(isKDSplit(idx)); return m_kdSplits[m_nodes[idx].data >> 2].axis; }
    float    getKDSplitPos     (int idx) const { UMBRA_ASSERT(isKDSplit(idx)); return m_kdSplits[m_nodes[idx].data >> 2].splitPos; }
    int      getMedianSplitAxis(int idx) const { UMBRA_ASSERT(isMedianSplit(idx)); return (m_nodes[idx].data >> 2) & 3; }
    int      getLeafValue      (int idx) const { UMBRA_ASSERT(isLeaf(idx)); return m_nodes[idx].data >> 2; }
    int      getInnerValue     (int idx) const { return m_nodes[idx].value; }
    int      getKDSplitLeftChild (int idx) const { UMBRA_ASSERT(isKDSplit(idx)); return idx+1; }
    int      getKDSplitRightChild(int idx) const { UMBRA_ASSERT(isKDSplit(idx)); return idx+1+getKDSplitLeftSize(idx); }
    int      getMedianSplitLeftChild (int idx) const { UMBRA_ASSERT(isMedianSplit(idx)); return idx+1; }
    int      getMedianSplitRightChild(int idx) const { UMBRA_ASSERT(isMedianSplit(idx)); return idx+1+getMedianSplitLeftSize(idx); }
    int      getSize (void) const { return getSize(0); }

    // NOTE: strict usage pattern

    struct Constructor
    {
    private:
        Constructor(RuntimeSpatialSubdivision& sss, Constructor* p, int idx) : sss(&sss), parent(p), idx(idx)
        {
            if (sss.m_nodes.getSize() < idx + 1)
            {
                int size = sss.m_nodes.getSize();
                if (!sss.m_nodes.resize(idx + 1))
                {
                    this->sss = NULL;
                    return;
                }
                //sss.m_nodes.resize(max2(idx + 1, sss.m_nodes.getSize() + max2(16, sss.m_nodes.getSize()/2)));
                memset(sss.m_nodes.getPtr() + size, 0, (sss.m_nodes.getSize() - size) * sizeof(UINT32));
            }
        }

        Constructor(const Constructor&);

        Constructor& operator=(const Constructor& c)
        {
            //UMBRA_ASSERT(!sss);
            sss = c.sss;
            parent = c.parent;
            idx = c.idx;
            return *this;
        }

    public:
        Constructor() : sss(0), parent(0), idx(0)
        {
        }

        ~Constructor()
        {
        }

        bool isOk() { return sss != NULL; }

        void finish()
        {
            if (parent && idx == parent->idx+1)
            {
                int s = sss->getSize(idx);

                UMBRA_ASSERT(s > 0);

                if (sss->getType(parent->idx) == MEDIAN_SPLIT)
                {
                    UMBRA_ASSERT(sss->getMedianSplitLeftSize(parent->idx) == 0);
                    sss->m_nodes[parent->idx].data &= 0xf;
                    sss->m_nodes[parent->idx].data |= s << 4;
                }
                else if (sss->getType(parent->idx) == KD_SPLIT)
                {
                    UMBRA_ASSERT(sss->getKDSplitLeftSize(parent->idx) == 0);
                    sss->m_kdSplits[sss->m_nodes[parent->idx].data >> 2].leftSize = s;
                }
                else
                    UMBRA_ASSERT(0);
            }
        }

        void medianSplit(int axis)
        {
            UMBRA_ASSERT(axis >= 0 && axis <= 2);
            sss->m_nodes[idx].data = MEDIAN_SPLIT | (axis << 2);
            sss->m_maxAssigned = max2(idx, sss->m_maxAssigned);
        }

        bool kdSplit(int axis, float p)
        {
            UMBRA_ASSERT(axis >= 0 && axis <= 2);
            KDSplit kds;
            kds.axis = axis;
            kds.leftSize = 0;
            kds.splitPos = p;
            int kdIdx;
            if (idx <= sss->m_maxAssigned && sss->isKDSplit(idx))
            {
                kdIdx = sss->m_nodes[idx].data >> 2;
                sss->m_kdSplits[kdIdx] = kds;
            } else
            {
                if (!sss->m_kdSplits.pushBack(kds))
                    return false;
                kdIdx = sss->m_kdSplits.getSize() - 1;
            }

            sss->m_nodes[idx].data = KD_SPLIT | (kdIdx << 2);
            sss->m_maxAssigned = max2(idx, sss->m_maxAssigned);
            return true;
        }

        void innerValue(int value)
        {
            sss->m_nodes[idx].value = value;
        }

        void leaf(int value)
        {
            UMBRA_ASSERT(value >= 0);
            sss->m_nodes[idx].data = LEAF | (value << 2);
            sss->m_maxAssigned = max2(idx, sss->m_maxAssigned);
        }

        void constructLeft(Constructor& c)
        {
            UMBRA_ASSERT(sss->getType(idx) == MEDIAN_SPLIT || sss->getType(idx) == KD_SPLIT);
            UMBRA_ASSERT(!sss->isMedianSplit(idx) || sss->getMedianSplitLeftSize(idx) == 0);
            UMBRA_ASSERT(!sss->isKDSplit(idx) || sss->getKDSplitLeftSize(idx) == 0);
            c = Constructor(*sss, this, idx + 1);
        }

        void constructRight(Constructor& c)
        {
            UMBRA_ASSERT(sss->getType(idx) == MEDIAN_SPLIT || sss->getType(idx) == KD_SPLIT);

            if (sss->isMedianSplit(idx))
                c = Constructor(*sss, this, idx + 1 + sss->getMedianSplitLeftSize(idx));
            else
                c = Constructor(*sss, this, idx + 1 + sss->getKDSplitLeftSize(idx));
        }

        int getParentAxis()
        {
            if (!parent)
                return 0;
            int parentIdx = parent->idx;
            if (sss->isMedianSplit(parentIdx))
                return sss->getMedianSplitAxis(parentIdx);
            if (sss->isKDSplit(parentIdx))
                return sss->getKDSplitAxis(parentIdx);
            return 0;
        }

        RuntimeSpatialSubdivision* sss;
        Constructor*              parent;
        int                       idx;
        int                       parentAxis;

        friend class RuntimeSpatialSubdivision;
    };

    void construct(Constructor& c, int maxNodes, Allocator* a)
    {
        UMBRA_ASSERT(m_nodes.getSize() == 0);
        UMBRA_ASSERT(m_kdSplits.getSize() == 0);
        m_maxAssigned = -1;
        m_nodes.setAllocator(a);
        m_nodes.reset(maxNodes);
        memset(m_nodes.getPtr(), 0, maxNodes * sizeof(Node));
        m_kdSplits.setAllocator(a);
        m_kdSplits.reset(maxNodes);
        c = Constructor(*this, 0, 0);
    }

    void getSerializedSize (size_t& dataSize, size_t& data2Size)
    {
        int numNodes = 0;
        int kdSplits = 0;
        getSize(0, numNodes, kdSplits);
        dataSize      = KDTree::getDataDwords(numNodes) * sizeof(UINT32);
        data2Size     = numNodes * sizeof(float); //KDTree::getKDDataDwords(numNodes, kdSplits) * sizeof(UINT32);
    }

    bool serialize(RuntimeStructBuilder& builder, SerializedTreeData& tree, Array<int>* leafMapping, UINT32** outTreeData = NULL, float** outKdData = NULL, const AABB& aabb = AABB())
    {
        size_t dataSize, data2Size;
        getSerializedSize(dataSize, data2Size);

        if (!builder.reserveHeap(dataSize + data2Size + 16*2))
            return false;

        UINT32* treeData = (UINT32*)builder.allocOutput(tree.m_treeData,    dataSize);
        float*  KdData   = (float*)builder.allocOutput (tree.m_splitValues, data2Size);

        builder.finishHeap();

        if (outTreeData)
            *outTreeData = treeData;
        if (outKdData)
            *outKdData = KdData;

        int nodeCount = 0;
        if (!serialize(nodeCount, treeData, KdData, leafMapping, aabb))
            return false;
        tree.setNodeCount(nodeCount);
        tree.m_numSplitValues = nodeCount;

        return true;
    }

    bool serialize(int& treeNodes, UINT32* treeData, float* splits, Array<int>* leafMapping, const AABB& aabb)
    {
        int idx = 0;

        int numNodes = 0;
        int kdSplits = 0;
        getSize(0, numNodes, kdSplits);

//        nodeStateSize = 0;
        //RuntimeArray<UINT32, Allocator>   state (UMBRA_BITVECTOR_DWORDS(numNodes), getAllocator());
        //RuntimeArray<float, Allocator>    splits(numNodes, getAllocator());
        FIFOQueue<FifoEntry>  fifo  (getAllocator(), (numNodes + 1) / 2);

        if (!fifo.isOk())
            return false;

        fifo.pushBack(FifoEntry(getRoot(), aabb));
        //state.setSize(state.getCapacity());
        //splits.setSize(0);

        while (fifo.getSize())
        {
            FifoEntry entry = fifo.popFront();
            int node = entry.idx;

            if (isLeaf(node))
            {
                if (leafMapping)
                    leafMapping->pushBack(getInnerValue(node));

                //if (leafMapping)
                    //leafMapping[leaves] = getLeafValue(node);
                //leaves++;

                set2BitValue(treeData, idx*2, 3);

                *(splits++) = 0.f;
                //UMBRA_ASSERT(idx < state.getSize() * 32);
                //clearBit(state.getPtr(), idx);
            } else if (isMedianSplit(node))
            {
                if (leafMapping)
                    //leafMapping->pushBack(-1);
                    leafMapping->pushBack(getInnerValue(node));

                int axis = getMedianSplitAxis(node);
                set2BitValue(treeData, idx*2, axis);

                int left  = getMedianSplitLeftChild(node);
                int right = getMedianSplitRightChild(node);

                AABB leftAABB  = entry.aabb;
                AABB rightAABB = entry.aabb;
                leftAABB.setMax(axis, entry.aabb.getCenter()[axis]);
                rightAABB.setMin(axis, entry.aabb.getCenter()[axis]);

                fifo.pushBack(FifoEntry(left, leftAABB));
                fifo.pushBack(FifoEntry(right, rightAABB));

                *(splits++) = entry.aabb.getCenter()[axis];
                //UMBRA_ASSERT(idx < state.getSize() * 32);
                //clearBit(state.getPtr(), idx);
            } else if (isKDSplit(node))
            {
                if (leafMapping)
                    //leafMapping->pushBack(-1);
                    leafMapping->pushBack(getInnerValue(node));

                int axis = getKDSplitAxis(node);
                set2BitValue(treeData, idx*2, axis);

                int left  = getKDSplitLeftChild(node);
                int right = getKDSplitRightChild(node);

                AABB leftAABB  = entry.aabb;
                AABB rightAABB = entry.aabb;
                leftAABB.setMax(axis,  getKDSplitPos(node));
                rightAABB.setMin(axis, getKDSplitPos(node));

                fifo.pushBack(FifoEntry(left, leftAABB));
                fifo.pushBack(FifoEntry(right, rightAABB));

                *(splits++) = getKDSplitPos(node);
                //UMBRA_ASSERT(idx < state.getSize() * 32);
                //setBit(state.getPtr(), idx);
                //splits.pushBackNoGrow(getKDSplitPos(node));
                //nodeStateSize = idx + 1;
            }

            idx++;
        }

        //UINT32* nodeState = data2;
        //UINT32* lut       = data2 + UMBRA_BITVECTOR_DWORDS(nodeStateSize);
        //float*  splits2   = (float*)(lut + (KDTree::getKDLUTSize(nodeStateSize) >> 2));

        //memcpy(nodeState, state.getPtr(), UMBRA_BITVECTOR_DWORDS(nodeStateSize) * sizeof(UINT32));
        //KDTree::buildKDLut(lut, nodeState, nodeStateSize);
        //memcpy(splits2, splits.getPtr(), sizeof(float) * splits.getSize());

        treeNodes = idx;
        KDTree::buildLut(treeData + UMBRA_BITVECTOR_DWORDS(numNodes * 2), treeData, numNodes);

        return true;
    }

private:

    struct FifoEntry
    {
        FifoEntry() {}
        FifoEntry(int idx, const AABB& aabb) : idx(idx), aabb(aabb) {}
        int     idx;
        AABB    aabb;
    };

    int getMedianSplitLeftSize(int idx) const { UMBRA_ASSERT(isMedianSplit(idx)); return (int)(m_nodes[idx].data >> 4); }
    int getKDSplitLeftSize(int idx) const { UMBRA_ASSERT(isKDSplit(idx)); return (int)(m_kdSplits[m_nodes[idx].data >> 2].leftSize); }

    void getSize(int idx, int& size, int& kdSplits) const
    {
        switch (getType(idx))
        {
        case LEAF:
            size++;
            return;
        case MEDIAN_SPLIT:
            size += 1;
            getSize(idx + 1, size, kdSplits);
            getSize(idx + 1 + getMedianSplitLeftSize(idx), size, kdSplits);
            return;
        case KD_SPLIT:
            kdSplits++;
            size += 1;
            getSize(idx + 1, size, kdSplits);
            getSize(idx + 1 + getKDSplitLeftSize(idx), size, kdSplits);
            return;
        }
        UMBRA_ASSERT(0);
        return;
    }

    int getSize(int idx) const
    {
        switch (getType(idx))
        {
        case LEAF:         return 1;
        case MEDIAN_SPLIT: return 1 + getMedianSplitLeftSize(idx) + getSize(idx + 1 + getMedianSplitLeftSize(idx));
        case KD_SPLIT:     return 1 + getKDSplitLeftSize(idx) + getSize(idx + 1 + getKDSplitLeftSize(idx));
        }
        UMBRA_ASSERT(0);
        return 0;
    }

    struct KDSplit
    {
        UINT32 axis     : 2;
        UINT32 leftSize : 30;
        float  splitPos;
    };

    struct Node
    {
        UINT32 data;
        int    value;
    };

    Allocator*          getAllocator() { return m_allocator; }
    Allocator*          m_allocator;
    Array<Node>         m_nodes;
    Array<KDSplit>      m_kdSplits;
    int                 m_maxAssigned;

    friend struct Constructor;
};

class RuntimeTomeGenerator
{
public:

    struct Result
    {
        Result();
        ~Result() { clear(); }

        Result& operator=(const Result& other)
        {
            m_allocator   = other.m_allocator;
            m_result      = other.m_result;
            m_extTiles    = other.m_extTiles;
            m_contexts    = other.m_contexts;
            m_numContexts = other.m_numContexts;
            return *this;
        }

        Allocator*      m_allocator;
        const ImpTome*  m_result;
        DataPtr         m_extTiles;
        DataPtr         m_contexts;
        int             m_numContexts;

        void            clear(bool freeResult = false);
    };

    RuntimeTomeGenerator (Allocator* a, Allocator* resultAlloc, UINT32 flags, const ImpTome** inTomes, int numInTomes, const AABB& aabb);
    ~RuntimeTomeGenerator (void);

    static size_t               estimateSize        (void);
    TomeCollection::ErrorCode   buildTome           (Result& result, const Result* oldResult, size_t fixedOutputSize = 0);

    struct ExternalPortal
    {
        UINT32  link;
        UINT32  idx_z;
        Recti   rect;
        int     next;   // linked list
    };

    struct GeneratorTree
    {
        KDTree          tree;
        BitDataArray    map;
        int             mapWidth;
        AABB            aabb;
    };

private:

    struct Estimate
    {
        int numTargets;
        int numMaxCells;
        int numGates;
        int numMaxClusters;
    };

    struct GeneratorTile
    {
        GeneratorTile() : m_tile(NULL), 
                          m_slot(0), 
                          m_local(0),
                          m_parentTile(-1), 
                          m_borderMask(g_faceMaskFull),
                          m_exitPortalMask(g_faceMaskFull),
                          m_extCells(NULL), 
                          m_extPortals(NULL)
                          {}

        const ImpTile*          m_tile;         // Input tile
        int                     m_slot;         // Output Slot number
        int                     m_local;        // Index local to input Tome
        int                     m_parentTile;   // Parent tile input idx
        int                     m_borderMask;   // Faces that are against a tome border
        int                     m_exitPortalMask;

        ExtCellNode*            m_extCells;
        Portal*                 m_extPortals;
    };

    struct EmptyTile
    {
        EmptyTile() : m_slot(-1), 
                      m_exitPortalMask(g_faceMaskFull),
                      m_extCells(NULL),
                      m_extPortals(NULL)
        {}

        AABB            m_aabb;
        int             m_slot;
        int             m_exitPortalMask;

        ExtCellNode*    m_extCells;
        Portal*         m_extPortals;
    };

    typedef Hash<UINT32, int>               ObjectHash;
    typedef Hash<UINT32, int>               GateHash;
    typedef Array<GeneratorTile>            TileArray;

    static size_t                           estimateTomeSize            (const ImpTome** inputs, int numInputs, const TileArray&);
    static void                             estimateTome                (const ImpTome** inputs, int numInputs, const TileArray&, Estimate& estimate);

    bool                                    collectTiles                (void);

    // empty tiles
    ImpTile*                                generateEmptyTile           (DataPtr& ptr, const AABB& bounds, int slot);

    // header
    bool                                    generateHeader              (ImpTome& tome, Result& result, const Result* oldResult);
    bool                                    outputObjects               (ImpTome& tome, ObjectHash& objectHash);

    // tile tree generation
    bool                                    makeTopLevel                (RuntimeSpatialSubdivision<Allocator>& topLevel);
    bool                                    serializeTopLevel           (RuntimeSpatialSubdivision<Allocator>& topLevel, ImpTome& global);
    bool                                    constructTopLevel           (RuntimeSpatialSubdivision<Allocator>::Constructor&, const AABB&, int*, int, Umbra::UINT32);
    bool                                    selectSplit                 (int*, int, const AABB&, int axis, float& splitPos, bool& isMedian);
    bool                                    constructSubtrees           (int*, int, RuntimeSpatialSubdivision<Allocator>::Constructor&, const AABB&, int, float, bool, Umbra::UINT32);
    bool                                    constructInputTree          (RuntimeSpatialSubdivision<Allocator>::Constructor& c, int inputIndex, KDTree& tree, int nodeIndex, const AABB&, int parent, Umbra::UINT32 borderMasks, Umbra::UINT32 exitPortalMask);
    inline bool                             isGoodSplit                 (int axis, float position, int* nodes, int n, int& left);
    inline int                              splitPartition              (int axis, float position, int* nodes, int n);
    void                                    makeOldNewMap               (const ImpTome* oldTome, KDTree& oldTree, int nodeIdx, int* indices, int N, const AABB& aabb);

    // tile
    inline GeneratorTree                    getMatchingTree             (int index, int face);
    inline GeneratorTree                    getInputTree                (int index);

    // hierarchy
    void                                    copyLeafPortals             (int innerIndex, int currentSlot, int level, Array<UINT32>& bitmap, int& bitmapFaces);
    void                                    combineHierarchyPortals     (int index);
    void                                    makeNeighborBitvector       (int index, const Array<int>& neighbors, bool doHierarchy, Array<UINT32>& bitmap, const Result* oldResult);
    bool                                    outputExternalPortals       (ExtTile& tile, int numCells, int index, const Result* result, const Array<UINT32>& bitmap, int bitmapFaces);

    // clusters
    void                                    combineClusterPortals       (int input, int leafIndex);
    bool                                    outputClusterPortals        (int input);

    // leaf portals
    void                                    findNeighbors               (Array<int>& outNeighbors, KDTree& topLevel, int nodeIdx, const AABB& bounds, int srcIndex);
    void                                    generatePortals             (int index, Array<int>& neighbors, const Result* oldResult, int& bitmapFaces);
    void                                    matchTiles                  (int faceA,
                                                                         KDTree& treeA, int idxA, const AABB& aabbA, BitDataArray& mappingA, int widthA,
                                                                         KDTree& treeB, int idxB, const AABB& aabbB, BitDataArray& mappingB, int widthB, const AABB& boundsA, int slotB);

    // helpers
    bool                                    isEmpty                     (int inputIndex) { return inputIndex >= m_tiles.getSize(); }
    int                                     getEmptyIndex               (int inputIndex) { return inputIndex - m_tiles.getSize(); }
    AABB                                    getAABBByIndex              (int inputIndex) { return isEmpty(inputIndex) ? m_emptyTiles[getEmptyIndex(inputIndex)].m_aabb : m_tiles[inputIndex].m_tile->getAABB(); }
    int                                     mapIndexToLocal             (int inputIndex)  { return m_tiles[inputIndex].m_local; }
    int                                     mapIndexToSlot              (int inputIndex)  { return isEmpty(inputIndex) ? m_emptyTiles[getEmptyIndex(inputIndex)].m_slot : m_tiles[inputIndex].m_slot; }
    int                                     getParentTile               (int inputIndex)  { return m_tiles[inputIndex].m_parentTile; }
    int                                     getNumTotalTiles            (void)           { return m_tiles.getSize() + m_emptyTiles.getSize(); }
    inline int                              getCluster                  (int index, int cell);
    inline int                              getParentCell               (int index, int cell, int ancestorIdx);

    void                                    setError                    (TomeCollection::ErrorCode errorCode) { m_errorCode = errorCode; }
    TomeCollection::ErrorCode               getError                    (void) { return m_errorCode; }

    Allocator*                              getAllocator                (void) { return m_mainAllocator; }
    Allocator*                              m_mainAllocator;
    Allocator*                              m_resultAllocator;

    Estimate                                m_estimate;
    KDTree                                  m_topLevelTree;

    UINT32                                  m_flags;

    const ImpTome**                         m_tomes;
    int                                     m_numTomes;
    TileArray                               m_tiles;        // Input tiles

    AABB                                    m_minBounds;
    AABB                                    m_bounds;

    Array<int>                              m_leafStarts;
    Array<int>                              m_slotToIndex;
    Array<EmptyTile>                        m_emptyTiles;
    Array<DataPtr>                          m_emptySlotOfs;
    int                                     m_numSlots;
    bool                                    m_missingMatchingData;

    static const int                        m_defaultTreeDwords = 1;
    static const UINT32                     m_defaultTreeData[m_defaultTreeDwords];
    static const UINT32                     m_defaultMap[2];
    KDTree                                  m_defaultTree;

    Array<int>                              m_oldNewTileMap;
    Array<int>                              m_newOldTileMap;

    int                                     m_connectedFaces;
    int                                     m_prematchedFaces;

    Array<ExternalPortal>                   m_extPortals;
    Array<int>                              m_cellPortalHeads;

    RuntimeStructBuilder                    m_builder;

    TomeContext*                            m_tomeContexts;
    ExtTile*                                m_extTiles;

    TomeCollection::ErrorCode               m_errorCode;
};

}

#endif
