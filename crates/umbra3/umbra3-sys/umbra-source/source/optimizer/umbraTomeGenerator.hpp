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

#include "umbraPrivateDefs.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraBuildContext.hpp"
#include "umbraGeometryBlock.hpp"
#include "umbraComputationTile.hpp"
#include "umbraStructBuilder.hpp"
#include "umbraThread.hpp"
#include "umbraIndexListCombiner.hpp"
#include "umbraPair.hpp"
#include "umbraProgress.hpp"
#include "umbraCubemap.hpp"

namespace Umbra
{

class Tome;

class TomeWriter : public BuilderBase
{
public:

                    TomeWriter              (BuildContext* ctx, const AABB& aabb, float* progress);
                    ~TomeWriter             (void);

    void            setNumThreads           (int n) { m_numThreads = n; }
    void            setCachePath            (const char* path) { m_cachePath = path; }

    void            addTileResult           (const ImpTileResult& tile);
    void            reset                   (void);

    Tome*           generateTome            (void);

    // Called from thread.
    bool            processJob              (void);

    void            setCompVisualizations   (bool b) { m_computeVisualizations = b; }
    void            setMatchingData         (bool b) { m_matchingData = b; }
    void            setStrictViewVolumes    (bool b) { m_strictViewVolumes = b; }
    void            setDepthMaps            (bool b) { m_depthMaps = b; }
    void            setDepthMapsInf         (bool b) { m_depthMapsInf = b; }

    void            setObjectGroupParams    (float cost, const Vector3& worldSize) { m_objectGroupCost = cost; m_objectGroupWorldSize = worldSize; }

    void            setHierarchyDetail      (float hd) { m_hierarchyDetail = hd; }
    void            setClusterSize          (float cs) { m_clusterSize = cs; }
    void            setMinAccurateDistance  (float mad) { m_minAccurateDistance = mad; }

    typedef Pair<int, int> GlobalCell;

private:

    struct Tile
    {
        Tile(BuildContext* ctx)
            :   m_isLeaf(false),
                m_cellGraph(ctx->getPlatform().allocator),
                m_viewVolume(ctx->getPlatform().allocator),
                m_featureSize(0.f),
                m_externalCellGraph(ctx->getPlatform().allocator),
                m_borderMask(0),
                m_parent(NULL),
                m_numClusters(0),
                m_leafCellMap(ctx->getPlatform().allocator),
                m_incomingPortals(0)
        {}

        const AABB& getAABB(void) const { return m_cellGraph.getAABB(); }

        bool isLeaf (void) const
        {
            return m_isLeaf;
        }

        int remapCell (int leafTile, int leafCell) const
        {
            if (m_isLeaf)
            {
                UMBRA_ASSERT(leafTile == m_slot);
                return leafCell;
            }
            const int* idx = m_leafCellMap.get(GlobalCell(leafTile, leafCell));
            return idx ? *idx : -1;
        }

        int getDepth (int i = 0) const
        {
            ++i;
            if (!m_parent)
                return i;
            return m_parent->getDepth(i);
        }

        bool overlaps (const Tile* o) const
        {
            return m_aabb.intersectsWithVolume(o->m_aabb);
        }

        bool                        m_isLeaf;
        AABBi                       m_aabb;
        CellGraph                   m_cellGraph;
        Array<AABB>                 m_viewVolume;
        float                       m_featureSize;

        ExternalCellGraph           m_externalCellGraph;
        int                         m_slot;
        ImpTile*                    m_imp;
        UINT32                      m_borderMask;
        Tile*                       m_parent;
        int                         m_numClusters;

        // used by tile hierarchy building, consider separating?
        Hash<GlobalCell, int>       m_leafCellMap;
        int                         m_incomingPortals;
    };

    struct HierarchyStackData
    {
        HierarchyStackData(Allocator* a)
            : numInputs(0), cellGraph(a), ext(a), leafTiles(a), leafCellCounts(a)
        {}

        AABBi aabb;
        int numInputs;
        int inputCellCount;
        CellGraph cellGraph;
        ExternalCellGraph ext;
        Array<int> leafTiles;
        Array<int> leafCellCounts;
        bool hasLeaves;
    };

    struct TileTreeNode
    {
        TileTreeNode() : m_splitAxis(-1), m_splitPos(0), m_tile(0), m_left(0), m_right(0), m_parent(0) {}

        bool isLeaf() const { UMBRA_ASSERT((m_left == 0) == (m_right == 0)); return m_left == 0; }

        int countNodes() const { return isLeaf() ? 1 : 1 + m_left->countNodes() + m_right->countNodes(); }

        int getAxis() const { UMBRA_ASSERT(!isLeaf()); UMBRA_ASSERT(m_splitAxis >= 0 && m_splitAxis <= 2); return m_splitAxis; }
        int getSplitPos() const { UMBRA_ASSERT(!isLeaf()); return m_splitPos; }

        TileTreeNode* getLeft() { UMBRA_ASSERT(!isLeaf()); return m_left; }
        TileTreeNode* getRight() { UMBRA_ASSERT(!isLeaf()); return m_right; }

