#pragma once

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "umbraAABB.hpp"
#include "umbraObjectPool.hpp"
#include "umbraBitMath.hpp"
#include "umbraArray.hpp"
#include "umbraHash.hpp"
#include "umbraFloat.hpp"
#include "umbraWeightedSampler.hpp"
#include "umbraSet.hpp"
#include <standard/Sampling.hpp>

#define RECURSIVE_ROTATE_COLLAPSE 0

namespace Umbra
{

/*
 * SubdivisionTree.
 */

class SubdivisionTree
{
public:
    enum NodeType
    {
        LEAF,
        MEDIAN,
        AXIAL,
        PLANE
    };

    struct LeafNode;
    struct MedianNode;
    struct AxialNode;
    struct PlaneNode;
    struct InnerNode;

    struct Node
    {
        NodeType            getType() const { return (NodeType)(m_data & 3); }
        bool                isLeaf() const { return getType() == LEAF; }
        bool                isMedian() const { return getType() == MEDIAN; }
        bool                isAxial() const { return getType() == AXIAL; }
        bool                isPlane() const { return getType() == PLANE; }
        bool                isInner() const { return !isLeaf(); }
        LeafNode*           getLeaf() { UMBRA_ASSERT(isLeaf()); return (LeafNode*)this; }
        const LeafNode*     getLeaf() const { UMBRA_ASSERT(isLeaf()); return (const LeafNode*)this; }
        MedianNode*         getMedian() { UMBRA_ASSERT(isMedian()); return (MedianNode*)this; }
        const MedianNode*   getMedian() const { UMBRA_ASSERT(isMedian()); return (const MedianNode*)this; }
        AxialNode*          getAxial() { UMBRA_ASSERT(isAxial()); return (AxialNode*)this; }
        const AxialNode*    getAxial() const { UMBRA_ASSERT(isAxial()); return (const AxialNode*)this; }
        PlaneNode*          getPlane() { UMBRA_ASSERT(isPlane()); return (PlaneNode*)this; }
        const PlaneNode*    getPlane() const { UMBRA_ASSERT(isPlane()); return (const PlaneNode*)this; }
        InnerNode*          getInner() { UMBRA_ASSERT(isInner()); return (InnerNode*)this; }
        const InnerNode*    getInner() const { UMBRA_ASSERT(isInner()); return (const InnerNode*)this; }

        uint64       m_data;
    };

    struct LeafNode : public Node
    {
        enum
        {
            INVALID_IDX = 0x7FFFFFFF
        };

        void  setIndex(int idx) { UMBRA_ASSERT(idx != INVALID_IDX); m_data = (m_data & 3) | (((uint64)(uint32)idx) << 2); }
        int   getIndex() const { return (int)(m_data >> 2); }

        bool operator==(const LeafNode& n) { return m_data == n.m_data; }
    };

    struct InnerNode : public Node
    {
        Node* getLeft() const { return (Node*)(m_data & ~3); }
        Node* getRight() const { return (Node*)(m_right & ~3); }
        void  setLeft (Node* node) { uint64 ptr = (uint64)node; UMBRA_ASSERT((ptr & 3) == 0); m_data = (m_data & 3) | ptr; }
        void  setRight(Node* node) { uint64 ptr = (uint64)node; UMBRA_ASSERT((ptr & 3) == 0); m_right = (m_right & 3) | ptr; }

        uint64       m_right;
    };

    struct MedianNode : public InnerNode
    {
        void  setAxis(int axis) { UMBRA_ASSERT(axis >= 0 && axis <= 2); m_right = (m_right & ~3) | (uint64)axis; }
        int   getAxis() const { UMBRA_ASSERT((m_right & 3) != 3); return m_right & 3; }

        bool operator==(const MedianNode& n) { return m_data == n.m_data && m_right == n.m_right; }
    };

    struct AxialNode : public InnerNode
    {
        void  setAxis(int axis) { UMBRA_ASSERT(axis >= 0 && axis <= 2); m_right = (m_right & ~3) | (uint64)axis; }
        int   getAxis() const { UMBRA_ASSERT((m_right & 3) != 3); return m_right & 3; }
        void  setPos(float p) { m_pos = p; }
        float getPos(void) const { return m_pos; }

        bool operator==(const AxialNode& n) { return m_data == n.m_data && m_right == n.m_right && m_pos == n.m_pos; }

        float        m_pos;
    };

    struct PlaneNode : public InnerNode
    {
        void    setPleq(const Vector4& p) { UMBRA_ASSERT(Float::isFinite(p.x) && Float::isFinite(p.y) && Float::isFinite(p.z) && Float::isFinite(p.w)); m_pleq = p; }
        Vector4 getPleq() const { return m_pleq; }

        bool operator==(const PlaneNode& n) { return m_data == n.m_data && m_right == n.m_right && m_pleq == n.m_pleq; }

        Vector4      m_pleq;
    };

    SubdivisionTree(Allocator* a) : m_allocator(a), m_aabb(), m_leaves(a), m_medians(a), m_axials(a), m_planes(a), m_root(0) {}
    ~SubdivisionTree() {}

    Allocator* getAllocator(void) const { return m_allocator; }

    void setRoot(Node* node);
    Node* getRoot(void) const { return m_root; }

    LeafNode*   newLeaf();
    MedianNode* newMedian();
    AxialNode*  newAxial();
    PlaneNode*  newPlane();

    void deleteNode (Node*);
    void deleteTree (Node*);

    void        setAABB(const AABB& aabb) { m_aabb = aabb; }
    const AABB& getAABB() const { return m_aabb; }

    struct Iterator
    {
        Node* node() { UMBRA_ASSERT(m_node); return m_node; }

