#if !defined(UMBRA_EXCLUDE_COMPUTATION)

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

#include "umbraReachabilityAnalysis.hpp"
#include "umbraLogger.hpp"

#define LOGE(...) UMBRA_LOG_E(getCtx()->getPlatform().logger, __VA_ARGS__)
#define LOGI(...) UMBRA_LOG_I(getCtx()->getPlatform().logger, __VA_ARGS__)
#define LOGW(...) UMBRA_LOG_W(getCtx()->getPlatform().logger, __VA_ARGS__)
#define LOGD(...) UMBRA_LOG_D(getCtx()->getPlatform().logger, __VA_ARGS__)

using namespace Umbra;

namespace Umbra
{
    float g_reachabilityAnalysisThreshold = 0.1f;

    static bool testPortalAABBVisibility(const AABB& aabb, int face, float z)
    {
        return ((face & 1)  && z >= aabb.getMin()[face>>1]) ||
               (!(face & 1) && z <= aabb.getMax()[face>>1]);
    }
}

/*---------------------------------------------------------------*//*!
 * \brief   Default constructor for analysis tile.
 *//*---------------------------------------------------------------*/

ReachabilityAnalysis::Tile::Tile()
    : m_cg(NULL), m_ecg(NULL), m_viewVolume(NULL), m_borderMask(0)
{
}

/*---------------------------------------------------------------*//*!
 * \brief   Constructor for analysis tile.
 *//*---------------------------------------------------------------*/

ReachabilityAnalysis::Tile::Tile(CellGraph& cg, ExternalCellGraph& ecg, const Array<AABB>* viewVolume, Umbra::UINT32 borderMask)
    : m_cg(&cg), m_ecg(&ecg), m_viewVolume(viewVolume), m_borderMask(borderMask)
{
}

/*---------------------------------------------------------------*//*!
 * \brief   ReachabilityAnalysis constructor
 *//*---------------------------------------------------------------*/

ReachabilityAnalysis::ReachabilityAnalysis(BuildContext* ctx)
: BuilderBase(ctx),
  m_tiles(ctx->getPlatform().allocator)
{
}

/*---------------------------------------------------------------*//*!
 * \brief   ReachabilityAnalysis destructor
 *//*---------------------------------------------------------------*/

ReachabilityAnalysis::~ReachabilityAnalysis()
{
}

/*---------------------------------------------------------------*//*!
 * \brief   Add a tile to analysis.
 *
 *          Tiles should be added in proper order, i.e. so that
 *          externel cellgraph tile indices
 *          (ecg.getCell(xyz).getPortal(abc).getTargetTile())
 *          can be used to find a tile from the added set.
 *//*---------------------------------------------------------------*/

void ReachabilityAnalysis::addTile(CellGraph& cg, ExternalCellGraph& ecg, const Array<AABB>* viewVolume, Umbra::UINT32 borderMask)
{
    m_tiles.pushBack(Tile(cg, ecg, viewVolume, borderMask));
}

