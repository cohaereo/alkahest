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
 * \brief   Umbra BSP trees
 *
 */

#include "umbraBSPTree.hpp"
#include "umbraTomePrivate.hpp"
#include <string.h>

using namespace Umbra;

namespace Umbra
{

class NodeLocator;
class CommonAncestorFinder;
class NeighborCollector;

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Get the count of pairwise set bits in the bit field a
 * \param   a   bitfield
 * \return  number of set bit pairs
 *//*-------------------------------------------------------------------*/

static inline int countBitPairsSet(Umbra::UINT32 a)
{
    a = ((a >> 1) & 0x55555555u) & (a & 0x55555555u);
    a = ((a >> 2) & 0x33333333u) + (a & 0x33333333u);
    a += a >> 4;
    a &= 0x0f0f0f0fu;
    a += a >> 8;
    a += a >> 16;
    a &= 0xff;
    return (int)(a);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Lookup table implementation for bit ranks in KD tree
 *//*-------------------------------------------------------------------*/

class KDRankLUT
{
public:
    KDRankLUT (UINT32* data, int numNodes): data(data), numNodes(numNodes) {}

    static int getSize (int numNodes)
    {
        return getEndOffset(numNodes) * sizeof(UINT32);
    }

    inline int lookup (int val)
    {
        int ret = 0;
        if (val & 0xFFFF0000)
        {
            int idx = (val >> 16) - 1;
            ret += getTopValue(idx);
        }
        if (val & 0x0000FF00)
        {
            int idx = (val >> 8) - (val >> 16) - 1;
            ret += getMidValue(idx);
        }
        if (val & 0x000000F0)
        {
            int idx = (val >> 4) - (val >> 8) - 1;
            ret += getBottomValue(idx);
        }
        return ret;
    }

    void construct (const UINT32* nodes)
    {
        if (!numNodes)
            return;

        UINT32 dwords = UMBRA_BITVECTOR_DWORDS(numNodes*2);
        int topIdx = 0;
        int midIdx = 0;
        int bottomIdx = 0;
        int curTop = 0;
        int curMid = 0;
        int curVal = 16 - countBitPairsSet(nodes[0]);

        memset(data, 0, getEndOffset(numNodes) * sizeof(UINT32));

        for (UINT32 i = 1; i < dwords; i++)
        {
            if ((i & 0xFFF) == 0)
            {
                curTop = curVal;
                curMid = curVal;
                setTopValue(topIdx++, curTop);
            }
            else if ((i & 0xF) == 0)
            {
                curMid = curVal;
                setMidValue(midIdx++, (UINT16)(curMid - curTop));
            }
            else
            {
                setBottomValue(bottomIdx++, (UINT8)(curVal - curMid));
            }

            curVal += 16 - countBitPairsSet(nodes[i]);
        }
    }

private:

    static UINT32 NUM_TOP_ENTRIES(UINT32 x)         { return x >> 16;               }
    static UINT32 NUM_MID_ENTRIES(UINT32 x)         { return (x >> 8) - (x >> 16);  }
    static UINT32 NUM_BOTTOM_ENTRIES(UINT32 x)      { return (x >> 4) - (x >> 8);   }

    static UINT32 getMidOffset(UINT32 nodes)        { return NUM_TOP_ENTRIES(nodes); }
    static UINT32 getBottomOffset(UINT32 nodes)     { return getMidOffset(nodes) + ((NUM_MID_ENTRIES(nodes)+1) >> 1); }
    static UINT32 getEndOffset(UINT32 nodes)        { return getBottomOffset(nodes) + ((NUM_BOTTOM_ENTRIES(nodes)+3) >> 2); }

    UINT32        getValue(int idx) const           { return data[idx]; }
    void          setValue(int idx, UINT32 value)   { data[idx] = value; }
    UINT32        getTopValue(int idx) const        { return getValue(idx); }
    void          setTopValue(int idx, UINT32 val)  { setValue(idx, val); }

    UINT32 getMidValue(int idx) const
    {
        UINT32 t_ofs = getMidOffset(numNodes) + (idx >> 1);
        UINT32 b_ofs = (idx & 1) << 4;
        UINT32 value = getValue(t_ofs);
        return ((value >> b_ofs) & 0xFF) | (((value >> (b_ofs + 8)) & 0xFF) << 8);
    }

    void setMidValue(int idx, UINT16 val)
    {
        UINT32 t_ofs = getMidOffset(numNodes) + (idx >> 1);
        UINT32 b_ofs = (idx & 1) << 4;
        UINT32 value = getValue(t_ofs);
        value |= (val & 0xFF) << b_ofs;
        value |= ((val >> 8) & 0xFF) << (b_ofs + 8);
        setValue(t_ofs, value);
    }

    UINT32 getBottomValue(int idx) const
    {
        UINT32 t_ofs = getBottomOffset(numNodes) + (idx >> 2);
        UINT32 b_ofs = (idx & 3) << 3;
        return (getValue(t_ofs) >> b_ofs) & 0xFF;
    }

    void setBottomValue(int idx, UINT8 val)
    {
        UINT32 t_ofs = getBottomOffset(numNodes) + (idx >> 2);
        UINT32 b_ofs = (idx & 3) << 3;
        UINT32 value = getValue(t_ofs);
        value |= (val << b_ofs);
        setValue(t_ofs, value);
    }

    UINT32* data;
    int numNodes;
};


} // namespace Umbra

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Build lut
 *//*-------------------------------------------------------------------*/

void Umbra::KDTree::buildLut (Umbra::UINT32* lut, const Umbra::UINT32* nodes, int numNodes)
{
    KDRankLUT rank(lut, numNodes);
    rank.construct(nodes);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Find maximum tree depth
 *//*-------------------------------------------------------------------*/

int Umbra::KDTree::getMaxDepth (void) const
{
    struct StackElem
    {
        int counter;
        int idx;
    };
    StackElem stack[UMBRA_MAX_KDTREE_DEPTH];

    int level = 0, maxLevel =  0;
    stack[0].idx = 0;
    stack[0].counter = 2;

    while (level >= 0)
    {
        StackElem& elem = stack[level];

        KDTree::Split s = getSplit(elem.idx);
        if (s == KDTree::LEAF)
        {
            elem.counter = 0;
            if (level > maxLevel)
                maxLevel = level;
        }

        if (elem.counter == 0)
        {
            level--;
            continue;
        }

		if (level + 1 >= UMBRA_MAX_KDTREE_DEPTH)
			return -1;

		UMBRA_ASSERT(level + 1 < UMBRA_MAX_KDTREE_DEPTH);

        StackElem& newelem = stack[level + 1];
        newelem.counter = 2;
        if (elem.counter-- == 2)
            newelem.idx = getLeftChildIdx(elem.idx);
        else
            newelem.idx = getRightChildIdx(elem.idx);

        level++;
    }

    return maxLevel+1; // depth == level+1
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Build lut
 *//*-------------------------------------------------------------------*/

int Umbra::KDTree::getLUTSize (int numNodes)
{
    return KDRankLUT::getSize(numNodes);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Build array of path bitvectors, indexed by node idx
 *//*-------------------------------------------------------------------*/

void Umbra::KDTree::getPaths (UINT32* out, int bitsPerPath) const
{
    struct StackElem
    {
        int counter;
        int idx;
    };

    StackElem stack[UMBRA_MAX_KDTREE_DEPTH];
    UINT64 path = 0;

    UMBRA_CT_ASSERT((sizeof(UINT64)*8) >= UMBRA_MAX_KDTREE_DEPTH);
    int level = 0;

    stack[0].idx = 0;
    stack[0].counter = 2;

    while (level >= 0)
    {
        StackElem& elem = stack[level];

        KDTree::Split s = getSplit(elem.idx);
        if (s == KDTree::LEAF)
            elem.counter = 0;
        path = (path & ((1ull << level) - 1ull));
        UMBRA_ASSERT(level < bitsPerPath);

        // clear this nodes's part of vector
        int nodeOfs = elem.idx * bitsPerPath;
        clearBitRange(out, nodeOfs, nodeOfs + bitsPerPath);
        // path length as (bitsPerPath - #zerobits - 1)
        setBit(out, nodeOfs + bitsPerPath - level - 1);
        // the actual path
        packElem(out, nodeOfs + bitsPerPath - level, path, level);

        if (elem.counter == 0)
        {
            level--;
            continue;
        }

        StackElem& newelem = stack[level + 1];
        newelem.counter = 2;
        if (elem.counter-- == 2)
        {
            newelem.idx = getLeftChildIdx(elem.idx);
            path |= (1ull << level);
        }
        else
        {
            newelem.idx = getRightChildIdx(elem.idx);
            path &= ~(1ull << level);
        }
        level++;
    }
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Get rank of (number of 1 bits left to) index
 *//*-------------------------------------------------------------------*/

int Umbra::KDTree::rank(int e_idx) const
{
    KDRankLUT lut((UINT32*)m_lut, m_numNodes);
    int idx = e_idx + 1;
    int r = lut.lookup(idx);
    int i = ((idx << 1)) >> 5;
    r += (idx & 0xF) - countBitPairsSet(m_data[i] & ((1 << ((idx & 0xF) << 1)) - 1));
    return r;
}

#if UMBRA_ARCH != UMBRA_SPU

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

void Umbra::KDTree::Node::getDebugLines(Umbra::Vector3* list) const
{
    const Vector3& aabbMin = getAABBMin();
    const Vector3& aabbMax = getAABBMax();

#define LINE(a, b, c, d, e, f) { *list++ = Vector3(a, b, c); *list++ = Vector3(d, e, f); }

    LINE(aabbMin[0], aabbMin[1], aabbMin[2], aabbMax[0], aabbMin[1], aabbMin[2]);
    LINE(aabbMin[0], aabbMax[1], aabbMin[2], aabbMax[0], aabbMax[1], aabbMin[2]);
    LINE(aabbMin[0], aabbMin[1], aabbMax[2], aabbMax[0], aabbMin[1], aabbMax[2]);
    LINE(aabbMin[0], aabbMax[1], aabbMax[2], aabbMax[0], aabbMax[1], aabbMax[2]);

    LINE(aabbMin[0], aabbMin[1], aabbMin[2], aabbMin[0], aabbMax[1], aabbMin[2]);
    LINE(aabbMin[0], aabbMin[1], aabbMax[2], aabbMin[0], aabbMax[1], aabbMax[2]);
    LINE(aabbMax[0], aabbMin[1], aabbMin[2], aabbMax[0], aabbMax[1], aabbMin[2]);
    LINE(aabbMax[0], aabbMin[1], aabbMax[2], aabbMax[0], aabbMax[1], aabbMax[2]);

    LINE(aabbMin[0], aabbMin[1], aabbMin[2], aabbMin[0], aabbMin[1], aabbMax[2]);
    LINE(aabbMin[0], aabbMax[1], aabbMin[2], aabbMin[0], aabbMax[1], aabbMax[2]);
    LINE(aabbMax[0], aabbMin[1], aabbMin[2], aabbMax[0], aabbMin[1], aabbMax[2]);
    LINE(aabbMax[0], aabbMax[1], aabbMin[2], aabbMax[0], aabbMax[1], aabbMax[2]);

#undef LINE
}

#endif  // UMBRA_ARCH != UMBRA_SPU

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/


bool Umbra::NodeLocator::findNode(const KDTree& tree, const AABB& bounds,
    const Vector3& coord, KDTree::Node& out)
{
    m_traversal.init(tree, bounds, PointTraverse<>(coord));
    return m_traversal.next(out);
}
