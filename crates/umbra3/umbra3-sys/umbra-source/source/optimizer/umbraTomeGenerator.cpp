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

#include "umbraTomeGenerator.hpp"
#include "umbraLogger.hpp"
#include "umbraReachabilityAnalysis.hpp"
#include "runtime/umbraTome.hpp"
#include "umbraFPUControl.hpp"
#include "runtime/umbraQuery.hpp"
#include "optimizer/umbraObjectGrouper.hpp"
#include "umbraPortalGrouper.hpp"
#include "umbraIndexListCombiner.hpp"
#include "umbraTileProcessor.hpp"
#include "umbraCubemap.hpp"
#include "umbraVolumetricQuery.hpp"
#include "umbraComputationTile.hpp"
#include "umbraInfo.hpp"
#include "umbraFileStream.hpp"

#include <limits.h>
#include <cmath>
#include <cstdio>

#define LOGE(...) UMBRA_LOG_E(getCtx()->getPlatform().logger, __VA_ARGS__)
#define LOGI(...) UMBRA_LOG_I(getCtx()->getPlatform().logger, __VA_ARGS__)
#define LOGW(...) UMBRA_LOG_W(getCtx()->getPlatform().logger, __VA_ARGS__)
#define LOGD(...) UMBRA_LOG_D(getCtx()->getPlatform().logger, __VA_ARGS__)

using namespace Umbra;

static const int TILE_CELL_COUNT_TARGET = 96;

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TomeWriter::TomeWriter (BuildContext* ctx, const AABB& emptyAABB, float* progress)
:   BuilderBase(ctx),
    m_timer(getAllocator()),
    m_numThreads(1),
    m_cachePath(getAllocator()),
    m_tiles(getAllocator()),
    m_emptyAABB(emptyAABB),
    m_root(NULL),
    m_vertices(getAllocator()),
    m_gateAABBHash(getAllocator()),
    m_targetIdToIndex(getAllocator()),
    m_targetObjs(getAllocator()),
    m_groupToTargetIds(getAllocator()),
    m_gateIdToIndex(getAllocator()),
    m_gateIndexToId(getAllocator()),
    m_gateIdxs(getAllocator()),
    m_gateIdHash(getAllocator()),
    m_objectLists(ctx),
    m_clusterLists(ctx),
    m_unitSize(0.f),
    m_lodDistance(0.f),
    m_objectGroupCost(-1.f),
    m_objectGroupWorldSize(FLT_MAX, FLT_MAX, FLT_MAX),
    m_computeVisualizations(false),
    m_hierarchyDetail(0.f),
    m_clusterSize(0.f),
    m_minAccurateDistance(0.f),
    m_minFeatureSize(FLT_MAX),
    m_maxSmallestHole(0.f),
    m_tileTreeNodes(getAllocator()),
    m_clusterGraph(getAllocator()),
    m_progress(getAllocator()),
    m_progressPtr(progress),
    m_computationString(getAllocator()),
    m_jobLock(getAllocator()),
    m_buildTileJobs(getAllocator()),
    m_numTotalJobs(0),
    m_jobTmpMap(0),
    m_nextDepthmapJob(0),
    m_depthmapJobs(getAllocator()),
    m_depthmapCellgraph(NULL),
    m_depthmapAllocator(NULL),
    m_graphics(ctx->getMemory())
{
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TomeWriter::~TomeWriter()
{
    for (int i = 0; i < m_tiles.getSize(); i++)
        UMBRA_DELETE(m_tiles[i]);

    for (int i = 0; i < m_tileTreeNodes.getSize(); i++)
        getAllocator()->deallocate(m_tileTreeNodes[i]);
    m_tileTreeNodes.clear();
}

void TomeWriter::addTileResult(const ImpTileResult& result)
{
    // TODO: should produce error if tile size doesn't match
    UMBRA_ASSERT(m_unitSize == 0.f || m_unitSize == result.m_unitSize);
    m_unitSize = result.m_unitSize;

    Tile* t = UMBRA_NEW(Tile, getCtx());

    t->m_aabb         = AABBi(result.m_aabbMin, result.m_aabbMax);
    t->m_viewVolume   = result.m_viewVolume;
    t->m_cellGraph    = result.m_cellGraph;
    t->m_featureSize = result.m_featureSize;

    t->m_externalCellGraph = ExternalCellGraph(&t->m_cellGraph);
    t->m_slot = -1;
    t->m_imp  = 0;

    m_tiles.pushBack(t);

    m_minFeatureSize = min2(m_minFeatureSize, result.m_featureSize);
    m_maxSmallestHole = max2(m_maxSmallestHole, result.m_cellGraph.getPortalExpand());
    m_aabb.grow(t->getAABB());

    if (m_computationString == "")
        m_computationString = result.m_computationString;
    else
    {
        if (m_computationString != result.m_computationString)
        {
            m_computationString = result.m_computationString;
            m_computationString += " X";
        }
    }

    m_graphics.appendCopy(result.m_graphics);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Tome* TomeWriter::generateTome()
{
    m_progress.reset();
    m_progress.addPhase(2.f,  "Reachability analysis");
    m_progress.addPhase(43.f, "Tile collapse");
    m_progress.addPhase(43.f, "LOD");
    m_progress.addPhase(5.f,  "Compress objects");
    m_progress.addPhase(5.f,  "Compress cluster");
    if (m_depthMaps || m_depthMapsInf)
        m_progress.addPhase(100.f, "Depthmaps");
    m_progress.addPhase(2.f,  "Finish");
    m_progress.start(m_progressPtr);    

    {
        char str[1024];
        std::sprintf(str, " - %d.%d.%d F %d %d OG %g",
                Umbra::getOptimizerInfoValue(Umbra::INFOVALUE_VERSION_MAJOR),
                Umbra::getOptimizerInfoValue(Umbra::INFOVALUE_VERSION_MINOR),
                Umbra::getOptimizerInfoValue(Umbra::INFOVALUE_VERSION_REVISION),
                m_matchingData ? 1 : 0,
                m_strictViewVolumes ? 1 : 0,
                m_objectGroupCost);
        m_computationString += str;
    }

    if (m_minAccurateDistance > 0.f)
        m_lodDistance = m_minAccurateDistance;
    else
        m_lodDistance = m_minFeatureSize * 4.f;
    m_lodScaling = m_unitSize / (m_minFeatureSize * 4.f);

    // Build top-level.

    if (!buildTopLevel())
        return NULL;

    if (!m_graphics.isEmpty())
    {
        int n = 0;
        for (GraphicsContainer::Iterator iter = m_graphics.iterate(); iter; iter++)
            n++;
        FileOutputStream ofs("test.dg");
        Serializer ser(&ofs);
        stream(ser, m_graphics);
        LOGI("wrote debugging graphics to test.dg (%d primitives)", n);
    }

    // Generate.

    generateIndexLists();

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (m_tiles[i])
        {
            ImpTile* tile = generateTile(m_tiles[i]);
            if (!tile)
                return NULL;

            m_tiles[i]->m_imp = tile;
        }
    }

    ImpTome* tome = generateImpTome();

    tome = computeStaticVisibility(tome);

    if (!tome)
        return NULL;

    LOGI("created tome with %d tiles, %d targets and %d gates", m_tiles.getSize(), m_targetObjs.getSize(), m_gateIndexToId.getSize());

    for (int i = 0; i < m_tiles.getSize(); i++)
        UMBRA_DELETE(m_tiles[i]);
    m_tiles.clear();

    m_progress.advancePhase();

    return (Tome*)tome;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TomeWriter::calculateReachability()
{
    m_timer.startTimer("calculateReachability");

    ReachabilityAnalysis analysis(getCtx());

    for (int tile = 0; tile < m_tiles.getSize(); tile++)
    {
        if (m_tiles[tile])
        {
            analysis.addTile(m_tiles[tile]->m_cellGraph, m_tiles[tile]->m_externalCellGraph,
                &m_tiles[tile]->m_viewVolume, m_tiles[tile]->m_borderMask);
        }
        else
            analysis.skipTile();
    }

    bool ret = analysis.execute(m_matchingData, m_strictViewVolumes);

    m_timer.stopTimer("calculateReachability");
    double time = m_timer.getTimerValue("calculateReachability");
    UMBRA_UNREF(time);

    LOGD("reachability analysis executed in %g seconds", time);

    m_progress.advancePhase();

    return ret;
}

int TomeWriter::countCells(bool reachable)
{
    int n = 0;
    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;

        for (int j = 0; j < m_tiles[i]->m_cellGraph.getCellCount(); j++)
            if (!reachable || m_tiles[i]->m_cellGraph.getCell(j).isReachable())
                n++;
    }
    return n;
}

bool TomeWriter::buildTopLevel()
{
    // Expand with (rounded up) empty AABB

    if (m_emptyAABB.isOK())
    {
        for (int i = 0; i < 3; i++)
        {
            m_emptyAABB.setMin(i, floorf(m_emptyAABB.getMin()[i] / m_unitSize) * m_unitSize);
            m_emptyAABB.setMax(i, ceilf(m_emptyAABB.getMax()[i] / m_unitSize) * m_unitSize);
        }
        m_aabb.grow(m_emptyAABB);
    }

    // Get integer AABB.

    UMBRA_ASSERT(m_aabb.isOK());

    AABBi aabbi;
    for (int i = 0; i < 3; i++)
    {
        aabbi.setMin(i, int(floorf(m_aabb.getMin()[i] / m_unitSize)));
        aabbi.setMax(i, int(floorf(m_aabb.getMax()[i] / m_unitSize)));
    }

    // Build top-level tree.

    Array<int> sorted(getAllocator());
    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        sorted.pushBack(i);

        // ensure all tiles fit (float rounding)
        aabbi.grow(m_tiles[i]->m_aabb);
    }

    m_root = buildTree(sorted.getPtr(), sorted.getSize(), aabbi.getMin(), aabbi.getMax());

    // Get level order for TileTreeNodes.

    UMBRA_ASSERT(m_tileTreeNodes.getSize() == 0);

    {
        FIFOQueue<TileTreeNode*> fifo(getAllocator(), m_root->countNodes());

        fifo.pushBack(m_root);

        while (fifo.getSize())
        {
            TileTreeNode* node = fifo.popFront();

            m_tileTreeNodes.pushBack(node);

            if (!node->isLeaf())
            {
                fifo.pushBack(node->getLeft());
                fifo.pushBack(node->getRight());
            }
        }
    }

    // Populate tile array with leaf tiles
    /* \todo [antti 15.11.2012]: clean up the remapping here */

    {
        Array<TileTreeNode*>& ary = m_tileTreeNodes;
        Array<Tile*> titu(ary.getSize(), getAllocator());
        Hash<TileTreeNode*, int> nodeToTile(getAllocator());

        for (int i = 0; i < ary.getSize(); i++)
        {
            Tile* t = NULL;
            if (ary[i]->isLeaf())
            {
                t = ary[i]->m_tile;
                t->m_slot = i;
                t->m_isLeaf = true;
            }
            titu[i] = t;
            nodeToTile.insert(ary[i], i);
        }

        m_tiles = titu;
    }

    // Connect tiles to other tiles

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (m_tiles[i])
            connectTile(aabbi, m_root, m_tiles[i], 63, false);
    }

    // Reachability analysis.
    if (!calculateReachability())
    {
        LOGE("Reachability analysis failed");
        return false;
    }

    int totalCells = countCells(false);
    int reachableCells = countCells(true);
    LOGI("%d/%d cells reachable after global analysis\n", reachableCells, totalCells);

    // Compute neighbor cluster ids for cells.

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;

        const CellGraph& cg = m_tiles[i]->m_cellGraph;

        for (int j = 0; j < cg.getCellCount(); j++)
        {
            const CellGraph::Cell& cell = cg.getCell(j);
            UMBRA_UNREF(cell);
            UMBRA_ASSERT(cell.getClusters().getSize() == 0);
        }
    }

    int maxId = 0;

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;

        const CellGraph& cg = m_tiles[i]->m_cellGraph;
        const ExternalCellGraph& ecg = m_tiles[i]->m_externalCellGraph;

        UnionFind<int> uf(getAllocator());

        for (int j = 0; j < cg.getCellCount(); j++)
        {
            const CellGraph::Cell& cell = cg.getCell(j);
            for (int k = 0; k < cell.getRectPortalCount(); k++)
                uf.unionSets(j, cell.getRectPortal(k).getTarget());
        }

        int firstId = maxId + 1;

        for (int j = 0; j < cg.getCellCount(); j++)
        {
            int id = firstId + uf.findSet(j);

            maxId = max2(maxId, id);

            for (int k = 0; k < ecg.getCell(j).getPortalCount(); k++)
            {
                const ExternalCellGraph::Portal& portal = ecg.getCell(j).getPortal(k);
                if (portal.getTargetTile() < 0)
                    continue;

                CellGraph::Cell& cell = m_tiles[portal.getTargetTile()]->m_cellGraph.getCell(portal.getTarget());
                cell.addClusterId(id);
            }
        }
    }

    // Sort and unique clusters ids.

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;

        CellGraph& cg = m_tiles[i]->m_cellGraph;

        for (int j = 0; j < cg.getCellCount(); j++)
        {
            CellGraph::Cell& cell = cg.getCell(j);
            Array<int>& clusters = cell.getClusters();

            if (clusters.getSize() <= 1)
                continue;

            quickSort(clusters.getPtr(), clusters.getSize());

            int n = 1;
            for (int k = 1; k < clusters.getSize(); k++)
                if (clusters[n-1] != clusters[k])
                {
                    swap2(clusters[n], clusters[k]);
                    n++;
                }

            clusters.resize(n);
        }
    }

    // Remove external cell graphs, it will be built again with higher quality.

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;
        m_tiles[i]->m_externalCellGraph = ExternalCellGraph(getAllocator());
    }

    // Remove non-reachables.

    if (totalCells != reachableCells)
    {
        Array<CellRemap> remaps(m_tiles.getSize(), getAllocator());

        for (int i = 0; i < m_tiles.getSize(); i++)
        {
            if (!m_tiles[i])
                continue;
            remaps[i].reset(m_tiles[i]->m_cellGraph.getCellCount());

            int next = 0;
            for (int j = 0; j < m_tiles[i]->m_cellGraph.getCellCount(); j++)
                if (m_tiles[i]->m_cellGraph.getCell(j).isReachable())
                    remaps[i].set(j, next++);

            if (next == m_tiles[i]->m_cellGraph.getCellCount())
                continue;

            m_tiles[i]->m_cellGraph.remapCells(remaps[i]);
            m_tiles[i]->m_cellGraph.checkConsistency(CellGraph::BIDI);
            m_tiles[i]->m_cellGraph.optimizeMemoryUsage();
        }
    }

    // Collapse small tile border cells.

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;

        PortalGrouperParams pgParams;
        pgParams.strategy = PortalGrouperParams::COLLAPSE_EXTERNALS;
        pgParams.featureSize = m_tiles[i]->m_featureSize;

        CellGraph in = m_tiles[i]->m_cellGraph;
        PortalGrouper pg(getCtx()->getPlatform(), m_tiles[i]->m_cellGraph, in, pgParams);
        pg.perform();
    }

    // Remove neighbor cluster ids from cells as they are not used anymore and
    // might be confused with actual cluster ids.

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;

        CellGraph& cg = m_tiles[i]->m_cellGraph;

        for (int j = 0; j < cg.getCellCount(); j++)
        {
            CellGraph::Cell& cell = cg.getCell(j);
            cell.clearClusters();
        }
    }

    // Collapse tiles.

