#pragma once
#ifndef __UMBRABSPTREE_H
#define __UMBRABSPTREE_H

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
 * \brief   Umbra BSP tree
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "umbraBitOps.hpp"
#include "umbraAABB.hpp"
#include "umbraIntersect.hpp"
#include "umbraMemoryAccess.hpp"

#define UMBRA_MAX_KDTREE_DEPTH 40

namespace Umbra
{

/*!
 * A binary space partitioning tree with axis aligned splits (along the center
 * or with custom split positions defined externally) The tree is typically not
 * balanced, tree depth is greater where there is more geometry/detail/whatever
 * to partition.
 *
 * Storage uses 2 bits (see Split) per node + a small lookup table for fast
 * retrieval of node "rank", i.e. how many inner nodes there are before each
 * node. The rank information is used for child retrieval.
 */

class KDTree
{
public:

    enum Split
    {
        SPLIT_X = 0,    // 00
        SPLIT_Y = 1,    // 01
        SPLIT_Z = 2,    // 10
        LEAF    = 3     // 11
    };

    class Node
    {
        enum { INVALID = -1 };

    public:
        Node(void): m_idx(INVALID) {}
        Node(int idx, const Vector3& aabbMin, const Vector3& aabbMax, int level, Split split, UINT32 boundary)
            : m_aabbMin(aabbMin), m_idx(idx), m_aabbMax(aabbMax), m_level((UINT8)level),
              m_split((UINT8)split), m_boundary((UINT8)boundary), m_reserved(0) {}

        bool operator==(const Node& node) { return node.m_idx == m_idx; }
        bool operator!=(const Node& node) { return !operator==(node); }

        int              getIndex       (void) const { return m_idx; }
        AABB             getAABB        (void) const { return AABB(m_aabbMin, m_aabbMax); }
        const Vector3&   getAABBMin     (void) const { return m_aabbMin; }
        const Vector3&   getAABBMax     (void) const { return m_aabbMax; }
        int              getLevel       (void) const { return m_level; }
        Split            getSplit       (void) const { return (Split)m_split; }
        UINT32           getBoundary    (void) const { return m_boundary; }
        bool             isLeaf         (void) const { return (m_split == LEAF); }
        void             getDebugLines  (Vector3* lines) const;

        Vector3 m_aabbMin;
        int     m_idx;
        Vector3 m_aabbMax;
        UINT8   m_level;
        UINT8   m_split;
        UINT8   m_boundary;
        UINT8   m_reserved;
    };


    UMBRA_INLINE KDTree(int nodes, const UINT32* data, const DataArray& splitValues)
    :   m_numNodes(nodes),
        m_data(data),
        m_lut(m_data + getNodeArraySize(nodes)),
        m_splitValues(splitValues)
    {
        UMBRA_ASSERT(m_splitValues.getCount() >= 0);
    }

    UMBRA_INLINE KDTree(void) : m_numNodes(0), m_data(0) {}

    int             getNumNodes         (void) const { return m_numNodes; }
    int             getNumLeaves        (void) const { return m_numNodes - rank(m_numNodes - 1); }
    int             getLeafIdx          (int idx) const { return idx - rank(idx); }
    int             getRightChildIdx    (int idx) const { return rank(idx) << 1; }
    int             getLeftChildIdx     (int idx) const { return getRightChildIdx(idx) - 1; }
    const UINT32*   getData             (void) const { return m_data; }
    void            getPaths            (UINT32* out, int bitsPerPath) const;
    int             getMaxDepth         (void) const;
    int             getRoot             (void) const { return 0; }

    UMBRA_INLINE bool isNonMedianSplit (int idx) const
    {
        return idx < m_splitValues.getCount();
    }

    UMBRA_INLINE float getNonMedianSplit (int idx) const
    {
        UMBRA_ASSERT(isNonMedianSplit(idx));
        float f;
        m_splitValues.getElem(f, idx);
        return f;
    }

    UMBRA_INLINE float getSplitValue (const Node& n) const
    {
        UMBRA_ASSERT(!n.isLeaf());

        if (isNonMedianSplit(n.getIndex()))
            return getNonMedianSplit(n.getIndex());

        int axis = (int)n.getSplit();
        return (n.getAABBMin()[axis] + n.getAABBMax()[axis]) * 0.5f;
    }

    UMBRA_INLINE Split getSplit (int idx) const
    {
        UMBRA_ASSERT(idx >= 0 && idx < getNumNodes());
        UMBRA_ASSERT(m_data);
        int dword = (idx << 1) >> 5;
        int offset = (idx << 1) & 0x1f;
        return (Split)((m_data[dword] >> offset) & 0x3);
    }

    static int getDataDwords (int numNodes)
    {
        return getNodeArraySize(numNodes) + (getLUTSize(numNodes) >> 2);
    }

    static int getTreeSize (int numNodes)
    {
        return getNodeArraySize(numNodes) * sizeof(UINT32);
    }

    static int getLUTSize (int numNodes);

private:

    static UINT32 getNodeArraySize (int numNodes)
    {
        return UMBRA_BITVECTOR_DWORDS(numNodes * 2);
    }