void ReachabilityAnalysis::buildClusters (Array<GlobalCluster>& clusters, bool insides, bool backfaceCull)
{
    UnionFind<Vector2i> uf(getAllocator());

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i].m_cg)
            continue;

        const CellGraph& cg = *m_tiles[i].m_cg;
        const ExternalCellGraph& ecg = *m_tiles[i].m_ecg;

        for (int j = 0; j < cg.getCellCount(); j++)
        {
            Vector2i a(i, j);

            const CellGraph::Cell& cell = cg.getCell(j);

            if (insides && cell.isOutside())
                continue;
            if (backfaceCull && !m_tiles[i].m_cellVis[j].isOK())
                continue;

            for (int k = 0; k < cell.getPortalCount(); k++)
            {
                const CellGraph::Portal& p = cell.getPortal(k);
                int target = p.getTarget();
                if (insides && cg.getCell(target).isOutside())
                    continue;
                if (backfaceCull && !m_tiles[i].m_cellVis[target].isOK())
                    continue;
                if (backfaceCull && !p.isGate() && !testPortalAABBVisibility(m_tiles[i].m_cellVis[j], p.getRectPortal().getFace(), p.getRectPortal().getZ()))
                    continue;
                Vector2i b(i, target);
                uf.unionSets(a, b);
            }

            const ExternalCellGraph::Cell& cell2 = ecg.getCell(j);

            for (int k = 0; k < cell2.getPortalCount(); k++)
            {
                const ExternalCellGraph::Portal& p = cell2.getPortal(k);
                int targetTile = p.getTargetTile();
                if (targetTile == -1)
                    continue;
                int target = p.getTarget();
                if (insides && m_tiles[targetTile].m_cg->getCell(target).isOutside())
                    continue;
                if (backfaceCull && !m_tiles[targetTile].m_cellVis[target].isOK())
                    continue;
                if (backfaceCull && !testPortalAABBVisibility(m_tiles[i].m_cellVis[j], p.getFace(), p.getZ()))
                    continue;
                Vector2i b(targetTile, target);
                uf.unionSets(a, b);
            }
        }
    }

    Hash<int, int> idToOutput(getAllocator());

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i].m_cg)
            continue;
        const CellGraph& cg = *m_tiles[i].m_cg;

        for (int j = 0; j < cg.getCellCount(); j++)
        {
            if (insides && cg.getCell(j).isOutside())
                continue;

            Vector2i cell(i, j);
            int id = uf.findSet(cell);
            int outIdx = idToOutput.getDefault(id, clusters.getSize());
            if (outIdx == clusters.getSize())
                clusters.resize(outIdx + 1);
            clusters[outIdx].cells.pushBack(cell);
            if (cg.getCell(j).isForceReachable())
                clusters[outIdx].forceReachable = true;
            if (!cg.getCell(j).isOutside())
                clusters[outIdx].inside = true;
        }
    }
}

bool ReachabilityAnalysis::sanitize (void)
{
    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i].m_cg)
            continue;

        Set<int> insides(getAllocator());
        SubdivisionTree st(getAllocator());
        m_tiles[i].m_cg->getViewTree().deserialize(st);

        SubdivisionTree::LeafIterator iter;
        for (st.iterate(iter); !iter.end(); iter.next())
        {
            int idx = iter.node()->getLeaf()->getIndex();
            if (idx < 0)
                continue;
            // reachability should not be set at this point
            if (m_tiles[i].m_cg->getCell(idx).isReachable())
            {
                UMBRA_ASSERT(!"reachable cell before reachability analysis");
                return false;
            }
            if (m_tiles[i].m_cg->getCell(idx).isOutside())
            {
                UMBRA_ASSERT(!"view tree refers to outside cell");
                return false;
            }
            insides.insert(idx);
        }

        for (int j = 0; j < m_tiles[i].m_cg->getCellCount(); j++)
        {
            // All inside cells must be reachable from view tree.
            // Currently the border gate mechanism may end up creating
            // these situations, so we fix the input here.
            if (!insides.contains(j))
                m_tiles[i].m_cg->getCell(j).setOutside(true);
        }
    }
    return true;
}

void ReachabilityAnalysis::tagBorderReachables (void)
{
    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i].m_cg)
            continue;

        for (int j = 0; j < 6; j++)
        {
            if (m_tiles[i].m_borderMask & (1 << j))
            {
                SubdivisionTree st(getAllocator());
                m_tiles[i].m_cg->getMatchingTree(j).deserialize(st);

                SubdivisionTree::LeafIterator iter;
                for (st.iterate(iter); !iter.end(); iter.next())
                {
                    int cellIdx = iter.node()->getLeaf()->getIndex();
                    if (cellIdx < 0)
                        continue;
                    m_tiles[i].m_cg->getCell(cellIdx).setForceReachable(true);
                }
            }
        }
    }
}

