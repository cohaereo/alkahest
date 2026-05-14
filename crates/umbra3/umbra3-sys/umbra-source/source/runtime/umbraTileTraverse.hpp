// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include "umbraQueryContext.hpp"
#include "umbraTransformer.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   Utility for geometry intersection based filtering
 *
 * Holds a single intersection shape of runtime determined type.
 *//*-------------------------------------------------------------------*/

class IntersectFilter
{
public:
    enum Type
    {
        FILTER_AABB,
        FILTER_QUAD,
        FILTER_POINT
    };

    void setAABB (const AABB* aabb) { m_type = FILTER_AABB; m_ptr.aabb = aabb; }
    void setQuad (const Quad* quad) { m_type = FILTER_QUAD; m_ptr.quad = quad; }
    void setPoint (const Vector3* pt) { m_type = FILTER_POINT; m_ptr.pt = pt; }
    Type getType(void) const { return m_type; }

    bool filter (const AABB& aabb) const
    {
        switch (m_type) {
        default:
            UMBRA_ASSERT(!"Unhandled filter type");
            return true;
        case FILTER_AABB: return intersect(aabb, *m_ptr.aabb);
        case FILTER_QUAD: return intersect(aabb, *m_ptr.quad);
        case FILTER_POINT: return aabb.contains(*m_ptr.pt);
        }
    }

    bool boundsCheck (const AABB& bounds) const
    {
        switch (m_type) {
        default:
            UMBRA_ASSERT(!"Unhandled filter type");
            return true;
        case FILTER_AABB: return bounds.contains(*m_ptr.aabb);
        case FILTER_POINT: return bounds.contains(*m_ptr.pt);
        case FILTER_QUAD:
            {
                for (int i = 0; i < 4; i++)
                    if (!bounds.contains((*m_ptr.quad)[i]))
                        return false;
                return true;
            }
        }
    }

private:
    union
    {
        const AABB* aabb;
        const Quad* quad;
        const Vector3* pt;
    } m_ptr;
    Type m_type;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Front-to-back comparison
 *//*-------------------------------------------------------------------*/

class FrontToBackCompare
{
public:
    FrontToBackCompare() {}

    static FrontToBackCompare sortByPoint(const Vector3& pt)
    {
        FrontToBackCompare s;
        s.m_point = pt;
        s.m_usePoint = true;
        return s;
    }
    static FrontToBackCompare sortByDir(const Vector3& pt)
    {
        FrontToBackCompare s;
        s.m_dirSigns[0] = (pt[0] >= 0.f);
        s.m_dirSigns[1] = (pt[1] >= 0.f);
        s.m_dirSigns[2] = (pt[2] >= 0.f);
        s.m_usePoint = false;
        return s;
    }

    /* 'true' means that negative side comes first */
    bool compare (Axis a, float coord)
    {
        if (m_usePoint)
            return m_point[a] < coord;
        else
            return m_dirSigns[a];
    }

private:
    Vector3 m_point;
    bool m_dirSigns[3];
    bool m_usePoint;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Utility for iterating over tome data tiles for culling
 *
 * This utility implements a front-to-back traverse over the data tiles,
 * selecting appropriate tile hierarchy levels and providing the book
 * keeping for only visiting tiles that have been queued (in which
 * either start cells are found or that have been encountered via a
 * inter-tile portal).
 *
 * There is 16 bits of state per traversable tile reserved for the user
 * of this utility.
 *//*-------------------------------------------------------------------*/

class TileTraverseTree
{
public:
    typedef UINT16* UserData;
    typedef int TileHandle;

    TileTraverseTree (QueryContext* q, int maxTreeNodes): m_query(q), m_maxTreeNodes(maxTreeNodes)
    {
        const UINT32* treeData = (const UINT32*)mapArray(q->getAllocator(), q->getTome()->getTreeData());
        m_tree = KDTree(q->getTome()->getTreeNodeCount(), treeData, q->getTome()->getTreeSplits());
        m_activeTiles = UMBRA_HEAP_NEW_ARRAY(q->getAllocator(), UINT32, UMBRA_BITVECTOR_DWORDS(q->getTome()->getTileArraySize()));
        if (m_activeTiles)
            memset(m_activeTiles, 0, UMBRA_BITVECTOR_SIZE(q->getTome()->getTileArraySize()));
        m_nodes = (Node*)UMBRA_HEAP_ALLOC(q->getAllocator(), sizeof(Node) * m_maxTreeNodes);
        m_nodes[0].clear();
    }