        bool end() const { return m_node == 0; }

        void next()
        {
            UMBRA_ASSERT(m_node);

            if (m_node->isLeaf())
            {
                if (m_st.getSize())
                    m_node = m_st.popBack();
                else
                    m_node = 0;
            }
            else
            {
                UMBRA_ASSERT(m_node->isInner());
                Node* left = m_node->getInner()->getLeft();
                Node* right = m_node->getInner()->getRight();
                m_node = left;
                m_st.pushBack(right);
            }
        }

        Node*        m_node;
        Array<Node*> m_st;
    };

    struct LeafIterator : Iterator
    {
        LeafNode* leaf(void) { return node()->getLeaf(); }

        void next()
        {
            Iterator::next();
            while (!end() && !node()->isLeaf())
                Iterator::next();
        }
    };

    void iterate(Iterator& iter) const
    {
        UMBRA_ASSERT(m_root);
        iter.m_node = m_root;
        iter.m_st.clear();
        iter.m_st.setAllocator(m_allocator);
    }

    void iterate(LeafIterator& iter) const
    {
        iterate(static_cast<Iterator&>(iter));
        if (!iter.node()->isLeaf())
            iter.next();
    }

    Iterator iterateAll(void) const
    {
        Iterator iter;
        iterate(iter);
        return iter;
    }

    LeafIterator iterateLeaves(void) const
    {
        LeafIterator iter;
        iterate(iter);
        return iter;
    }

private:
    Allocator*             m_allocator;
    AABB                   m_aabb;
    ObjectPool<LeafNode>   m_leaves;
    ObjectPool<MedianNode> m_medians;
    ObjectPool<AxialNode>  m_axials;
    ObjectPool<PlaneNode>  m_planes;
    Node*                  m_root;
};

/*
 * SubdivisionTreeUtils.
 */

class SubdivisionTreeUtils
{
public:
    SubdivisionTreeUtils(SubdivisionTree& st) : m_st(st) {}

    static SubdivisionTree::Node* getLeft(const SubdivisionTree::Node* node)
    {
        UMBRA_ASSERT(node && !node->isLeaf());
        return node->getInner()->getLeft();
    }

    static SubdivisionTree::Node* getRight(const SubdivisionTree::Node* node)
    {
        UMBRA_ASSERT(node && !node->isLeaf());
        return node->getInner()->getRight();
    }

    static void setLeft(SubdivisionTree::Node* node, SubdivisionTree::Node* l)
    {
        UMBRA_ASSERT(node && l);
        UMBRA_ASSERT(!node->isLeaf());
        node->getInner()->setLeft(l);
    }

    static void setRight(SubdivisionTree::Node* node, SubdivisionTree::Node* r)
    {
        UMBRA_ASSERT(node && r);
        UMBRA_ASSERT(!node->isLeaf());
        node->getInner()->setRight(r);
    }

    static int countNodes(const SubdivisionTree::Node* node)
    {
        UMBRA_ASSERT(node);
        if (node->isLeaf())
            return 1;
        return 1 + countNodes(getLeft(node)) + countNodes(getRight(node));
    }

    static int findMaxLeafIndex(SubdivisionTree::Node* node)
    {
        if (node->isLeaf())
            return node->getLeaf()->getIndex();

        return max2(findMaxLeafIndex(getLeft(node)), findMaxLeafIndex(getRight(node)));
    }

    static bool findLeafIndex(SubdivisionTree::Node* node, int& idx, bool acceptNegative)
    {
        if (node->isLeaf())
        {
            int leafIdx = ((SubdivisionTree::LeafNode*)node)->getIndex();
            if (acceptNegative && (leafIdx < 0))
                return true;
            if (idx == SubdivisionTree::LeafNode::INVALID_IDX)
                idx = leafIdx;
            return idx == leafIdx;
        }

        return findLeafIndex(getLeft(node), idx, acceptNegative) && findLeafIndex(getRight(node), idx, acceptNegative);
    }

    static bool containsSplit(SubdivisionTree::Node* node, int axis)
    {
        if (node->isLeaf())
            return false;

        if (node->isMedian() && node->getMedian()->getAxis() == axis)
            return true;

        if (node->isAxial() && node->getAxial()->getAxis() == axis)
            return true;

        return containsSplit(node->getInner()->getLeft(), axis) ||
               containsSplit(node->getInner()->getRight(), axis);
    }

    static bool containsMedianCut(SubdivisionTree::Node* node, int axis)
    {
        if (!node->isMedian())
            return false;

        if (node->getMedian()->getAxis() == axis)
            return true;

        return containsMedianCut(node->getInner()->getLeft(), axis) &&
               containsMedianCut(node->getInner()->getRight(), axis);
    }

    static bool containsPlaneCut(SubdivisionTree::Node* node, const Vector4& pleq)
    {
        if (!node->isPlane())
            return false;

        if (node->getPlane()->getPleq() == pleq)
            return true;

        return containsPlaneCut(node->getInner()->getLeft(), pleq) &&
               containsPlaneCut(node->getInner()->getRight(), pleq);
    }

    static bool containsPlaneNodes(SubdivisionTree::Node* node)
    {
        if (node->isLeaf())
            return false;
        if (node->isPlane())
            return true;
        return containsPlaneNodes(node->getInner()->getLeft()) ||
               containsPlaneNodes(node->getInner()->getRight());
    }

    static void collectPlanes(SubdivisionTree::Node* node, Set<Vector4>& planes)
    {
        if (!node->isPlane())
            return;
        planes.insert(node->getPlane()->getPleq());
        collectPlanes(node->getPlane()->getLeft(), planes);
        collectPlanes(node->getPlane()->getRight(), planes);
    }

