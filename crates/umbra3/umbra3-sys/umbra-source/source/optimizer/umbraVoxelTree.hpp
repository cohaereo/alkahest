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
 * \brief   Tome generation
 *
 */

#include "umbraVector.hpp"
#include "umbraArray.hpp"
#include "umbraBitMath.hpp"
#include "umbraAABB.hpp"
#include <string.h>
#include <stdio.h>

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Voxel tree used by cell generator.
 *//*-------------------------------------------------------------------*/

class VoxelTree
{
public:
    enum LeafType
    {
        EMPTY,
        SOLID,
        GATE,
        BORDER_GATE
    };

    enum
    {
        INVALID_IDX = 0x1FFFFFF
    };

    class BorderGateData;

    VoxelTree(Allocator* a): m_data(a) {}
    ~VoxelTree() {}

    class LeafData
    {
    public:
        LeafData(LeafType type)
            :   m_leaf(1),
                m_type(type),
                m_tmp(0),
                m_outside(0),
                m_index(INVALID_IDX)
        {}

        void setType(LeafType type)
        {
            // changing type to/from border gate is not allowed!
            UMBRA_ASSERT(type != BORDER_GATE && m_type != BORDER_GATE);
            m_type = type;
        }
        LeafType getType() const { return (LeafType)m_type; }
        bool isEmpty() const { return getType() == EMPTY; }
        bool isSolid() const { return getType() == SOLID; }
        bool isGate() const { return getType() == GATE || getType() == BORDER_GATE; }
        bool isBorderGate() const { return getType() == BORDER_GATE; }
        bool hasViewCell(void) const { return getType() == EMPTY || getType() == BORDER_GATE; }
        BorderGateData& getBorderGateData() const { UMBRA_ASSERT(isBorderGate()); return *(BorderGateData*)this; }

        int getViewCellIndex() const
        {
            if (isBorderGate())
                return getBorderGateData().getBorderCellIndex();
            UMBRA_ASSERT(getType() == EMPTY);
            return getIndex();
        }

        void setViewCellIndex (int idx)
        {
            if (isBorderGate())
            {
                getBorderGateData().setBorderCellIndex(idx);
            }
            else
            {
                UMBRA_ASSERT(getType() == EMPTY);
                setIndex(idx);
            }
        }

        int getExactIndex() const { UMBRA_ASSERT(isGate()); return getIndex(); }
        void setExactIndex (int idx) { UMBRA_ASSERT(isGate()); setIndex(idx); }

        void setTmp(int tmp) { m_tmp = tmp; }
        int getTmp(void) const { return m_tmp; }

        bool isOutside() const { return m_outside == 1; }
        void setOutside(bool t) { m_outside = t ? 1 : 0; }

    private:

        int getIndex(void) const { return (int)m_index; }
        void setIndex(int idx) { UMBRA_ASSERT(idx >= 0 && idx < INVALID_IDX); m_index = idx; }

        unsigned int    m_leaf        : 1;
        unsigned int    m_type        : 2;
        unsigned int    m_tmp         : 2;
        unsigned int    m_outside     : 1;
        unsigned int    m_index       : 25;
    };

    class BorderGateData: public LeafData
    {
    public:
        BorderGateData()
            :   LeafData(BORDER_GATE),
                m_cellIdx(INVALID_IDX),
                m_faceMask(0)
        {}

        int getBorderCellIndex (void) const
        {
            UMBRA_ASSERT(isBorderGate());
            return (int)m_cellIdx;
        }
        void setBorderCellIndex (int idx)
        {
            UMBRA_ASSERT(isBorderGate());
            UMBRA_ASSERT(idx >= 0 && idx < INVALID_IDX);
            m_cellIdx = idx;
        }
        UINT32 getFaceMask(void) const
        {
            UMBRA_ASSERT(isBorderGate());
            return m_faceMask;
        }
        void setFaceMask(UINT32 mask)
        {
            UMBRA_ASSERT(isBorderGate());
            m_faceMask = mask;
        }