    ~TileTraverseTree (void)
    {
        UMBRA_HEAP_FREE(m_query->getAllocator(), m_nodes);
        UMBRA_HEAP_DELETE_ARRAY(m_query->getAllocator(), m_activeTiles);
        unmapArray(m_query->getAllocator(), m_tree.getData());
    }

    /* Set tile sort function */
    void setTileCompareFunc (const FrontToBackCompare& cmp) { m_tileSort = cmp; }

    /* Initialize the tile queue */
    template <class StartTileCallback>
    bool init (const Transformer& camera, const IntersectFilter& startAABBFilter,
        StartTileCallback startTileCallback, float lodDistance);

    /* Are there more (potential) tiles in the queue? */
    bool hasMore (void) const { return !m_stack.isEmpty(); }

    /* Move to the next tile to process. Returns 0 if no more tiles. */
    TileHandle next (int& tileIndex);

    /* Can the tile by this index (still) be traversed? */
    bool isTileTraversable (int tileIdx) const { return testBit(m_activeTiles, tileIdx); }

    /* Get a handle on a tile in the traversable set */
    TileHandle getTraversableTile (int tileIdx);

    /* Get user data for tile */
    UserData getUserData (TileHandle tile) const { checkTileHandle(tile); return m_nodes[tile].getUserData(); }

    /* Queue up traversable tile */
    void queueTile (TileHandle tile);

private:

    enum
    {
        NODE_NONE   = 0,
        NODE_ROOT   = 1
    };

    struct Node
    {
    public:
        // common for inner&leaf
        void clear (void) { flagsAndData = 0; }
        bool isLeaf (void) const { return (flagsAndData & NODEFLAG_INNER) ? false : true; }
        // inner nodes have flag set, leaf nodes have non-zero user data
        bool hasPending (void) const { return (flagsAndData & ((NODEFLAG_PENDING << 1) - 1)) != 0; }

        // inner specific
        void setPending (void) { UMBRA_ASSERT(!isLeaf()); flagsAndData |= NODEFLAG_PENDING; }
        UINT16 getLeftChild(void) const { UMBRA_ASSERT(!isLeaf()); return flagsAndData & dataMask(); }
        UINT16 getRightChild(void) const { UMBRA_ASSERT(!isLeaf()); return getLeftChild() + 1; }

        // leaf specific
        UINT16* getUserData(void) { UMBRA_ASSERT(isLeaf()); return &flagsAndData; }

        void makeInner(UINT16 left)
        {
            UMBRA_ASSERT(isLeaf());
            UMBRA_ASSERT(flagsAndData == 0);
            UMBRA_ASSERT((left & ~dataMask()) == 0);
            flagsAndData = NODEFLAG_INNER | left;
        }

    private:
        enum
        {
            NODEFLAG_INNER       = 1 << 15,
            NODEFLAG_PENDING     = 1 << 14,
            NODEFLAG_LAST        = 1 << 13
        };

        UINT16 flagsAndData;

        static UINT32 dataMask() { return (NODEFLAG_LAST << 1) - 1; }
    };

    struct BacktraceElem
    {
        BacktraceElem(void) {}
        BacktraceElem(UINT16 n): node(n), cnt(2) {}

        UINT16 node;
        UINT16 cnt;
    };

    UMBRA_CT_ASSERT(sizeof(Node) == 2);

    UMBRA_INLINE int allocNode (void)
    {
        int idx = m_nextFree++;
        m_nodes[idx].clear();
        return idx;
    }

    UMBRA_INLINE int expandNode (int idx)
    {
        int left = allocNode();
        int right = allocNode();
        UMBRA_UNREF(right);
        m_nodes[idx].makeInner((UINT16)left);
        return left;
    }

    bool ranOutOfMemory (void) const
    {
        return m_nextFree >= m_maxTreeNodes;
    }

    void checkTileHandle (TileHandle tile) const
    {
        UMBRA_ASSERT(tile >= 0 && tile < m_maxTreeNodes);
        UMBRA_UNREF(tile);
    }