    static void sanityCheck(const SubdivisionTree::Node* node, bool planes)
    {
        if (node->isLeaf())
            return;
        if (node->isPlane())
            planes = true;
        else if (planes)
            UMBRA_ASSERT(!"malformed tree");
        sanityCheck(node->getInner()->getLeft(), planes);
        sanityCheck(node->getInner()->getRight(), planes);
    }

    static void splitBounds(const SubdivisionTree::Node* node, const AABB& bounds, AABB& left, AABB& right)
    {
        left = bounds;
        right = bounds;

        if (node->isMedian() || node->isAxial())
        {
            int axis;
            float pos;

            if (node->isMedian())
            {
                axis = node->getMedian()->getAxis();
                pos = bounds.getCenter()[node->getMedian()->getAxis()];
            }
            else
            {
                axis = node->getAxial()->getAxis();
                pos = node->getAxial()->getPos();
            }

            left.setMax(axis, pos);
            right.setMin(axis, pos);
        }
    }

    SubdivisionTree::Node* collapsePlanesToNegatives(SubdivisionTree::Node* node)
    {
        UMBRA_ASSERT(m_st.getAABB().isOK());
        if (containsPlaneNodes(node))
            return collapseRec(node, m_st.getAABB(), false, true);
        else
            return node;
    }

    SubdivisionTree::Node* collapse(SubdivisionTree::Node* node, bool collapseNegatives = false)
    {
        UMBRA_ASSERT(m_st.getAABB().isOK());
        return collapseRec(node, m_st.getAABB(), collapseNegatives, collapseNegatives);
    }

    SubdivisionTree::Node* collapseSingleNode (SubdivisionTree::Node* node, const AABB& bounds, bool collapseNeg, bool collapseNegToPlane)
    {
        UMBRA_UNREF(bounds);
        UMBRA_ASSERT(node);

        // Do we have multiple indices?

        int leafIdx = SubdivisionTree::LeafNode::INVALID_IDX;
        if (findLeafIndex(node, leafIdx, collapseNeg))
        {
            SubdivisionTree::LeafNode* leaf = m_st.newLeaf();
            leaf->setIndex(leafIdx == SubdivisionTree::LeafNode::INVALID_IDX ? -1 : leafIdx);
            m_st.deleteTree(node);
            return leaf;
        }

        SubdivisionTree::Node* left = getLeft(node);
        SubdivisionTree::Node* right = getRight(node);

        // Matching subtrees

        if (compareNodes(left, right))
        {
            UMBRA_ASSERT(!left->isLeaf());

            bool collapse = false;
            if (node->isPlane() || left->isPlane())
            {
                collapse = true;
            }
            else if (left->isMedian() && !containsSplit(left, node->isMedian() ?
                node->getMedian()->getAxis() : node->getAxial()->getAxis()))
            {
                collapse = true;
            }
            if (collapse)
            {
                m_st.deleteTree(right);
                m_st.deleteNode(node);
                return left;

            }
        }

        // Collapse zeros

        if (collapseNeg || collapseNegToPlane)
        {
            for (int i = 0; i < 2; i++)
            {
                SubdivisionTree::Node* cur = (i == 0) ? left : right;
                SubdivisionTree::Node* other = (i == 0) ? right : left;
                bool collapse = false;

                if (cur->isLeaf() && (cur->getLeaf()->getIndex() < 0))
                {
                    if (collapseNegToPlane && other->isPlane())
                        collapse = true;
                    else if (collapseNeg && (node->isMedian() || node->isAxial()))
                    {
                        int axis = node->isMedian() ? node->getMedian()->getAxis() : node->getAxial()->getAxis();
                        if (!containsSplit(other, axis))
                            collapse = true;
                    }
                }
                if (collapse)
                {
                    m_st.deleteTree(cur);
                    m_st.deleteNode(node);
                    return other;
                }
            }
        }

        // collapse redundant plane splits
        /* \todo [antti 1.3.2013]: axial splits, too */

        if (node->isPlane())
        {
            Vector4 pleq = node->getPlane()->getPleq();
            int mask = classifyAABB(pleq, bounds);

            // All points on same side of plane or consequtive same planes

            if ((mask == 1) || (left->isPlane() && left->getPlane()->getPleq() == pleq))
            {
                m_st.deleteTree(right);
                m_st.deleteNode(node);
                return left;
            }
            if ((mask == 2) || (right->isPlane() && right->getPlane()->getPleq() == pleq))
            {
                m_st.deleteTree(left);
                m_st.deleteNode(node);
                return right;
            }
        }

        // plane collapses

        if ((node->isMedian() || node->isAxial()) && (right->isPlane() || left->isPlane()))
        {
            AABB leftBounds, rightBounds;
            splitBounds(node, bounds, leftBounds, rightBounds);
            if (left->isLeaf() || right->isLeaf())
            {
                // collapse planes to leaves

                if (left->isLeaf() && testCollapsing(right, leftBounds, left->getLeaf()->getIndex()))
                {
                    m_st.deleteNode(left);
                    m_st.deleteNode(node);
                    return right;
                }
                if (right->isLeaf() && testCollapsing(left, rightBounds, right->getLeaf()->getIndex()))
                {
                    m_st.deleteNode(right);
                    m_st.deleteNode(node);
                    return left;
                }
            }
            else
            {
                // sample bsp

                if (left->isPlane() && bspSampleCollapse(left, right, leftBounds, rightBounds, collapseNegToPlane))
                {
                    m_st.deleteTree(right);
                    m_st.deleteNode(node);
                    return left;
                }
                if (right->isPlane() && bspSampleCollapse(right, left, rightBounds, leftBounds, collapseNegToPlane))
                {
                    m_st.deleteTree(left);
                    m_st.deleteNode(node);
                    return right;
                }
            }
        }

        return node;
    }

