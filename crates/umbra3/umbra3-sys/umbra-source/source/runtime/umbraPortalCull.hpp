#pragma once
#ifndef __UMBRAPORTALCULL_H
#define __UMBRAPORTALCULL_H

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra runtime portal culling
 *
 */

#include "umbraQueryContext.hpp"
#include "umbraTileTraverse.hpp"
#include "umbraTransformer.hpp"
#include "umbraPortalRaster.hpp"
#include "umbraDepthBuffer.hpp"
#include "umbraPortalTraversal.hpp"
#include "umbraAABB.hpp"
#include "umbraStaticHeap.hpp"

#define NUM_CELLS_DEFAULT       1024
#define NUM_TREE_NODES_DEFAULT  4096

#if UMBRA_ARCH == UMBRA_SPU
#   define OBJ_BOUNDS_BATCH    16
#   define NUM_OBJ_BANKS       2
#else
#   define OBJ_BOUNDS_BATCH    32
#   define NUM_OBJ_BANKS       1
#endif

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

template<typename RefType> struct ListNode
{
    enum
    {
        EMPTY = (RefType)0,
        FREE = (RefType)-1
    };

    RefType next;
};

template<typename RefType> struct ListNodeDouble : public ListNode<RefType>
{
    RefType prev;
};

template<typename T, typename RefType> class LinkedList
{
public:
    typedef ListNode<RefType> Node;

    UMBRA_FORCE_INLINE LinkedList(T* arr, Node* head): m_arr(arr), m_head(head) {}

    UMBRA_FORCE_INLINE bool isEmpty (void) const
    {
        return m_head->next == Node::EMPTY;
    }

    UMBRA_FORCE_INLINE void clear (void)
    {
        m_head->next = Node::EMPTY;
    }

    void populate (RefType first, RefType last)
    {
        m_head->next = first;
        for (RefType i = first; i < last; i++)
            m_arr[i].next = i + 1;
        m_arr[last - 1].next = Node::EMPTY;
    }

    int count (void) const
    {
        RefType c = first();
        int n = 0;
        while (c != Node::EMPTY)
        {
            c = next(c);
            n++;
        }
        return n;
    }

    void remove (RefType ref)
    {
        RefType* ptr = &m_head->next;
        while (*ptr != ref)
        {
            ptr = &getNode(*ptr).next;
        }
        *ptr = getNode(ref).next;
        UMBRA_DEBUG_CODE(getNode(ref).next = Node::FREE);
    }

    UMBRA_FORCE_INLINE void insert (RefType ref)
    {
        Node& n = getNode(ref);
        UMBRA_ASSERT(n.next == Node::FREE);
        n.next = m_head->next;
        m_head->next = ref;
    }

    UMBRA_FORCE_INLINE RefType removeFirst (void)
    {
        RefType ret = first();
        if (ret != Node::EMPTY)
        {
            Node& n = getNode(ret);
            m_head->next = n.next;
            UMBRA_DEBUG_CODE(n.next = Node::FREE);
        }
        return ret;
    }

    UMBRA_FORCE_INLINE RefType first (void) const
    {
        return m_head->next;
    }

    UMBRA_FORCE_INLINE RefType next (RefType elem) const
    {
        Node& n = getNode(elem);
        return n.next;
    }

private:
    UMBRA_FORCE_INLINE Node& getNode (RefType ref) const
    {
        return m_arr[ref];
    }

    T* m_arr;
    Node* m_head;
};