    QueryContext*   m_query;
    FrontToBackCompare m_tileSort;
    UINT32*         m_activeTiles;
    KDTree          m_tree;
    Node*           m_nodes;
    int             m_maxTreeNodes;
    int             m_nextFree;
    KDTraverseStack<int> m_stack;
    BacktraceElem   m_backtrace[UMBRA_MAX_KDTREE_DEPTH];
    int             m_backtraceLen;
    int             m_curTraversableTile;
};

template <class StartTileCallback>
bool TileTraverseTree::init(
    const Transformer& camera,
    const IntersectFilter& startAABBFilter,
    StartTileCallback startTileCallback,
    float lodDistance)
{
    m_nextFree = NODE_ROOT;
    m_stack.init(m_tree, m_query->getTome()->getAABB());
    m_stack.data() = allocNode();
    m_backtraceLen = 0;
    m_curTraversableTile = -1;
    ArrayMapper tileLods(m_query, m_query->getTome()->getTileLodLevels());

    while (!m_stack.isEmpty() && !ranOutOfMemory())
    {
        int nodeIdx = m_stack.data();

        // Is this a leaf node?
        bool isLeaf = (m_stack.node().getSplit() == KDTree::LEAF);

        // Does this node potentially contain start tiles?
        bool containsStartTiles = startAABBFilter.filter(m_stack.node().getAABB());

        // Does this node have a tile attached to it?
        bool hasTile = isLeaf || m_query->hasTile(m_stack.node().getIndex());

        // Start tile handling
        if (isLeaf && containsStartTiles)
        {
            if (startTileCallback.processStartTile(m_stack.node().getIndex(), nodeIdx))
            {
                UMBRA_ASSERT(m_nodes[nodeIdx].hasPending());
                int btIdx = m_backtraceLen;
                while (btIdx--)
                {
                    if (m_nodes[m_backtrace[btIdx].node].hasPending())
                        break;
                    m_nodes[m_backtrace[btIdx].node].setPending();
                }
            }
            else
            {
                UMBRA_ASSERT(!m_nodes[nodeIdx].hasPending());
                containsStartTiles = false;
            }
        }

        bool active = true;
        if (!containsStartTiles && hasTile)
        {
            // Frustum culling only for nodes that have tile (need to know portal expand)
            float portalExpand = m_query->getPortalExpand(m_stack.node().getIndex());
            SIMDRegister exp = SIMDAdd(SIMDLoadW0(portalExpand), camera.getPrediction());
            SIMDRegister mn = SIMDSub(SIMDLoadW1(m_stack.node().getAABBMin()), exp);
            SIMDRegister mx = SIMDAdd(SIMDLoadW1(m_stack.node().getAABBMax()), exp);
            active = camera.frustumTestBoundsZeroPlane(mn, mx);
        }

        bool enterNode = false;
        if (active)
        {
            // Level cull. TODO: check that start tiles are at proper level when entry in
            // hierarchy tiles is supported.
            enterNode = !isLeaf;
            if (!containsStartTiles && !isLeaf && hasTile && lodDistance > 0.f)
            {
                float distanceFromCamera = m_stack.node().getAABB().getDistance(camera.getCameraPos());
                float lodLevel;
                tileLods.get(lodLevel, m_stack.node().getIndex());
                if (distanceFromCamera > lodDistance * lodLevel)
                    enterNode = false;
            }

            if (!enterNode)
            {
                UMBRA_ASSERT(hasTile);
                // The tile "active" bit means whether it can be traversed to or not. Note that all tiles
                // with start cells have this bit on, regardless of whether they are in frustum or not.
                setBit(m_activeTiles, m_stack.node().getIndex());
            }
        }

        if (enterNode)
        {
            int leftChild = expandNode(nodeIdx);
            m_stack.pushChildren<true>(ENTER_BOTH, m_tree.getSplitValue(m_stack.node()), true, leftChild, leftChild + 1);
            m_backtrace[m_backtraceLen++] = BacktraceElem((UINT16)nodeIdx);
        }
        else
        {
            m_stack.pop();
            while (m_backtraceLen)
            {
                if (--m_backtrace[m_backtraceLen-1].cnt == 0)
                    --m_backtraceLen;
                else
                    break;
            }
        }
    }

    // Reinit traversal stack
    if (m_nodes[NODE_ROOT].hasPending())
    {
        m_stack.init(m_tree, m_query->getTome()->getAABB());
        m_stack.data() = NODE_ROOT;
    }

    return !ranOutOfMemory();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline TileTraverseTree::TileHandle TileTraverseTree::next (int& tileIndex)
{
    while (!m_stack.isEmpty())
    {
        int nodeIdx = m_stack.data();
        Node* n = &m_nodes[nodeIdx];

        if (!n->hasPending())
        {
            // case 1: defunct subtree, prune
            // remove active bits for this subtree
            int targetDepth = m_stack.depth() - 1;
            while (m_stack.depth() > targetDepth)
            {
                nodeIdx = m_stack.data();
                n = &m_nodes[nodeIdx];
                UMBRA_ASSERT(!n->hasPending());
                if (n->isLeaf())
                {
                    clearBit(m_activeTiles, m_stack.node().getIndex());
                    m_stack.pop();
                }
                else
                {
                    UMBRA_ASSERT(!testBit(m_activeTiles, m_stack.node().getIndex()));
                    m_stack.pushChildren<false>(ENTER_BOTH, 0.f, true, n->getLeftChild(), n->getRightChild());
                }
            }
        }
        else if (n->isLeaf())
        {
            // case 2: pending leaf, return tile info
            tileIndex = m_stack.node().getIndex();
            UMBRA_ASSERT(testBit(m_activeTiles, tileIndex));
            return nodeIdx;
        }
        else
        {
            // case 3: pending subtree, traverse deeper
            float mid = m_tree.getSplitValue(m_stack.node());
            bool leftFirst = false;
            if (!m_nodes[n->getRightChild()].hasPending())
            {
                leftFirst = true;
            }
            else if (m_nodes[n->getLeftChild()].hasPending())
            {
                leftFirst = m_tileSort.compare((Axis)m_stack.node().getSplit(), mid);
            }
            m_stack.pushChildren<true>(ENTER_BOTH, mid, leftFirst, n->getLeftChild(), n->getRightChild());
        }
    }

    return 0;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline TileTraverseTree::TileHandle TileTraverseTree::getTraversableTile (int tileIdx)
{
    UMBRA_ASSERT(isTileTraversable(tileIdx));

    // single entry "cache"
    if (tileIdx == m_curTraversableTile)
        return m_backtrace[m_backtraceLen].node;

    ArrayMapper* paths = m_query->getState()->getTilePaths();
    int bitsPerTile = m_query->getState()->getBitsPerTilePath();
    int bitOfs = tileIdx * bitsPerTile;
    int dwordOfs = UMBRA_BIT_DWORD(bitOfs);
    int curOfs = UMBRA_BIT_IDX(bitOfs);
    int bitsLeft = bitsPerTile;
    int zeroTrim = 0;
    int nodeIdx = NODE_ROOT;

    m_backtraceLen = 0;
    m_curTraversableTile = tileIdx;

    while (bitsLeft)
    {
        UINT32 val;
        paths->get(val, dwordOfs++);
        int count = min2(32 - curOfs, bitsLeft);
        bitsLeft -= count;
        val >>= curOfs;

        while (count--)
        {
            int bit = (val & 1);
            UMBRA_ASSERT(nodeIdx != NODE_NONE);
            const Node& n = m_nodes[nodeIdx];
            UMBRA_ASSERT(!n.isLeaf());
            if (zeroTrim)
            {
                if (!n.hasPending())
                    m_backtrace[m_backtraceLen++].node = (UINT16)nodeIdx;
                nodeIdx = n.getRightChild() - bit;
            }
            zeroTrim |= bit;
            val >>= 1;
        }

        curOfs = 0;
    }

    m_backtrace[m_backtraceLen].node = (UINT16)nodeIdx;
    UMBRA_ASSERT(m_nodes[nodeIdx].isLeaf());
    return nodeIdx;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline void TileTraverseTree::queueTile (TileHandle tile)
{
    checkTileHandle(tile);
    // This is the only supported usage pattern right now, 'tile'
    // must be the tile that was last retrieved with getTraversableTile.
    UMBRA_ASSERT(tile == m_backtrace[m_backtraceLen].node);
    UMBRA_ASSERT(m_nodes[tile].hasPending());
    int btIdx = m_backtraceLen;
    while (btIdx--)
        m_nodes[m_backtrace[btIdx].node].setPending();
}

}