    bool bspSampleCollapse (SubdivisionTree::Node* node, SubdivisionTree::Node* other,
        const AABB& bounds, const AABB& otherBounds, bool collapseNeg)
    {
        UMBRA_ASSERT(node->isPlane());

        static const int SAMPLES = 1234;

        for (int i = 0; i < SAMPLES; i++)
        {
            Vector3 rand;
            rand.x = (i + .5f) / float(SAMPLES);
            rand.y = haltonf<2>(i);
            rand.z = haltonf<3>(i);
            Vector3 p = otherBounds.getMin() + otherBounds.getDimensions().scale(rand);

            int index = findLeafIndex(node, bounds, p);
            int otherIndex = findLeafIndex(other, otherBounds, p);

            if (collapseNeg && (otherIndex < 0))
                continue;
            if (index != otherIndex)
                return false;
        }

        return true;
    }

    void rotateCollapseMedian (SubdivisionTree::Node* node, int targetAxis, const AABB& bounds, bool collapseNeg, bool collapseNegToPlane)
    {
        UMBRA_ASSERT(node->isMedian());

        if (node->getMedian()->getAxis() == targetAxis)
            return;

        AABB leftBounds, rightBounds;
        splitBounds(node, bounds, leftBounds, rightBounds);

        SubdivisionTree::Node* left = getLeft(node);
        SubdivisionTree::Node* right = getRight(node);
        UMBRA_ASSERT(left->isMedian());
        UMBRA_ASSERT(right->isMedian());

        bool leftLeaf = RECURSIVE_ROTATE_COLLAPSE ? (left->getMedian()->getAxis() == targetAxis) : false;
        bool rightLeaf = RECURSIVE_ROTATE_COLLAPSE ? (right->getMedian()->getAxis() == targetAxis) : false;

        // Recurse

        rotateCollapseMedian(left, targetAxis, leftBounds, collapseNeg, collapseNegToPlane);
        rotateCollapseMedian(right, targetAxis, rightBounds, collapseNeg, collapseNegToPlane);

        UMBRA_ASSERT(left->getMedian()->getAxis() == targetAxis);
        UMBRA_ASSERT(right->getMedian()->getAxis() == targetAxis);

        // Rotate

        int axis = node->getMedian()->getAxis();
        UMBRA_ASSERT(axis != targetAxis);
        node->getMedian()->setAxis(targetAxis);
        left->getMedian()->setAxis(axis);
        right->getMedian()->setAxis(axis);
        SubdivisionTree::Node* tmp = getRight(left);
        setRight(left, getLeft(right));
        setLeft(right, tmp);

        // Re-collapse
        splitBounds(node, bounds, leftBounds, rightBounds);
        if (leftLeaf)
            setLeft(node, collapseRec(left, leftBounds, collapseNeg, collapseNegToPlane, false));
        else
            setLeft(node, collapseSingleNode(left, leftBounds, collapseNeg, collapseNegToPlane));
        if (rightLeaf)
            setRight(node, collapseRec(right, rightBounds, collapseNeg, collapseNegToPlane, false));
        else
            setRight(node, collapseSingleNode(right, rightBounds, collapseNeg, collapseNegToPlane));
    }

    SubdivisionTree::Node* collapseRec (SubdivisionTree::Node* node, const AABB& bounds, bool collapseNeg, bool collapseNegToPlane, bool children = true)
    {
        UMBRA_ASSERT(node);

        if (node->isLeaf())
            return node;

        if (children)
        {
            AABB leftBounds, rightBounds;
            splitBounds(node, bounds, leftBounds, rightBounds);
            setLeft(node, collapseRec(getLeft(node), leftBounds, collapseNeg, collapseNegToPlane));
            setRight(node, collapseRec(getRight(node), rightBounds, collapseNeg, collapseNegToPlane));
        }

        // See if we can collapse this node

        SubdivisionTree::Node* collapsed = collapseSingleNode(node, bounds, collapseNeg, collapseNegToPlane);
        if (collapsed != node)
            return collapsed;

        // Rotate split axis and re-collapse

        if (node->isMedian())
        {
            int curAxis = node->getMedian()->getAxis();
            for (int j = 1; j < 4; j++)
            {
                int a = (curAxis + j) % 3;
                if ((node->getMedian()->getAxis() == a) || !containsMedianCut(node, a))
                    continue;
                rotateCollapseMedian(node, a, bounds, collapseNeg, collapseNegToPlane);
                SubdivisionTree::Node* collapsed = collapseSingleNode(node, bounds, collapseNeg, collapseNegToPlane);
                if (collapsed != node)
                    return collapsed;
            }
        }

        return node;
    }

    static bool compareNodes(const SubdivisionTree::Node* a, const SubdivisionTree::Node* b)
    {
        UMBRA_ASSERT(a && b);

        if (a->getType() != b->getType())
            return false;

        if (a->isLeaf())
            return a->getLeaf()->getIndex() == b->getLeaf()->getIndex();

        if (a->isMedian() && a->getMedian()->getAxis() != b->getMedian()->getAxis())
            return false;

        if (a->isAxial() && (a->getAxial()->getAxis() != b->getAxial()->getAxis() ||
                             a->getAxial()->getPos() != b->getAxial()->getPos()))
            return false;

        if (a->isPlane() && a->getPlane()->getPleq() != b->getPlane()->getPleq())
            return false;

        UMBRA_ASSERT(a->isLeaf() || a->isMedian() || a->isAxial() || a->isPlane());

        return compareNodes(getLeft(a), getLeft(b)) &&
               compareNodes(getRight(a), getRight(b));
    }