#if 1
    int numOldLeafTiles = (m_tileTreeNodes.getSize() + 1) / 2;

    m_tileTreeNodes.reset(0);

    int tilesProcessed = 0;
    m_progress.nextPhase();
    collapseTree(m_root, tilesProcessed);
    m_progress.setPhaseProgress(1.f);

    // Get level order for TileTreeNodes.
    // TODO: FIX REPEATING!

    {
        FIFOQueue<TileTreeNode*> fifo(getAllocator(), m_root->countNodes());

        fifo.pushBack(m_root);

        while (fifo.getSize())
        {
            TileTreeNode* node = fifo.popFront();

            m_tileTreeNodes.pushBack(node);

            if (!node->isLeaf())
            {
                fifo.pushBack(node->getLeft());
                fifo.pushBack(node->getRight());
            }
        }
    }

    LOGI("collapsed leaf tiles %d => %d", numOldLeafTiles, (m_tileTreeNodes.getSize() + 1) / 2);

    // Populate tile array with leaf tiles
    // TODO: FIX REPEATING!

    {
        Hash<TileTreeNode*, int> nodeToTile(getAllocator());
        Array<TileTreeNode*>& ary = m_tileTreeNodes;
        Array<Tile*> titu(ary.getSize(), getAllocator());

        for (int i = 0; i < ary.getSize(); i++)
        {
            Tile* t = NULL;
            if (ary[i]->isLeaf())
            {
                t = ary[i]->m_tile;
                t->m_slot = i;
                t->m_isLeaf = true;
            }
            titu[i] = t;
            nodeToTile.insert(ary[i], i);
        }

        m_tiles = titu;
    }

    // Connect tiles to other tiles
    // TODO: FIX REPEATING!

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (m_tiles[i])
        {
            connectTile(aabbi, m_root, m_tiles[i], 63, true);
            m_tiles[i]->m_externalCellGraph.optimizeMemoryUsage();
        }
    }
#endif

    // Collect targets and gates from each cell graph.

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (m_tiles[i])
            collectObjects(m_tiles[i]);
    }

    // Compute groups from target objects.

    if (m_objectGroupCost > 0.f)
    {
        // Default to tome aabb for object grouping scale

        ObjectGrouperParams params(m_objectGroupCost, m_objectGroupWorldSize.x,
            m_objectGroupWorldSize.y, m_objectGroupWorldSize.z);
        if (!params.isWorldSizeValid())
        {
            params.worldSizeX = m_aabb.getDimensions().x;
            params.worldSizeY = m_aabb.getDimensions().y;
            params.worldSizeZ = m_aabb.getDimensions().z;
        }

        // PlatformServices for grouper

        PlatformServices platform = getCtx()->getPlatform();
        NullLogger grouperLogger;
        platform.logger = &grouperLogger;

        // Only objects that share the distance range calculation can be grouped together.
        // Objects that do not have distance bounds set by the user are anyway not guaranteed
        // to be distance culled accurately, so we can conservatively inflate the distance bounds
        // to the bounds of the grouped object.

        struct GrouperInputKey: public Pair<Vector2, AABB>
        {
            GrouperInputKey(void) {}

            GrouperInputKey(const ObjectParams& obj): Pair<Vector2, AABB>(obj.m_drawDistance, obj.m_distanceBound)
            {
                bool hasDistanceRange = (a.x > 0.f) || (a.y >= 0.f);
                if (!hasDistanceRange || !b.isOK())
                    b = AABB();
            }

            const Vector2& limit(void) const { return a; }
            const AABB& bounds(void) const { return b; }
        };

        Hash<Pair<Vector2, AABB>, ObjectGrouperInput*> inputDivision(getAllocator());
        Array<ObjectParams> groups(getAllocator());

        m_groupToTargetIds.clear();
        m_targetIdToIndex.clear();

        for (int i = 0; i < m_targetObjs.getSize(); i++)
        {
            const ObjectParams& input = m_targetObjs[i];
            GrouperInputKey divisionSpec(input);
            ObjectGrouperInput** ogin = inputDivision.get(divisionSpec);
            if (!ogin)
                ogin = inputDivision.insert(divisionSpec, UMBRA_NEW(ObjectGrouperInput, platform));

            (*ogin)->add(m_targetObjs[i].getId(), m_targetObjs[i].m_bounds.getMin(),
                    m_targetObjs[i].m_bounds.getMax(), m_targetObjs[i].getCost());
        }

        Hash<Pair<Vector2, AABB>, ObjectGrouperInput*>::Iterator iter = inputDivision.iterate();
        while (inputDivision.isValid(iter))
        {
            const GrouperInputKey& key = (const GrouperInputKey&)inputDivision.getKey(iter);
            ObjectGrouperInput* input = inputDivision.getValue(iter);
            ObjectGrouper og(platform, *input, params);
            int firstGroup = groups.getSize();
            int totalGroups = groups.getSize() + og.getGroupCount();
            groups.resize(totalGroups);
            m_groupToTargetIds.resize(totalGroups);

            // Init group params

            for (int i = 0; i < og.getGroupCount(); i++)
            {
                ObjectParams& group = groups[firstGroup + i];

                // multiple IDs
                group.m_id = (UINT32)-1;
                group.m_cost = 1.f;
                // not used at this point
                group.m_flags = ObjectParams::TARGET;
                Vector3 mn, mx;
                og.getGroupAABB(mn, mx, i);
                group.m_bounds = AABB(mn, mx);
                if (key.bounds().isOK())
                    group.m_distanceBound = key.bounds();
                else
                    group.m_distanceBound = group.m_bounds;
                group.m_drawDistance = key.limit();
            }

            // Update mapping

            for (int i = 0; i < input->getObjectCount(); i++)
            {
                UINT32 id = input->getObjectId(i);
                int groupIdx = firstGroup + og.getGroupIndex(id);
                m_groupToTargetIds[groupIdx].pushBack(id);
                m_targetIdToIndex.insert(id, groupIdx);
            }

            UMBRA_DELETE(input);
            inputDivision.next(iter);
        }

        LOGI("Grouped %d objects into %d groups", m_targetObjs.getSize(), groups.getSize());
        if (inputDivision.getNumKeys() > 1)
            LOGI("Grouping was performed on %d separate distance segments", inputDivision.getNumKeys());

        m_targetObjs = groups;
    }

    // Sort target objects.

    {
        Set<int> usedGroups(getAllocator());
        Array<int> remap(m_targetObjs.getSize(), getAllocator());
        int numRemaps = 0;

        remapObjects(m_root, usedGroups, remap, numRemaps);
        UMBRA_ASSERT(numRemaps == remap.getSize());

        Hash<UINT32, int>::Iterator iter = m_targetIdToIndex.iterate();

        while (m_targetIdToIndex.isValid(iter))
        {
            m_targetIdToIndex.getValue(iter) = remap[m_targetIdToIndex.getValue(iter)];
            m_targetIdToIndex.next(iter);
        }

        Array<ObjectParams> targetObjs(remap.getSize(), getAllocator());
        for (int i = 0; i < remap.getSize(); i++)
            targetObjs[remap[i]] = m_targetObjs[i];
        m_targetObjs = targetObjs;

        if (m_groupToTargetIds.getSize())
        {
            Array<Array<UINT32> > groupToTargetIds(remap.getSize(), getAllocator());
            for (int i = 0; i < remap.getSize(); i++)
                groupToTargetIds[remap[i]] = m_groupToTargetIds[i];
            m_groupToTargetIds = groupToTargetIds;
        }
    }

    // build reachable cell map & leaf cell map, assign cluster IDs

    m_totalClusters = 0;
    m_totalCells = 0;

    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (!m_tiles[i])
            continue;

        const CellGraph& cg = m_tiles[i]->m_cellGraph;
        m_tiles[i]->m_numClusters = m_tiles[i]->m_cellGraph.assignClusters(m_totalClusters);
        m_totalClusters += m_tiles[i]->m_numClusters;

        int numCells = cg.getCellCount();
        m_totalCells += numCells;

        if (numCells > UMBRA_MAX_CELLS_PER_TILE)
        {
            LOGE("UMBRA_MAX_CELLS_PER_TILE exceeded by tile %d: %d > %d",
                i, numCells, UMBRA_MAX_CELLS_PER_TILE);
            return false;
        }
    }

    // Compute conservative view volume for occlusion splitter.

    for (int i = 0; i < m_tiles.getSize(); i++)
        if (m_tiles[i])
            for (int j = 0; j < m_tiles[i]->m_viewVolume.getSize(); j++)
                m_viewVolume.grow(m_tiles[i]->m_viewVolume[j]);

    // build tile hierarchy and cluster graph

    m_numThreads = max2(1, min2(64, m_numThreads));

    LOGI("Generating LOD tiles using %d threads", m_numThreads);
    m_progress.nextPhase();

    {
        Hash<TileTreeNode*, int> nodeToTile(getAllocator());
        for (int i = 0; i < m_tileTreeNodes.getSize(); i++)
            nodeToTile.insert(m_tileTreeNodes[i], i);
        m_jobTmpMap = &nodeToTile;

        HierarchyStackData topData(getAllocator());
        BuildTileJob topJob(getAllocator());

        collectBuildTileJobs(m_root, &topData, &topJob);

        if (m_numThreads > 1)
        {
            BuildTileThread runner;

            Array<Thread*> threads(getAllocator());

            for (int i = 0; i < m_numThreads; i++)
            {
                Thread* th = UMBRA_NEW(Thread, getAllocator());
                th->setFunction(&runner);
                th->run(this);
                threads.pushBack(th);
            }

            for (int i = 0; i < threads.getSize(); i++)
                threads[i]->waitToFinish();

            for (int i = 0; i < threads.getSize(); i++)
                UMBRA_DELETE(threads[i]);
        }
        else
        {
            buildTileJobsThread();
        }

        UMBRA_ASSERT(topJob.state == BuildTileJob::NOT_READY_2);

        PortalGrouperParams params;
        params.debug = false;
        params.strategy = PortalGrouperParams::CLUSTER;
        PortalGrouper grouper(getCtx()->getPlatform(), m_clusterGraph, topData.cellGraph, params);
        grouper.perform();
    }

    m_progress.setPhaseProgress(1.f);
    setTileParents(m_root, NULL);

    LOGI("Connecting LOD tiles");

    // Create external portals between hierarchy levels
    for (int i = 0; i < m_tiles.getSize(); i++)
        connectInnerTiles(m_tiles[i]);

    for (int i = 0; i < m_tiles.getSize(); i++)
        pruneDisconnectedTiles(m_tiles[i]);

    return true;
}

void TomeWriter::buildTileJobsThread()
{
    UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY

    for (;;)
    {
        BuildTileJob* job = 0;

        {
            ScopedLock lock(m_jobLock);

            for (int i = m_buildTileJobs.getSize()-1; i >= 0; i--)
            {
                UMBRA_ASSERT(m_buildTileJobs[i]->state == BuildTileJob::NOT_READY ||
                             m_buildTileJobs[i]->state == BuildTileJob::NOT_READY_2 ||
                             m_buildTileJobs[i]->state == BuildTileJob::READY);

                if (m_buildTileJobs[i]->state != BuildTileJob::READY)
                    continue;

                job = m_buildTileJobs[i];
                job->state = BuildTileJob::IN_PROGRESS;

                m_buildTileJobs[i] = m_buildTileJobs[m_buildTileJobs.getSize()-1];
                m_buildTileJobs.popBack();

                break;
            }
        }

        if (!job)
            break;

        UMBRA_ASSERT(job->out->numInputs > 0);
        UMBRA_ASSERT(job->right.numInputs > 0);

        buildInnerTile(*job->out, job->right, job->node);

        {
            ScopedLock lock(m_jobLock);
            float p = (float)(m_numTotalJobs - m_buildTileJobs.getSize()) / (float)m_numTotalJobs;
            p = p * p;
            m_progress.setPhaseProgress(p);
        }

        {
            ScopedLock lock(m_jobLock);

            UMBRA_ASSERT(job->state == BuildTileJob::IN_PROGRESS);
            job->state = BuildTileJob::DONE;
            job->parent->childDone();
        }

        UMBRA_ASSERT(job->state == BuildTileJob::DONE);
        UMBRA_DELETE(job);
    }
}

void TomeWriter::collectBuildTileJobs(TileTreeNode* node, HierarchyStackData* dst, BuildTileJob* parent)
{
    if (node->isLeaf())
    {
        HierarchyStackData&             out = *dst;

        // View tree not needed, matching trees required by occlusion splitter
        node->m_tile->m_cellGraph.clone(out.cellGraph, false, m_hierarchyDetail > 0.f);
        out.ext = node->m_tile->m_externalCellGraph;
        out.numInputs = 1;
        out.inputCellCount = out.cellGraph.getCellCount();
        out.aabb = node->m_tile->m_aabb;
        out.leafTiles.pushBack(node->m_tile->m_slot);
        out.leafCellCounts.pushBack(out.cellGraph.getCellCount());
        out.hasLeaves = true;

        parent->childDone();

        return;
    }

    BuildTileJob* job = UMBRA_NEW(BuildTileJob, getAllocator());
    m_buildTileJobs.pushBack(job);
    m_numTotalJobs = m_buildTileJobs.getSize();

    job->state  = BuildTileJob::NOT_READY;
    job->node   = node;
    job->out    = dst;
    job->parent = parent;

    collectBuildTileJobs(node->m_left,  dst,         job);
    collectBuildTileJobs(node->m_right, &job->right, job);
}

/*---------------------------------------------------------------*//*!
 * \brief   Hierarchical tile building
 *//*---------------------------------------------------------------*/

/* \todo [antti 29.11.2012]: document! */
#define MIN_CHILDREN_TO_GROUP 5
#define CELL_GROUP_FACTOR 0.7f