    private:
        unsigned int m_cellIdx  : 26;
        unsigned int m_faceMask : 6;
    };

    static int getLeafSize (LeafType type)
    {
        if (type == BORDER_GATE)
            return sizeof(BorderGateData);
        return sizeof(LeafData);
    }

    static LeafData* constructLeaf(LeafType type, void* addr)
    {
        if (type == BORDER_GATE)
            return new(addr) BorderGateData();
        return new(addr) LeafData(type);
    }

    // Assert size just to be sure. Size may be changed if needed.
    UMBRA_CT_ASSERT(sizeof(LeafData) == 4);
    UMBRA_CT_ASSERT(sizeof(BorderGateData) == 8);

private:
    struct InnerData
    {
        unsigned int    m_leaf        : 1;  // zero
        unsigned int    m_size        : 31; // size
    };

    UMBRA_CT_ASSERT(sizeof(InnerData) == 4);

    void resize (int newSize)
    {
        m_data.resize(newSize);
    }

    InnerData* getInner (int p) const
    {
        UMBRA_ASSERT(p >= 0 && p+(int)sizeof(InnerData) <= m_data.getSize());
        return (InnerData*)(m_data.getPtr() + p);
    }

    LeafData* getLeaf (int p) const
    {
        UMBRA_ASSERT(p >= 0 && p+(int)sizeof(LeafData) <= m_data.getSize());
        return (LeafData*)(m_data.getPtr() + p);
    }

    Array<UINT8> m_data;

    friend class VoxelConstructor;
    friend class VoxelTraversal;
    friend class VoxelIterator;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Class to construct VoxelTree in a streamed way.
 * \note    Be very careful about the usage pattern.
 *//*-------------------------------------------------------------------*/

class VoxelConstructor
{
    VoxelConstructor& operator=(const VoxelConstructor&) { return *this; } // deny

public:
    VoxelConstructor(VoxelTree& vt) : m_voxelTree(vt), m_parent(0), m_position(0)
    {
        UMBRA_ASSERT(vt.m_data.getSize() == 0);
#ifdef UMBRA_DEBUG
        m_refCount = 0;
        m_terminated = 0;
#endif
    }

    ~VoxelConstructor()
    {
        UMBRA_ASSERT(m_refCount == 0);

        if (m_parent)
        {
#if 0 // Cannot assert this, this doesn't hold with OOM exception
            UMBRA_ASSERT(m_terminated == 8);
#endif
            UMBRA_ASSERT(m_parent->getInnerData().m_leaf == 0);
            UMBRA_ASSERT(m_parent->getInnerData().m_size == 0);

            m_parent->getInnerData().m_size = m_position - m_parent->m_position;
            m_parent->m_position += m_parent->getInnerData().m_size;
#ifdef UMBRA_DEBUG
            m_parent->m_refCount--;
#endif
        }
    }

    VoxelConstructor split()
    {
        m_voxelTree.resize(m_position + sizeof(VoxelTree::InnerData));
        getInnerData().m_leaf = 0;
        getInnerData().m_size = 0;
        UMBRA_DEBUG_CODE(m_terminated++);
        return VoxelConstructor(m_voxelTree, this, m_position + sizeof(VoxelTree::InnerData));
    }

    VoxelTree::LeafData& terminate(VoxelTree::LeafType type)
    {
        int size = VoxelTree::getLeafSize(type);
        m_voxelTree.resize(m_position + size);
        VoxelTree::LeafData* leaf = VoxelTree::constructLeaf(type, m_voxelTree.getLeaf(m_position));
        m_position += size;
        UMBRA_DEBUG_CODE(m_terminated++);
        return *leaf;
    }

private:
    VoxelConstructor(VoxelTree& vt, VoxelConstructor* p, int a) : m_voxelTree(vt), m_parent(p), m_position(a)
    {
#ifdef UMBRA_DEBUG
        m_parent->m_refCount++;
        m_refCount = 0;
        m_terminated = 0;
#endif
    }