void ReachabilityAnalysis::removeNonreachableLinks (void)
{
    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i].m_cg)
            continue;

        for (int j = 0; j < m_tiles[i].m_cg->getCellCount(); j++)
        {
            CellGraph::Cell& cell  = m_tiles[i].m_cg->getCell(j);
            ExternalCellGraph::Cell& ecell = m_tiles[i].m_ecg->getCell(j);

            if (!cell.isReachable())
                continue;

            Array<CellGraph::RectPortal> rectPortals(getAllocator());
            for (int k = 0; k < cell.getRectPortalCount(); k++)
            {
                const CellGraph::RectPortal& p = cell.getRectPortal(k);
                if (!m_tiles[i].m_cg->getCell(p.getTarget()).isReachable())
                    continue;
                rectPortals.pushBack(p);
            }

            Array<CellGraph::GatePortal> gatePortals(getAllocator());
            for (int k = 0; k < cell.getGatePortalCount(); k++)
            {
                const CellGraph::GatePortal& p = cell.getGatePortal(k);
                if (!m_tiles[i].m_cg->getCell(p.getTarget()).isReachable())
                    continue;
                gatePortals.pushBack(p);
            }

            cell.clearPortals();
            for (int k = 0; k < rectPortals.getSize(); k++)
                cell.addRectPortal(rectPortals[k]);
            for (int k = 0; k < gatePortals.getSize(); k++)
                cell.addGatePortal(gatePortals[k]);

            Array<ExternalCellGraph::Portal> eportals(getAllocator());
            for (int k = 0; k < ecell.getPortalCount(); k++)
            {
                const ExternalCellGraph::Portal& p = ecell.getPortal(k);
                int tile = p.getTargetTile();
                if (tile != -1 && !m_tiles[tile].m_cg->getCell(p.getTarget()).isReachable())
                    continue;
                eportals.pushBack(p);
            }

            ecell.clearPortals();
            for (int k = 0; k < eportals.getSize(); k++)
                ecell.addPortal(eportals[k]);
        }
    }
}

bool ReachabilityAnalysis::execute (bool borderVisibility, bool strictViewVolumes)
{
    // Sanitize input
    // Should fix cellgen to not need this!

    if (!sanitize())
        return false;

    // Tag border cells force reachable

    if (borderVisibility)
        tagBorderReachables();

    // Remove isolated inside clusters
    {
        Array<GlobalCluster> insideClusters(getAllocator());
        buildClusters(insideClusters, true, false);

        int largest = 0;
        for (int i = 0; i < insideClusters.getSize(); i++)
            largest = max2(largest, insideClusters[i].cells.getSize());

        int limit = (int)(g_reachabilityAnalysisThreshold * largest);
        int filtered = 0;

        for (int i = 0; i < insideClusters.getSize(); i++)
        {
            if (insideClusters[i].forceReachable)
                continue;
            if (insideClusters[i].cells.getSize() >= limit)
                continue;
            for (int j = 0; j < insideClusters[i].cells.getSize(); j++)
            {
                Vector2i cell = insideClusters[i].cells[j];
                m_tiles[cell.i].m_cg->getCell(cell.j).setOutside(true);
            }
            filtered++;
        }

        LOGI("Filtered %d/%d inside clusters", filtered, insideClusters.getSize());
    }

    // Global backface cull portals

    if (!borderVisibility)
        buildCellVisibility(strictViewVolumes);

    // Build global clusters and tag reachables

    Array<GlobalCluster> clusters(getAllocator());
    buildClusters(clusters, false, !borderVisibility);

    int largest = 0;
    for (int i = 0; i < clusters.getSize(); i++)
        largest = max2(largest, clusters[i].cells.getSize());

    int limit = (int)(g_reachabilityAnalysisThreshold * largest);
    int reachable = 0;

    for (int i = 0; i < clusters.getSize(); i++)
    {
        if (!clusters[i].forceReachable)
        {
            // does not contain any inside cells
            if (!clusters[i].inside)
                continue;
            // smaller than threshold
            if (clusters[i].cells.getSize() < limit)
                continue;
        }
        for (int j = 0; j < clusters[i].cells.getSize(); j++)
        {
            Vector2i cell = clusters[i].cells[j];
            m_tiles[cell.i].m_cg->getCell(cell.j).setReachable(true);
        }
        reachable++;
    }

    removeNonreachableLinks();

    LOGI("Filtered %d/%d global clusters", clusters.getSize() - reachable, clusters.getSize());

    return true;
}