    int         rank                (int idx) const;
    bool        isLeftChild         (int idx) const { return (idx & 1) ? true : false; }
    int         getSiblingIdx       (int idx) const { return ((idx - 1) + ((idx & 1) << 1)); }

    static void buildLut            (UINT32* lut, const UINT32* nodes, int numNodes);

    UINT32          m_numNodes;
    const UINT32*   m_data;
    const UINT32*   m_lut;
    DataArray       m_splitValues;

    friend class TomeWriter;
    friend class RuntimeTomeGenerator;
    template<class Allocator>
    friend class RuntimeSpatialSubdivision;
};

/*!
 * Utility classes for depth-first traversing KDTrees.
 */

enum KDTraverseEntry
{
    ENTER_NONE       = 0,
    ENTER_LEFT       = 1,
    ENTER_RIGHT      = 2,
    ENTER_BOTH       = (ENTER_LEFT | ENTER_RIGHT)
};

struct NoUserData {};

/*!
 * KDTraverseStack is the actual traverse machinery. You can use this directly, or
 * wrap it with a KDTraversal for convenience.
 *
 * Use KDTraverseStack like this:
 *
 * KDTraverseStack stack;
 * stack.init(tree, aabb);
 * while (!stack.isEmpty())
 * {
 *     const KDTree::Node& n = stack.node();
 *     stack.data() = childDataFromParentData(stack.data());
 *     if (shouldPush(n, stack.data()))
 *         stack.traverse(ENTER_BOTH);
 * }
 */

template <class UserStackData = NoUserData>
class KDTraverseStack
{
public:
    void init (const KDTree& tree, const AABB& bounds)
    {
        m_head = -1;
        m_tree = tree;
        if (tree.getNumNodes())
        {
            // push root node
            m_stack[0].node = KDTree::Node(0, bounds.getMin(), bounds.getMax(), 0, tree.getSplit(0), 0x3F);
            m_head = 0;
        }
    }
    bool isEmpty() const { return m_head == -1; }

    // access to the current element
    const KDTree::Node& node() const { return m_stack[m_head].node; }
    UserStackData& data() const { return m_stack[m_head].data; }
    const KDTree& tree() const { return m_tree; }
    KDTraverseEntry whichChild() const { return m_stack[m_head].which; }
    int depth() const { return m_head; }

    void pop()
    {
        --m_head;
    }

    template <bool UpdateNodeData>
    void pushChildren (KDTraverseEntry which, float mid, bool leftFirst, UserStackData leftData, UserStackData rightData)
    {
        UMBRA_ASSERT(which != ENTER_NONE);
        UMBRA_ASSERT(m_head + 1 < UMBRA_MAX_KDTREE_DEPTH);

        UINT32 both = (which == ENTER_BOTH) ? 1 : 0;

        // We modify stack elements in-place, so duplicate element if two will be pushed.
        // This can be done unconditionally if necessary.
        if (UpdateNodeData && (UMBRA_OPT_AVOID_BRANCHES || both))
            m_stack[m_head + 1].node = m_stack[m_head].node;

        int leftOfs = (leftFirst ? 1 : 0) & both;
        int rightOfs = (leftOfs ^ 1) & both;

        int rightChild = m_tree.getRightChildIdx(node().getIndex());
        KDTree::Split split = node().getSplit();

        if (which & ENTER_RIGHT)
        {
            StackElem& rightElem = m_stack[m_head + rightOfs];
            rightElem.node.m_idx = rightChild;
            rightElem.node.m_split = (UINT8)m_tree.getSplit(rightChild);
            rightElem.which = ENTER_RIGHT;
            rightElem.data = rightData;
            if (UpdateNodeData)
            {
                rightElem.node.m_aabbMin[split] = mid;
                rightElem.node.m_level++;
                rightElem.node.m_boundary &= ~(1 << split*2);
            }
        }

        if (which & ENTER_LEFT)
        {
            StackElem& leftElem = m_stack[m_head + leftOfs];
            leftElem.node.m_idx = rightChild - 1;
            leftElem.node.m_split = (UINT8)m_tree.getSplit(rightChild - 1);
            leftElem.which = ENTER_LEFT;
            leftElem.data = leftData;
            if (UpdateNodeData)
            {
                leftElem.node.m_aabbMax[split] = mid;
                leftElem.node.m_level++;
                leftElem.node.m_boundary &= ~(2 << split*2);
            }
        }

        m_head += both;
    }

    // traverse forward: pops current and pushes children
    void traverse (KDTraverseEntry which)
    {
        UMBRA_ASSERT(!isEmpty());

        if (which == ENTER_NONE || node().getSplit() == KDTree::LEAF)
        {
            pop();
        }
        else
        {
            pushChildren<true>(which, m_tree.getSplitValue(node()), true, data(), data());
        }
    }

private:
    struct StackElem
    {
        KDTree::Node node;
        KDTraverseEntry which;
        mutable UserStackData data;
    };

