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
 * \brief   Tile computation
 *
 */

#include "umbraTileProcessor.hpp"
#include "umbraComputationTile.hpp"
#include "umbraRT.hpp"
#include "umbraPortalGrouper.hpp"
#include "umbraFPUControl.hpp"
#include "umbraOptimizerTextCommand.hpp"
#include <optimizer/DebugCollector.hpp>
#include <optimizer/VisualizeHelper.hpp>
#include <standard/MigrateFromCommon.hpp>

using namespace Umbra;

#define COLLAPSE_GROUPING_ITERS 1

TileProcessor::TileProcessor (BuildContext* ctx) : BuilderBase(ctx)
{
}

TileProcessor::~TileProcessor (void)
{
}

ImpTileResult* TileProcessor::execute (const ImpTileInput& in)
{
    UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY;
    UMBRA_DISABLE_FLOATING_POINT_EXCEPTIONS_TEMPORARY;

    // Compute local cell graph.

    ImpTileResult* res = UMBRA_NEW(ImpTileResult, getCtx());
    res->m_aabbMin = in.getAABBMin();
    res->m_aabbMax = in.getAABBMax();
    res->m_unitSize = in.getUnitSize();
    res->m_featureSize = in.getCellGeneratorParams().featureSize;
    res->m_computationString = in.getComputationString();

    DebugCollector dc(getCtx()->getMemory(), res->m_graphics);
    dc.setFilterSpec(g_debugVisualizationSpec);

    if (dc.pushActive("compute_tile_bounds"))
    {
        VisualizeHelper(dc).aabbEdges(migrate(in.getAABB()));
        dc.popActive();
    }

    for (int i = 0; i < in.getGeometry().getViewVolumeCount(); i++)
    {
        if (!in.getGeometry().getViewVolume(i).isClusterMarker)
            res->m_viewVolume.pushBack(in.getGeometry().getViewVolume(i).aabb);
    }

    {
        // Compute ray tracing hierarchy.

        RayTracer rt(getCtx()->getPlatform());
        rt.buildBVH(in.getGeometry());

        // Run cell generation

        TopCellGenerator cg(getCtx()->getPlatform(), in.getGeometry(), in.getCellGeneratorParams(),
            in.getAABBMax() - in.getAABBMin(), &rt, dc);
        cg.perform();
        res->m_cellGraph = cg.getCellGraph();
    }

    res->m_cellGraph.removeNonConnectedCells();

    if (in.getCellGeneratorParams().featureSize > 0.f)
    {
        PortalGrouperParams pgParams;
        pgParams.featureSize = in.getCellGeneratorParams().featureSize;

        // Box grouping

        {
            pgParams.strategy = PortalGrouperParams::BOX;
            CellGraph in = res->m_cellGraph;
            PortalGrouper pg(getCtx()->getPlatform(), res->m_cellGraph, in, pgParams);
            pg.perform();
        }

        // Additional collapse grouping rounds

        int iters = COLLAPSE_GROUPING_ITERS;
        while (iters--)
        {
            pgParams.strategy = PortalGrouperParams::COLLAPSE;
            CellGraph in = res->m_cellGraph;
            PortalGrouper pg(getCtx()->getPlatform(), res->m_cellGraph, in, pgParams);
            pg.perform();
        }
    }

    SubdivisionTree st(getCtx()->getPlatform().allocator);
    res->m_cellGraph.getViewTree().deserialize(st);
    st.setRoot(SubdivisionTreeUtils(st).collapse(st.getRoot(), true));
    res->m_cellGraph.getViewTree().serialize(st);

    res->m_cellGraph.removeNonConnectedCells();

    return res;
}

#endif