        int           m_splitAxis;
        int           m_splitPos;
        Tile*         m_tile;
        TileTreeNode* m_left;
        TileTreeNode* m_right;
        TileTreeNode* m_parent;
    };

    void                createTilesFromResults  (void);

    ImpTome*            generateImpTome         (void);
    ImpTile*            generateTile            (Tile* srcTile);

    void                generateIndexLists      (void);
    void                serializeTreeData       (BaseStructBuilder& builder, SerializedTreeData& out, SubdivisionTree& st, int mapWidth = -1);

    bool                calculateReachability   (void);
    int                 countCells              (bool reachable);

    bool                buildTopLevel           (void);
    bool                buildCellGraph          (Tile* srcTile, Array<CellNode>& cells, Array<Portal>& portals);
    bool                buildClusterGraph       (Array<ClusterNode>& cells, Array<Portal>& portals);

    struct BuildTileJob;

    void                buildTileJobsThread     (void);
    void                collectBuildTileJobs    (TileTreeNode* node, HierarchyStackData* dst, BuildTileJob* parent);

    void                buildDepthmapThread      (void);  

    void                buildInnerTile          (HierarchyStackData& out, const HierarchyStackData& right, TileTreeNode* node);
    void                connectInnerTiles       (Tile* t);
    void                pruneDisconnectedTiles  (Tile* t);

    void                processPortal           (Tile* srcTile, Tile* dstTile, const CellGraph::Portal& edge, Array<Portal>& portals);
    void                processPortal           (Tile* srcTile, Tile* dstTile, const ExternalCellGraph::Portal& edge, Array<Portal>& portals);

    void                collectObjects          (Tile* srcTile);
    void                remapObjects            (TileTreeNode* node, Set<int>& used, Array<int>& remap, int& n);
    void                connectTile             (AABBi aabb, TileTreeNode* node, Tile* srcTile, int faceMask, bool hq);

    TileTreeNode*       buildTree               (int* tiles, int n, Vector3i mn, Vector3i mx);
    void                collapseTree            (TileTreeNode* node, int&);

    void                bitpackTree             (Umbra::UINT32* bv, float* pos, const Array<TileTreeNode*>& nodes) const;
    void                bitpackTree             (Umbra::UINT32* bv, const Array<const SubdivisionTree::Node*>& nodes) const;

    ImpTome*            computeStaticVisibility (ImpTome* inTome);

    float        getLodLevel                    (const AABBi& aabb)
    {
        float vol = aabb.toFloat(m_lodScaling).getVolume();
        return (float)pow((double)vol, 1 / 3.0);
    }

    bool                containsFace            (const AABB& src, const AABB& target, int face)
    {
        int axis = getFaceAxis(face);
        AABB targetRect(target);
        if (getFaceDirection(face))
        {
            Vector3 mx = targetRect.getMax();
            mx[axis] = targetRect.getMin()[axis];
            targetRect.setMax(mx);
        }
        else
        {
            Vector3 mn = targetRect.getMin();
            mn[axis] = targetRect.getMax()[axis];
            targetRect.setMin(mn);
        }
        return src.contains(targetRect);
    }

    void setTileParents (TileTreeNode* node, Tile* parent)
    {
        if (node->m_tile)
        {
            node->m_tile->m_parent = parent;
            parent = node->m_tile;
        }
        if (!node->isLeaf())
        {
            setTileParents(node->getLeft(), parent);
            setTileParents(node->getRight(), parent);
        }
    }

    Timer                                   m_timer;
    int                                     m_numThreads;
    String                                  m_cachePath;
    Array<Tile*>                            m_tiles;
    AABB                                    m_emptyAABB;
    AABB                                    m_aabb;
    TileTreeNode*                           m_root;
    Array<Vector3>                          m_vertices;
    Hash<AABB, int>                         m_gateAABBHash;
    Hash<UINT32, int>                       m_targetIdToIndex;
    Array<ObjectParams>                     m_targetObjs;
    Array<Array<UINT32> >                   m_groupToTargetIds;
    Hash<int, int>                          m_gateIdToIndex;
    Array<int>                              m_gateIndexToId;
    Array<int>                              m_gateIdxs;
    Hash<Set<int>, int>                     m_gateIdHash;
    IndexListCombiner<GlobalCell>           m_objectLists;
    IndexListCombiner<GlobalCell>           m_clusterLists;
    float                                   m_unitSize;
    float                                   m_lodDistance;
    float                                   m_lodScaling;
    float                                   m_objectGroupCost;
    Vector3                                 m_objectGroupWorldSize;
    bool                                    m_computeVisualizations;
    bool                                    m_matchingData;
    bool                                    m_strictViewVolumes;
    bool                                    m_depthMaps;
    bool                                    m_depthMapsInf;
    float                                   m_hierarchyDetail;
    float                                   m_clusterSize;
    float                                   m_minAccurateDistance;
    int                                     m_curPortalOffset;
    int                                     m_totalClusters;
    int                                     m_totalCells;
    float                                   m_minFeatureSize;
    float                                   m_maxSmallestHole;
    Array<TileTreeNode*>                    m_tileTreeNodes;
    AABB                                    m_viewVolume;
    CellGraph                               m_clusterGraph;
    Progress                                m_progress;
    float*                                  m_progressPtr;
    String                                  m_computationString;