    SubdivisionTree::Node* clone(const SubdivisionTree::Node* orig)
    {
        SubdivisionTree::Node* node = NULL;

        switch (orig->getType())
        {
        case SubdivisionTree::LEAF:
            node = m_st.newLeaf();
            node->getLeaf()->setIndex(orig->getLeaf()->getIndex());
            return node;

        case SubdivisionTree::MEDIAN:
            node = m_st.newMedian();
            node->getMedian()->setAxis(orig->getMedian()->getAxis());
            break;

        case SubdivisionTree::AXIAL:
            node = m_st.newAxial();
            node->getAxial()->setAxis(orig->getAxial()->getAxis());
            node->getAxial()->setPos(orig->getAxial()->getPos());
            break;

        case SubdivisionTree::PLANE:
            node = m_st.newPlane();
            node->getPlane()->setPleq(orig->getPlane()->getPleq());
            break;
        }

        setLeft(node, clone(getLeft(orig)));
        setRight(node, clone(getRight(orig)));

        return node;
    }

    SubdivisionTree::Node* join(int axis, float pos, const SubdivisionTree::Node* left, const SubdivisionTree::Node* right)
    {
        SubdivisionTree::AxialNode* split = m_st.newAxial();
        split->setAxis(axis);
        split->setPos(pos);
        split->setLeft(clone(getLeft(left)));
        split->setRight(clone(getRight(right)));
        return split;
    }

    static void getLevelOrder(const SubdivisionTree::Node* node, Array<const SubdivisionTree::Node*>& ary)
    {
        FIFOQueue<const SubdivisionTree::Node*> fifo(ary.getAllocator(), countNodes(node));

        fifo.pushBack(node);

        while (fifo.getSize())
        {
            const SubdivisionTree::Node* node = fifo.popFront();

            ary.pushBack(node);

            if (!node->isLeaf())
            {
                fifo.pushBack(getLeft(node));
                fifo.pushBack(getRight(node));
            }
        }
    }

    SubdivisionTree::Node* replacePlaneNodesWithLeaves(SubdivisionTree::Node* node, const Hash<SubdivisionTree::PlaneNode*, int>& planes)
    {
        if (node->isLeaf())
            return node;
        else if (node->isPlane())
        {
            SubdivisionTree::LeafNode* node2 = m_st.newLeaf();

            const int* idx = planes.get(node->getPlane());
            UMBRA_ASSERT(idx);

            if (idx)
                node2->setIndex(*idx);

            return node2;
        }
        else
        {
            SubdivisionTree::Node* left = replacePlaneNodesWithLeaves(getLeft(node), planes);
            SubdivisionTree::Node* right = replacePlaneNodesWithLeaves(getRight(node), planes);

            setLeft(node, left);
            setRight(node, right);

            return node;
        }
    }

    static bool hasOtherLeafIndices(const SubdivisionTree::Node* node, int idx)
    {
        if (node->isLeaf())
            return node->getLeaf()->getIndex() != idx;
        else
            return hasOtherLeafIndices(node->getInner()->getLeft(), idx) || hasOtherLeafIndices(node->getInner()->getRight(), idx);
    }

    static bool hasAnyLeafIndices(const SubdivisionTree::Node* node, int idx)
    {
        if (node->isLeaf())
            return node->getLeaf()->getIndex() == idx;
        else
            return hasAnyLeafIndices(node->getInner()->getLeft(), idx) || hasAnyLeafIndices(node->getInner()->getRight(), idx);
    }

    static int classifyAABB(const Vector4& pleq, const AABB& aabb)
    {
        int mask = 0;

        for (int i = 0; i < 8; i++)
        {
            float d = dot(pleq, aabb.getCorner((AABB::Corner)i));
            if (d < 0.f)
                mask |= 1;
            else if (d > 0.f)
                mask |= 2;
        }

        return mask;
    }

    static bool testCollapsingInner (const SubdivisionTree::Node* node, const AABB& aabb, int idx, int depth)
    {
        UMBRA_ASSERT(node->isPlane());
        const SubdivisionTree::PlaneNode* plane = node->getPlane();

        int mask = classifyAABB(plane->getPleq(), aabb);
        UMBRA_ASSERT(mask != 0);

        if (mask < 3)
        {
            return testCollapsing((mask == 1) ? plane->getLeft() : plane->getRight(), aabb, idx);
        }
        else
        {
            UMBRA_ASSERT(mask == 3);

            int axis = aabb.getLongestAxis();
            float rel = aabb.getDimensions()[axis] / max2(fabsf(aabb.getMin()[axis]), fabsf(aabb.getMax()[axis]));
            if (depth > 8 && rel < 0.01f) // TODO: better epsilon
                return true;

            AABB left = aabb;
            AABB right = aabb;

            left.setMax(axis, aabb.getCenter()[axis]);
            right.setMin(axis, aabb.getCenter()[axis]);

            return testCollapsingInner(plane, left, idx, depth+1) &&
                   testCollapsingInner(plane, right, idx, depth+1);
        }
    }

    static bool testCollapsing (const SubdivisionTree::Node* node, const AABB& aabb, int idx)
    {
        if (!hasOtherLeafIndices(node, idx))
            return true;
        if (!hasAnyLeafIndices(node, idx))
            return false;
        return testCollapsingInner(node, aabb, idx, 0);
    }