void TomeWriter::buildInnerTile(HierarchyStackData& out, const HierarchyStackData& right, TileTreeNode* node)
{
    const Hash<TileTreeNode*, int>& map = *m_jobTmpMap;

    UMBRA_ASSERT(!node->isLeaf());

    bool hasLeaves = out.hasLeaves || right.hasLeaves;

    {
        // combine raw cell graphs

        int firstRight = out.cellGraph.getCellCount();
        out.cellGraph.joinRight(right.cellGraph, false, m_minFeatureSize);
        out.numInputs += right.numInputs;
        out.inputCellCount += right.inputCellCount;
        out.aabb.grow(right.aabb);
        out.leafTiles.append(right.leafTiles);
        out.leafCellCounts.append(right.leafCellCounts);
        out.hasLeaves = hasLeaves;

        // distance filter target objects

        float minViewDistance = getLodLevel(out.aabb) * m_lodDistance;
        for (int i = 0; i < out.cellGraph.getCellCount(); i++)
        {
            CellGraph::Cell& c = out.cellGraph.getCell(i);
            Array<int> objs(c.getObjectCount(), getAllocator());
            c.getObjects(objs);
            for (int j = 0; j < objs.getSize(); j++)
            {
                int* idx = m_targetIdToIndex.get(out.cellGraph.getTargetObject(objs[j]).getId());
                if (!idx)
                    continue;
                const ObjectParams& obj = m_targetObjs[*idx];
                if (obj.m_drawDistance.y < 0.f)
                    continue;
                const AABB& tileAABB = out.cellGraph.getAABB();
                const AABB& distAABB = obj.m_distanceBound.isOK() ? obj.m_distanceBound : obj.m_bounds;
                float l = 0.f;
                // get max distance from tile AABB to distance bounds
                for (int i = 0; i < 3; i++)
                {
                    float left = tileAABB.getMin()[i] - distAABB.getMin()[i];
                    float right = distAABB.getMax()[i] - tileAABB.getMax()[i];
                    float d = max2(0.f, max2(left, right));
                    l += d * d;
                }
                if (obj.m_drawDistance.y < minViewDistance - std::sqrt(l))
                {
                    c.removeObject(objs[j]);
                }
            }
        }

        // create portals

        ExternalCellGraph outExt(&out.cellGraph);
        for (int i = 0; i < 2; i++)
        {
            const ExternalCellGraph* ext = (i == 0) ? &out.ext : &right.ext;
            int offset = (i == 0) ? 0 : firstRight;
            for (int j = 0; j < ext->getCellCount(); j++)
            {
                const ExternalCellGraph::Cell& c = ext->getCell(j);
                for (int k = 0; k < c.getPortalCount(); k++)
                {
                    const ExternalCellGraph::Portal& ep = c.getPortal(k);
                    if (!out.leafTiles.contains(ep.getTargetTile()))
                    {
                        // still external
                        outExt.getCell(offset + j).addPortal(ep);
                        continue;
                    }
                    CellGraph::RectPortal p;
                    p.setFace(ep.getFace());
                    p.setRect(ep.getRect());
                    p.setZ(ep.getZ());
                    int targetOffset = 0;
                    for (int l = 0; l < out.leafTiles.getSize(); l++)
                    {
                        if (out.leafTiles[l] == ep.getTargetTile())
                            break;
                        targetOffset += out.leafCellCounts[l];
                    }
                    p.setTarget(targetOffset + ep.getTarget());
                    out.cellGraph.getCell(offset + j).addRectPortal(p);
                }
            }
        }
        out.ext = outExt;
    }

    // figure out if this level should be considered at all
    /* \todo [antti 19.11.2012]: think this criteria through carefully! */

    if (hasLeaves && m_clusterSize == 0.f)
    {
        if (out.numInputs < 2)
            return;
    }
    else
    {
        if (out.numInputs < MIN_CHILDREN_TO_GROUP)
            return;
    }

    out.hasLeaves = false;

    Tile* self = UMBRA_NEW(Tile, getCtx());
    self->m_slot = *map.get(node);
    self->m_aabb = out.aabb;
    self->m_featureSize = m_minFeatureSize * getLodLevel(self->m_aabb);

    // group

    PortalGrouperParams params;
    params.featureSize = self->m_featureSize;
    if (m_hierarchyDetail > 0.f)
    {
        params.featureSize /= m_hierarchyDetail;
        params.strategy = PortalGrouperParams::OCCLUSION_SPLITTER;
    }
    else
    {
        params.strategy = PortalGrouperParams::AGGRESSIVE;
    }

    params.debug = false;
    if (!m_matchingData)
        params.viewVolume = m_viewVolume;

    PortalGrouper pg(getCtx()->getPlatform(), self->m_cellGraph, out.cellGraph, params);
    pg.perform(&out.ext);

    // see if this result pleases us

    if ((self->m_cellGraph.getCellCount() > UMBRA_MAX_CELLS_PER_TILE) ||
        (!hasLeaves && (self->m_cellGraph.getCellCount() > (CELL_GROUP_FACTOR * out.inputCellCount))))
    {
        LOGD("discarding tile %d: %d, %d", self->m_slot, self->m_cellGraph.getCellCount(), out.inputCellCount);
        UMBRA_DELETE(self);
        self = NULL;
        return;
    }

    // reset stack data here

    out.numInputs = 1;
    out.inputCellCount = self->m_cellGraph.getCellCount();

    // build external cell graph for grouped graph

    self->m_externalCellGraph.setCellGraph(&self->m_cellGraph);
    for (int i = 0; i < out.ext.getCellCount(); i++)
    {
        int mapped = pg.getGrouping()[i];
        if (mapped < 0)
            continue;
        const ExternalCellGraph::Cell& c = out.ext.getCell(i);
        for (int j = 0; j < c.getPortalCount(); j++)
            self->m_externalCellGraph.getCell(mapped).addPortal(c.getPortal(j));
    }

    // build leaf cell mapping

    int cell = 0;
    for (int i = 0; i < out.leafTiles.getSize(); i++)
    {
        int leafTile = out.leafTiles[i];
        int cellCount = out.leafCellCounts[i];

        for (int j = 0; j < cellCount; j++)
        {
            int rawCell = cell + j;
            int mapped = pg.getGrouping()[rawCell];
            if (mapped == -1)
                continue;
            self->m_leafCellMap.insert(GlobalCell(leafTile, j), mapped);
        }
        cell += cellCount;
    }

    // attach result

    UMBRA_ASSERT(m_tiles[self->m_slot] == NULL);
    m_tiles[self->m_slot] = self;
    node->m_tile = self;
}

/*---------------------------------------------------------------*//*!
 * \brief   Create external portals between LOD levels for one tile
 *          (leaf or inner)
 *//*---------------------------------------------------------------*/

struct TargetData
{
    TargetData(Allocator* a = NULL, float fs = 0.f): rects(a, fs*fs) { rects.setStrategy(RectGrouper::FOUR_QUADRANTS); }

    void setAllocator (Allocator* a)
    {
        rects.setAllocator(a);
    }

    RectGrouper rects;
};

