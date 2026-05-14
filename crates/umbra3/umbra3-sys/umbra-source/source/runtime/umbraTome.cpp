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
 * \brief   Umbra tome implementation
 *
 */

#include "umbraTomePrivate.hpp"
#include "umbraQueryContext.hpp"
#include "umbraChecksum.hpp"

UMBRA_CT_ASSERT((sizeof(Umbra::ImpTome) & 0xF) == 0);
UMBRA_CT_ASSERT((sizeof(Umbra::ImpTile) & 0xF) == 0);
UMBRA_CT_ASSERT((sizeof(Umbra::ExtTile) & 0xF) == 0);
UMBRA_CT_ASSERT((sizeof(Umbra::TomeContext) & 0xF) == 0);

using namespace Umbra;

namespace Umbra
{
const SIMDRegister g_intScale = SIMDLoad(1.f / 65535.0f);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

float Portal::getZ(const Vector3& pmn, const Vector3& pmx) const
{
	UMBRA_ASSERT(!isUser());
	int axis = getFaceAxis(getFace());
	return fixedLerp((int)(idx_z & 0xFFFF), pmn[axis], pmx[axis]);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

void Portal::getQuad (const Vector3& pmn, const Vector3& pmx, float portalExpand, Vector3& x0y0, Vector3& x0y1, Vector3& x1y1, Vector3& x1y0) const
{
    getMinMax(pmn, pmx, portalExpand, x0y0, x1y1);

    int a = ((int)getFace())>>1;
    int u = (a+1)%3;
    int v = (a+2)%3;

    x0y1 = x0y0;
    x0y1[v] = x1y1[v];
    x1y0 = x0y0;
    x1y0[u] = x1y1[u];
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpTome::computeCRC32 (void) const
{
    // Use everything after m_crc32
    return crc32Hash(((UINT32*)&m_crc32) + 1, getSize() - 2 * sizeof(UINT32));
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpTome::getPrivateStatistic (const ImpTome* t, PrivateStatistic type)
{
    switch (type)
    {
    case PRIVSTAT_PORTAL_GEOMETRY_DATA_SIZE:
        return t->m_numGateVertices * sizeof(Vector3);

    case PRIVSTAT_BSP_NODE_DATA_SIZE:
    case PRIVSTAT_BSP_PLANE_DATA_SIZE:
    case PRIVSTAT_BASE_TILE_COUNT:
    case PRIVSTAT_HIERARCHY_TILE_COUNT:
        {
            int sum = 0;

            for (int i = 0; i < t->getNumTiles(); i++)
            {
                const ImpTile* tile = t->getTile(i, false);
                if (!tile)
                    continue;

                if (type == PRIVSTAT_BSP_NODE_DATA_SIZE)
                    sum += tile->getNumBSPNodes() * sizeof(TempBspNode);
                else if (type == PRIVSTAT_BSP_PLANE_DATA_SIZE)
                    sum += tile->getNumPlanes() * sizeof(Vector4);
                else if (type == PRIVSTAT_BASE_TILE_COUNT)
                    sum += tile->isLeaf() ? 1 : 0;
                else if (type == PRIVSTAT_HIERARCHY_TILE_COUNT)
                    sum += tile->isLeaf() ? 0 : 1;
            }

            return sum;
        }

    case PRIVSTAT_REGULAR_PORTAL_COUNT:
    case PRIVSTAT_GATE_PORTAL_COUNT:
    case PRIVSTAT_HIERARCHY_PORTAL_COUNT:
        {
            int sum = 0;

            for (int i = 0; i < t->getNumTiles(); i++)
            {
                const ImpTile* tile = t->getTile(i, false);
                if (!tile)
                    continue;

                int numCells = tile->getNumCells();

                for (int j = 0; j < numCells; j++)
                {
                    CellNode cn;
                    tile->getCellNodes().getElem(cn, j);

                    DataArray portals = tile->getPortals(cn);

                    for (int k = 0; k < portals.m_count; k++)
                    {
                        Portal p;
                        portals.getElem(p, k);

                        if (type == PRIVSTAT_REGULAR_PORTAL_COUNT)
                            sum += (!p.isUser() && !p.isHierarchy()) ? 1 : 0;
                        else if (type == PRIVSTAT_GATE_PORTAL_COUNT)
                            sum += p.isUser() ? 1 : 0;
                        else if (type == PRIVSTAT_HIERARCHY_PORTAL_COUNT)
                            sum += p.isHierarchy() ? 1 : 0;
                    }
                }
            }

            return sum;
        }
    }

    return 0;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpTile::getNodeData (int nodeIdx) const
{
    BitDataArray cellMap = getCellMap();

    // if no portal data
    if (!cellMap)
        return (UINT32)-1;

    int width = m_viewTree.getMapWidth();
    UINT32 cell = cellMap.getElem(nodeIdx * width, width);
    if (cell & (1 << (width - 1)))
    {
        // highest bit stands for node having entry in bsp tree
        UINT32 mask = (1 << width) - 1;
        if (cell == mask)
            cell = (UINT32)-1;
        else
            cell = 0x80000000 | (cell & (mask >> 1));
    }
    return cell;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

int ImpTile::getCellIndex (int nodeIdx, const Vector3& coord) const
{
    UINT32 data = getNodeData(nodeIdx);

    if (data & 0x80000000)
    {
        if (data == 0xFFFFFFFF)
            return -1;

        // find from BSP

        int ofs = data & 0x7fffffff;

        float d = 0.f;

        for (;;)
        {
            TempBspNode cur;
            getBSPTriangles().getElem(cur, ofs);

            Vector4 pleq;
            getPlanes().getElem(pleq, cur.getPlaneIndex());

            d = dot(pleq, coord);

            // TODO: here back and front are reversed, let's rename them to negative and positive asap

            if (d < 0.f)
            {
                if (cur.isBackLeaf())
                {
                    UMBRA_ASSERT(cur.getBack() > -1);
                    return cur.getBack() == 0xffff ? -1 : cur.getBack();
                }
                else
                    ofs = cur.getBack();
            }
            else
            {
                if (cur.isFrontLeaf())
                {
                    UMBRA_ASSERT(cur.getFront() > -1);
                    return cur.getFront() == 0xffff ? -1 : cur.getFront();
                }
                else
                    ofs = cur.getFront();
            }
        }
    }

    return data;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpTile::getPortalDataSize (const ImpTile* t)
{
    DataArray cells(t, t->m_cells, sizeof(CellNode), t->getNumCells());
    if (!cells)
        return 0;
    int numCells = t->getNumCells();
    UMBRA_ASSERT(numCells);
    CellNode data;
    cells.getElem(data, numCells - 1);
    return numCells * sizeof(CellNode) +
        data.getLastPortal() * sizeof(Portal);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpTile::getAccurateFindDataSize (const ImpTile* t)
{
    return t->m_numBspNodes * sizeof(TempBspNode) +
        t->m_numPlanes * sizeof(Vector4);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpTile::getViewTreeDataSize (const ImpTile* t)
{
    return KDTree::getDataDwords(t->m_viewTree.getNodeCount()) * sizeof(UINT32) +
        UMBRA_BITVECTOR_SIZE(((t->m_viewTree.getNodeCount() + 1) / 2) * t->m_viewTree.getMapWidth());
}