    KDTree m_tree;
    int m_head;
    StackElem m_stack[UMBRA_MAX_KDTREE_DEPTH];
};

/*!
 * KDTraversal separates the process of filtering nodes from iterating leaves.
 *
 * With a proper TraverseSpec in place, you can iterate over all active leave
 * like this:
 *
 * KDTree::Node n;
 * while (m_traverse.next(n))
 * {
 *     // process n
 * }
 *
 * Implement per-node filtering in TraverseSpec::pushNode(). This is useful,
 * for example, for limiting the traverse to volumes intersecting a given
 * search primitive. The callback is called both for inner nodes and leaf nodes.
 *
 * TraverseSpec::splitNode(), on the other hand, is only called for inner nodes
 * and is used to decide which side of the current split (or both) needs to be
 * traversed into. This is mainly useful for things like finding a single leaf
 * in the tree.
 */

template <class UserDataType>
class KDTraverseNode
{
public:
    KDTraverseNode(const KDTraverseStack<UserDataType>& stack): m_stack(stack) {}

    const KDTree::Node& treeNode(void) const { return m_stack.node(); }
    float getSplitValue(void) const { return m_stack.tree().getSplitValue(m_stack.node()); }
    UserDataType& userData(void) const { return m_stack.data(); }
    KDTraverseEntry whichChild(void) const { return m_stack.whichChild(); }

private:
    KDTraverseNode& operator= (const KDTraverseNode&);

    const KDTraverseStack<UserDataType>& m_stack;
};

template <class TUserDataType = NoUserData>
class TraverseFilter
{
public:
    typedef TUserDataType UserDataType;
    typedef KDTraverseNode<TUserDataType> NodeType;

    bool pushNode (const NodeType&) const
    {
        return true;
    }

    bool initialNode (const NodeType&) const
    {
        return true;
    }

    KDTraverseEntry splitNode (const NodeType&) const
    {
        return ENTER_BOTH;
    }
};

template <class TraverseSpec = TraverseFilter<> >
class KDTraversal
{
public:
    typedef typename TraverseSpec::UserDataType UserDataType;

    const KDTree& getTree(void) const { return m_stack.tree(); }
    const TraverseSpec& getSpec (void) const { return m_spec; }

    void init (const KDTree& tree, const AABB& bounds, const TraverseSpec& s = TraverseSpec())
    {
        m_spec = s;
        m_stack.init(tree, bounds);
        m_first = true;
    }

    void perform (void)
    {
        KDTree::Node n;
        while (next(n)) ;
    }

    bool next (KDTree::Node& n, UserDataType& data)
    {
        while (!m_stack.isEmpty())
        {
            // Should this node be visited? Note that this is done lazily, the node
            // was already "pushed" on the stack.

            bool ok = false;
            if (m_first)
            {
                ok = m_spec.initialNode(KDTraverseNode<UserDataType>(m_stack));
                m_first = false;
            }
            else
            {
                ok = m_spec.pushNode(KDTraverseNode<UserDataType>(m_stack));
            }

            // Return leaf nodes, split inner nodes

            KDTraverseEntry e = ENTER_NONE;
            bool doBreak = false;
            if (ok)
            {
                KDTree::Split split = m_stack.node().getSplit();
                if (split == KDTree::LEAF)
                {
                    n = m_stack.node();
                    data = m_stack.data();
                    doBreak = true;
                }
                else
                {
                    e = m_spec.splitNode(KDTraverseNode<UserDataType>(m_stack));
                }
            }

            // Pop current, push possible children onto stack

            m_stack.traverse(e);
            if (doBreak)
                return true;
        }
        return false;
    }

    bool next (KDTree::Node& n)
    {
        UserDataType d;
        return next(n, d);
    }


private:
    TraverseSpec m_spec;
    KDTraverseStack<UserDataType> m_stack;
    bool m_first;
};

template <bool CheckBounds = true>
class PointTraverse: public TraverseFilter<>
{
public:
    PointTraverse(void) {}
    PointTraverse(const Vector3& coord)
        : m_coord(coord) {}

    bool initialNode (const NodeType& n) const
    {
        if (CheckBounds && !n.treeNode().getAABB().contains(m_coord))
            return false;
        return pushNode(n);
    }

    KDTraverseEntry splitNode (const NodeType& n) const
    {
        if (m_coord[n.treeNode().getSplit()] < n.getSplitValue())
            return ENTER_LEFT;
        return ENTER_RIGHT;
    }

private:
    Vector3 m_coord;
};

template <class ShapeType>
class IntersectTraverse : public TraverseFilter<>
{
    IntersectTraverse(void) {}
    IntersectTraverse(const ShapeType& filter): m_filter(filter) {}

    bool pushNode (const NodeType& n) const
    {
        return intersect(n.treeNode().getAABB(), m_filter);
    }

    ShapeType m_filter;
};

typedef IntersectTraverse<Quad> QuadIntersectTraverse;
typedef IntersectTraverse<AABB> AABBIntersectTraverse;


class NodeLocator
{
public:
    bool findNode (const KDTree& tree, const AABB& bounds, const Vector3& coord, KDTree::Node& out);

private:
    KDTraversal<PointTraverse<> > m_traversal;
};

} // namespace Umbra

#endif