    VoxelTree::InnerData& getInnerData()
    {
        return *m_voxelTree.getInner(m_position);
    }

private:
    VoxelTree&          m_voxelTree;
    VoxelConstructor*   m_parent;
    int                 m_position;
#ifdef UMBRA_DEBUG
    int                 m_refCount;
    int                 m_terminated;
#endif
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Class to traverse voxel tree.
 * \note    Be very careful about the usage pattern.
 *//*-------------------------------------------------------------------*/

class VoxelTraversal
{
public:
    VoxelTraversal(const VoxelTraversal& vt)
    {
        *this = vt;
    }

    VoxelTraversal& operator=(const VoxelTraversal& vt)
    {
        if (this == &vt)
            return *this;

        memcpy(this, &vt, sizeof(*this));

#ifdef UMBRA_DEBUG
        m_refCount = 0;
        if (m_parent)
            m_parent->m_refCount++;
#endif

        return *this;
    }

    VoxelTraversal(const VoxelTree& vt) : m_voxelTree(&vt), m_parent(0), m_position(0), m_childIdx(0)
    {
#ifdef UMBRA_DEBUG
        m_refCount = 0;
#endif
    }

    ~VoxelTraversal()
    {
#ifdef UMBRA_DEBUG
        if (m_parent)
            m_parent->m_refCount--;
        // \todo [Hannu] there seems to be a bug in reference counting
        //UMBRA_ASSERT(m_refCount == 0);
#endif
    }

    bool hasParent() const
    {
        return !!m_parent;
    }

    int indexInParent() const
    {
        return m_childIdx;
    }

    bool isLeaf() const
    {
        return !!getInnerData().m_leaf;
    }

    VoxelTraversal parent() const
    {
        UMBRA_ASSERT(hasParent());
        return *m_parent;
    }

    VoxelTraversal cutParentLink() const
    {
        return VoxelTraversal(m_voxelTree, 0, m_position);
    }

    VoxelTraversal firstChild() const
    {
        UMBRA_ASSERT(!isLeaf());
        return VoxelTraversal(m_voxelTree, this, m_position + sizeof(VoxelTree::InnerData));
    }

    VoxelTraversal child(int i) const
    {
        UMBRA_ASSERT(i >= 0 && i < 8);
        VoxelTraversal vt = firstChild();
        for (int j = 0; j < i; j++)
            vt.nextSibling();
        return vt;
    }

    void nextSibling()
    {
        m_childIdx++;
        UMBRA_ASSERT(m_childIdx <= 8); // 8 is allowed to ease for loops

        if (isLeaf())
            m_position += VoxelTree::getLeafSize(getLeafData().getType());
        else
        {
            UMBRA_ASSERT(getInnerData().m_size > 0);
            m_position += getInnerData().m_size;
        }
    }

    void getPath(uint64& path, int& d) const
    {
        path = 0;
        d    = 0;

        const VoxelTraversal* vt = this;

        while (vt->m_parent)
        {
            path |= (uint64)vt->indexInParent();
            path <<= 3;
            d++;
            vt = vt->m_parent;
        }
    }

    int getPathDepth() const
    {
        uint64 p;
        int d;
        getPath(p, d);
        return d;
    }

    VoxelTraversal neighborParent(UINT32 faceMask) const
    {
        const VoxelTraversal* vt = this;

        while (faceMask && vt->m_parent)
        {
            int idx = vt->indexInParent();
            for (int axis = 0; axis < 3; axis++)
            {
                if (idx & (1 << axis))
                {
                    UINT32 face = axis << 1;
                    faceMask &= ~(1 << face);
                }
                else
                {
                    UINT32 face = (axis << 1) | 1;
                    faceMask &= ~(1 << face);
                }
            }
            vt = vt->m_parent;
        }

        return *vt;
    }

