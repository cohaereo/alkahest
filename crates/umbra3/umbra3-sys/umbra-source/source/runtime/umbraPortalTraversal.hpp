#pragma once
#ifndef __UMBRAPORTALTRAVERSAL_H
#define __UMBRAPORTALTRAVERSAL_H

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
 * \brief   Umbra portal graph traversal
 *
 */

#include "umbraQueryContext.hpp"
#include "umbraTransformer.hpp"

namespace Umbra
{

struct Bounds
{
    SIMDRegister mn;
    SIMDRegister mx;
};

struct PortalBounds
{
    SIMDRegister mn0;
    SIMDRegister mn1;
    SIMDRegister mn2;
    SIMDRegister size0;
    SIMDRegister size1;
    SIMDRegister size2;
};

struct PortalNode
{
    PortalNode (int slot, int local, int global): slot(slot), local(local), global(global) {}
    PortalNode (void): slot(-1) {}

    int slot;
    int local;
    int global;
};

UMBRA_FORCE_INLINE bool operator==(const PortalNode& a, const PortalNode& b)
{
    return (a.slot == b.slot) && (a.local == b.local) && (a.global == b.global);
}

static UMBRA_FORCE_INLINE bool enterPortal(
    QueryContext* ctx,
    const MappedTile& tile,
    PortalNode& target,
    const Portal& portal,
    bool isExternal,
    ArrayMapper& starts,
    const UINT32* visited,
    bool testVisitedBit=true)
{
    UMBRA_UNREF(visited);
    UMBRA_ASSERT(portal.hasTarget());
    int slotIdx = portal.getTargetTileIdx();
    if (!isExternal)
        slotIdx = tile.getMappedTome().mapLocalTile(slotIdx);
    //starts.prefetch(slotIdx);

    if (portal.isUser())
    {
        if (!ctx->isGateOpen(tile, portal))
            return false;
    }

    target.slot = slotIdx;
    target.local = portal.getTargetIndex();
    starts.get(target.global, slotIdx);
    target.global += target.local;

    if (testVisitedBit)
        return !testBit(visited, target.global);
    return true;
}

UMBRA_FORCE_INLINE void getPortalBounds (const SIMDRegister& mn, const SIMDRegister& mx, PortalBounds& bounds)
{
    SIMDRegister size = SIMDMultiply(SIMDSub(mx, mn), SIMDLoad(1 / 65535.f));

    bounds.size0 = SIMDReplicate(size, 0);
    bounds.size1 = SIMDReplicate(size, 1);
    bounds.size2 = SIMDReplicate(size, 2);
    bounds.mn0 = SIMDReplicate(mn, 0);
    bounds.mn1 = SIMDReplicate(mn, 1);
    bounds.mn2 = SIMDReplicate(mn, 2);
}

UMBRA_FORCE_INLINE void getBounds (const Vector3& mn, const Vector3& mx, PortalBounds& bounds)
{
    const SIMDRegister boundsMin = SIMDLoadW1(mn);
    const SIMDRegister boundsMax = SIMDLoadW1(mx);
    const SIMDRegister unit      = SIMDMultiply(SIMDSub(boundsMax, boundsMin), SIMDLoad(1 / 65535.f));

    bounds.size0 = SIMDReplicate(unit, 0);
    bounds.size1 = SIMDReplicate(unit, 1);
    bounds.size2 = SIMDReplicate(unit, 2);
    bounds.mn0 = SIMDReplicate(boundsMin, 0);
    bounds.mn1 = SIMDReplicate(boundsMin, 1);
    bounds.mn2 = SIMDReplicate(boundsMin, 2);
}

class CellGraphTraversal
{
public:
    CellGraphTraversal(QueryContext* q, const Vector3& cameraPos, UINT32* visitedCells)
        :   m_query(q),
            m_cameraPosGlobal(cameraPos),
            m_cellStartMap(q, q->getTome()->getCellStarts()),
            m_cellNodeMap(q, sizeof(CellNode)),
            m_extCellNodeMap(q, sizeof(ExtCellNode)),
            m_portalIter(q->getAllocator(), q->getTagManager()),
            m_objectIter(q->getAllocator(), q->getTagManager()),
            m_visitedCells(visitedCells),
            m_slot(-1),
            m_numNodes(0)
    {
    }

    ~CellGraphTraversal(void)
    {
        m_query->unmapTile(m_mappedTile);
    }

	void endTraversal()
	{
		m_query->unmapTile(m_mappedTile);
	}

    int getCluster (const PortalNode& node)
    {
        return m_mappedTile.getTile()->getClusterIndex(node.local);
    }

    void prefetchNode (const PortalNode& node)
    {
        if (m_slot != node.slot)
        {
            m_query->unmapTile(m_mappedTile);
            m_query->mapTile(m_mappedTile, node.slot);
            m_slot = node.slot;
            m_cellNodeMap.setArray(m_mappedTile.getTile()->getCellNodes());
            m_extCellNodeMap.setArray(m_mappedTile.getExtCellNodes());
            // update tile bounds
            m_slotMin = m_mappedTile.getTile()->getTreeMin();
            m_slotMax = m_mappedTile.getTile()->getTreeMax();

            getBounds(m_slotMin, m_slotMax, m_portalBounds);
            m_cameraPos.i = (int)((m_cameraPosGlobal.x - m_slotMin.x) * (65535.f / (m_slotMax.x - m_slotMin.x)));
            m_cameraPos.j = (int)((m_cameraPosGlobal.y - m_slotMin.y) * (65535.f / (m_slotMax.y - m_slotMin.y)));
            m_cameraPos.k = (int)((m_cameraPosGlobal.z - m_slotMin.z) * (65535.f / (m_slotMax.z - m_slotMin.z)));
            SIMDRegister mn = SIMDLoadW1(m_slotMin);
            SIMDRegister mx = SIMDLoadW1(m_slotMax);
            m_slotMinSIMD = mn;
            m_slotMaxSIMD = mx;
            m_portalExpand = m_mappedTile.getTile()->getPortalExpand();

        }
        m_cellNodeMap.prefetch(node.local);
        m_extCellNodeMap.prefetch(node.local);
    }