    int findLeafIndex(const Vector3& p) const
    {
        UMBRA_ASSERT(m_st.getAABB().contains(p));
        return findLeafIndex(m_st.getRoot(), m_st.getAABB(), p);
    }

    static int findLeafIndex(const SubdivisionTree::Node* node, const AABB& aabb, const Vector3& p)
    {
        UMBRA_ASSERT(aabb.isOK());

        switch (node->getType())
        {
        case SubdivisionTree::LEAF:
            return node->getLeaf()->getIndex();

        case SubdivisionTree::MEDIAN:
            {
                UMBRA_ASSERT(aabb.contains(p));
                int axis = node->getMedian()->getAxis();

                AABB newAABB = aabb;

                if (p[axis] <= aabb.getCenter()[axis])
                {
                    newAABB.setMax(axis, aabb.getCenter()[axis]);
                    return findLeafIndex(node->getMedian()->getLeft(), newAABB, p);
                }
                else
                {
                    newAABB.setMin(axis, aabb.getCenter()[axis]);
                    return findLeafIndex(node->getMedian()->getRight(), newAABB, p);
                }
            }
            break;

        case SubdivisionTree::AXIAL:
            {
                UMBRA_ASSERT(aabb.contains(p));
                int axis = node->getAxial()->getAxis();
                float pos = node->getAxial()->getPos();

                AABB newAABB = aabb;

                if (p[axis] < pos)
                {
                    newAABB.setMax(axis, pos);
                    return findLeafIndex(node->getAxial()->getLeft(), newAABB, p);
                }
                else
                {
                    newAABB.setMin(axis, pos);
                    return findLeafIndex(node->getAxial()->getRight(), newAABB, p);
                }
            }
            break;

        case SubdivisionTree::PLANE:
            {
                if (dot(node->getPlane()->getPleq(), p) < 0.f)
                    return findLeafIndex(node->getPlane()->getLeft(), aabb, p);
                else
                    return findLeafIndex(node->getPlane()->getRight(), aabb, p);
            }
            break;
        }

        UMBRA_ASSERT(0);
        return 0;
    }

    // \note this is for bottom-up unification and it hashes pointer values instead of contents

    struct UnifiedNode
    {
        SubdivisionTree::Node* node;

        bool operator==(const UnifiedNode& b)
        {
            if (node->getType() != b.node->getType())
                return false;

            switch (node->getType())
            {
            case SubdivisionTree::LEAF:   return *node->getLeaf() == *b.node->getLeaf();
            case SubdivisionTree::MEDIAN: return *node->getMedian() == *b.node->getMedian();
            case SubdivisionTree::AXIAL:  return *node->getAxial() == *b.node->getAxial();
            case SubdivisionTree::PLANE:  return *node->getPlane() == *b.node->getPlane();
            }

            UMBRA_ASSERT(0);
            return false;
        }
    };

    static SubdivisionTree::Node* unifyNodes(SubdivisionTree::Node* node, Hash<UnifiedNode, SubdivisionTree::Node*>& hash)
    {
        if (node->isInner())
        {
            SubdivisionTree::InnerNode* inner = node->getInner();
            inner->setLeft(unifyNodes(inner->getLeft(), hash));
            inner->setRight(unifyNodes(inner->getRight(), hash));
        }

        UnifiedNode un;
        un.node = node;

        SubdivisionTree::Node** v = hash.get(un);
        if (v)
            return *v;

        hash.insert(un, node);
        return node;
    }

    static SubdivisionTree::Node* unifyNodes(Allocator* allocator, SubdivisionTree::Node* node)
    {
        Hash<UnifiedNode, SubdivisionTree::Node*> nodeUnification(allocator);
        return unifyNodes(node, nodeUnification);
    }

    // Checks that axial splits actually split space.

    bool hasInvalidNodes(const SubdivisionTree::Node* node, const AABB& aabb) const
    {
        if (node->isLeaf())
            return false;

        if (node->isPlane())
            return hasInvalidNodes(node->getPlane()->getLeft(), aabb) ||
                   hasInvalidNodes(node->getPlane()->getRight(), aabb);

        AABB left, right;
        splitBounds(node, aabb, left, right);

        return hasInvalidNodes(node->getInner()->getLeft(), left) ||
               hasInvalidNodes(node->getInner()->getRight(), right);
    }

    bool hasInvalidNodes() const
    {
        return hasInvalidNodes(m_st.getRoot(), m_st.getAABB());
    }

    SubdivisionTree::Node* changeMediansToAxials(SubdivisionTree::Node* node, const AABB& aabb, const Set<const SubdivisionTree::Node*>& changeSet)
    {
        if (node->isLeaf())
            return node;

        if (node->isPlane())
        {
            UMBRA_ASSERT(!"plane nodes should have been removed at this point");
            return node;
        }

        UMBRA_ASSERT(node->isMedian() || node->isAxial());

        AABB left, right;
        splitBounds(node, aabb, left, right);

        node->getInner()->setLeft(changeMediansToAxials(node->getInner()->getLeft(), left, changeSet));
        node->getInner()->setRight(changeMediansToAxials(node->getInner()->getRight(), right, changeSet));

        if (node->isMedian() && changeSet.contains(node))
        {
            SubdivisionTree::AxialNode* axial = m_st.newAxial();

            int axis = node->getMedian()->getAxis();
            axial->setAxis(axis);
            axial->setPos(aabb.getCenter()[axis]);

            axial->setLeft(node->getInner()->getLeft());
            axial->setRight(node->getInner()->getRight());

            return axial;
        }

        return node;
    }

    // Changes all medians to axial splits before last axial split (level order).

