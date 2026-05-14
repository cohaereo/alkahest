#pragma once

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2007-2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Reachability analysis
 *
 */

#include "umbraCellGraph.hpp"
#include "umbraExtCellGraph.hpp"
#include "umbraBuildContext.hpp"
#include "umbraUnionFind.hpp"
#include "umbraSet.hpp"

namespace Umbra
{

    class ReachabilityAnalysis : public BuilderBase
    {
    public:

        ReachabilityAnalysis  (BuildContext* ctx);
        ~ReachabilityAnalysis (void);

        void skipTile() { m_tiles.pushBack(Tile()); }
        void addTile(CellGraph& cg, ExternalCellGraph& ecg, const Array<AABB>* viewVolume, UINT32 borderMask);

        bool execute(bool borderVisibility, bool strictViewVolumes);

        struct Tile
        {
            Tile();
            Tile(CellGraph& cg, ExternalCellGraph& ecg, const Array<AABB>* viewVolume, UINT32 borderMask);

            void setAllocator (Allocator* heap)
            {
                m_cellVis.setAllocator(heap);
            }

            CellGraph*                m_cg;
            ExternalCellGraph*        m_ecg;
            const Array<AABB>*        m_viewVolume;
            Array<AABB>               m_cellVis;
            UINT32                    m_borderMask;
        };

        struct GlobalCluster
        {
            GlobalCluster(void): forceReachable(false), inside(false) {}

            Array<Vector2i> cells;
            bool forceReachable;
            bool inside;
        };

    private:

        bool sanitize (void);
        void buildClusters (Array<GlobalCluster>& clusters, bool insides, bool backfaceCull);
        bool propagateVisibility (int tile, int cell, const AABB& aabb, int depth);
        void tagBorderReachables (void);
        void buildCellVisibility (bool strictViewVolumes);
        void removeNonreachableLinks (void);

        Array<Tile> m_tiles;
    };

    static inline void copyHeap (ReachabilityAnalysis::Tile* elem, Allocator* heap)
    {
        elem->setAllocator(heap);
    }

    static inline void copyHeap (ReachabilityAnalysis::GlobalCluster* elem, Allocator* heap)
    {
        elem->cells.setAllocator(heap);
    }

} // namespace Umbra