    bool enterNode (const PortalNode& node)
    {
        if (testAndSetBit(m_visitedCells, node.global))
            return false;

        CellNode cell;
        m_cellNodeMap.get(cell, node.local);
        ExtCellNode extCell;
        if (m_mappedTile.hasExternalPortals())
            m_extCellNodeMap.get(extCell, node.local);
        m_objectIter.setArray(m_mappedTile.getMappedTome().getTome()->getObjectLists(),
            m_mappedTile.getMappedTome().getTome()->getObjectListElemWidth(),
            m_mappedTile.getMappedTome().getTome()->getObjectListCountWidth(),
            cell.getObjectIndex(), cell.getObjectCount());
        m_portalIter.init(m_mappedTile, cell, &extCell);
        m_numNodes++;
        return true;
    }

    bool enterNodeNoTest (const PortalNode& node)
    {
        CellNode cell;
        m_cellNodeMap.get(cell, node.local);
        ExtCellNode extCell;
        if (m_mappedTile.hasExternalPortals())
            m_extCellNodeMap.get(extCell, node.local);

        m_objectIter.setArray(m_mappedTile.getMappedTome().getTome()->getObjectLists(),
            m_mappedTile.getMappedTome().getTome()->getObjectListElemWidth(),
            m_mappedTile.getMappedTome().getTome()->getObjectListCountWidth(),
            cell.getObjectIndex(), cell.getObjectCount());
        m_portalIter.init(m_mappedTile, cell, &extCell);
        m_numNodes++;
        return true;
    }

    void cellBounds (Vector3& mn, Vector3& mx, const PortalNode& node)
    {
        // use stored cell bounds
        m_mappedTile.getTile()->getCellBounds(mn, mx, node.local);
    }

    UMBRA_FORCE_INLINE void bounds (Vector3& mn, Vector3& mx)
    {
        mn = m_slotMin;
        mx = m_slotMax;
    }

    UMBRA_FORCE_INLINE void getTileBounds (Bounds& bounds)
    {
        bounds.mn = SIMDLoadW1(m_slotMin); //m_slotMinSIMD;
        bounds.mx = SIMDLoadW1(m_slotMax); //m_slotMaxSIMD;
    }

    UMBRA_FORCE_INLINE const MappedTile& getTile (void)
    {
        return m_mappedTile;
    }

    Vector4 debugColor (void) { return Vector4(1.f, 1.f, 1.f, 1.f); }

    RangeIterator& getObjects (void) { return m_objectIter; }
    PortalIterator& getPortals (void) { return m_portalIter; }
    ArrayMapper& getElemStartMap (void) { return m_cellStartMap; }
    const UINT32* getVisited (void) const { return m_visitedCells; }

    QueryContext*           m_query;
    Vector3                 m_cameraPosGlobal;
    Vector3                 m_slotMin; // \todo [petri] convert all code to use SIMD variants.
    Vector3                 m_slotMax;
    SIMDRegister            m_slotMinSIMD;
    SIMDRegister            m_slotMaxSIMD;
    PortalBounds            m_portalBounds;
    ArrayMapper             m_cellStartMap;
    ArrayMapper             m_cellNodeMap;
    ArrayMapper             m_extCellNodeMap;
    PortalIterator          m_portalIter;
    RangeIterator           m_objectIter;
    UINT32*                 m_visitedCells;
    int                     m_slot;
    MappedTile              m_mappedTile;
    Vector3i                m_cameraPos;
    int                     m_numNodes;
    float                   m_portalExpand;
};

UMBRA_INLINE SIMDRegister distanceAABBPointSqrSIMD(SIMDRegister p, SIMDRegister mn, SIMDRegister mx)
{
    SIMDRegister closest = SIMDMin(mx, SIMDMax(mn, p));
    SIMDRegister v = SIMDBitwiseAnd(SIMDSub(closest, p), SIMDMaskXYZ());
    return SIMDDot4(v, v);
}

UMBRA_INLINE bool distanceInRange(SIMDRegister p, const ObjectDistance& distParams, SIMDRegister scaleSqr)
{
    SIMDRegister distMn = SIMDLoadAligned((float*)&distParams.boundMin.x);
    SIMDRegister distMx = SIMDLoadAligned((float*)&distParams.boundMax.x);
    SIMDRegister distanceSqr = distanceAABBPointSqrSIMD(p, distMn, distMx);
    SIMDRegister scaled = SIMDMultiply(distanceSqr, scaleSqr);
    SIMDRegister out_near = SIMDCompareGT(SIMDReplicate(distMn, 3), scaled);
    SIMDRegister out_far = SIMDCompareGE(scaled, SIMDReplicate(distMx, 3));
    return !SIMDBitwiseOrTestAny(out_near, out_far);
}

}

#endif