    void forceTopLevelSplitsToAxials(Allocator* a)
    {
        Array<const SubdivisionTree::Node*> ary(a);
        getLevelOrder(m_st.getRoot(), ary);

        int lastAxial = -1;

        for (int i = 0; i < ary.getSize(); i++)
            if (ary[i]->isAxial())
                lastAxial = i;

        if (lastAxial < 0)
            return;

        Set<const SubdivisionTree::Node*> changeSet(a);

        for (int i = 0; i <= lastAxial; i++)
            if (ary[i]->isMedian())
                changeSet.insert(ary[i]);

        m_st.setRoot(changeMediansToAxials(m_st.getRoot(), m_st.getAABB(), changeSet));
    }

    static void remapLeafIndices (SubdivisionTree& tree, const Array<int>& remap)
    {
        // TODO: implement remapping in serialized format
        for (SubdivisionTree::LeafIterator iter = tree.iterateLeaves(); !iter.end(); iter.next())
        {
            if (iter.leaf()->getIndex() < 0)
                continue;
            UMBRA_ASSERT(iter.leaf()->getIndex() < remap.getSize());
            iter.leaf()->setIndex(remap[iter.leaf()->getIndex()]);
        }
    }

private:
    SubdivisionTree& m_st;

    SubdivisionTreeUtils& operator=(const SubdivisionTreeUtils&); // deny
};

template <> inline unsigned int getHashValue (const SubdivisionTreeUtils::UnifiedNode& n)
{
    uint32 a = 0xa9a26f44, b = 0xdb71a632, c = 0xba687907;

    switch (n.node->getType())
    {
    case SubdivisionTree::LEAF:   shuffleInts(a, b, c, (const uint32*)n.node->getLeaf(), sizeof(SubdivisionTree::LeafNode) / sizeof(uint32)); break;
    case SubdivisionTree::MEDIAN: shuffleInts(a, b, c, (const uint32*)n.node->getMedian(), sizeof(SubdivisionTree::MedianNode) / sizeof(uint32)); break;

    // AXIAL and PLANE have floats, which must be hashed differently.

    case SubdivisionTree::AXIAL:
        shuffleInts(a, b, c, (const uint32*)n.node->getInner(), sizeof(SubdivisionTree::InnerNode) / sizeof(uint32));
        a += getHashValue(n.node->getAxial()->m_pos);
        break;
    case SubdivisionTree::PLANE:
        shuffleInts(a, b, c, (const uint32*)n.node->getInner(), sizeof(SubdivisionTree::InnerNode) / sizeof(uint32));
        a += getHashValue(n.node->getPlane()->m_pleq);
        break;
    }

    return a;
}

/*
 * SubdivisionTreeSerialization.
 */

class SubdivisionTreeSerialization
{
public:
    SubdivisionTreeSerialization(Allocator* a = 0) : m_leafBits(0), m_indexOffset(0), m_bits(0, a), m_numBits(0) {}
    ~SubdivisionTreeSerialization() {}

    void setAllocator(Allocator* a)
    {
        m_bits.setAllocator(a);
    }

    bool isEmpty() const
    {
        return m_bits.getSize() == 0;
    }

    void join(int axis, float pos, int firstRightCell, const SubdivisionTreeSerialization& left, const SubdivisionTreeSerialization& right)
    {
        UMBRA_ASSERT(left.m_aabb.getMax()[axis] == right.m_aabb.getMin()[axis]);
        UMBRA_ASSERT(left.m_aabb.getFaceRect((axis<<1)|1) == right.m_aabb.getFaceRect(axis<<1));

        m_aabb = left.m_aabb;
        m_aabb.grow(right.m_aabb);

        m_bits = BitVector();
        m_numBits = 0;

        m_leafBits = left.m_leafBits;
        m_indexOffset = left.m_indexOffset;

        BitOutputStream s(m_bits);

        s.write3((int)AXIAL);
        s.write2(axis);
        s.write(floatBitPattern(pos), 32);

        s.writeBits(left.m_bits, left.m_numBits);

        s.write3((int)SUBTREE);
        s.write(right.m_leafBits, 32);
        s.write(right.m_indexOffset, 32);
        s.write(firstRightCell, 32);

        s.writeBits(right.m_bits, right.m_numBits);

        m_numBits = s.getBitCount();
    }

    void serialize(const SubdivisionTree& st)
    {
        m_bits = BitVector();
        m_numBits = 0;

        SubdivisionTree::LeafIterator iter;

        m_indexOffset = -1;
        int maxIndex = INT_MIN;
        for (st.iterate(iter); !iter.end(); iter.next())
        {
            m_indexOffset = min2(m_indexOffset, iter.leaf()->getIndex());
            maxIndex = max2(maxIndex, iter.leaf()->getIndex());
        }
        UMBRA_ASSERT(maxIndex >= m_indexOffset);
        m_leafBits = bitsForValue(maxIndex - m_indexOffset);
        m_aabb = st.getAABB();

        BitOutputStream stream(m_bits);
        serialize(stream, st.getRoot());
        m_numBits = stream.getBitCount();
    }

    void deserialize(SubdivisionTree& st) const
    {
        UMBRA_ASSERT(!isEmpty());
        st.setAABB(m_aabb);
        BitInputStream stream(m_bits);
        st.setRoot(deserialize(stream, st, m_leafBits, m_indexOffset, 0));
    }

    bool canRemapIndices() const
    {
        BitInputStream stream(m_bits);
        return canRemapIndices(stream, m_leafBits);
    }