void ReachabilityAnalysis::buildCellVisibility(bool strictViewVolumes)
{
    // Initial cell visibility from inside cells.

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i].m_cg)
            continue;

        const CellGraph& cg = *m_tiles[i].m_cg;

        m_tiles[i].m_cellVis.resize(cg.getCellCount());
        for (int j = 0; j < cg.getCellCount(); j++)
        {
            if (!cg.getCell(j).isOutside())
            {
                // limit cell AABB to (strict) view volumes
                const AABB& cellAABB = cg.getCell(j).getAABB();
                AABB insideAABB;
                if (!strictViewVolumes)
                    insideAABB = cellAABB;
                else
                {
                    for (int v = 0; v < m_tiles[i].m_viewVolume->getSize(); v++)
                    {
                        const AABB& vol = (*m_tiles[i].m_viewVolume)[v];
                        if (!cellAABB.intersectsWithVolume(vol))
                            continue;
                        AABB intersection = cellAABB;
                        intersection.clamp(vol);
                        insideAABB.grow(intersection);
                    }
                }
                m_tiles[i].m_cellVis[j] = insideAABB;
            }
        }
    }

    // Propagate visibility.

    bool done;
    do
    {
        done = true;

        for (int i = 0; i < m_tiles.getSize(); i++)
        {
            if (!m_tiles[i].m_cg)
                continue;

            for (int j = 0; j < m_tiles[i].m_cg->getCellCount(); j++)
                if (propagateVisibility(i, j, AABB(), 0))
                    done = false;
        }
    } while (!done);
}

bool ReachabilityAnalysis::propagateVisibility(int tile, int cellIdx, const AABB& aabb2, int depth)
{
    if (depth > 100)
        return true;

    if (tile == -1)
        return false;

    AABB& aabb = m_tiles[tile].m_cellVis[cellIdx];
    if (aabb == aabb2)
        return false;

    bool ret = false;

    if (aabb2.isOK())
    {
        AABB oldAABB = aabb;
        aabb.grow(aabb2);
        if (oldAABB != aabb)
            ret = true;
    }

    const CellGraph::Cell& cell = m_tiles[tile].m_cg->getCell(cellIdx);
    for (int i = 0; i < cell.getRectPortalCount(); i++)
    {
        const CellGraph::RectPortal& p = cell.getRectPortal(i);

        if (!testPortalAABBVisibility(aabb, p.getFace(), p.getZ()))
            continue;

        if (propagateVisibility(tile, p.getTarget(), aabb, depth+1))
            ret = true;
    }

    for (int i = 0; i < cell.getGatePortalCount(); i++)
    {
        const CellGraph::GatePortal& p = cell.getGatePortal(i);
        if (propagateVisibility(tile, p.getTarget(), aabb, depth+1))
            ret = true;
    }

    const ExternalCellGraph::Cell& ecell = m_tiles[tile].m_ecg->getCell(cellIdx);
    for (int i = 0; i < ecell.getPortalCount(); i++)
    {
        const ExternalCellGraph::Portal& p = ecell.getPortal(i);

        if (!testPortalAABBVisibility(aabb, p.getFace(), p.getZ()))
            continue;

        if (propagateVisibility(p.getTargetTile(), p.getTarget(), aabb, depth+1))
            ret = true;
    }

    return ret;
}

#endif