    VoxelTree::LeafData& getLeafData() const
    {
        UMBRA_ASSERT(isLeaf());
        return *m_voxelTree->getLeaf(m_position);
    }

    static int pathDistance(const VoxelTraversal& a, const VoxelTraversal& b)
    {
        UMBRA_ASSERT(a.m_voxelTree == b.m_voxelTree);

        if (a.m_position == b.m_position)
            return 0;

        if (a.getPathDepth() >= b.getPathDepth())
            return pathDistance(a.parent(), b) + 1;
        else
            return pathDistance(a, b.parent()) + 1;
    }

private:
    VoxelTraversal(const VoxelTree* vt, const VoxelTraversal* parent, int pos) : m_voxelTree(vt), m_parent(parent), m_position(pos), m_childIdx(0)
    {
#ifdef UMBRA_DEBUG
        m_refCount = 0;
        if (m_parent)
            m_parent->m_refCount++;
#endif
    }

    VoxelTree::InnerData& getInnerData() const
    {
        return *m_voxelTree->getInner(m_position);
    }

private:
    const VoxelTree*        m_voxelTree;
    const VoxelTraversal*   m_parent;
    int                     m_position;
    int                     m_childIdx;
public:
    void*                   m_userData;
#ifdef UMBRA_DEBUG
    mutable int             m_refCount;
#endif

    friend class VoxelIterator;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Iterate over all voxels
 *//*-------------------------------------------------------------------*/

class VoxelIterator
{
public:
    VoxelIterator(const VoxelTree& tree): m_tree(tree)
    {
        m_position = 0;
        m_end = INT_MAX;
        m_end = isLeaf() ? VoxelTree::getLeafSize(get().getType()) : getInnerData().m_size;
        m_first = true;
    }

    VoxelIterator(const VoxelTraversal& traverse): m_tree(*traverse.m_voxelTree)
    {
        m_position = traverse.m_position;
        m_end = INT_MAX;
        m_end = m_position + (isLeaf() ? VoxelTree::getLeafSize(get().getType()) : getInnerData().m_size);
        m_first = true;
        UMBRA_ASSERT(hasMore());
    }

    bool next (void)
    {
        if (m_first)
            m_first = false;
        else
            m_position += getNodeSize();

        while (hasMore() && !isLeaf())
            m_position += getNodeSize();

        return hasMore();
    }

    bool hasMore (void) const
    {
        return m_position < m_end;
    }

    VoxelTree::LeafData& get() const
    {
        UMBRA_ASSERT(m_position >= 0 && m_position < m_end);
        UMBRA_ASSERT(isLeaf());
        return *m_tree.getLeaf(m_position);
    }

private:

    VoxelIterator(const VoxelIterator& vt);
    VoxelIterator& operator=(const VoxelIterator& vt);

    int getNodeSize() const
    {
        if (isLeaf())
            return VoxelTree::getLeafSize(get().getType());
        return sizeof(VoxelTree::InnerData);
    }

    bool isLeaf() const
    {
        return !!getInnerData().m_leaf;
    }

    VoxelTree::InnerData& getInnerData() const
    {
        return *m_tree.getInner(m_position);
    }

    const VoxelTree& m_tree;
    int m_position;
    int m_end;
    bool m_first;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Neighbor finding using VoxelTraversal
 *//*-------------------------------------------------------------------*/

template<typename Collector>
struct NeighborFinder
{
    NeighborFinder(Collector& c) : collector(c) {}