static inline void copyHeap (TargetData* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

void TomeWriter::connectInnerTiles (Tile* t)
{
    if (!t)
        return;

    for (int i = 0; i < t->m_externalCellGraph.getCellCount(); i++)
    {
        ExternalCellGraph::Cell& c = t->m_externalCellGraph.getCell(i);

        // collect portals per target tile
        Hash<int, Array<ExternalCellGraph::Portal> > portalsPerTargetTile(getAllocator());
        for (int j = 0; j < c.getPortalCount(); j++)
        {
            const ExternalCellGraph::Portal& p = c.getPortal(j);

            int leaf = p.getTargetTile();
            if (leaf == -1)
                portalsPerTargetTile.getDefault(leaf, Array<ExternalCellGraph::Portal>(getAllocator())).pushBack(p);
            else
            {
                Tile* firstTarget = m_tiles[leaf];
                UMBRA_ASSERT(firstTarget && firstTarget->isLeaf());
                float thisLod = getLodLevel(t->m_aabb);

                // Find first level to create links to.
                // For non-leaf tiles, a border portal can't lead to a smaller LOD level target.
                // Currently only doing this for simple cases where border face is fully contained
                // in the tile face. There might be ways of limiting portals in non-trivial cases too.
                if (!t->isLeaf())
                {
                    for (;;)
                    {
                        Tile* parent = firstTarget->m_parent;
                        if (!parent)
                            break;
                        if (!containsFace(t->getAABB(), parent->getAABB(), p.getFace()))
                            break;
                        if (getLodLevel(parent->m_aabb) > thisLod)
                            break;
                        firstTarget = parent;
                    }
                }

                float refLod = FLT_MAX;
                if (t->m_parent)
                {
                    refLod = getLodLevel(t->m_parent->m_aabb);
                    refLod += (t->m_parent->getAABB().getMaxAxisLength() / m_lodDistance);
                }
                // spam portals to all levels
                for (Tile* target = firstTarget; target != NULL; target = target->m_parent)
                {
                    // exclude targets that contain this tile
                    if (target->overlaps(t))
                        break;
                    if (!target->isLeaf() && (getLodLevel(target->m_aabb) > refLod))
                        break;
                    UMBRA_ASSERT(!target->getAABB().intersectsWithVolume(t->getAABB()));
                    UMBRA_ASSERT(target->getAABB().intersectsWithArea(t->getAABB()));
                    portalsPerTargetTile.getDefault(target->m_slot, Array<ExternalCellGraph::Portal>(getAllocator())).pushBack(p);
                    target->m_incomingPortals++;
                }
            }
        }

        c.clearPortals();

        // generate portals to target tile by unioning rectangles to cells
        /* \todo [antti 16.11.2012]: rethink rectangle combining strategy, currently can loose quite a bit of occlusion */
        Hash<int, Array<ExternalCellGraph::Portal> >::Iterator iter = portalsPerTargetTile.iterate();
        while (portalsPerTargetTile.isValid(iter))
        {
            Hash<int, TargetData> targetData(getAllocator());
            int targetTile = portalsPerTargetTile.getKey(iter);
            const Array<ExternalCellGraph::Portal>& portals = portalsPerTargetTile.getValue(iter);
            int face = -1;
            for (int j = 0; j < portals.getSize(); j++)
            {
                const ExternalCellGraph::Portal& p = portals[j];
                // collect per target tile, for external portals per face
                int targetCell = (targetTile == -1) ? p.getFace() : m_tiles[targetTile]->remapCell(p.getTargetTile(), p.getTarget());
                TargetData& data = targetData.getDefault(targetCell, TargetData(getAllocator(), t->m_featureSize));
                data.rects.addRect(p.getRect());
                UMBRA_ASSERT(targetTile == -1 || face == -1 || face == p.getFace());
                face = p.getFace();
            }
            Hash<int, TargetData>::Iterator targetIter = targetData.iterate();
            while (targetData.isValid(targetIter))
            {
                int targetCell = targetData.getKey(targetIter);
                TargetData& data = targetData.getValue(targetIter);
                data.rects.execute();
                for (int j = 0; j < data.rects.getResult().getSize(); j++)
                {
                    ExternalCellGraph::Portal out;
                    out.m_tileIdx = targetTile;
                    out.m_target = (targetTile == -1) ? -1 : targetCell;
                    out.m_rect = data.rects.getResult()[j];
                    out.m_face = (targetTile == -1) ? targetCell : face;
                    out.m_z = t->getAABB().getFaceDist(out.m_face);
                    t->m_externalCellGraph.getCell(i).addPortal(out);
                }

                targetData.next(targetIter);
            }
            portalsPerTargetTile.next(iter);
        }
    }
}

void TomeWriter::pruneDisconnectedTiles(Tile* t)
{
    if (!t)
        return;
    if (!t->isLeaf() && !t->m_incomingPortals)
    {
        m_tiles[t->m_slot] = NULL;

        for (int i = 0; i < m_tiles.getSize(); i++)
        {
            if (m_tiles[i] && m_tiles[i]->m_parent == t)
                m_tiles[i]->m_parent = t->m_parent;
        }

        UMBRA_DELETE(t);
    }
}

/*---------------------------------------------------------------*//*!
 * \brief   Wrapper for StructBuilder::pack that throws on OOM.
 *//*---------------------------------------------------------------*/

template<class T>
static T* builderPack(StructBuilder<T>& builder)
{
    T* packed = builder.pack();
    if (!packed)
        throw OOMException();
    return packed;
}

/*---------------------------------------------------------------*//*!
 * \brief   Wrapper for StructBuilder::setDataRef that throws on OOM.
 *//*---------------------------------------------------------------*/

static void* allocDataRef(BaseStructBuilder& builder, DataPtr& loc, Umbra::UINT32 size, bool zeroMem = false)
{
    void* block = builder.allocDataRef(loc, size, zeroMem);
    if (size && !block)
        throw OOMException();
    return block;
}

/*---------------------------------------------------------------*//*!
 * \brief   Wrapper for StructBuilder::setDataRef that throws on OOM.
 *//*---------------------------------------------------------------*/

static void setDataRef(BaseStructBuilder& builder, DataPtr& loc, const void* block, Umbra::UINT32 size)
{
    if (!builder.setDataRef(loc, block, size))
        throw OOMException();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

template<class A>
static void setBuilderArray (BaseStructBuilder& builder, DataPtr& loc, const Array<A>& arr)
{
    if (!arr.getSize())
    {
        setDataRef(builder, loc, NULL, 0);
        return;
    }
    void* buf = UMBRA_HEAP_ALLOC(arr.getAllocator(), arr.getSize() * sizeof(A));
    memcpy(buf, arr.getPtr(), arr.getSize() * sizeof(A));
    setDataRef(builder, loc, buf, arr.getSize() * sizeof(A));
}

void TomeWriter::serializeTreeData (BaseStructBuilder& builder, SerializedTreeData& out, SubdivisionTree& st, int mapWidth)
{
    if (mapWidth < 0)
    {
        int maxIndex = SubdivisionTreeUtils::findMaxLeafIndex(st.getRoot());
        if (maxIndex == -1)
        {
            // Serialize fully outside as a special empty tree.
            // Can save a small amount of work at runtime if we know
            // matching is unnecessary. Note that SerializedTreeData must
            // still exists, since it corresponds to a border face.
            out.setNodeCount(0);
            out.setMapWidth(0);
            out.m_numSplitValues = 0;
            out.m_splitValues    = DataPtr(0);
            out.m_treeData       = DataPtr(0);
            out.m_map            = DataPtr(0);
            return;
        } else
        {
            // +1 because we need to be able to write -1 as well
            maxIndex++;
            UMBRA_ASSERT(maxIndex >= 0);
            mapWidth = bitsForValue(maxIndex);
        }
    }

    // Clone here because tree might have been unified and forceTopLevelSplitsAxials() doesn't work with that
    st.setRoot(SubdivisionTreeUtils(st).clone(st.getRoot()));

    SubdivisionTreeUtils(st).forceTopLevelSplitsToAxials(getAllocator());

    Array<const SubdivisionTree::Node*> nodes(getAllocator());
    SubdivisionTreeUtils::getLevelOrder(st.getRoot(), nodes);

    out.setNodeCount(nodes.getSize());
    out.setMapWidth(mapWidth);
    out.m_numSplitValues = 0;
    out.m_splitValues = DataPtr(0);
    UINT32* data = (UINT32*)allocDataRef(builder, out.m_treeData, KDTree::getDataDwords(out.getNodeCount()) * sizeof(UINT32));
    // TODO: inline the logic of bitpackTree to here (this should be the only place where it is called for SubdivisionTree::Nodes)
    bitpackTree(data, nodes);
    KDTree::buildLut(data + UMBRA_BITVECTOR_DWORDS(out.getNodeCount() * 2), data, out.getNodeCount());

    int lastAxial = -1;
    for (int i = 0; i < nodes.getSize(); i++)
        if (nodes[i]->isAxial())
            lastAxial = i;

    Array<float> pos(getAllocator());
    for (int i = 0; i <= lastAxial; i++)
    {
        UMBRA_ASSERT(nodes[i]->isLeaf() || nodes[i]->isAxial());
        if (nodes[i]->isAxial())
            pos.pushBack(nodes[i]->getAxial()->getPos());
        else
            pos.pushBack(0.f);
    }

    out.m_numSplitValues = pos.getSize();
    setBuilderArray(builder, out.m_splitValues, pos);

    int leaves = (nodes.getSize() + 1) / 2;
    UINT32* map = (UINT32*)builder.allocDataRef(out.m_map, UMBRA_BITVECTOR_SIZE(mapWidth * leaves), true);
    int mapOfs = 0;
    for (int i = 0; i < nodes.getSize(); ++i)
        if (nodes[i]->isLeaf())
        {
            int idx = nodes[i]->getLeaf()->getIndex();
            UMBRA_ASSERT(idx >= -1);
            UMBRA_ASSERT(idx == -1 || bitsForValue(idx) <= mapWidth);
            // this writes all ones for -1 values, which is correct
            copyBitRange(map, mapOfs, (UINT32*)&idx, 0, mapWidth);
            mapOfs += mapWidth;
        }
}

void TomeWriter::generateIndexLists(void)
{
    LOGI("Compressing lists");

    m_objectLists.clear();
    m_clusterLists.clear();

    int rangeSum = 0;
    int rangeCount = 0;

    for (int tileIdx = 0; tileIdx < m_tiles.getSize(); tileIdx++)
    {
        Tile* tile = m_tiles[tileIdx];

        if (!tile)
            continue;

        const CellGraph& cg = tile->m_cellGraph;

        for (int i = 0; i < cg.getCellCount(); i++)
        {
            const CellGraph::Cell& cell = cg.getCell(i);

            Set<int>    groupSet(getAllocator());
            Array<int>  objects(getAllocator());
            const Array<int>& clusters = cell.getClusters();
            
            cell.getObjects(objects);
            int minId = INT_MAX;
            int maxId = INT_MIN;
            for (int j = 0; j < objects.getSize(); j++)
            {
                int* g = m_targetIdToIndex.get(cg.getTargetObject(objects[j]).getId());
                if (!g)
                    continue;
                minId = min2(minId, *g);
                maxId = max2(maxId, *g);
                groupSet.insert(*g);
            }

            int delta = maxId - minId;
            if (delta > 0)
            {
                rangeSum += delta;
                rangeCount++;
            }

            Array<int> groups(getAllocator());
            groupSet.getArray(groups, true);

            m_objectLists.insert (groups.getPtr(),  groups.getSize(), GlobalCell(tile->m_slot, i));

            if (clusters.getSize() > 1)
                m_clusterLists.insert(clusters.getPtr(), clusters.getSize(), GlobalCell(tile->m_slot, i));
        }
    }

    LOGI("Average index range size: %.2f\n", rangeCount ? (float)rangeSum / rangeCount : -1.f);

    m_progress.nextPhase();
    m_objectLists.combineRanged(&m_progress);
    m_progress.nextPhase();
    m_clusterLists.combineRanged(&m_progress);
}

void TomeWriter::buildDepthmapThread(void)
{
    UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY

    if (!m_depthmapJobs.getSize())
        return;

    AABB world = m_viewVolume;
    if (!world.isOK())
        world = m_depthmapTome->getAABB();
    else
    {
        world.inflate(world.getDimensions() * 0.05f);

        if (world.contains(m_depthmapTome->getAABB()))
            world = m_depthmapTome->getAABB();
    }


    // Initialize solver for this thread.
    // Allocates some common memory and precomputes some common data.
    DepthmapSolver solver(getAllocator(), m_depthmapTome, world);

    int iter = 0;
    for (;;)
    {
        int          objectIdx = 0;
        DepthmapJob* job = NULL;

        // Get new job
        ////////////////

        {
            ScopedLock lock(m_jobLock);

            // Exit if all jobs consumed
            if (m_nextDepthmapJob == m_depthmapJobs.getSize())
                return;

            // Get job
            objectIdx = m_nextDepthmapJob++;
            job = &m_depthmapJobs[objectIdx];

            // Advance progress
            m_progress.setPhaseProgress((float)m_nextDepthmapJob / (float)m_depthmapJobs.getSize());
        }
        
        // Have solver split the aabb to computation units for suitable parts.
        // (We do this so that tile traversal order is unambiguous for each piece).
        Array<AABB> units(getAllocator());
        solver.partitionComputationUnits(job->aabb, units);
        
        // Unpacked depthmap
        RawDepthmap raw;
        memset(&raw, 0, sizeof(RawDepthmap));

        // Set depthmap center (reference) point
        raw.center = job->aabb.getCenter();

        // Bail out if there're too many parts.
        // Worst-case objects would take too long, and wouldn't
        // generate useful occlusion information anyways.
        if (units.getSize() > 32)
        {
            // Generate an always visible depthmap.
            for (int face = 0; face < 6; face++)
            for (int y = 0; y < (int)DepthmapData::Resolution; y++)
            for (int x = 0; x < (int)DepthmapData::Resolution; x++)
            {
                setBit(raw.inf[face], y * (int)DepthmapData::Resolution + x);
                raw.depthmap[face][x][y] = 1.f;
            }
        } else
        {
            // Voxel size multiplier for searching cells with some target object.
            // Corresponds to m_targetInflation in umbraCellGenerator.cpp.
            const float targetInflationMultiplier = 1.01f;

            // Process computation units
            for (int i = 0; i < units.getSize(); i++)
            {
                // Find target inflation
                // We can't simply use m_maxSmallestHole - it might grow arbitralily big. 
                // outside view volumes. Find tiles intersecting the object, and pick 
                // max local smallest hole from those.

                AABB inflated = units[i];
                inflated.inflate(Vector3(m_maxSmallestHole, m_maxSmallestHole, m_maxSmallestHole) * targetInflationMultiplier);
                float localSmallestHole = 0.f;

                for (int j = 0; j < m_tiles.getSize(); j++)
                {
                    if (m_tiles[j] && 
                        m_tiles[j]->isLeaf() && 
                        m_tiles[j]->getAABB().intersects(inflated))
                        localSmallestHole = max2(localSmallestHole, m_tiles[j]->m_cellGraph.getPortalExpand());
                }

                // Call solver.

                solver.solve(raw, units[i], objectIdx, localSmallestHole * targetInflationMultiplier);
            }
        }

        if (m_depthMapsInf && !m_depthMaps)
        {
            // Output pixels.
            for (int face = 0; face < (int)DepthmapData::FaceCount; face++)
            {
                memset(job->faces[face].face.pixels, 0, sizeof(job->faces[face].face.pixels));
                for (int y = 0; y < (int)DepthmapData::Resolution; y++)
                for (int x = 0; x < (int)DepthmapData::Resolution; x++)
                {
                    int idx = y * (int)DepthmapData::Resolution + x;
                    if (testBit(raw.inf[face], idx))
                        setBit(job->faces[face].face.pixels, idx);                    
                }
            }
        } else
        {
            // Compress data - generate palette
            // using historgram equalization.
            ////////////////////////////////////

            Array<float>     values(getAllocator()); // Array of unique values
            Hash<float, int> hash(getAllocator());   // Number of values for each unique value

            // Process data for each faces
            for (int face = 0; face < (int)DepthmapData::FaceCount; face++)
            {
                values.clear();
                hash.clear();

                bool hasInf = false;

                // Collect unique values

                for (int y = 0; y < (int)DepthmapData::Resolution; y++)
                for (int x = 0; x < (int)DepthmapData::Resolution; x++)
                {
                    float value = raw.depthmap[face][x][y];

                    // Inf-pixels become FLT_MAX
                    if (testBit(raw.inf[face], y * (int)DepthmapData::Resolution + x))
                    {
                        value = FLT_MAX;
                        hasInf = true;
                    }

                    // Increment count for this value
                    int& count = hash.getDefault(value, 0);
                    count++;
                }
                
                // Get array of unique values.
                hash.getKeyArray(values);
                // Sort to increasing order.
                quickSort(values.getPtr(), values.getSize());
                
                // Clear palette.
                memset(job->faces[face].palette.palette, 0, sizeof(DepthmapData::DepthmapPalette));
            
                // Set palette size.
                job->faces[face].paletteSize = values.getSize();

                // If there are <= maximum number of values, just output data as such.
                if (values.getSize() <= (int)DepthmapData::PaletteEntries)
                {
                    for (int i = 0; i < values.getSize(); i++)
                    {
                        *hash.get(values[i]) = i;
                        job->faces[face].palette.palette[i] = floatBitPattern(values[i]) >> 16;
                    }
                } else
                {
                    // There are more than maximum number of unique values.

                    // Always reserve one entry for infinite values, if present.
                    int entries = DepthmapData::PaletteEntries - 1;
                    if (hasInf)
                    {
                        *hash.get(FLT_MAX) = entries;
                        job->faces[face].palette.palette[entries] = floatBitPattern(FLT_MAX) >> 16;
                        entries--;
                    }

                    // Histogram equalization.
                    int   mn  = *hash.get(values[0]);
                    float div = (float)((int)DepthmapData::Resolution * (int)DepthmapData::Resolution - mn);
                    int total = 0;
                    for (int j = 0; j < values.getSize(); j++)
                    {
                        if (values[j] == FLT_MAX)
                            continue;

                        int* value = hash.get(values[j]);
                        total += *value;

                        int paletteValue = 0;
                        if (div != 0.f)
                            paletteValue = (int)((float)entries * (float)(total - mn) / div);
                        UMBRA_ASSERT(paletteValue >= 0 && paletteValue < (int)DepthmapData::PaletteEntries);
                        UINT32 value16 = floatBitPattern(values[j]) >> 16;
                        job->faces[face].palette.palette[paletteValue] = max2((UINT16)value16, job->faces[face].palette.palette[paletteValue]);

                        *value = paletteValue;
                    }

                    job->faces[face].paletteSize = DepthmapData::PaletteEntries;
                }

                // Output pixels.
                for (int y = 0; y < (int)DepthmapData::Resolution; y++)
                for (int x = 0; x < (int)DepthmapData::Resolution; x++)
                {
                    // Get pixel value.
                    float depth = raw.depthmap[face][x][y];
                    if (testBit(raw.inf[face], y * (int)DepthmapData::Resolution + x))
                        depth = FLT_MAX;

                    // Get output value.
                    int value = *hash.get(depth);

                    {
                        UMBRA_ASSERT(value >= 0 && value < (int)DepthmapData::PaletteEntries);

                        // Get dword index
                        Vector3i i(x, y, face);
                        int dword = (int)DepthmapReader::getDwordIdx(i);

                        // Set data - expects 4-bit.
                        UINT32& data = job->faces[i.k].face.pixels[dword];
                        int shift = (i.i & 7) << 2; // Bit index within dword - (x % 8) * 4
                        data &= ~(0xf << shift);
                        data |= value << shift;
                    }
                }
            }
        }

        // Yield occasionally
        // This operation easily consumes all of CPU and makes computer unresponsive,
        // although I don't think this yielding is enough.
        if (!(iter % 10))
            Thread::yield();
        iter++;

    }
}

struct MapCacheHeader
{
    Umbra::UINT32 size;
    Umbra::UINT32 version;
    Umbra::UINT32 dataOffset;
    Umbra::UINT32 paletteOffset;
    Umbra::UINT32 faceOffset;
    int           numFaces;
};

static const Umbra::UINT32 MapCacheDataVersion = 1;

ImpTome* TomeWriter::computeStaticVisibility(ImpTome* inTome)
{
    if ((!m_depthMaps && !m_depthMapsInf) || !inTome) 
        return inTome;

    if (!m_cachePath.length())
    {
        LOGE("Cache path not given and static visibility optimization requested");
        UMBRA_HEAP_FREE(getAllocator(), inTome);
        return NULL;
    }

    String cacheFile(getAllocator());
    {
        struct Params
        {
            UINT32 storageMode;
        } p;

        if (m_depthMapsInf && !m_depthMaps)
            p.storageMode = 0;
        else
            p.storageMode = 1;

        HashGenerator hasher(getAllocator());
        hasher.write(&p, sizeof(Params));
        hasher.write(inTome, inTome->getSize());
        cacheFile = m_cachePath;
        if (!cacheFile.endsWith(String("/", getAllocator())))
            cacheFile += String("/", getAllocator());
        cacheFile += hasher.getHashValue() + String("_depthmaps.umbracache", getAllocator());
    }

    // Advance progress
    m_progress.nextPhase();
    
    FileInputStream in(cacheFile.toCharPtr());
    if (in.isOpen())
    {        
        MapCacheHeader header;
        in.read(&header, sizeof(MapCacheHeader));
        
        if (header.version == MapCacheDataVersion)
        {
            UINT32 newSize = inTome->getSize() + header.size;
            newSize = UMBRA_ALIGN(newSize, 16);
        
            UINT8* mem = (UINT8*)UMBRA_HEAP_ALLOC(getAllocator(), newSize);
            ImpTome* outTome = (ImpTome*)mem;

            // Clear
            memset(mem, 0, newSize);
            memcpy(mem, inTome, inTome->getSize());
            in.read(mem + inTome->getSize(), header.size);

            outTome->m_objectDepthmaps = DataPtr(inTome->getSize() + header.dataOffset);
            outTome->m_depthmapPalettes = DataPtr(inTome->getSize() + header.paletteOffset);
            outTome->m_depthmapFaces = DataPtr(inTome->getSize() + header.faceOffset);
            outTome->m_numFaces = header.numFaces;

            if (m_depthMapsInf && !m_depthMaps)
                outTome->m_flags |= ImpTome::TOMEFLAG_SHADOW_DEPTHMAPS;
            else
                outTome->m_flags |= ImpTome::TOMEFLAG_DEPTHMAPS;

            // Reset size, recompute checksum
            outTome->m_size  = newSize;
            outTome->m_crc32 = 0;
            outTome->m_crc32 = outTome->computeCRC32();

            // We can free the input tome
            UMBRA_HEAP_FREE(getAllocator(), inTome);

            LOGI("Loaded depthmaps from cache\n");
            m_progress.setPhaseProgress(1.f);

            // Return new tome
            return outTome;
        }
    }

    LOGI("Computing depthmaps\n");

    m_timer.startTimer("depthmaps");

    // Prepare depthmap jobs - one job per target
    //////////////////////////////////////////////

    m_depthmapJobs.setAllocator(getAllocator());
    m_depthmapJobs.reset(inTome->getNumObjects());

    m_depthmapTome = inTome;
    m_nextDepthmapJob = 0;
    DataArray aabbs = inTome->getObjectBounds();

    // Setup jobs
    for (int i = 0; i < m_depthmapJobs.getSize(); i++)
    {
        // Object id for debugging
        UINT32 id = ((const Tome*)inTome)->getObjectUserID(i);
        m_depthmapJobs[i].objId = id;

        // Clear palette
        for (int face = 0; face < 6; face++)
            m_depthmapJobs[i].faces[face].paletteSize = 0;

        // AABB from tome
        aabbs.getElem(m_depthmapJobs[i].aabb, i);
    }

    int numThreads = m_numThreads;

    // Execute threads
    if (numThreads > 1)
    {
        DepthmapThread runner;

        Array<Thread*> threads(getAllocator());
        
        for (int i = 0; i < numThreads; i++)
        {
            Thread* th = UMBRA_HEAP_NEW(getAllocator(), Thread, getAllocator());
            th->setFunction(&runner);
            th->run(this);
            threads.pushBack(th);
        }

        for (int i = 0; i < threads.getSize(); i++)
            threads[i]->waitToFinish();

        for (int i = 0; i < threads.getSize(); i++)
            UMBRA_HEAP_DELETE(getAllocator(), threads[i]);
    } else
    {
        // Don't launch any threads without permission.
        // If numThreads is set to 1, user's allocator or logger might not expect 
        // calls from another thread.
        DepthmapThread runner;
        runner.run(this);
    }

    ImpTome* outTome = NULL;
    UINT32   size = 0;

    // Output results
    //////////////////

    if (m_depthMapsInf && !m_depthMaps)
    {
        Array<UINT32> data(getAllocator());
        data.resize(UMBRA_BITVECTOR_DWORDS(m_depthmapJobs.getSize() * 6 * DepthmapData::Resolution * DepthmapData::Resolution));
        memset(data.getPtr(), 0, data.getByteSize());

        int bitIdx = 0;
        for (int face = 0; face < 6; face++)
        for (int value = 0; value < (int)(DepthmapData::Resolution * DepthmapData::Resolution); value++)
        for (int i = 0; i < m_depthmapJobs.getSize(); i++)
        {
            if (testBit(m_depthmapJobs[i].faces[face].face.pixels, value))
                setBit(data.getPtr(), bitIdx);
            bitIdx++;
        }

        UINT32 offset = 0;
        size = inTome->getSize() + data.getByteSize();
        size = UMBRA_ALIGN(size, 16);

        // Allocate new tome
        UINT8* mem = (UINT8*)UMBRA_HEAP_ALLOC(getAllocator(), size);
        outTome = (ImpTome*)mem;

        // Clear
        memset(mem, 0, size);

        // Data begins with the new tome
        memcpy(mem, inTome, inTome->getSize());
        offset += inTome->getSize();

        memcpy(mem + offset, data.getPtr(), data.getByteSize());
        outTome->m_depthmapFaces = DataPtr(offset);
        offset += data.getByteSize();

        // Total face count - needed at runtime for accessing shuffled data
        outTome->m_numFaces = m_depthmapJobs.getSize() * 6;

        // Enable flag
        outTome->m_flags |= ImpTome::TOMEFLAG_SHADOW_DEPTHMAPS;

    } else
    {
        /*
            Following methods are used for depthmap compression:

            1) Palettes
               - Typically a face has many pixels with same value
            2) Identical face sharing (i.e. face indirection)
               - In a general case not very useful, but worth doing for faces
                 consisting only of one value.
            4) Palette indirection & variable palette size (max 16 elements) 
               - Tighly packed palette data
               - Indirection to achieve this
               - Store offset to palette data
        */

        Array<DepthmapData>                  datas(getAllocator());     // Object data (face & palette index, center point)
        Array<DepthmapData::DepthmapFace>    faces(getAllocator());     // Bit-packed pixel data for each face (4 bit / pixel)
        Array<UINT16>                        palettes(getAllocator());  // UINT16 x 16 palette data for each face

        // Hashes to avoid data duplication.
        Hash<DepthmapData::DepthmapFace,    UINT32> faceIndices(getAllocator());    // Index for each unique face
        Hash<DepthmapData::DepthmapPalette, UINT32> paletteOffsets(getAllocator()); // Index for each unique palette

        for (int i = 0; i < m_depthmapJobs.getSize(); i++)
        {
            DepthmapData data;

            for (int j = 0; j < 6; j++)
            {
    #if 0       
                // We could store palettes with only one element like this,
                // but I think it's better without the runtime if.
                if (m_depthmapJobs[i].faces[j].paletteSize == 1)
                {
                    data.faces[j].faceIdx    = (UINT32)DepthmapData::InvalidIdx;
                    data.faces[j].paletteIdx = (UINT32)m_depthmapJobs[i].faces[j].palette.palette[0];
                    continue;
                }
    #endif

                // Input face and palette
                DepthmapData::DepthmapFace&    srcFace    = m_depthmapJobs[i].faces[j].face;
                DepthmapData::DepthmapPalette& srcPalette = m_depthmapJobs[i].faces[j].palette;

                // Find/insert unique face index
                UINT32 faceIndex = faceIndices.getDefault(srcFace, faces.getSize());
                if (faceIndex == (UINT32)faces.getSize())
                    faces.pushBack(srcFace);

                // Find/insert unique palette index
                UINT32 paletteOffset = paletteOffsets.getDefault(srcPalette, palettes.getSize());
                if (paletteOffset == (UINT32)palettes.getSize())
                {
                    // Insert only existing elements
                    int numEntries = m_depthmapJobs[i].faces[j].paletteSize;
                    palettes.append(srcPalette.palette, numEntries);

                    // Ensure even number of entries so that data is big-endian swappable.
                    // For performance reasons palette unpacking with endianess is handled as a special case: 
                    // this is different than elsewhere in Umbra. Endian swap switches each two consecutive 
                    // 16-bit elements in palette array. We can then access the data using palette[index^1] 
                    // on big-endian platforms.
                    if (numEntries & 1)
                        palettes.pushBack(0);
                }

                // Set face index and palette offset
                data.faces[j].faceIdx       = faceIndex;
                data.faces[j].paletteOffset = paletteOffset; // in number of 16-bit elements
            }

            // Center point
            data.reference = m_depthmapJobs[i].aabb.getCenter();

            // Push data into array
            datas.pushBack(data);
        }

        // Clear some fields no longer needed
        m_depthmapTome = NULL;
        m_depthmapJobs.clear();
        m_depthmapJobs.shrinkToFit();
    
        // Generate new tome with depthmap data (this is a bit sketchy)
        ////////////////////////////////////////////////////////////////

        // Tome size with data added
        size   = inTome->getSize() + datas.getByteSize() + faces.getByteSize() + palettes.getByteSize();
        size = UMBRA_ALIGN(size, 16);

        // Current offset
        UINT32 offset = 0;
#if 0
        static UINT32 totalSize = 0, objects = 0;
        totalSize += size - inTome->getSize();
        objects   += m_targetObjs.getSize();

        String memStr = String::formatSize1k(size - inTome->getSize(), getAllocator());
        LOGI("Size: %s", memStr.toCharPtr());
        memStr = String::formatSize1k(totalSize, getAllocator());
        LOGI("Total: %s", memStr.toCharPtr());
        memStr = String::formatSize((size_t)((float)totalSize/(float)objects), getAllocator());
        LOGI("Total: %s/object", memStr.toCharPtr());
    #endif

        // Allocate new tome
        UINT8* mem = (UINT8*)UMBRA_HEAP_ALLOC(getAllocator(), size);
        outTome = (ImpTome*)mem;

        // Clear
        memset(mem, 0, size);

        // Data begins with the new tome
        memcpy(mem, inTome, inTome->getSize());
        offset += inTome->getSize();

        // Insert per-target data next
        memcpy(mem + offset, datas.getPtr(), datas.getByteSize());
        outTome->m_objectDepthmaps = DataPtr(offset);
        offset += datas.getByteSize();

        /*
            Here we reorder face data in a cache-friendly order.        

            We're typically reading nearby pixels from targets that are consecutive in indexing.
            If the pixel data would be arranged per-face, this is all over in memory and practically
            random.

            This reorders the face data so that first is stored pixel 0 from each face, then pixel 1 
            and so on. Pixels with same index are nearby in memory.

            This helps both portal/frustum query and shadow lookup.
        */

        Array<UINT32> shuffled(faces.getByteSize() / sizeof(UINT32), getAllocator());
        memset(shuffled.getPtr(), 0, shuffled.getByteSize());

        for (int value = 0; value < (int)(DepthmapData::Resolution * DepthmapData::Resolution); value++)
        for (int face = 0; face < faces.getSize(); face++)
        {
            UINT32 data;
            data = unpackElem(faces[face].pixels, value * DepthmapData::DepthBits, DepthmapData::DepthBits);
            packElem(shuffled.getPtr(), value * faces.getSize() * DepthmapData::DepthBits + face * DepthmapData::DepthBits, data, DepthmapData::DepthBits);
        }

        // Insert shuffled face data next
        memcpy(mem + offset, shuffled.getPtr(), shuffled.getByteSize());
        outTome->m_depthmapFaces = DataPtr(offset);
        offset += faces.getByteSize();

        // Total face count - needed at runtime for accessing shuffled data
        outTome->m_numFaces = faces.getSize();

        // Insert palettes as-is
        memcpy(mem + offset, palettes.getPtr(), palettes.getByteSize());
        outTome->m_depthmapPalettes = DataPtr(offset);

        // Enable flag
        outTome->m_flags |= ImpTome::TOMEFLAG_DEPTHMAPS;
    }

    // Timing
    m_timer.stopTimer("depthmaps");
    double time = m_timer.getTimerValue("depthmaps");
    LOGI("Computed %d depthmaps using %d threads in %d m %d sec, avg %g ms/object\n", m_targetObjs.getSize(), numThreads, ((int)time) / 60, ((int)time) % 60, 1000.0 * (float)numThreads * time / (double)m_targetObjs.getSize());

    m_depthmapAllocator = NULL;
    m_progress.setPhaseProgress(1.f);

    if (outTome)
    {
        // Reset size, recompute checksum
        outTome->m_size = size;
        outTome->m_crc32 = 0;
        outTome->m_crc32 = outTome->computeCRC32();
        
        FileOutputStream out(cacheFile.toCharPtr());
        if (out.isOpen())
        {
            MapCacheHeader header;
            header.size = size - inTome->getSize();
            header.version = MapCacheDataVersion;
            header.dataOffset = outTome->m_objectDepthmaps.getOffset() - inTome->getSize();
            header.paletteOffset = outTome->m_depthmapPalettes.getOffset() - inTome->getSize();
            header.faceOffset = outTome->m_depthmapFaces.getOffset() - inTome->getSize();
            header.numFaces = outTome->m_numFaces;
            out.write(&header, sizeof(MapCacheHeader));
            out.write((UINT8*)outTome + inTome->getSize(), header.size);
        }

        // We can free the input tome
        UMBRA_HEAP_FREE(getAllocator(), inTome);

        // Return new tome
        return outTome;
    } else
        return inTome;
}

ImpTome* TomeWriter::generateImpTome (void)
{
    StructBuilder<ImpTome> builder(getAllocator());

    builder.m_versionMagic = (((UINT32)TOME_MAGIC << 16) | (UINT32)TOME_VERSION);

    //////////////////

    const Array<TileTreeNode*>& tileNodes = m_tileTreeNodes;
    int leafTiles = tileNodes.getSize() / 2 + 1;

    if (tileNodes.getSize() != m_tiles.getSize())
        return NULL;

    builder.m_flags     = 0;
    builder.m_treeMin   = m_aabb.getMin();
    builder.m_treeMax   = m_aabb.getMax();
    int treeNodes     = tileNodes.getSize();
    builder.m_tileTree.setNodeCount(treeNodes);

    UINT32* data = (UINT32*)allocDataRef(builder, builder.m_tileTree.m_treeData, KDTree::getDataDwords(treeNodes) * sizeof(UINT32));
    // TODO: not all nodes have split values (especially leaves' don't have those)
    float* pos = (float*)allocDataRef(builder, builder.m_tileTree.m_splitValues, treeNodes * sizeof(float));

    bitpackTree(data, pos, tileNodes);
    builder.m_tileTree.m_numSplitValues = tileNodes.getSize();
    KDTree::buildLut(data + UMBRA_BITVECTOR_DWORDS(treeNodes * 2), data, treeNodes);

    builder.m_numLeafTiles = leafTiles;
    builder.m_numTiles = tileNodes.getSize(); /* \todo [antti 13.11.2012]: this is not correct! */
    builder.m_numObjects = m_targetObjs.getSize();
    builder.m_numGates = m_gateIndexToId.getSize();

    KDTree topLevel(treeNodes, data, DataArray());
    int bitsPerPath = topLevel.getMaxDepth(); // maxdepth-1 bits for path and one bit for path len marker

	if (bitsPerPath == -1)
	{
		LOGE("Top level tree too deep\n(%g,%g,%g -> %g,%g,%g), nodes %d",
			m_aabb.getMin().x, m_aabb.getMin().y, m_aabb.getMin().z,
			m_aabb.getMax().x, m_aabb.getMax().y, m_aabb.getMax().z,
			treeNodes);
		return NULL;
	}

    UINT32* slotPaths = (UINT32*)allocDataRef(builder, builder.m_slotPaths, UMBRA_BITVECTOR_SIZE(treeNodes * bitsPerPath));
    memset(slotPaths, 0, UMBRA_BITVECTOR_SIZE(treeNodes * bitsPerPath));
    topLevel.getPaths(slotPaths, bitsPerPath);
    builder.m_bitsPerSlotPath = bitsPerPath;
    builder.m_lodBaseDistance = m_lodDistance;

    // tile lod levels
    float* lodlevels = (float*)allocDataRef(builder, builder.m_tileLodLevels, treeNodes * sizeof(float));
    for (int i = 0; i < treeNodes; i++)
    {
        Tile* tile = m_tiles[i];
        if (!tile)
            lodlevels[i] = -1.f;
        else
            lodlevels[i] = getLodLevel(tile->m_aabb);
    }

    // fill in cell map array

    int* cellStarts = (int*)allocDataRef(builder, builder.m_cellStarts, (treeNodes + 1) * sizeof(int));

    int numClusters = 0;
    int numCells = 0;
    for (int i = 0; i < treeNodes; i++)
    {
        Tile* tile = m_tiles[i];
        cellStarts[i] = numCells;
        if (tile)
        {
            numClusters += tile->m_imp->getNumClusters();
            numCells    += tile->m_imp->getNumCells();
        }
    }

    builder.m_numClusters = numClusters;
    cellStarts[builder.m_numTiles] = numCells;

    UINT32 clusterSize = 0, objectSize = 0;
    builder.m_listWidths = 0;

    for (int i = 0; i < 2; i++)
    {
        const Array<int>& combined = i == 0 ? m_objectLists.getOutput() : m_clusterLists.getOutput();

        // object & cluster lists
        if (combined.getSize())
        {
            int maxElem = INT_MIN;
            int maxCount = INT_MIN;
            for (int j = 0; j < combined.getSize(); j += 2)
            {
                maxElem  = max2(maxElem, combined[j]);
                maxCount = max2(maxCount, combined[j+1]);
            }

            int elemWidth  = max2(1, bitsForValue(maxElem));
            int countWidth = max2(1, bitsForValue(maxCount));
            builder.m_listWidths |= (elemWidth | (countWidth << 5)) << (i * 10);

            UINT32* list = NULL;
            if (i == 0)
            {
                builder.m_objectListSize = combined.getSize() / 2;
                objectSize = UMBRA_BITVECTOR_SIZE((elemWidth + countWidth) * (combined.getSize() / 2));
                list = (UINT32*)allocDataRef(builder, builder.m_objectLists, objectSize, true);
            } else
            {
                builder.m_clusterListSize = combined.getSize() / 2;
                clusterSize = UMBRA_BITVECTOR_SIZE((elemWidth + countWidth) * (combined.getSize() / 2));
                list = (UINT32*)allocDataRef(builder, builder.m_clusterLists, clusterSize, true);
            }

            int bitOffset = 0;
            for (int j = 0; j < combined.getSize(); j += 2)
            {
                int value = combined[j];
                int count = combined[j + 1];
                UMBRA_ASSERT(bitsForValue(value) <= elemWidth);
                UMBRA_ASSERT(bitsForValue(count) <= countWidth);
                copyBitRange(list, bitOffset,             (UINT32*)&value, 0, elemWidth);
                copyBitRange(list, bitOffset + elemWidth, (UINT32*)&count, 0, countWidth);
                bitOffset += elemWidth + countWidth;
            }
        }
    }

    /*{
        const Array<int>& combined = m_clusterLists.getOutput();

        if (combined.getSize())
        {
            builder.m_clusterListElemWidth = max2(1, bitsForValue(numClusters));
            builder.m_clusterListSize = combined.getSize();
            clusterSize = UMBRA_BITVECTOR_SIZE(builder.m_clusterListElemWidth * combined.getSize());
            UINT32* clusterLists = (UINT32*)allocDataRef(builder, builder.m_clusterLists, clusterSize, true);
            for (int i = 0; i < combined.getSize(); i++)
            {
                int value = combined[i];
                UMBRA_ASSERT(bitsForValue(value) <= builder.m_clusterListElemWidth);
                copyBitRange(clusterLists, i * builder.m_clusterListElemWidth, (UINT32*)&value, 0, builder.m_clusterListElemWidth);
            }
        }
    } */

    String cSize = String::formatSize(clusterSize, getAllocator());
    String oSize = String::formatSize(objectSize, getAllocator());
    LOGI("object list: %s, cluster list: %s", oSize.toCharPtr(), cSize.toCharPtr());

    // build cluster graph

    Array<ClusterNode> cells(getAllocator());
    Array<Portal> portals(getAllocator());
    if (!buildClusterGraph(cells, portals))
        return NULL;

    setBuilderArray(builder, builder.m_clusters, cells);
    setBuilderArray(builder, builder.m_clusterPortals, portals);

    // Unify vertices.

    setBuilderArray(builder, builder.m_gateVertices, m_vertices);
    builder.m_numGateVertices = m_vertices.getSize();
    setBuilderArray(builder, builder.m_gateIndices, m_gateIdxs);

    // Target AABBs.

    ObjectBounds* objBounds = (ObjectBounds*)allocDataRef(builder, builder.m_objBounds, m_targetObjs.getSize() * sizeof(ObjectBounds));
    for (int i = 0; i < m_targetObjs.getSize(); i++)
    {
        const AABB& aabb = m_targetObjs[i].m_bounds;
        objBounds[i].mn = aabb.getMin();
        objBounds[i].mx = aabb.getMax();
    }

    // Target distance limits

    bool objectLODsUsed = false;

    for (int i = 0; i < m_targetObjs.getSize(); i++)
    {
        const ObjectParams& p = m_targetObjs[i];
        if (p.m_drawDistance.x > 0.f || (p.m_drawDistance.y >= 0.f) || p.m_distanceBound.isOK())
        {
            objectLODsUsed = true;
            break;
        }
    }

    if (objectLODsUsed)
    {
        ObjectDistance* objDistances = (ObjectDistance*)allocDataRef(builder,
            builder.m_objDistances, m_targetObjs.getSize() * sizeof(ObjectDistance));
        for (int i = 0; i < m_targetObjs.getSize(); i++)
        {
            const ObjectParams& p = m_targetObjs[i];
            if (p.m_distanceBound.isOK())
            {
                objDistances[i].boundMin = p.m_distanceBound.getMin();
                objDistances[i].boundMax = p.m_distanceBound.getMax();
            }
            else
            {
                objDistances[i].boundMin = p.m_bounds.getMin();
                objDistances[i].boundMax = p.m_bounds.getMax();
            }
            objDistances[i].nearLimit = p.m_drawDistance.x * p.m_drawDistance.x;
            objDistances[i].farLimit = (p.m_drawDistance.y < 0.f) ? FLT_MAX : p.m_drawDistance.y * p.m_drawDistance.y;
        }
    }

    // Object ids.

    if (m_groupToTargetIds.getSize())
    {
        // One to many mapping (groups enabled)

        int* userIdStarts = (int*)allocDataRef(builder, builder.m_userIDStarts, (m_groupToTargetIds.getSize()+1) * sizeof(int));
        userIdStarts[0] = 0;

        int n = 0;
        for (int i = 0; i < m_groupToTargetIds.getSize(); i++)
        {
            n += m_groupToTargetIds[i].getSize();
            userIdStarts[i+1] = n;
        }

        UINT32* userIds = (UINT32*)allocDataRef(builder, builder.m_userIDs, n * sizeof(UINT32));

        n = 0;
        for (int i = 0; i < m_groupToTargetIds.getSize(); i++)
        {
            UMBRA_ASSERT(userIdStarts[i] == n);
            for (int j = 0; j < m_groupToTargetIds[i].getSize(); j++)
                userIds[n++] = m_groupToTargetIds[i][j];
        }
    }
    else
    {
        // One-to-one mapping if grouping is disabled.

        UINT32* objIndices = (UINT32*)allocDataRef(builder, builder.m_userIDs, m_targetObjs.getSize() * sizeof(UINT32));
        for (int i = 0; i < m_targetObjs.getSize(); i++)
            objIndices[i] = m_targetObjs[i].getId();
    }

    //////////////////

    DataPtr* tileOffsets = (DataPtr*)allocDataRef(builder, builder.m_tiles, m_tiles.getSize() * sizeof(DataPtr), true);
    for (int i = 0; i < m_tiles.getSize(); i++)
    {
        if (m_tiles[i])
            setDataRef(builder, tileOffsets[i], m_tiles[i]->m_imp, m_tiles[i]->m_imp->getSize());
    }

    int* gateIndices = (int*)allocDataRef(builder, builder.m_gateIndexMap, m_gateIndexToId.getSize() * sizeof(UINT32));
    for (int i = 0; i < m_gateIndexToId.getSize(); i++)
        gateIndices[i] = m_gateIndexToId[i];

    // face match data

    if (m_matchingData)
    {
        // count number of entries

        int faceMatchTrees = 0;
        int numLeaves = 0;
        for (int i = 0; i < m_tiles.getSize(); i++)
        {
            if (!m_tiles[i] || !m_tiles[i]->isLeaf())
                continue;
            faceMatchTrees += countOnes(m_tiles[i]->m_borderMask);
            ++numLeaves;
        }

        builder.m_numMatchingTrees = faceMatchTrees;
        LeafTileMatchData*  leafData = (LeafTileMatchData*)allocDataRef(builder, builder.m_tileMatchingData, sizeof(LeafTileMatchData) * numLeaves);
        SerializedTreeData* trees    = (SerializedTreeData*)allocDataRef(builder, builder.m_matchingTrees, sizeof(SerializedTreeData) * faceMatchTrees);

        int treeIdx = 0;
        int leafIdx = 0;
        for (int i = 0; i < m_tiles.getSize(); i++)
        {
            if (!m_tiles[i] || !m_tiles[i]->isLeaf())
                continue;
            LeafTileMatchData& perLeaf = leafData[leafIdx++];
            memset(&perLeaf, 0, sizeof(LeafTileMatchData));
            if (!m_tiles[i]->m_borderMask)
            {
                perLeaf.setMatchTreeOfsAndCount(0, 0);
            }
            else
            {
                int treeOfs = treeIdx;
                for (int face = 0; face < 6; ++face)
                {
                    if (!(m_tiles[i]->m_borderMask & (1 << face)))
                        continue;
                    SerializedTreeData& tree = trees[treeIdx++];
                    SubdivisionTree st(getAllocator());
                    m_tiles[i]->m_cellGraph.getMatchingTree(face).deserialize(st);
                    st.setRoot(SubdivisionTreeUtils(st).collapse(st.getRoot(), false));
                    serializeTreeData(builder, tree, st);
                }
                perLeaf.setMatchTreeOfsAndCount(treeOfs, treeIdx - treeOfs);

                int depth = m_tiles[i]->getDepth() - 1;
                if (depth)
                {
                    Array<UINT32> parentCells(getAllocator());
                    parentCells.reset(depth * m_tiles[i]->m_cellGraph.getCellCount());
                    memset(parentCells.getPtr(), 0, parentCells.getSize() * sizeof(UINT32));

                    Tile* ancestor    = m_tiles[i]->m_parent;
                    int   ancestorIdx = 0;
                    int   maxValue    = 0;

                    while (ancestor)
                    {
                        UMBRA_ASSERT(ancestorIdx < depth);
                        for (int cell = 0; cell < m_tiles[i]->m_cellGraph.getCellCount(); cell++)
                        {
                            int outputCell = cell;
                            int cellInAncestor = ancestor->remapCell(i, cell);
                            if (cellInAncestor < 0)
                                cellInAncestor = 0; // TODO: write some garbage here
                            parentCells[depth * outputCell + ancestorIdx] = cellInAncestor;
                            maxValue = max2(maxValue, cellInAncestor);
                        }

                        ancestor = ancestor->m_parent;
                        ancestorIdx++;
                    }

                    perLeaf.m_cellLodElemWidth = depth;
                    perLeaf.m_cellLodBitWidth = bitsForValue(maxValue);

                    if (perLeaf.m_cellLodBitWidth)
                    {
                        UINT32* packedMap = (UINT32*)builder.allocDataRef(perLeaf.m_cellLodMapping,
                            UMBRA_BITVECTOR_SIZE(parentCells.getSize() * perLeaf.m_cellLodBitWidth), true);
                        bitPackIntArray(parentCells.getPtr(), parentCells.getSize(), packedMap, perLeaf.m_cellLodBitWidth);
                    }
                }
            }
        }
    }

    // Computation string.

    memset(builder.m_computationString, 0, sizeof(builder.m_computationString));
    strncpy(builder.m_computationString, m_computationString.toCharPtr(), sizeof(builder.m_computationString)-1);

    builder.m_size = builder.getCurrentSize();
    ImpTome* tome = builderPack(builder);
    tome->m_crc32 = tome->computeCRC32();
    return tome;
}

ImpTile* TomeWriter::generateTile(Tile* srcTile)
{
    StructBuilder<ImpTile> builder(getAllocator());

    const CellGraph& cg = srcTile->m_cellGraph;
    int clusterCount = srcTile->m_numClusters;
    int cellCount = srcTile->m_cellGraph.getCellCount();

    UMBRA_ASSERT((srcTile->m_slot & 0xFF000000) == 0);
    UMBRA_ASSERT((clusterCount & 0xFFFF0000) == 0);
    UMBRA_ASSERT((cellCount & 0xFFFF0000) == 0);

    builder.m_numCellsAndClusters = (clusterCount << 16) | cellCount;
    builder.m_treeMin = srcTile->getAABB().getMin();
    builder.m_treeMax = srcTile->getAABB().getMax();
    builder.m_sizeAndFlags = 0;
    builder.m_portalExpand = cg.getPortalExpand();

    // cell graph

    {
        Array<CellNode> cells(getAllocator());
        Array<Portal> portals(getAllocator());
        if (!buildCellGraph(srcTile, cells, portals))
            return NULL;
        setBuilderArray(builder, builder.m_cells, cells);
        setBuilderArray(builder, builder.m_portals, portals);
    }

    if (srcTile->isLeaf())
    {
        UMBRA_ASSERT(!cg.getViewTree().isEmpty());

        SubdivisionTree st(getAllocator());
        cg.getViewTree().deserialize(st);

        // Remove outside references from view tree
        // TODO: move this step to earlier
        for (SubdivisionTree::LeafIterator iter = st.iterateLeaves(); !iter.end(); iter.next())
        {
            int idx = iter.node()->getLeaf()->getIndex();
            if (idx >= 0 && cg.getCell(idx).isOutside())
                iter.node()->getLeaf()->setIndex(-1);
        }

        st.setRoot(SubdivisionTreeUtils(st).collapse(st.getRoot(), true));

        // Handle plane nodes

        st.setRoot(SubdivisionTreeUtils::unifyNodes(getAllocator(), st.getRoot()));

        Array<Vector4> pleqs(getAllocator());
        Hash<Vector4, int> pleqMap(getAllocator());
        Array<SubdivisionTree::PlaneNode*> planeNodes(getAllocator());
        Hash<SubdivisionTree::PlaneNode*, int> planeNodeMap(getAllocator());

        for (SubdivisionTree::Iterator iter = st.iterateAll(); !iter.end(); iter.next())
        {
            if (!iter.node()->isPlane())
                continue;

            Vector4 pleq = iter.node()->getPlane()->getPleq();
            if (pleqMap.getDefault(pleq, pleqs.getSize()) == pleqs.getSize())
                pleqs.pushBack(pleq);
            if (planeNodeMap.getDefault(iter.node()->getPlane(), planeNodes.getSize()) == planeNodes.getSize())
                planeNodes.pushBack(iter.node()->getPlane());
        }

        builder.m_numPlanes = pleqs.getSize();
        setBuilderArray(builder, builder.m_planes, pleqs);

        int maxIndex = max2(SubdivisionTreeUtils::findMaxLeafIndex(st.getRoot()), planeNodes.getSize());
        int mapWidth = bitsForValue(maxIndex) + 1; // Highest bit tells if this is a plane node
        UINT32 planeMask = 1 << (mapWidth - 1);

        Hash<SubdivisionTree::PlaneNode*, int>::Iterator hashIter = planeNodeMap.iterate();
        while (planeNodeMap.isValid(hashIter))
        {
            planeNodeMap.getValue(hashIter) |= planeMask;
            planeNodeMap.next(hashIter);
        }

        st.setRoot(SubdivisionTreeUtils(st).replacePlaneNodesWithLeaves(st.getRoot(), planeNodeMap));

        // Serialize tree.

        serializeTreeData(builder, builder.m_viewTree, st, mapWidth);

        // Output plane nodes.

        Array<TempBspNode> bspNodes(getAllocator());

        for (int i = 0; i < planeNodes.getSize(); i++)
        {
            SubdivisionTree::PlaneNode* pn = planeNodes[i];
            TempBspNode bsp;
            bsp.set(*pleqMap.get(pn->getPleq()),
                    pn->getRight()->isLeaf(),
                    pn->getRight()->isLeaf() ?
                      pn->getRight()->getLeaf()->getIndex() :
                      (*planeNodeMap.get(pn->getRight()->getPlane()) & ~planeMask),
                    pn->getLeft()->isLeaf(),
                    pn->getLeft()->isLeaf() ?
                      pn->getLeft()->getLeaf()->getIndex() :
                      (*planeNodeMap.get(pn->getLeft()->getPlane()) & ~planeMask));

            bspNodes.pushBack(bsp);
        }

        setBuilderArray(builder, builder.m_bsp, bspNodes);
        builder.m_numBspNodes = bspNodes.getSize();
    }


    builder.m_sizeAndFlags = (builder.getCurrentSize() << 8);
    if (srcTile->isLeaf())
        builder.m_sizeAndFlags |= ImpTile::TILEFLAG_ISLEAF;

    return builderPack(builder);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

static void initPortal (Portal& p, Umbra::UINT32 link, int targetIdx, const AABB& aabb, const AABB& bounds)
{
    p.link = link;

    int axis = getFaceAxis(p.getFace());
    Vector3 dim = bounds.getDimensions();

    float z = (aabb.getMin()[axis] - bounds.getMin()[axis]) / dim[axis];
    axis = (axis + 1) % 3;
    float xmn = (aabb.getMin()[axis] - bounds.getMin()[axis]) / dim[axis];
    float xmx = (aabb.getMax()[axis] - bounds.getMin()[axis]) / dim[axis];
    axis = (axis + 1) % 3;
    float ymn = (aabb.getMin()[axis] - bounds.getMin()[axis]) / dim[axis];
    float ymx = (aabb.getMax()[axis] - bounds.getMin()[axis]) / dim[axis];

    UMBRA_ASSERT(z >= 0.f && z <= 1.f);
    UMBRA_ASSERT(xmn >= 0.f && xmn <= 1.f);
    UMBRA_ASSERT(xmx >= 0.f && xmx <= 1.f);
    UMBRA_ASSERT(xmn < xmx);
    UMBRA_ASSERT(ymn >= 0.f && ymn <= 1.f);
    UMBRA_ASSERT(ymx >= 0.f && ymx <= 1.f);
    UMBRA_ASSERT(ymn < ymx);

    p.idx_z = (targetIdx << 16) | ((int)(z * 65535.f));
    p.xmn_xmx = ((int)(xmn * 65535.f)) << 16 | ((int)(xmx * 65535.f));
    p.ymn_ymx = ((int)(ymn * 65535.f)) << 16 | ((int)(ymx * 65535.f));
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

static void initUserPortal (Portal& p, Umbra::UINT32 link, int targetIdx, int gateOffset, int gateCount, int geometryOffset, int vertexCount)
{
    // \todo are these correct?
    // \todo test with extreme user portal geometry input
    UMBRA_ASSERT((vertexCount & (0xffffffff << 12)) == 0);
    UMBRA_ASSERT(geometryOffset == ((geometryOffset << 12) >> 12));

    UMBRA_ASSERT((gateCount & (0xffffffff << 12)) == 0);
    UMBRA_ASSERT(gateOffset == ((gateOffset << 12) >> 12));

    p.link = link;
    p.idx_z = targetIdx << 16;
    p.xmn_xmx = gateOffset << 12 | gateCount;
    p.ymn_ymx = geometryOffset << 12 | vertexCount;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TomeWriter::processPortal(Tile* srcTile, Tile* dstTile, const CellGraph::Portal& edge, Array<Portal>& outPortals)
{
    int face = edge.isGate() ? 0 : edge.getRectPortal().getFace();
    int user = edge.isGate() ? 1 : 0;
    int outside = dstTile ? 0 : 1;
    int hierarchy = dstTile ? (dstTile->isLeaf() ? 0 : 1) : 0;
    int targetCell = dstTile ? edge.getTarget() : 0;

    // should never have portals leading to non-reachable cells
    UMBRA_ASSERT(targetCell >= 0);

    UINT32 plink = BUILD_PORTAL_LINK(face, outside, user, hierarchy, dstTile ? dstTile->m_slot : Portal::getMaxSlotIdx());

    if (user)
    {
        const Hash<Vector4, CellGraph::PortalHull>& hulls = edge.getGatePortal().getPortalHulls();
        Hash<Vector4, CellGraph::PortalHull>::Iterator iter = hulls.iterate();

        while (hulls.isValid(iter))
        {
            const CellGraph::PortalHull& portalHull = hulls.getValue(iter);
            Portal portal;

            // Compute portal AABB and store vertices
            AABB aabb;

            for (int i = 0; i < portalHull.getVertexCount(); i++)
            {
                Vector3 v = portalHull.getVertex(i);
                aabb.grow(v);
            }

            int geomOffset = m_gateAABBHash.getDefault(aabb, m_vertices.getSize());
            if (geomOffset == m_vertices.getSize())
            {
                m_vertices.pushBack(aabb.getMin());
                m_vertices.pushBack(aabb.getMax());
            }

            int gateOffset = m_gateIdHash.getDefault(edge.getGatePortal().getGateIDs(), m_gateIdxs.getSize());
            if (gateOffset == m_gateIdxs.getSize())
            {
                Set<int>::Iterator iter = edge.getGatePortal().getGateIDs().iterate();
                while (iter.next())
                    m_gateIdxs.pushBack(*m_gateIdToIndex.get(iter.getValue()));
            }

            initUserPortal(portal, plink, targetCell, gateOffset, edge.getGatePortal().getGateIDs().getSize(), geomOffset, 2);
            outPortals.pushBack(portal);
            hulls.next(iter);
        }
    }
    else
    {
        Portal portal;

        int axis = getFaceAxis((Face)face);
        int axisX = (axis + 1) % 3;
        int axisY = (axis + 2) % 3;
        Vector3 mn, mx;

        mn[axis]  = edge.getRectPortal().getZ();
        mx[axis]  = edge.getRectPortal().getZ();
        mn[axisX] = edge.getRectPortal().getRect()[0];
        mx[axisX] = edge.getRectPortal().getRect()[2];
        mn[axisY] = edge.getRectPortal().getRect()[1];
        mx[axisY] = edge.getRectPortal().getRect()[3];

        initPortal(portal, plink, targetCell, AABB(mn, mx), srcTile->getAABB());
        outPortals.pushBack(portal);
    }
}

void TomeWriter::processPortal(Tile* srcTile, Tile* dstTile, const ExternalCellGraph::Portal& edge, Array<Portal>& outPortals)
{
    int face = edge.getFace();
    int user = 0;
    int outside = dstTile ? 0 : 1;
    int hierarchy = dstTile ? (dstTile->isLeaf() ? 0 : 1) : 0;
    int targetCell = dstTile ? edge.getTarget() : 0;

    // should never have portals leading to non-reachable cells
    UMBRA_ASSERT(targetCell >= 0);

    UINT32 plink = BUILD_PORTAL_LINK(face, outside, user, hierarchy, dstTile ? dstTile->m_slot : Portal::getMaxSlotIdx());
    Portal portal;

    int axis = getFaceAxis((Face)face);
    int axisX = (axis + 1) % 3;
    int axisY = (axis + 2) % 3;
    Vector3 mn, mx;

    mn[axis]  = edge.getZ();
    mx[axis]  = edge.getZ();
    mn[axisX] = edge.getRect()[0];
    mx[axisX] = edge.getRect()[2];
    mn[axisY] = edge.getRect()[1];
    mx[axisY] = edge.getRect()[3];

    initPortal(portal, plink, targetCell, AABB(mn, mx), srcTile->getAABB());
    outPortals.pushBack(portal);
}


/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TomeWriter::buildCellGraph (Tile* tile, Array<CellNode>& cells, Array<Portal>& portals)
{
    const CellGraph& cg = tile->m_cellGraph;
    const ExternalCellGraph& ecg = tile->m_externalCellGraph;

    for (int i = 0; i < cg.getCellCount(); i++)
    {
        const CellGraph::Cell& cell = cg.getCell(i);
        const ExternalCellGraph::Cell& ecell = ecg.getCell(i);

        UINT32 portalIdx = portals.getSize();

        // Internal portals.

        for (int k = 0; k < cell.getPortalCount(); k++)
        {
            const CellGraph::Portal& edge = cell.getPortal(k);

            processPortal(tile, tile, edge, portals);
        }

        // External portals.

        for (int k = 0; k < ecell.getPortalCount(); k++)
        {
            const ExternalCellGraph::Portal& edge = ecell.getPortal(k);
            if (edge.getTargetTile() < 0)
            {
                processPortal(tile, 0, edge, portals);
            }
            else
            {
                Tile* dstTile = m_tiles[edge.getTargetTile()];
                processPortal(tile, dstTile, edge, portals);
            }
        }

        // Objects & Clusters

        UMBRA_ASSERT(cell.getClusters().getSize() > 0);
        int clusterIdx;
        int numClusters;
        if (cell.getClusters().getSize() > 1)
        {
            m_clusterLists.getRange(GlobalCell(tile->m_slot, i), clusterIdx, numClusters);
        }
        else
        {
            clusterIdx = cell.getClusters()[0];
            numClusters = 0;
        }

        int objIdx = 0, numObjs = 0;
        m_objectLists.getRange(GlobalCell(tile->m_slot, i), objIdx, numObjs);

        int numPortals  = portals.getSize() - portalIdx;

        PackedAABB bounds;
        bounds.pack(tile->getAABB(), cell.getAABB());

        CellNode cellData;
        cellData.setPortalIdxAndCount(portalIdx, numPortals);
        cellData.setObjects(objIdx, numObjs);
        cellData.setClusters(clusterIdx, numClusters);
        cellData.setBounds(bounds);
        cells.pushBack(cellData);
    }

    UMBRA_ASSERT(cells.getSize() <= UMBRA_MAX_CELLS_PER_TILE);

    return true;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TomeWriter::buildClusterGraph (Array<ClusterNode>& output, Array<Portal>& portals)
{
    Hash<Array<Vector3>, int> hullOffsets(getAllocator());

    for (int i = 0; i < m_clusterGraph.getCellCount(); i++)
    {
        const CellGraph::Cell& cell = m_clusterGraph.getCell(i);
        UINT32 portalIdx = portals.getSize();

        for (int j = 0; j < cell.getRectPortalCount(); j++)
        {
            const CellGraph::RectPortal& portal = cell.getRectPortal(j);

            Portal outp;
            int axis = getFaceAxis(portal.getFace());
            int axisX = (axis + 1) % 3;
            int axisY = (axis + 2) % 3;
            Vector3 mn, mx;
            mn[axis]  = portal.getZ();
            mx[axis]  = portal.getZ();
            mn[axisX] = portal.getRect()[0];
            mx[axisX] = portal.getRect()[2];
            mn[axisY] = portal.getRect()[1];
            mx[axisY] = portal.getRect()[3];

            initPortal(outp, BUILD_PORTAL_LINK(portal.getFace(), 0, 0, 0, portal.getTarget()),
                    0xFFFF, AABB(mn, mx), m_aabb);
            portals.pushBack(outp);
        }

        for (int j = 0; j < cell.getGatePortalCount(); j++)
        {
            const CellGraph::GatePortal& portal = cell.getGatePortal(j);

            const Hash<Vector4, CellGraph::PortalHull>& hulls = portal.getPortalHulls();
            Hash<Vector4, CellGraph::PortalHull>::Iterator iter = hulls.iterate();

            while (hulls.isValid(iter))
            {
                const Vector4& pleq = hulls.getKey(iter);
                const CellGraph::PortalHull& portalHull = hulls.getValue(iter);
                Portal outp;

                int geometryOffset = hullOffsets.getDefault(portalHull.getVertices(), m_vertices.getSize());
                if (geometryOffset == m_vertices.getSize())
                {
                    Vector3 center = portalHull.getCenter();
                    UMBRA_ASSERT(m_aabb.contains(center));
                    m_vertices.pushBack(center);
                    m_vertices.pushBack(Vector3(pleq.x, pleq.y, pleq.z));
                    m_vertices.pushBack(Vector3(pleq.w, portalHull.getMinRadius(center), portalHull.getMaxRadius(center)));
                    m_vertices.append(portalHull.getVertices());
                }

                int geometryLen = 3 + portalHull.getVertexCount();

                int gateOffset = m_gateIdHash.getDefault(portal.getGateIDs(), m_gateIdxs.getSize());
                if (gateOffset == m_gateIdxs.getSize())
                {
                    Set<int>::Iterator iter = portal.getGateIDs().iterate();
                    while (iter.next())
                        m_gateIdxs.pushBack(*m_gateIdToIndex.get(iter.getValue()));
                }

                initUserPortal(outp, BUILD_PORTAL_LINK(0, 0, 1, 0, portal.getTarget()),
                    0xFFFF, gateOffset, portal.getGateIDs().getSize(), geometryOffset, geometryLen);
                portals.pushBack(outp);
                hulls.next(iter);
            }
        }

        ClusterNode clusterEntry;
        clusterEntry.setPortalIdxAndCount(portalIdx, portals.getSize() - portalIdx);
        PackedAABB pAABB;
        pAABB.pack(m_aabb, cell.getAABB());
        clusterEntry.setBounds(pAABB);
        output.pushBack(clusterEntry);
    }

    return true;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TomeWriter::collectObjects(Tile* srcTile)
{
    const CellGraph& cg = srcTile->m_cellGraph;

    for (int i = 0; i < cg.getCellCount(); i++)
    {
        const CellGraph::Cell& cell = cg.getCell(i);

        Array<int> objs(getAllocator());
        Array<AABB> bounds(getAllocator());

        cell.getObjects(objs);
        cell.getObjectBounds(bounds);

        for (int j = 0; j < cell.getObjectCount(); j++)
        {
            ObjectParams p = cg.getTargetObject(objs[j]);
            // replace original object bounds with per-cell bounds here
            p.m_bounds = bounds[j];
            int outIdx = m_targetIdToIndex.getDefault(p.getId(), m_targetObjs.getSize());
            if (outIdx == m_targetObjs.getSize())
                m_targetObjs.pushBack(p);
            else
                m_targetObjs[outIdx].m_bounds.grow(p.m_bounds);
        }

        // todo [turkka] gates in non-reachable cells?
        for (int j = 0; j < cell.getGatePortalCount(); j++)
        {
            const CellGraph::GatePortal& portal = cell.getGatePortal(j);

            Array<int> gates(getAllocator());
            portal.getGateIDs().getArray(gates);

            for (int g = 0; g < gates.getSize(); g++)
            {
                int id = gates[g];

                if (!m_gateIdToIndex.contains(id))
                {
                    m_gateIdToIndex.insert(id, m_gateIndexToId.getSize());
                    m_gateIndexToId.pushBack(id);
                }
            }
        }
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TomeWriter::remapObjects(TileTreeNode* node, Set<int>& used, Array<int>& remap, int& n)
{
    if (node->isLeaf())
    {
        const Tile* t = m_tiles[node->m_tile->m_slot];
        const CellGraph& cg = t->m_cellGraph;

        for (int i = 0; i < cg.getCellCount(); i++)
        {
            const CellGraph::Cell& cell = cg.getCell(i);

            Array<int>  objects(getAllocator());
            cell.getObjects(objects);

            for (int j = 0; j < objects.getSize(); j++)
            {
                int* g = m_targetIdToIndex.get(cg.getTargetObject(objects[j]).getId());
                if (!g)
                    continue;
                if (used.contains(*g))
                    continue;

                remap[*g] = n++;
                used.insert(*g);
            }
        }
    }
    else
    {
        remapObjects(node->getLeft(), used, remap, n);
        remapObjects(node->getRight(), used, remap, n);
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TomeWriter::connectTile(AABBi aabb, TileTreeNode* node, Tile* srcTile, int faceMask, bool hq)
{
    // Process only srcTile's neighbors

    if (!aabb.intersectsWithArea(AABBi(srcTile->m_aabb)))
        return;

    if (node->isLeaf())
    {
        // Neighboring tile found: connect
        int otherIdx = node->m_tile->m_slot;
        UMBRA_ASSERT(m_tiles[otherIdx]->m_aabb == aabb);
        if (otherIdx == srcTile->m_slot)
        {
            // Connect to borders.

            for (int i = 0; i < 6; i++)
            {
                if (faceMask & (1 << i))
                {
                    srcTile->m_externalCellGraph.connectBorder(i);
                    srcTile->m_borderMask |= (1 << i);
                }
            }

            return;
        }

        float fs = min2(srcTile->m_featureSize, m_tiles[otherIdx]->m_featureSize) * 0.25f;

        srcTile->m_externalCellGraph.connectTo(&m_tiles[otherIdx]->m_cellGraph, otherIdx, hq ? fs : 0.f);
    }
    else
    {
        // Process left and right children

        int axis = node->m_splitAxis;
        int pos = node->m_splitPos;

        Vector3i mn = aabb.getMin();
        Vector3i mx = aabb.getMax();

        Vector3i mn2 = aabb.getMin();
        mn2[axis] = pos;
        Vector3i mx2 = aabb.getMax();
        mx2[axis] = pos;

        connectTile(AABBi(mn, mx2), node->getLeft(), srcTile, faceMask & ~(1 << (axis*2+1)), hq);
        connectTile(AABBi(mn2, mx), node->getRight(), srcTile, faceMask & ~(1 << (axis*2)), hq);
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TomeWriter::TileTreeNode* TomeWriter::buildTree(int* tiles, int n, Vector3i mn, Vector3i mx)
{
    for (int i = 0; i < n; i++)
    {
        Vector3i mn2 = m_tiles[tiles[i]]->m_aabb.getMin();
        Vector3i mx2 = m_tiles[tiles[i]]->m_aabb.getMax();

        UMBRA_UNREF(mn2);
        UMBRA_UNREF(mx2);
        UMBRA_ASSERT(AABBi(mn, mx).contains(AABBi(mn2, mx2)));
    }

    UMBRA_ASSERT(mx.i - mn.i >= 1);
    UMBRA_ASSERT(mx.j - mn.j >= 1);
    UMBRA_ASSERT(mx.k - mn.k >= 1);
    UMBRA_ASSERT(mx - mn != Vector3i(1, 1, 1) || n <= 1);

    // Leaf if all aabbs match.

    bool isLeaf = true;

    // \todo [Hannu] assumes that tiles have the smallest possible size
    if (n > 1)
        isLeaf = false;

    // Empty space around the tile?
    if (n == 1 && (m_tiles[tiles[0]]->m_aabb.getMin() != mn || m_tiles[tiles[0]]->m_aabb.getMax() != mx))
        isLeaf = false;

    if (isLeaf)
    {
        TileTreeNode* node = UMBRA_NEW(TileTreeNode);

        if (n)
        {
            UMBRA_ASSERT(n == 1);
            node->m_tile = m_tiles[tiles[0]];
        }
        else
        {
            // Fill (outside) empty tile.

            Tile* t = UMBRA_NEW(Tile, getCtx());

            AABB aabb = AABBi(mn, mx).toFloat(m_unitSize);

            CellGraph& cg = t->m_cellGraph;
            cg.setAABB(aabb);

            for (int j = 0; j < 6; j++)
            {
                SubdivisionTree st(getAllocator());
                SubdivisionTree::LeafNode* node = st.newLeaf();
                node->setIndex(0);
                st.setRoot(node);
                AABB aabb2 = aabb;
                aabb2.flattenToFace(j);
                st.setAABB(aabb2);
                cg.getMatchingTree(j).serialize(st);
            }

            SubdivisionTree tree(getAllocator());
            SubdivisionTree::LeafNode* n = tree.newLeaf();
            n->setIndex(m_strictViewVolumes ? -1 : 0);
            tree.setRoot(n);
            tree.setAABB(aabb);
            cg.getViewTree().serialize(tree);

            CellGraph::Cell& c = cg.addCell();
            c.setAABB(aabb);

            t->m_externalCellGraph = ExternalCellGraph(&t->m_cellGraph);

            t->m_aabb.m_min = mn;
            t->m_aabb.m_max = mx;

            t->m_slot = -1;
            t->m_imp  = 0;

            node->m_tile = t;
            m_tiles.pushBack(t);
        }

        return node;
    }

    // Find split
    // TODO: rethink splitting heuristic

    int   axis = -1;
    int   pos  = -1;
    float bestRatio = 0.f;
    int   bestDiff = INT_MAX;

    for (int i = 0; i < 3; i++)
    {
        BitVector bv(mx[i] - mn[i] - 1, getAllocator());
        bv.clearAll();

        for (int j = 0; j < n; j++)
        {
            int s = m_tiles[tiles[j]]->m_aabb.m_min[i]+1;
            int e = m_tiles[tiles[j]]->m_aabb.m_max[i];

            for (int k = s; k < e; k++)
                bv.set(k - (mn[i]+1));
        }

        for (int j = mn[i]+1; j < mx[i]; j++)
        {
            if (bv.test(j - (mn[i]+1)))
                continue;

            AABBi left = AABBi(mn, mx);
            AABBi right = AABBi(mn, mx);

            left.setMax(i, j);
            right.setMin(i, j);

            int tilesToTheLeft = 0;
            for (int k = 0; k < n; k++)
            {
                const AABBi& childAABB = m_tiles[tiles[k]]->m_aabb;
                UMBRA_ASSERT(childAABB.m_min[i] >= j ||
                             childAABB.m_max[i] <= j);
                if (childAABB.m_max[i] <= j)
                    tilesToTheLeft++;                
            }                        

            float ratio = min2(left.getMinAxisLength() / (float)left.getMaxAxisLength(),
                               right.getMinAxisLength() / (float)right.getMaxAxisLength());
            int   diff = abs(n / 2 - tilesToTheLeft);

            if (ratio < bestRatio)
                continue;

            // If ratios match, prefer scene's longer axises first. The
            // motivation is that occlusion doesn't happen in the shortest
            // direction and tiles collapse more easily.

            if (ratio == bestRatio)
            {
                if (diff > bestDiff)
                    continue;
                if (diff == bestDiff)
                {
                    Vector3 sceneSize = m_aabb.getDimensions();
                    if (sceneSize[i] < sceneSize[axis])
                        continue;
                }
            }

            // Test that it is splittable.

#if 0
            int k;
            for (k = 0; k < n; k++)
            {
                if (j > m_tiles[tiles[k]]->m_aabb.m_min[i] &&
                    j < m_tiles[tiles[k]]->m_aabb.m_max[i])
                    break;
            }

            if (k != n)
                continue;
#endif

            // Test that one of the tiles touches the splitting position (avoids unnecessary empty space tiles).

            int k;
            for (k = 0; k < n; k++)
            {
                if (m_tiles[tiles[k]]->m_aabb.m_min[i] == j ||
                    m_tiles[tiles[k]]->m_aabb.m_max[i] == j)
                    break;
            }

            if (k == n)
                continue;

            bestDiff = diff;
            bestRatio = ratio;
            axis = i;
            pos = j;
        }
    }

    UMBRA_ASSERT(axis >= 0);

    // Sort.

    int i, j;
    for (i = 0, j = 0; i < n; i++)
    {
        if (m_tiles[tiles[i]]->m_aabb.m_max[axis] <= pos)
        {
            if (i != j)
                swap2(tiles[i], tiles[j]);
            j++;
        }
        else if (m_tiles[tiles[i]]->m_aabb.m_min[axis] >= pos)
            ;
        else
        {
            UMBRA_ASSERT(0);
            return 0;
        }
    }

    // Recurse.

    Vector3i mn2 = mn;
    mn2[axis] = pos;
    Vector3i mx2 = mx;
    mx2[axis] = pos;

    TileTreeNode* node = UMBRA_NEW(TileTreeNode);
    node->m_splitAxis = axis;
    node->m_splitPos = pos;
    node->m_left = buildTree(&tiles[0], j, mn, mx2);
    node->m_right = buildTree(&tiles[j], n-j, mn2, mx);

    return node;
}

void TomeWriter::collapseTree(TileTreeNode* node, int& tiles)
{
    tiles++;
    float p = (float)tiles / (float)m_tiles.getSize();
    p = min2(1.f, max2(p, m_progress.getPhaseProgress()));
    m_progress.setPhaseProgress(p);

    if (node->isLeaf())
    {
        node->m_tile->m_externalCellGraph = ExternalCellGraph(&node->m_tile->m_cellGraph);
        return;
    }

    collapseTree(node->m_left, tiles);
    collapseTree(node->m_right, tiles);

    if (!node->m_left->isLeaf() || !node->m_right->isLeaf())
        return;

    // Check if this tile should be collapsed.

    Tile* left = node->m_left->m_tile;
    Tile* right = node->m_right->m_tile;

    int numLeft = left->m_cellGraph.getCellCount();
    int numRight = right->m_cellGraph.getCellCount();
    UMBRA_ASSERT(!node->m_tile);

    AABB newAABB = left->getAABB();
    newAABB.grow(right->getAABB());

    bool hasViewVolume = left->m_viewVolume.getSize() >= 0 || right->m_viewVolume.getSize() >= 0;

    if (hasViewVolume && m_clusterSize > 0.f && newAABB.getMaxAxisLength() > m_clusterSize)
        return;

    int collapseLimit = TILE_CELL_COUNT_TARGET;

    if (numLeft + numRight >= collapseLimit)
        return;

    // Collapse tiles.

    Tile* t = UMBRA_NEW(Tile, getCtx());

    t->m_aabb         = left->m_aabb;
    t->m_aabb.grow(right->m_aabb);
    t->m_featureSize = min2(left->m_featureSize, right->m_featureSize);

    t->m_slot = -1;
    t->m_borderMask = left->m_borderMask | right->m_borderMask;

    t->m_viewVolume   = left->m_viewVolume;
#if 1
    for (int i = 0; i < right->m_viewVolume.getSize(); i++)
    {
        int j;
        for (j = 0; j < t->m_viewVolume.getSize(); j++)
            if (t->m_viewVolume[j] == right->m_viewVolume[i])
                break;
        if (j == t->m_viewVolume.getSize())
            t->m_viewVolume.pushBack(right->m_viewVolume[i]);
    }
#else
    t->m_viewVolume.append(right->m_viewVolume);
#endif

    CellGraph cg = left->m_cellGraph;
    cg.joinRight(right->m_cellGraph, true, t->m_featureSize * 0.25f);

#if 1
    PortalGrouperParams params;
    params.featureSize = min2(left->m_featureSize, right->m_featureSize) * .5f;
    params.strategy = PortalGrouperParams::COLLAPSE;

    if (left->m_aabb.getMax()[0] == right->m_aabb.getMin()[0])
        params.planeAxis = 0;
    else if (left->m_aabb.getMax()[1] == right->m_aabb.getMin()[1])
        params.planeAxis = 1;
    else
    {
        UMBRA_ASSERT(left->m_aabb.getMax()[2] == right->m_aabb.getMin()[2]);
        params.planeAxis = 2;
    }

    params.planeZ = left->m_cellGraph.getAABB().getMax()[params.planeAxis];

    PortalGrouper pg(getCtx()->getPlatform(), t->m_cellGraph, cg, params);
    pg.perform();
#else
    t->m_cellGraph = cg;
#endif

    t->m_externalCellGraph = ExternalCellGraph(&t->m_cellGraph);

    t->m_isLeaf = true;

    left->m_cellGraph = CellGraph();
    right->m_cellGraph = CellGraph();

    UMBRA_DELETE(node->m_left->m_tile);
    UMBRA_DELETE(node->m_left);
    for (int i = 0; i < m_tiles.getSize(); i++)
        if (m_tiles[i] == left)
        {
            m_tiles[i] = m_tiles[m_tiles.getSize()-1];
            m_tiles.popBack();
            tiles--;
        }

    UMBRA_DELETE(node->m_right->m_tile);
    UMBRA_DELETE(node->m_right);
    for (int i = 0; i < m_tiles.getSize(); i++)
        if (m_tiles[i] == right)
        {
            m_tiles[i] = m_tiles[m_tiles.getSize()-1];
            m_tiles.popBack();
            tiles--;
        }

    node->m_left = 0;
    node->m_right = 0;

    node->m_tile = t;
}

void TomeWriter::bitpackTree (Umbra::UINT32* bv, float* pos, const Array<TileTreeNode*>& nodes) const
{
    UMBRA_ASSERT(nodes.getSize());

    for (int i = 0; i < nodes.getSize(); i++)
    {
        if (nodes[i]->isLeaf())
        {
            setBit(bv, i*2);
            setBit(bv, i*2+1);

            pos[i] = FLT_MAX;
        }
        else
        {
            int axis = nodes[i]->getAxis();
            if (axis == 1)
                setBit(bv, i*2);
            else
                clearBit(bv, i*2);
            if (axis == 2)
                setBit(bv, i*2+1);
            else
                clearBit(bv, i*2+1);

            pos[i] = nodes[i]->getSplitPos() * m_unitSize;
        }
    }

    for (unsigned i = nodes.getSize()*2; i < UMBRA_BITVECTOR_SIZE(nodes.getSize()*2) * 8; i++)
        clearBit(bv, i);
}


/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TomeWriter::bitpackTree (Umbra::UINT32* bv, const Array<const SubdivisionTree::Node*>& nodes) const
{
    UMBRA_ASSERT(nodes.getSize());

    for (int i = 0; i < nodes.getSize(); i++)
    {
        if (nodes[i]->isLeaf())
        {
            setBit(bv, i*2);
            setBit(bv, i*2+1);
            continue;
        }

        int axis;
        if (nodes[i]->isMedian())
            axis = nodes[i]->getMedian()->getAxis();
        else
            axis = nodes[i]->getAxial()->getAxis();

        if (axis == 1)
            setBit(bv, i*2);
        else
            clearBit(bv, i*2);
        if (axis == 2)
            setBit(bv, i*2+1);
        else
            clearBit(bv, i*2+1);
    }

    for (unsigned i = nodes.getSize()*2; i < UMBRA_BITVECTOR_SIZE(nodes.getSize()*2) * 8; i++)
        clearBit(bv, i);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

ImpTomeGenerator::ImpTomeGenerator (BuildContext* ctx, const ComputationParams& params, const AABB& aabb)
:   BuilderBase(ctx),
    m_visLock(getAllocator()),
    m_params(getAllocator()),
    m_numThreads(1),
    m_cachePath(getAllocator()),
    m_aabb(aabb),
    m_serializedInput(getAllocator()),
    m_rebuild(true),
    m_tome(NULL),
    m_progress(0.f)
{
    m_params = params;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

ImpTomeGenerator::~ImpTomeGenerator (void)
{
    Hash<AABBi, SerializedTile>::Iterator it = m_serializedInput.iterate();
    while(m_serializedInput.isValid(it))
    {
        UMBRA_DELETE(m_serializedInput.getValue(it).m_serialized);
        m_serializedInput.next(it);
    }

    if (m_tome)
    {
        UMBRA_FREE(m_tome);
        m_tome = NULL;
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error ImpTomeGenerator::addTile (const ImpTileResult* tile)
{
    AABBi aabb(tile->m_aabbMin, tile->m_aabbMax);

    BuildContext* ctx = getCtx()->enter();
    Builder::Error err = Builder::SUCCESS;
    try
    {
        // TODO: check tileSize matching, overlapping, etc.

        ScopedCriticalSectionEnter lock(m_visLock);

        SerializedTile* serializedTile = m_serializedInput.get(aabb);
        UMBRA_ASSERT(!serializedTile);
        if(serializedTile)
        {
            UMBRA_DELETE(serializedTile->m_serialized);
            serializedTile->m_serialized = NULL;
        } else
            serializedTile = m_serializedInput.insert(aabb, SerializedTile());

        MemOutputStream* tileStream = UMBRA_NEW(MemOutputStream, getAllocator());

        serializedTile->m_aabb       = tile->m_cellGraph.getAABB();
        serializedTile->m_serialized = tileStream;

        Serializer serializer(tileStream);
        stream(serializer, *(ImpTileResult*)tile);
        UMBRA_ASSERT(serializer.isOk());
    }
    catch (OOMException)
    {
        err = Builder::ERROR_OUT_OF_MEMORY;

        SerializedTile* serializedTile = m_serializedInput.get(aabb);
        if (serializedTile)
        {
            UMBRA_DELETE(serializedTile->m_serialized);
            serializedTile->m_serialized = NULL;
            m_serializedInput.remove(aabb);
        }
    }
    ctx->leave();

    m_rebuild = true;

    return err;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

float ImpTomeGenerator::getProgress()
{
    return m_progress;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error ImpTomeGenerator::getTomeSize (Umbra::UINT32& size)
{
    updateTome();
    if (!m_tome)
        return Builder::ERROR_PARAM;
    size = m_tome->getSize();
    return Builder::SUCCESS;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

const Tome* ImpTomeGenerator::getTome (Umbra::UINT8* buf, Umbra::UINT32 bufSize)
{
    updateTome();
    if (!m_tome)
        return NULL;
    memcpy(buf, m_tome, min2(m_tome->getSize(), bufSize));
    return (Tome*)buf;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpTomeGenerator::updateTome()
{
    if (!m_rebuild || !m_serializedInput.getNumKeys())
        return;

    if (m_tome)
    {
        UMBRA_FREE(m_tome);
        m_tome = NULL;
    }

    TomeWriter tw(getCtx(), m_aabb, &m_progress);

    tw.setNumThreads(m_numThreads);

    float groupCost;
    Vector3 worldSize;
    m_params.getParam(ComputationParams::OBJECT_GROUP_COST, groupCost);
    m_params.getParam(ComputationParams::WORLD_SIZE, worldSize);
    tw.setObjectGroupParams(groupCost, worldSize);

    float hierarchyDetail;
    m_params.getParam(ComputationParams::HIERARCHY_DETAIL, hierarchyDetail);
    tw.setHierarchyDetail(hierarchyDetail);

    float clusterSize;
    m_params.getParam(ComputationParams::CLUSTER_SIZE, clusterSize);
    if (clusterSize > 0.f)
        tw.setClusterSize(clusterSize);

    float minAccurateDistance;
    m_params.getParam(ComputationParams::MINIMUM_ACCURATE_DISTANCE, minAccurateDistance);
    tw.setMinAccurateDistance(minAccurateDistance);

    UINT32 flags;
    m_params.getParam(ComputationParams::OUTPUT_FLAGS, flags);

    tw.setCompVisualizations(!!(flags & ComputationParams::DATA_VISUALIZATIONS));
    tw.setMatchingData(!!(flags & ComputationParams::DATA_TOME_MATCH));
    tw.setStrictViewVolumes(!!(flags & ComputationParams::DATA_STRICT_VIEW_VOLUMES));
    tw.setDepthMaps(!!(flags & ComputationParams::DATA_OBJECT_OPTIMIZATIONS));
    tw.setDepthMapsInf(!!(flags & ComputationParams::DATA_SHADOW_OPTIMIZATIONS));
    tw.setCachePath(m_cachePath.toCharPtr());

    LOGI("Strict view volumes: %s\n", !!(flags & ComputationParams::DATA_STRICT_VIEW_VOLUMES) ? "yes" : "no");
    // Deserialize tiles and add to TomeWriter.

    Hash<AABBi, SerializedTile>::Iterator it = m_serializedInput.iterate();
    while(m_serializedInput.isValid(it))
    {
        SerializedTile& tile = m_serializedInput.getValue(it);

        MemInputStream inputStream(tile.m_serialized->getPtr(), tile.m_serialized->getSize());
        Deserializer loader(&inputStream, getAllocator());

        ImpTileResult result(getCtx());
        stream(loader, result);
        UMBRA_ASSERT(loader.isOk());
        tw.addTileResult(result);
        m_serializedInput.next(it);
    }

    // Generate tome.

    {
        UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY
        m_tome = tw.generateTome();
    }

    m_rebuild = false;
    m_progress = 1.f;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpTomeGenerator::visualize(DebugRenderer* debug)
{
    UMBRA_ASSERT(debug);

    ScopedCriticalSectionEnter lock(m_visLock);

    Hash<AABBi, SerializedTile>::Iterator it = m_serializedInput.iterate();
    while (m_serializedInput.isValid(it))
    {
        const SerializedTile& tile = m_serializedInput.getValue(it);
        const AABB& aabb = tile.m_aabb;

        debug->addAABB(aabb.getMin(), aabb.getMax(), Vector4(0,0,1,1));

        m_serializedInput.next(it);
    }
}
#endif