template<typename T, typename RefType> class DoublyLinkedList
{
public:
    typedef ListNodeDouble<RefType> Node;

    UMBRA_FORCE_INLINE DoublyLinkedList(T* arr)
        : m_arr(arr), m_head(&m_arr[Node::EMPTY])
    {
    }

    UMBRA_FORCE_INLINE bool isEmpty (void) const
    {
        return m_head->next == Node::EMPTY;
    }

    UMBRA_FORCE_INLINE void clear (void)
    {
        m_head->next = m_head->prev = Node::EMPTY;
    }

    UMBRA_FORCE_INLINE void insertFirst (RefType elem)
    {
        insertAfter(elem, Node::EMPTY);
    }

    UMBRA_FORCE_INLINE void insertLast (RefType elem)
    {
        insertBefore(elem, Node::EMPTY);
    }

    UMBRA_FORCE_INLINE void insertAfter (RefType elem, RefType after)
    {
        Node& n = getNode(elem);
        Node& other = getNode(after);
        UMBRA_ASSERT(n.next == Node::FREE);
        n.next = other.next;
        n.prev = after;
        getNode(other.next).prev = elem;
        other.next = elem;
    }

    UMBRA_FORCE_INLINE void insertBefore (RefType elem, RefType before)
    {
        Node& n = getNode(elem);
        Node& other = getNode(before);
        UMBRA_ASSERT(n.next == Node::FREE);
        n.prev = other.prev;
        n.next = before;
        getNode(other.prev).next = elem;
        other.prev = elem;
    }

    UMBRA_FORCE_INLINE RefType first (void) const
    {
        return m_head->next;
    }

    UMBRA_FORCE_INLINE RefType next (RefType elem) const
    {
        Node& n = getNode(elem);
        return n.next;
    }

    UMBRA_FORCE_INLINE void remove (RefType elem)
    {
        Node& n = getNode(elem);
        UMBRA_ASSERT(n.next != Node::FREE);
        m_arr[n.next].prev = n.prev;
        m_arr[n.prev].next = n.next;
        UMBRA_DEBUG_CODE(n.next = Node::FREE);
    }

    UMBRA_FORCE_INLINE RefType removeFirst (void)
    {
        RefType ret = first();
        if (ret != Node::EMPTY)
            remove(ret);
        return ret;
    }

private:
    UMBRA_FORCE_INLINE Node& getNode (RefType elem) const
    {
        return m_arr[elem];
    }

    T* m_arr;
    Node* m_head;
};

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

class PortalCuller
{
public:
                        PortalCuller   (QueryContext* q, Transformer* camera, float clusterThreshold, const ImpObjectDistanceParams* objDist,
                            int maxCells = NUM_CELLS_DEFAULT, int maxTreeNodes = NUM_TREE_NODES_DEFAULT);
                        ~PortalCuller  (void);

    Query::ErrorCode    execute         (VisibilityResult& res, bool useDepthMaps, bool ignoreCameraPos, const AABB& initialAABB, int objectIdx = -1);

private:

    enum CellState
    {
        CellState_Free = 0,             // cell was just initialized
        CellState_Inactive,             // cell is pending to be finalized, or reactivated
        CellState_Queued,               // cell is pending to be expanded
        CellState_Backtrace,            // cell is in back trace queue
        CellState_Last,
        CellState_Force32 = 0x7FFFFFFF
    };
    // cell state is 2 bits, for current slot state vector
    UMBRA_CT_ASSERT(CellState_Last <= 4);

    class CellData : public ListNodeDouble<UINT16>
    {
    private:
        UINT16              id;
        UINT16              reserved;
        BlockRasterBuffer   buffer;

    public:

        int                 getId       (void) const { return id; }
        void                setId       (int i) { id = (UINT16)i; }
        BlockRasterBuffer&  getBuf      (void) { return buffer; }
        const BlockRasterBuffer& getBuf (void) const { return buffer; }
    };

    typedef UINT16 Cell;
    typedef LinkedList<CellData, Cell> CellList;
    typedef DoublyLinkedList<CellData, Cell> CellListLocal;

    struct Tile
    {
        Tile(): handle(0), tileIndex(-1) {}
        TileTraverseTree::TileHandle handle;
        int tileIndex;
    };

    struct ObjectBank
    {
        ArrayIndexedIterator<ObjectBounds, OBJ_BOUNDS_BATCH> bounds;
        ArrayIndexedIterator<ObjectDistance, OBJ_BOUNDS_BATCH> distances;
        UINT32 indices[OBJ_BOUNDS_BATCH];
        UINT32 localIndices[OBJ_BOUNDS_BATCH];
        int size;
        int origSize;
    };