    inline void find(VoxelTraversal vt)
    {
        int mask = 63;

        uint64 st = 0;
        int sp = 0;

        while (vt.hasParent() && mask)
        {
            int idx = vt.indexInParent();
            vt = vt.parent();

            for (int axis = 0; axis < 3; axis++)
            {
                if (!(idx & (1 << axis)))
                {
                    if (!(mask & (2 << (axis*2))))
                        continue;
                }
                else
                {
                    if (!(mask & (1 << (axis*2))))
                        continue;
                }

                VoxelTraversal vt2 = vt.child(idx ^ (1 << axis));
                if (!findNeighborsRec(vt2, (axis<<1) | !(idx & (1 << axis)), st, sp))
                    return;
            }

            // \todo [Hannu] figure out the bit twiddling for this
            if (idx & 1)
                mask &= ~1;
            else
                mask &= ~2;
            if (idx & 2)
                mask &= ~4;
            else
                mask &= ~8;
            if (idx & 4)
                mask &= ~16;
            else
                mask &= ~32;

            st |= uint64(idx) << (sp*3);
            sp++;
        }

        collector.done(mask);
    }

private:
    inline bool findNeighborsRec(VoxelTraversal& vt, int face, uint64 st, int sp)
    {
        if (vt.isLeaf())
            return collector.collect(vt, face^1, sp);

        int axis = face >> 1;
        int dir = face & 1;

        if (sp <= 0)
        {
            int mask = 0;

            for (int i = 0; i < 4; i++)
            {
                int j = (!dir) << axis;
                j |= (i&1) << ((axis+1)%3);
                j |= ((i&2)>>1) << ((axis+2)%3);
                mask |= 1 << j;
            }

            VoxelTraversal vt2 = vt.firstChild();

            for (int i = 0; i < 8; i++)
            {
                if (mask & (1 << i) && !findNeighborsRec(vt2, face, st, sp-1))
                    return false;
                vt2.nextSibling();
            }

            return true;
        }

        sp--;
        int idx = (st >> (sp*3)) & 7;

        UMBRA_ASSERT(((idx & (1 << axis)) != 0) == dir);

        idx ^= 1 << axis;

        VoxelTraversal vt2 = vt.child(idx);
        return findNeighborsRec(vt2, face, st, sp);
    }

    Collector& collector;

private:
    NeighborFinder& operator=(const NeighborFinder&) { return *this; } // deny
};

/* \todo [antti 28.10.2011]: temporarily here, need Rect util class */

static inline Vector4 rectIntersection (const Vector4& a, const Vector4& b)
{
    Vector4 ret;
    ret.x = max2(a.x, b.x);
    ret.y = max2(a.y, b.y);
    ret.z = min2(a.z, b.z);
    ret.w = min2(a.w, b.w);
    return ret;
}

static inline Vector4 rectUnion (const Vector4& a, const Vector4& b)
{
    Vector4 ret;
    ret.x = min2(a.x, b.x);
    ret.y = min2(a.y, b.y);
    ret.z = max2(a.z, b.z);
    ret.w = max2(a.w, b.w);
    return ret;
}

static inline float rectArea (const Vector4& a)
{
    return (max2(0.f, a.z - a.x) * max2(0.f, a.w - a.y));
}

static inline bool rectIntersects (const Vector4& a, const Vector4& b)
{
    Vector4 i = rectIntersection(a, b);
    return rectArea(i) > 0.f;
}

static inline Vector2 rectCenter (const Vector4& rect)
{
    return Vector2((rect.x + rect.z) * 0.5f, (rect.y + rect.w) * 0.5f);
}

static inline bool rectIsValid(const Vector4& r)
{
    return r.x <= r.z && r.y <= r.w;
}

static inline Vector4 rectInvalid()
{
    return Vector4(FLT_MAX, FLT_MAX, -FLT_MAX, -FLT_MAX);
}

static inline AABB octreeSplit(const AABB& aabb, int idx)
{
    UMBRA_ASSERT(aabb.isOK());
    UMBRA_ASSERT(idx >= 0 && idx < 8);

    Vector3 p = aabb.getCenter();

    AABB aabb2 = aabb;
    for (int axis = 0; axis < 3; axis++)
        if (idx & (1 << axis))
            aabb2.setMin(axis, p[axis]);
        else
            aabb2.setMax(axis, p[axis]);

    return aabb2;
}

} // namespace Umbra