    void remapIndices(const Array<int>& remap)
    {
        if (remap.getSize() == 0)
            return;

        int newIndexOffset = m_indexOffset;
        for (int i = 0; i < remap.getSize(); i++)
            newIndexOffset = min2(newIndexOffset, remap[i]);

        BitInputStream stream(m_bits);
        remapIndices(stream, m_leafBits, m_indexOffset, remap, newIndexOffset);

        m_indexOffset = newIndexOffset;
    }

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, m_aabb);
        stream(op, m_leafBits);
        stream(op, m_indexOffset);
        stream(op, m_bits);
        stream(op, m_numBits);
    }

    const AABB& getAABB() const { return m_aabb; }

private:
    enum
    {
        LEAF    = 0,
        MEDIAN  = 1,
        AXIAL   = 2,
        PLANE   = 3,
        SUBTREE = 4
    };

    void serialize(BitOutputStream& s, const SubdivisionTree::Node* node)
    {
        if (node->isLeaf())
        {
            s.write3((int)LEAF);
            s.write(node->getLeaf()->getIndex() - m_indexOffset, m_leafBits);
            return;
        }

        if (node->isMedian())
        {
            s.write3((int)MEDIAN);
            s.write2(node->getMedian()->getAxis());
        }
        else if (node->isAxial())
        {
            s.write3((int)AXIAL);
            s.write2(node->getAxial()->getAxis());
            s.write(floatBitPattern(node->getAxial()->getPos()), 32);
        }
        else if (node->isPlane())
        {
            s.write3((int)PLANE);
            s.write(floatBitPattern(node->getPlane()->getPleq().x), 32);
            s.write(floatBitPattern(node->getPlane()->getPleq().y), 32);
            s.write(floatBitPattern(node->getPlane()->getPleq().z), 32);
            s.write(floatBitPattern(node->getPlane()->getPleq().w), 32);
        }
        else
            UMBRA_ASSERT(0);

        serialize(s, node->getInner()->getLeft());
        serialize(s, node->getInner()->getRight());
    }

    // indexOffset is something that is always applied and indexIncrement only
    // for non-negative values. The point is that negatives stay negative.

    SubdivisionTree::Node* deserialize(BitInputStream& s, SubdivisionTree& st, int leafBits, int indexOffset, int indexIncrement) const
    {
        SubdivisionTree::Node* node = 0;

        switch (s.read3())
        {
        case LEAF:
            node = st.newLeaf();
            {
                int idx = s.read(leafBits) + indexOffset;
                // Only apply index increment for non-negative indices.
                if (idx >= 0)
                    idx += indexIncrement;
                node->getLeaf()->setIndex(idx);
            }
            return node;

        case MEDIAN:
            node = st.newMedian();
            node->getMedian()->setAxis(s.read2());
            break;

        case AXIAL:
            node = st.newAxial();
            node->getAxial()->setAxis(s.read2());
            node->getAxial()->setPos(bitPatternFloat(s.read(32)));
            break;

        case PLANE:
            node = st.newPlane();
            {
                Vector4 pleq;
                pleq.x = bitPatternFloat(s.read(32));
                pleq.y = bitPatternFloat(s.read(32));
                pleq.z = bitPatternFloat(s.read(32));
                pleq.w = bitPatternFloat(s.read(32));
                node->getPlane()->setPleq(pleq);
            }
            break;

        case SUBTREE:
            {
                leafBits       = s.read(32);
                indexOffset    = s.read(32);
                indexIncrement += s.read(32);
                node = deserialize(s, st, leafBits, indexOffset, indexIncrement);
            }
            return node;
        }

        UMBRA_ASSERT(node);

        node->getInner()->setLeft(deserialize(s, st, leafBits, indexOffset, indexIncrement));
        node->getInner()->setRight(deserialize(s, st, leafBits, indexOffset, indexIncrement));

        return node;
    }

    void remapIndices(BitInputStream& s, int leafBits, int indexOffset, const Array<int>& remap, int newIndexOffset)
    {
        switch (s.read3())
        {
        case LEAF:
            {
                int pos = s.getPosition();

                int origIdx = s.read(leafBits) + indexOffset;

                int mappedIdx = (origIdx >= 0) ? remap[origIdx] : origIdx;

                mappedIdx -= newIndexOffset;

                UMBRA_ASSERT(mappedIdx >= 0);
                UMBRA_ASSERT(mappedIdx < (1 << leafBits));

                Umbra::UINT32 idx2 = (Umbra::UINT32)mappedIdx;

                copyBitRange(m_bits.getArray(), pos, &idx2, 0, leafBits);
            }
            return;

        case MEDIAN:
            s.read(2);
            break;

        case AXIAL:
            s.read(2);
            s.read(32);
            break;

        case PLANE:
            s.read(32);
            s.read(32);
            s.read(32);
            s.read(32);
            break;

        case SUBTREE:
            UMBRA_ASSERT(!"in-place index remapping in subtree is not supported");
            return;
        }

        remapIndices(s, leafBits, indexOffset, remap, newIndexOffset);
        remapIndices(s, leafBits, indexOffset, remap, newIndexOffset);
    }

    bool canRemapIndices(BitInputStream& s, int leafBits) const
    {
        switch (s.read3())
        {
        case LEAF:
            s.read(leafBits);
            return true;

        case MEDIAN:
            s.read(2);
            break;

        case AXIAL:
            s.read(2);
            s.read(32);
            break;

        case PLANE:
            s.read(32);
            s.read(32);
            s.read(32);
            s.read(32);
            break;

        case SUBTREE:
            return false;
        }

        return canRemapIndices(s, leafBits) && canRemapIndices(s, leafBits);
    }

    AABB             m_aabb;
    int              m_leafBits;
    int              m_indexOffset;
    BitVector        m_bits;
    int              m_numBits;
};

}