    CellListLocal getLocalCellQueue         (void) { return CellListLocal(m_cells); }
    CellList getCellInactiveQueue           (void) { return CellList(m_cells, &m_cellInactiveList); }
    CellList getCellFreeList                (void) { return CellList(m_cells, &m_cellFreeList); }
    CellList getTileCellQueue               (const Tile& b) { return CellList(m_cells, (ListNode<UINT16>*)m_tiles.getUserData(b.handle)); }

    void                traverse            (void);
    bool                init                (bool ignoreCameraPos, bool useDepthMaps, const AABB& aabb, int objectIdx);
    void                addStartCell        (const Tile& tile, int cellIdx);
    Cell                nextCellToProcess   (CellListLocal& queue);
    void                enterTile           (const Tile& tile);
    void                leaveTile           (void);
    bool                enterPortal         (const Portal& portal) const;
    void                visualizePortal     (const Portal& portal, bool, bool) const;

    Cell                findRemoteCell      (Tile& bucket, int slot, int id, CellState& state);
    Cell                findLocalCell       (int id, CellState& state);
    Cell                getFreeCell         (int id);
    Cell                freeOneCell         (void);
    void                finalizeCell        (Cell cell);
    void                freeCellBuffer      (Cell cell);

    void                resetLocalCells     (void) { memset(m_localCellMap, 0, sizeof(m_localCellMap)); }
    Cell                getLocalCell        (int idx) const;
    CellState           getLocalCellState   (int idx) const;
    void                setLocalCellState   (Cell cell, CellState state);

    float               getCellFarDeviceZ   (const CellNode& cell);
    bool                isBackfacing        (const Portal& portal) const;
    CellData&           getCellData         (Cell cell) { UMBRA_ASSERT(cell > 0 && cell < m_maxCells); return m_cells[cell]; }

    // globals

    QueryContext*               m_query;
    Transformer*                m_transformer;
    VisibilityResult*           m_result;
    ObjectBank                  m_objBanks[NUM_OBJ_BANKS];
    BufferAllocator*            m_bufferAllocator;
    float                       m_accurateDistance;
    UINT32                      m_nearSigns[3];
    SIMDRegister                m_lodDistanceScaleSqr;
    SIMDRegister                m_lodRef;
    BlockRasterBuffer           m_fullyVisible;
    DepthBuffer                 m_depthBuffer;
    DepthBuffer                 m_inputDepth;


    // traverse state

    TileTraverseTree            m_tiles;
    int                         m_maxCells;
    CellData*                   m_cells;
    ListNode<UINT16>            m_cellFreeList;
    ListNode<UINT16>            m_cellInactiveList;

    // per slot

    SIMDRegister                m_tileOffset;
    SIMDRegister                m_tileScale;
    Vector4                     UMBRA_ATTRIBUTE_ALIGNED16(m_portalExpand);
    AxisNormals                 UMBRA_ATTRIBUTE_ALIGNED16(m_axisNormals)[6];
    Vector4i                    UMBRA_ATTRIBUTE_ALIGNED16(m_cameraPosLocal);
    Vector4i                    UMBRA_ATTRIBUTE_ALIGNED16(m_portalExpandLocal);
    Face                        m_orthoPortalFace;
    MappedTile                  m_mappedTile;
    ActivePlaneSet              m_slotPlaneSet;
    ArrayMapper                 m_cellNodeMap;
    ArrayMapper                 m_extCellNodeMap;
    ArrayIterator<Portal>::Resources m_portalIter;
    RangeIterator               m_objectIter;
    // 2: CellState, 14: local id to m_cells index (currently 9 would be sufficient!)
    /* \todo [antti 5.11.2011]: assert bit widths */
    UINT16                      m_localCellMap[UMBRA_MAX_CELLS_PER_TILE];
    Cell                        m_outsideCell;
    int                         m_numCellsQueued;
    float                       m_depthmapOffset;
    bool                        m_depthmapsEnabled;
    bool                        m_testDepthmaps;
    float                       m_minContribution;

    // For infinite loop detection.
    int                         m_freedCellCounter;

    friend class StartCellFinder;
};

} // namespace Umbra

#endif /* __UMBRAPORTALCULL_H */