    struct BuildTileJob
    {
        enum State
        {
            NOT_READY,   // Neither left or right finished
            NOT_READY_2, // Left or right finished, not both
            READY,       // Left and right finished
            IN_PROGRESS,
            DONE
        };

        BuildTileJob(Allocator* a) : state(NOT_READY), node(0), out(0), right(a), parent(0) {}

        void childDone()
        {
            UMBRA_ASSERT(state == NOT_READY || state == NOT_READY_2);
            if (state == NOT_READY)
                state = NOT_READY_2;
            else
                state = READY;
        }

        bool isReady() const
        {
            return state == READY;
        }

        State state;
        TileTreeNode* node;
        HierarchyStackData* out;
        HierarchyStackData right;
        BuildTileJob* parent;
    };

    struct DepthmapJob
    {
        UINT32          objId;
        AABB            aabb;
        struct 
        {
            DepthmapData::DepthmapFace    face;
            DepthmapData::DepthmapPalette palette;
            int                           paletteSize;
        } faces[6];
    };

    struct BuildTileThread : public Runnable
    {
        virtual unsigned long run(void* p)
        {
            ((TomeWriter*)p)->buildTileJobsThread();
            return 0;
        }
    };

    struct DepthmapThread : public Runnable
    {
        virtual unsigned long run(void* p)
        {
            ((TomeWriter*)p)->buildDepthmapThread();
            return 0;
        }
    };

    Mutex                                   m_jobLock;
    Array<BuildTileJob*>                    m_buildTileJobs;
    int                                     m_numTotalJobs;
    const Hash<TileTreeNode*, int>*         m_jobTmpMap;

    int                                     m_nextDepthmapJob;
    Array<DepthmapJob>                      m_depthmapJobs;
    const ImpTome*                          m_depthmapTome;
    class ComputationCellgraph*             m_depthmapCellgraph;
    Allocator*                              m_depthmapAllocator;
    AABB                                    m_depthmapExitBounds;

    GraphicsContainer                       m_graphics;

    friend struct BuildTileThread;
};

class ImpTomeGenerator : public BuilderBase
{
public:
                             ImpTomeGenerator       (BuildContext* ctx, const ComputationParams& params, const AABB& aabb);
                            ~ImpTomeGenerator       (void);

    void                    setNumThreads           (int n) { m_numThreads = n; }
    void                    setCachePath            (const char* path) { m_cachePath = path; }

    Builder::Error          addTile                 (const ImpTileResult* tile);

    Builder::Error          getTomeSize             (UINT32& size);
    const Tome*             getTome                 (UINT8* buf, UINT32 bufSize);
    float                   getProgress             (void);

    void                    visualize               (class DebugRenderer*);

private:

    struct SerializedTile
    {
        SerializedTile() : m_serialized(NULL) {}
        SerializedTile(const AABB& aabb, MemOutputStream* serialized)
            : m_aabb(aabb), m_serialized(serialized) {}

        AABB                        m_aabb;
        MemOutputStream*            m_serialized;
    };

    void updateTome      ();

    CriticalSection             m_visLock;
    ComputationParams           m_params;
    int                         m_numThreads;
    String                      m_cachePath;
    AABB                        m_aabb;
    Hash<AABBi, SerializedTile> m_serializedInput;
    bool                        m_rebuild;
    Tome*                       m_tome;
    float                       m_progress;
};

/* \todo [antti 16.11.2012]: move somewhere */
class RectCombiner
{
public:
    RectCombiner(Allocator* a = NULL): m_rects(a) {}
    void setAllocator (Allocator* a) { m_rects.setAllocator(a); }

    int numRects (void) const { return m_rects.getSize(); }
    Vector4 getRect (int i) const { return m_rects[i]; }

    void addRect (const Vector4& rect)
    {
        Vector4 cur(rect);
        bool combine = true;
        while (combine)
        {
            combine = false;
            for (int r = 0; r < m_rects.getSize(); r++)
            {
                if (rectTouches(m_rects[r], cur))
                {
                    cur = rectUnion(m_rects[r], cur);
                    if (r != (m_rects.getSize() - 1))
                        m_rects[r] = m_rects[m_rects.getSize() - 1];
                    m_rects.popBack();
                    combine = true;
                    break;
                }
            }
        }
        m_rects.pushBack(cur);
    }

private:

    bool rectTouches (const Vector4& a, const Vector4& b)
    {
        int touch = 0;
        int tarea = 0;
        if (a.x <= b.z && a.z >= b.x)
            touch++;
        if (a.x < b.z && a.z > b.x)
            tarea++;
        if (a.y <= b.w && a.w >= b.y)
            touch++;
        if (a.y < b.w && a.w > b.y)
            tarea++;
        return (touch == 2 && tarea >= 1);
    }

    Array<Vector4> m_rects;
};

static inline void copyHeap (RectCombiner* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

} // namespace Umbra
