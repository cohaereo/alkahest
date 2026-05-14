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
 * \brief   Umbra runtime portal culling
 *
 */


#include "umbraPortalCull.hpp"

#include "umbraIntersect.hpp"
#include "umbraStaticHeap.hpp"
#include "umbraAABB.hpp"
#include "runtime/umbraQuery.hpp" // for error codes
#include "umbraCubemap.hpp"

#define UMBRA_SUPPORT_OUTSIDE_SCENE 0

#if 0 // enable for traces
#if UMBRA_ARCH == UMBRA_SPU
#include <spu_printf.h>
#define UMBRA_TRACE(x) spu_printf x
#else
#include <stdio.h>
#define UMBRA_TRACE(x) printf x
#endif
#else
#define UMBRA_TRACE(x)
#endif

#if UMBRA_ARCH == UMBRA_SPU
#define UMBRA_DISABLE_VISUALIZATIONS 1
#define UMBRA_CULLER_STATS 0
#else
#define UMBRA_DISABLE_VISUALIZATIONS 0
#define UMBRA_CULLER_STATS 1
#endif

#if UMBRA_CULLER_STATS
#   define UMBRA_QUERYSTAT_ADD(stat, N) \
    (m_query->setQueryStatInt(QUERYSTAT_ ## stat, m_query->getQueryStatInt(QUERYSTAT_ ## stat) + (N)))
#else
#   define UMBRA_QUERYSTAT_ADD(stat, N)
#endif
#define UMBRA_QUERYSTAT_INC(stat) UMBRA_QUERYSTAT_ADD(stat, 1)

using namespace Umbra;

namespace Umbra
{

class StartCellFinder
{
public:
    StartCellFinder(PortalCuller& culler, const IntersectFilter& filter, int objectIdx)
        : culler(culler), filter(filter), objectIdx(objectIdx) {}

    bool processStartTile(int tileIndex, TileTraverseTree::TileHandle handle)
    {
        PortalCuller::Tile tile;
        tile.tileIndex = tileIndex;
        tile.handle = handle;

        MappedTile accessor;
        culler.m_query->mapTile(accessor, tileIndex);
        const ImpTile* tileData = accessor.getTile();
        bool ret = false;
        if (!tileData || !tileData->getCellNodes() || !tileData->getTreeData())
        {
            culler.m_query->setError((UINT32)Query::ERROR_UNSUPPORTED_OPERATION);
        }
        else
        {
            culler.resetLocalCells();

            if (objectIdx != -1)
            {
#if UMBRA_ARCH == UMBRA_SPU
                culler.m_query->setError(Query::ERROR_UNSUPPORTED_OPERATION);
#else
                ret = findStartCellsForObject(tile, accessor);
#endif
            }
            else
            {
                ret = findStartCells(tile, tileData);
            }
        }

        culler.m_query->unmapTile(accessor);
        return ret;
    }

private:

    bool findStartCells(PortalCuller::Tile tile, const ImpTile* tileData)
    {
        const UINT32* viewTreeData = (const UINT32*)mapArray(culler.m_query->getAllocator(), tileData->getTreeData(), 0, -1);
        if (!viewTreeData)
        {
            culler.m_query->setError((UINT32)Query::ERROR_OUT_OF_MEMORY);
            return false;
        }

        KDTraverseStack<> stack;
        stack.init(KDTree(tileData->getTreeNodeCount(), viewTreeData, tileData->getTreeSplits()), tileData->getAABB());

        bool found = false;
        while (!stack.isEmpty())
        {
            if (!filter.filter(stack.node().getAABB()))
            {
                stack.traverse(ENTER_NONE);
                continue;
            }
            if (stack.node().isLeaf())
            {
                int nodeIdx = stack.tree().getLeafIdx(stack.node().getIndex());
                UMBRA_ASSERT(nodeIdx >= 0);
                int cellIdx = tileData->getCellIndex(nodeIdx, culler.m_transformer->getCameraPos());
                if (cellIdx >= 0)
                {
                    culler.addStartCell(tile, cellIdx);
                    if (!UMBRA_DISABLE_VISUALIZATIONS && culler.m_query->debugEnabled(Query::DEBUGFLAG_VIEWCELL))
                        culler.m_query->visualizeCell(tileData, cellIdx);
                    found = true;
                }
            }
            stack.traverse(ENTER_BOTH);
        }

        unmapArray(culler.m_query->getAllocator(), viewTreeData);
        return found;
    }

    bool findStartCellsForObject(PortalCuller::Tile tile, const MappedTile& tileData)
    {
        UMBRA_ASSERT(filter.getType() == IntersectFilter::FILTER_AABB);

        culler.m_cellNodeMap.setArray(tileData.getTile()->getCellNodes());
        bool found = false;
        for (int i = 0; i < tileData.getTile()->getNumCells(); i++)
        {
            CellNode cellNode;
            culler.m_cellNodeMap.get(cellNode, i);
            Vector3 cellMn, cellMx;
            cellNode.getBounds().unpack(tileData.getTile()->getAABB(), cellMn, cellMx);
            if (!filter.filter(AABB(cellMn, cellMx)))
                continue;

            int num = culler.m_objectIter.setArray(
                tileData.getMappedTome().getTome()->getObjectLists(),
                tileData.getMappedTome().getTome()->getObjectListElemWidth(),
                tileData.getMappedTome().getTome()->getObjectListCountWidth(),
                cellNode.getObjectIndex(), cellNode.getObjectCount());

            for (int k = 0; k < num; k++)
            {
                int localIdx  = culler.m_objectIter.next();
                int globalIdx = tileData.getMappedTome().mapLocalObject(localIdx);

                if (globalIdx == objectIdx)
                {
                    culler.addStartCell(tile, i);
                    if (!UMBRA_DISABLE_VISUALIZATIONS && culler.m_query->debugEnabled(Query::DEBUGFLAG_VIEWCELL))
                        culler.m_query->visualizeCell(tileData.getTile(), i);
                    found = true;
                    break;
                }
            }
        }

        return found;
    }

    StartCellFinder& operator= (const StartCellFinder& f);

    PortalCuller& culler;
    const IntersectFilter& filter;
    int objectIdx;
};

}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

PortalCuller::PortalCuller (QueryContext* q, Transformer* camera, float accurateDistance, const ImpObjectDistanceParams* objDist, int maxCells, int maxTreeNodes)
:   m_query(q),
    m_transformer(camera),
    m_bufferAllocator(NULL),
    m_accurateDistance(accurateDistance),
    m_depthBuffer(q),
    m_inputDepth(q),
    m_tiles(q, maxTreeNodes),
    m_maxCells(maxCells),
    m_cellNodeMap(q, sizeof(CellNode)),
    m_extCellNodeMap(q, sizeof(ExtCellNode)),
    m_portalIter(q->getAllocator(), q->getTagManager()),
    m_objectIter(q->getAllocator(), q->getTagManager()),
    m_depthmapOffset(0.f)
{
    m_cells = (CellData*)UMBRA_HEAP_NEW_ARRAY(q->getAllocator(), CellData, m_maxCells);

    float dirMax = 0.f;
    for (int i = 0; i < 3; i++)
    {
        float v = m_transformer->getNearPlane()[i];
        m_nearSigns[i] = (v >= 0.f) ? 1 : 0;
        float l = fabsf(v);
        if (l > dirMax)
        {
            m_orthoPortalFace = buildFace(i, m_nearSigns[i]);
            dirMax = l;
        }
    }

    for (int i = NUM_OBJ_BANKS - 1; i >= 0; i--)
    {
        m_objBanks[i].bounds.init(q, q->getTome()->getObjectBounds());
        m_objBanks[i].distances.init(q, q->getTome()->getObjectDistances());
        m_objBanks[i].size = 0;
    }

    float objDistScale = ImpObjectDistanceParams::getEffectiveScale(objDist);
    m_lodDistanceScaleSqr = SIMDLoad(objDistScale * objDistScale); 
    m_accurateDistance = max2(m_accurateDistance, q->getTome()->getLodBaseDistance() / objDistScale);
    m_minContribution = ImpObjectDistanceParams::getEffectiveMinContribution(objDist);

    Vector3 lodRef = ImpObjectDistanceParams::getEffectiveReference(objDist, m_transformer->getCameraPos());
    // force disable hierarchy traverse when distance culling from a custom reference point
    if (!!q->getTome()->getObjectDistances() && (lodRef != m_transformer->getCameraPos()))
        m_accurateDistance = -1.f;
    m_lodRef = SIMDLoadW1(lodRef);

    // compute axis-aligned plane normals

    Vector4 axisXBase = m_transformer->getWorldToClipTranspose()[0];
    Vector4 axisYBase = m_transformer->getWorldToClipTranspose()[1];
    Vector4 axisZBase = m_transformer->getWorldToClipTranspose()[2];
    Vector3 axisXBase3(axisXBase.x, axisXBase.y, axisXBase.w);
    Vector3 axisYBase3(axisYBase.x, axisYBase.y, axisYBase.w);
    Vector3 axisZBase3(axisZBase.x, axisZBase.y, axisZBase.w);
    Vector3 normals[3];
    normals[0] = cross(axisYBase3, axisZBase3);
    normals[1] = cross(axisZBase3, axisXBase3);
    normals[2] = cross(axisXBase3, axisYBase3);

    for (int i = 0; i < 3; i++)
    {
        int axisX = (1 << i) & 3;
        int axisY = (1 << axisX) & 3;

        const Vector3& normal1 = normals[axisX];
        const Vector3& normal2 = normals[axisY];

        int face0 = i*2;
        int face1 = i*2+1;

        if (!m_transformer->getFlipPortalWinding())
            swap2(face0, face1);

        m_axisNormals[face0].x = Vector4(normal1.x, -normal2.x, -normal1.x, normal2.x);
        m_axisNormals[face0].y = Vector4(normal1.y, -normal2.y, -normal1.y, normal2.y);
        m_axisNormals[face0].z = Vector4(normal1.z, -normal2.z, -normal1.z, normal2.z);
        m_axisNormals[face1].x = Vector4(-normal2.x, normal1.x, normal2.x, -normal1.x);
        m_axisNormals[face1].y = Vector4(-normal2.y, normal1.y, normal2.y, -normal1.y);
        m_axisNormals[face1].z = Vector4(-normal2.z, normal1.z, normal2.z, -normal1.z);
    }

    // create a fully visible raster buffer for start cells

    m_fullyVisible = BlockRasterBuffer(
        BlockRasterBuffer::boundsToBlockRect(m_transformer->getScissor()),
        (UINT32*)UMBRA_HEAP_ALLOC(q->getAllocator(), UMBRA_BITVECTOR_SIZE(UMBRA_PORTAL_RASTER_SIZE * UMBRA_PORTAL_RASTER_SIZE)));
    if (m_fullyVisible.getBufferPtr())
        RasterOps::fillOnes(m_fullyVisible);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

PortalCuller::~PortalCuller (void)
{
    UMBRA_HEAP_DELETE(m_query->getAllocator(), m_bufferAllocator);
    UMBRA_HEAP_FREE(m_query->getAllocator(), m_fullyVisible.getBufferPtr());
    for (int i = 0; i < NUM_OBJ_BANKS; i++)
    {
        m_objBanks[i].distances.deinit(m_query);
        m_objBanks[i].bounds.deinit(m_query);
    }
    UMBRA_HEAP_DELETE_ARRAY(m_query->getAllocator(), m_cells);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline void PortalCuller::enterTile (const Tile& bucket)
{
    UMBRA_ASSERT(!m_mappedTile);
    UMBRA_QUERYSTAT_INC(TILES_VISITED);

    m_query->mapTile(m_mappedTile, bucket.tileIndex);
    const ImpTile* tile = m_mappedTile.getTile();

    m_testDepthmaps = m_depthmapsEnabled && m_mappedTile.getMappedTome().getTome()->hasObjectDepthmaps();

    m_cellNodeMap.setArray(tile->getCellNodes());
    m_extCellNodeMap.setArray(m_mappedTile.getExtCellNodes());

    SIMDRegister tileMin = SIMDLoadAlignedW1((Vector4&)tile->getTreeMin());
    SIMDRegister tileMax = SIMDLoadW1((Vector4&)tile->getTreeMax());

    // tile coordinate conversion factors
    m_tileOffset = tileMin;
    m_tileScale = SIMDMultiply(SIMDSub(tileMax, tileMin), SIMDLoad(1 / 65535.f));
    SIMDRegister portalExpand = SIMDAdd(SIMDLoadW0(tile->getPortalExpand()), m_transformer->getPrediction());

    // compute camera position and prediction amount in tile coordinate system
    SIMDRegister cameraPos = SIMDLoadW0(m_transformer->getCameraPos());
    SIMDRegister invScale = SIMDReciprocalAccurate(m_tileScale);
    SIMDRegister32 localPos = SIMDFloatToInt(SIMDMultiply(SIMDSub(cameraPos, m_tileOffset), invScale));
    SIMDStoreAligned32(localPos, &m_cameraPosLocal.i);
    SIMDRegister32 localExp = SIMDFloatToInt(SIMDMultiplyAdd(portalExpand, invScale, SIMDOne()));
    SIMDStoreAligned32(localExp, &m_portalExpandLocal.i);
    SIMDStoreAligned(portalExpand, m_portalExpand);

    // Active cull plane set
    m_transformer->computeActivePlaneSet(m_slotPlaneSet, SIMDSub(tileMin, portalExpand), SIMDAdd(tileMax, portalExpand));

    CellList incoming = getTileCellQueue(bucket);
    CellListLocal queue = getLocalCellQueue();
    UMBRA_ASSERT(queue.isEmpty());

    m_numCellsQueued = 0;
    for (;;)
    {
        Cell cell = incoming.removeFirst();
        if (cell == CellData::EMPTY)
            break;
        queue.insertLast(cell);
        setLocalCellState(cell, CellState_Queued);
        m_numCellsQueued++;
    }

    m_freedCellCounter = tile->getNumCells() * 2;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE void PortalCuller::leaveTile (void)
{
    m_query->unmapTile(m_mappedTile);
    UMBRA_DEBUG_CODE(m_mappedTile = MappedTile());
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE bool PortalCuller::isBackfacing (const Portal& portal) const
{
    UINT32 a = getFaceAxis(portal.getFace());

    if (!m_transformer->isOrtho())
    {
        int d = m_cameraPosLocal[a] - (portal.idx_z & 0xFFFF);
        // flip sign bit conditionally, without multiply or branch
        UINT32 signMask = (UINT32)(getFaceDirection(portal.getFace()) - 1);
        d = (d ^ signMask) - signMask;
        return d > m_portalExpandLocal[a];
    }
    else
    {
        return ((UINT32)getFaceDirection(portal.getFace()) != m_nearSigns[a]);
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE bool PortalCuller::enterPortal (const Portal& portal) const
{
    if (!portal.isUser())
    {
        return !isBackfacing(portal);
    }
    return m_query->isGateOpen(m_mappedTile, portal);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void PortalCuller::visualizePortal (const Portal& portal, bool tileExternal, bool tomeExternal) const
{
    Vector4 clr;
    if (tomeExternal)
        clr = Vector4(0.8f, 0.0f, 0.0f, 0.5f);
    else if (tileExternal)
        clr = Vector4(0.6f, 0.8f, 0.6f, 0.5f);
    else
        clr = Vector4(0.6f, 0.6f, 0.8f, 0.5f);

    Vector3 portalMin, portalMax;

    if (portal.isUser())
    {
        clr.x = 1.f;
        m_mappedTile.getMappedTome().getTome()->getGateBounds(portal, m_portalExpand.xyz(), portalMin, portalMax);
    }
    else
    {
        SIMDRegister mn, mx;
        portal.getQuad(mn, mx, SIMDLoadAligned(m_portalExpand), m_tileScale, m_tileOffset);
        SIMDStore(mn, portalMin);
        SIMDStore(mx, portalMax);
    }

    m_query->addQueryDebugAABB(portalMin, portalMax, clr);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void PortalCuller::finalizeCell (PortalCuller::Cell cell)
{
    const CellData& cellData = getCellData(cell);
    CellNode cellNode;
    m_cellNodeMap.get(cellNode, cellData.getId());

    const ImpTome* tome = m_mappedTile.getMappedTome().getTome();
    DepthmapReader reader(tome);

    if (m_result->hasObjectVisibility())
    {
        int objsToFetch = m_objectIter.setArray(
            tome->getObjectLists(),
            tome->getObjectListElemWidth(),
            tome->getObjectListCountWidth(),
            cellNode.getObjectIndex(), cellNode.getObjectCount());
        int objsToProcess = objsToFetch;
        int curIdx = 0;
        ObjectBank* cur = &m_objBanks[0];
        for (int i = 1; i < NUM_OBJ_BANKS; i++)
        {
            m_objBanks[i].size = 0;
            m_objBanks[i].origSize = 0;
        }
        UMBRA_ASSERT(objsToFetch >= 0);
        bool hasDistances = cur->distances.hasArray();

        Vector4i UMBRA_ATTRIBUTE_ALIGNED16(cellBounds) = cellData.getBuf().getBounds();
        SIMDRegister32 cellBoundsSIMD = SIMDLoadAligned32(&cellBounds.i);

        bool hasMultipleTomes = m_mappedTile.getExtTile() != NULL;
        bool isOrtho          = m_transformer->isOrtho();
        Vector3 invCameraDir  = -m_transformer->getNearPlane().xyz();

        while (objsToProcess)
        {
            // Read a batch of input objects
            int batch = min2(objsToFetch, OBJ_BOUNDS_BATCH);
            objsToFetch -= batch;

            int objsToTest = 0;
            for (int i = 0; i < batch; i++)
            {
                int localIdx  = m_objectIter.next();
                int globalIdx = m_mappedTile.getMappedTome().mapLocalObject(localIdx);

                if (!testBit(m_result->m_processedObjectVector, globalIdx))
                {
                    //if (globalIdx == debugObject)
                    //    visualizeDepthmap(localIdx);

                    cur->localIndices[objsToTest] = localIdx;
                    cur->indices[objsToTest++]    = globalIdx;
                }
            }

            if (objsToTest)
            {
                if (hasDistances)
                    cur->distances.fetch(cur->indices, objsToTest);
                cur->bounds.fetch(cur->indices, objsToTest);
            }

            if (NUM_OBJ_BANKS > 1)
            {
                cur->size = objsToTest;
                cur->origSize = batch;
                curIdx = (curIdx + 1) % NUM_OBJ_BANKS;
                cur = &m_objBanks[curIdx];
                objsToTest = cur->size;
                batch = cur->origSize;
            }

            objsToProcess -= batch;
            if (!objsToTest)
                continue;
            if (hasDistances)
                cur->distances.process();
            cur->bounds.process();

            for (int i = 0; i < objsToTest; i++)
            {
                UMBRA_QUERYSTAT_INC(OBJECTS_TESTED);

                int objNdx   = cur->indices[i];
                int localIdx = cur->localIndices[i];

                // Distance range culling

                if (hasDistances && !distanceInRange(m_lodRef, *cur->distances.get(i), m_lodDistanceScaleSqr))
                {
                    setBit(m_result->m_processedObjectVector, objNdx);
                    UMBRA_QUERYSTAT_INC(OBJECTS_DISTANCECULLED);
                    continue;
                }

                // Frustum culling

                const ObjectBounds* bounds = cur->bounds.get(i);
#if UMBRA_OS == UMBRA_PS3
                SIMDRegister mn = SIMDLoadW1(bounds->mn);
                SIMDRegister mx = SIMDLoadW1(bounds->mx);
#else
                SIMDRegister mn = SIMDLoadW1((Vector4&)bounds->mn);
                SIMDRegister mx = SIMDLoadW1((Vector4&)bounds->mx);
#endif
                // TODO: get rid of this
                mn = SIMDSub(mn, m_transformer->getPrediction());
                mx = SIMDAdd(mx, m_transformer->getPrediction());

                if (!m_transformer->frustumTestBounds(&m_slotPlaneSet, mn, mx))
                {
                    setBit(m_result->m_processedObjectVector, objNdx);
                    UMBRA_QUERYSTAT_INC(OBJECTS_FRUSTUMCULLED);
                    continue;
                }

                if (m_testDepthmaps)
                {
                    if ((!isOrtho && !reader.testPosition(localIdx, m_transformer->getCameraPos(), m_depthmapOffset)) ||
                        ( isOrtho && !reader.testDirection(localIdx, invCameraDir, m_transformer->getNearPlane())))
                    {
                        if (!hasMultipleTomes)
                            setBit(m_result->m_processedObjectVector, objNdx);
                        UMBRA_QUERYSTAT_INC(OBJECTS_STATICALLY_CULLED);
                        continue;
                    }
                }

                // Transform and test against cell buffer

                float contribution;
                Vector4i UMBRA_ATTRIBUTE_ALIGNED16(mnmx);
                m_transformer->transformBox(mnmx, mn, mx, true, cellBoundsSIMD, contribution);

                if (contribution < m_minContribution)
                {
                    setBit(m_result->m_processedObjectVector, objNdx);
                    UMBRA_QUERYSTAT_INC(OBJECTS_CONTRIBUTIONCULLED);
                    continue;
                }

                bool isVisible = false;
                if ((mnmx.k > mnmx.i) && (mnmx.l > mnmx.j))
                {
                    if (m_result->m_inputDepthBuffer)
                    {
                        float minZ = m_transformer->getMinDeviceZ(mn, mx);
                        isVisible = RasterOps::testRectAny(cellData.getBuf(), mnmx, &m_inputDepth, minZ);
                    }
                    else
                    {
                        isVisible = RasterOps::testRectAny(cellData.getBuf(), mnmx);
                    }
                }

                if (isVisible)
                {
                    UMBRA_QUERYSTAT_INC(OBJECTS_VISIBLE);
                    setBit(m_result->m_processedObjectVector, objNdx);
                    if (m_result->m_objects)
                        m_result->m_objects->pushBack(objNdx);
                    if (m_result->m_visibleObjectVector)
                        setBit(m_result->m_visibleObjectVector, objNdx);
                    if (m_result->m_objectDistances.getBuf())
                    {
                        SIMDRegister distMn = SIMDLoad(hasDistances ? (float*)&cur->distances.get(i)->boundMin : (float*)&bounds->mn);
                        SIMDRegister distMx = SIMDLoad(hasDistances ? (float*)&cur->distances.get(i)->boundMax : (float*)&bounds->mx);
                        float d;
                        SIMDStore(distanceAABBPointSqrSIMD(m_lodRef, distMn, distMx), d);
                        m_result->m_objectDistances.pushBack(d);
                    }

                    if (m_result->m_objectContributions.getBuf())
                        m_result->m_objectContributions.pushBack(contribution);

#if !UMBRA_DISABLE_VISUALIZATIONS
                    if (m_query->debugEnabled(Query::DEBUGFLAG_OBJECT_BOUNDS))
                    {
                        m_query->addQueryDebugAABB(Vector3(bounds->mn[0], bounds->mn[1], bounds->mn[2]),
                            Vector3(bounds->mx[0], bounds->mx[1], bounds->mx[2]), Vector4(0.1f, 0.f, 1.2f, 1.f));
                    }
#endif
                }
            }
        }
    }

    // Update depth buffer if present.
    if (m_result->m_occlusionBuffer)
    {
        float farZ = getCellFarDeviceZ(cellNode);
        if (farZ > ImpOcclusionBuffer::getMaxDepth())
            farZ = ImpOcclusionBuffer::getMaxDepth();
        RasterOps::updateDepthBuffer(cellData.getBuf(), m_depthBuffer, farZ);
    }

    // Update visible clusters
    if (m_result->m_clusters)
    {
        if (cellNode.getClusterCount() == 0)
        {
            int idx = m_mappedTile.getMappedTome().mapLocalCluster(cellNode.getClusterIndex());
            if (!testAndSetBit(m_result->m_clusterVector, idx))
                m_result->m_clusters->pushBack(idx);
        }
        else
        {
            int clusters = m_objectIter.setArray(tome->getClusterLists(),
                tome->getClusterListElemWidth(),
                tome->getClusterListCountWidth(),
                cellNode.getClusterIndex(), cellNode.getClusterCount());
            while (clusters--)
            {
                int idx = m_mappedTile.getMappedTome().mapLocalCluster(m_objectIter.next());
                if (!testAndSetBit(m_result->m_clusterVector, idx))
                    m_result->m_clusters->pushBack(idx);
            }
        }
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline void PortalCuller::freeCellBuffer (PortalCuller::Cell cell)
{
    CellData& data = getCellData(cell);
    m_bufferAllocator->releaseBuffer(data.getBuf());
    data.getBuf().reset();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline PortalCuller::Cell PortalCuller::freeOneCell()
{
    CellList inactive = getCellInactiveQueue();
    Cell cell = inactive.removeFirst();
    if (cell != CellData::EMPTY)
    {
        m_freedCellCounter--;
        finalizeCell(cell);
        freeCellBuffer(cell);
        setLocalCellState(cell, CellState_Free);
    }
    return cell;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline PortalCuller::Cell PortalCuller::getFreeCell(int id)
{
    Cell ret = getCellFreeList().removeFirst();
    if (ret == CellData::EMPTY)
    {
        ret = freeOneCell();
        if (ret == CellData::EMPTY)
        {
            //m_query->setError(Query::ERROR_OUT_OF_MEMORY);
            return CellData::EMPTY;
        }
    }
    UMBRA_ASSERT(getCellData(ret).getBuf().isEmpty());
    getCellData(ret).setId(id);
    return ret;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline PortalCuller::Cell PortalCuller::findLocalCell(int id, CellState& state)
{
    state = getLocalCellState(id);
    if (state != CellState_Free)
        return getLocalCell(id);
    return getFreeCell(id);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline PortalCuller::Cell PortalCuller::findRemoteCell(
    Tile& tile, int tileIndex, int id, CellState& state)
{
    UMBRA_ASSERT(m_tiles.isTileTraversable(tileIndex));

    tile.handle = m_tiles.getTraversableTile(tileIndex);
    tile.tileIndex = tileIndex;

    CellList cellQueue = getTileCellQueue(tile);
    Cell cell = cellQueue.first();
    while (cell != CellData::EMPTY)
    {
        if (getCellData(cell).getId() == id)
            break;
        cell = cellQueue.next(cell);
    }

    if (cell != CellData::EMPTY)
    {
        state = CellState_Queued;
    }
    else
    {
        state = CellState_Free;
        cell = getFreeCell(id);
    }
    return cell;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE PortalCuller::Cell PortalCuller::getLocalCell (int idx) const
{
    UINT16 val = m_localCellMap[idx];
    val &= 0x3FFF;
    UMBRA_ASSERT(val);
    return val;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE PortalCuller::CellState PortalCuller::getLocalCellState (int idx) const
{
    UINT16 val = m_localCellMap[idx];
    return (CellState)(val >> 14);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE void PortalCuller::setLocalCellState (Cell cell, CellState state)
{
    int idx = getCellData(cell).getId();
    UMBRA_ASSERT(idx >= 0 && idx < UMBRA_MAX_CELLS_PER_TILE);
    m_localCellMap[idx] = (((UINT16)state) << 14) | cell;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE float PortalCuller::getCellFarDeviceZ (const CellNode& cell)
{
    const PackedAABB& aabb = cell.getBounds();

    SIMDRegister localMin = SIMDIntToFloat(SIMDLoad32(aabb.getMnx(), aabb.getMny(), aabb.getMnz(), 0));
    SIMDRegister localMax = SIMDIntToFloat(SIMDLoad32(aabb.getMxx(), aabb.getMxy(), aabb.getMxz(), 0));
    // TODO: get rid of having to do this here
    SIMDRegister ofsMn = SIMDSub(m_tileOffset, SIMDLoadAligned(m_portalExpand));
    SIMDRegister ofsMx = SIMDAdd(m_tileOffset, SIMDLoadAligned(m_portalExpand));
    SIMDRegister bmn = SIMDMultiplyAdd(localMin, m_tileScale, ofsMn);
    SIMDRegister bmx = SIMDMultiplyAdd(localMax, m_tileScale, ofsMx);

    return m_transformer->getMaxDeviceZ(bmn, bmx);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void PortalCuller::addStartCell(const Tile& tile, int cellIdx)
{
    if (getLocalCellState(cellIdx) != CellState_Free)
        return;
    Cell cell = getFreeCell(cellIdx);
    if (cell == CellData::EMPTY)
        return;
    getCellData(cell).getBuf() = m_fullyVisible;
    getTileCellQueue(tile).insert(cell);
    setLocalCellState(cell, CellState_Queued);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool PortalCuller::init (bool ignoreCameraPos, bool useDepthMaps, const AABB& initialAABB, int objectIdx)
{
    getCellInactiveQueue().clear();
    getLocalCellQueue().clear();
    getCellFreeList().populate(1, (Cell)m_maxCells);
    m_outsideCell = getFreeCell(0);
    m_depthmapsEnabled = useDepthMaps && !m_transformer->hasPrediction();

    // force "ignore camera position" for ortho views
    if (m_transformer->isOrtho())
        ignoreCameraPos = true;

    if (ignoreCameraPos)
    {
        const Vector4& nearPleq = m_transformer->getNearPlane();
        float nearDistance = -dot(nearPleq, m_transformer->getCameraPos()) / nearPleq.xyz().length();
        m_depthmapOffset = nearDistance;
    }
    
    // Setup filter utility for start cells/tiles

    Quad nearQuad;
    IntersectFilter startCellFilter;

    if (initialAABB.isOK())
    {
        startCellFilter.setAABB(&initialAABB);
    }
    else if (ignoreCameraPos)
    {
        nearQuad = m_transformer->getNearPlaneQuad();
        startCellFilter.setQuad(&nearQuad);
    }
    else
    {
        startCellFilter.setPoint(&m_transformer->getCameraPos());
    }

    // Currently require the query shape to be completely within tome bounds
    if (!startCellFilter.boundsCheck(m_query->getTome()->getAABB()))
    {
        m_query->setError((UINT32)Query::ERROR_OUTSIDE_SCENE);
        return false;
    }

    // Establish front-to-back ordering

    m_tiles.setTileCompareFunc(m_transformer->isOrtho() ?
        FrontToBackCompare::sortByDir(m_transformer->getNearPlane().xyz()) :
        FrontToBackCompare::sortByPoint(m_transformer->getCameraPos()));

    // Traverse init

    if (!m_tiles.init(*m_transformer, startCellFilter, StartCellFinder(*this, startCellFilter, objectIdx), m_accurateDistance) ||
        getCellFreeList().isEmpty())
    {
        // ran out of tile tree nodes or start cells ate up all cell handles
        m_query->setError(Query::ERROR_OUT_OF_MEMORY);
    }
    else if (!m_tiles.hasMore())
    {
        // no tiles queued, so no start cells found
        m_query->setError(Query::ERROR_OUTSIDE_SCENE);
    }

    if (m_query->hasError())
        return false;

    // Create buffer allocator (done after tile init to limit peak mem use)

    m_bufferAllocator = UMBRA_HEAP_NEW(m_query->getAllocator(), BufferAllocator);
    if (!m_bufferAllocator)
    {
        m_query->setError(Query::ERROR_OUT_OF_MEMORY);
        return false;
    }
    m_bufferAllocator->setPersistent(m_fullyVisible);

    resetLocalCells();
    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline PortalCuller::Cell PortalCuller::nextCellToProcess (CellListLocal& queue)
{
    Cell cell = queue.removeFirst();
    if (cell == CellData::EMPTY)
        return CellData::EMPTY;
    int cellsLeft = --m_numCellsQueued;
    int thisTile = m_mappedTile.getLocalSlot();

    while (cellsLeft)
    {
        bool found = false;
        CellNode cellNode;
        PortalIteratorT<false> iter(m_portalIter);

        m_cellNodeMap.get(cellNode, getCellData(cell).getId());
        iter.init(m_mappedTile, cellNode, NULL);

        while (iter.hasMore())
        {
            const Portal& portal = iter.next();

            if (!portal.hasTarget() || portal.getTargetTileIdx() != thisTile)
                break;

            if (isBackfacing(portal) || portal.isUser())
            {
                int targetCellId = portal.getTargetIndex();
                CellState targetState = getLocalCellState(targetCellId);
                if (targetState != CellState_Queued)
                    continue;

                // found queued predecessor
                queue.insertFirst(cell);
                setLocalCellState(cell, CellState_Backtrace);
                cell = getLocalCell(targetCellId);
                queue.remove(cell);
                UMBRA_QUERYSTAT_INC(CELL_SORT_FAILURES);
                cellsLeft--;
                found = true;
                break;
            }
        }
        if (!found)
            break;
    }

    // Purge backtrace
    Cell btCell = queue.first();
    while (btCell != CellData::EMPTY)
    {
        int id = getCellData(btCell).getId();
        if (getLocalCellState(id) == CellState_Queued)
            break;
        setLocalCellState(btCell, CellState_Queued);
        btCell = queue.next(btCell);
    }

    return cell;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void PortalCuller::traverse(void)
{
    CellListLocal cellQueue = getLocalCellQueue();
    CellList inactiveQueue = getCellInactiveQueue();
    CellList freeQueue = getCellFreeList();
    Tile tile;

    while ((tile.handle = m_tiles.next(tile.tileIndex)) != 0)
    {
        enterTile(tile);

        UMBRA_TRACE(("T%d\n", tile.tileIndex));

        for (;;)
        {
            // Detect infinite loop within a slot caused by OOM. If more cells
            // are freed than there are in the tile, infinite loop has likely
            // occured.

            if (m_freedCellCounter < 0)
            {
                leaveTile();
                m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                return;
            }

            Cell currentCell = nextCellToProcess(cellQueue);
            if (currentCell == CellData::EMPTY)
                break;

            UMBRA_TRACE(("T%d\tC%d\n", tile.tileIndex, getCellData(currentCell).getId()));

            PortalIterator iter(m_portalIter);

            {
                CellNode cellNode;
                ExtCellNode extCellNode;
                m_cellNodeMap.get(cellNode, getCellData(currentCell).getId());
                if (m_extCellNodeMap.getCount())
                    m_extCellNodeMap.get(extCellNode, getCellData(currentCell).getId());
                iter.init(m_mappedTile, cellNode, &extCellNode);
            }

            Vector4i UMBRA_ATTRIBUTE_ALIGNED16(cellBounds) = getCellData(currentCell).getBuf().getBounds();
            SIMDRegister32 cellBoundsSIMD = SIMDLoadAligned32(&cellBounds.i);

            while (iter.hasMore())
            {
                const Portal& portal = iter.next();

                // trivial portal rejection

                if (!enterPortal(portal))
                    continue;

                // is this a tile internal portal?

                bool tileInternal = !iter.isExternal() && !portal.isOutside() &&
                    (portal.getTargetTileIdx() == m_mappedTile.getLocalSlot());

                // tile external portal rejection checks

                int targetTileIdx = -1;
                if (!tileInternal)
                {
                    if (portal.isOutside())
                    {
                        if (!m_result->m_occlusionBuffer)
                            continue;
                        if (!((1 << portal.getFace()) & m_mappedTile.getExitPortalMask()))
                            continue;
                    }
                    else
                    {
                        if (!iter.isExternal())
                            targetTileIdx = m_mappedTile.getMappedTome().mapLocalTile(portal.getTargetTileIdx());
                        else
                            targetTileIdx = portal.getTargetTileIdx();
                        if (!m_tiles.isTileTraversable(targetTileIdx))
                            continue;
                    }
                }

                // Get portal AABB and choose plane for rasterization

                SIMDRegister mn, mx;
                int portalFace = portal.getFace();

                if (!portal.isUser())
                {
                    Vector4i UMBRA_ATTRIBUTE_ALIGNED16(localMin);
                    Vector4i UMBRA_ATTRIBUTE_ALIGNED16(localMax);
                    portal.getIntMinMax(localMin, localMax);

                    mn = SIMDMultiplyAdd(SIMDIntToFloat(SIMDLoadAligned32(&localMin.i)), m_tileScale, m_tileOffset);
                    mx = SIMDMultiplyAdd(SIMDIntToFloat(SIMDLoadAligned32(&localMax.i)), m_tileScale, m_tileOffset);
                    mn = SIMDSub(mn, SIMDLoadAligned(m_portalExpand));
                    mx = SIMDAdd(mx, SIMDLoadAligned(m_portalExpand));

                    // are we inside portal front plane?

                    if (!m_transformer->isOrtho())
                    {
                        int axis = getFaceAxis(portal.getFace());
                        int d = (portal.idx_z & 0xFFFF) - m_cameraPosLocal[axis];
                        UINT32 signMask = (UINT32)(getFaceDirection(portal.getFace()) - 1);
                        d = (d ^ signMask) - signMask;
                        if (d <= m_portalExpandLocal[axis])
                        {
                            UMBRA_ASSERT(d >= -m_portalExpandLocal[axis]);
                            int axisX = (1 << axis) & 3;
                            int axisY = (1 << axisX) & 3;
                            int best = 0;
                            portalFace = -1;
                            int d = (localMin[axisX] - m_portalExpandLocal[axisX]) - m_cameraPosLocal[axisX];
                            if (d > best)
                            {
                                best = d;
                                portalFace = buildFace(axisX, 1);
                            }
                            d = m_cameraPosLocal[axisX] - (localMax[axisX] + m_portalExpandLocal[axisX]);
                            if (d > best)
                            {
                                best = d;
                                portalFace = buildFace(axisX, 0);
                            }
                            d = (localMin[axisY] - m_portalExpandLocal[axisY]) - m_cameraPosLocal[axisY];
                            if (d > best)
                            {
                                best = d;
                                portalFace = buildFace(axisY, 1);
                            }
                            d = m_cameraPosLocal[axisY] - (localMax[axisY] + m_portalExpandLocal[axisY]);
                            if (d > best)
                            {
                                best = d;
                                portalFace = buildFace(axisY, 0);
                            }
                        }
                    }
                }
                else
                {
                    Vector3 portalMin, portalMax;
                    m_mappedTile.getMappedTome().getTome()->getGateBounds(portal, m_portalExpand.xyz(), portalMin, portalMax);
                    mn = SIMDLoadW1(portalMin);
                    mx = SIMDLoadW1(portalMax);

                    if (m_transformer->isOrtho())
                    {
                        portalFace = m_orthoPortalFace;
                    }
                    else
                    {
                        portalFace = -1;
                        float bestDist = 0.f;
                        for (int i = 0; i < 3; i++)
                        {
                            float d = portalMin[i] - m_transformer->getCameraPos()[i];
                            if (d > bestDist)
                            {
                                bestDist = d;
                                portalFace = buildFace(i, 1);
                            }
                            else
                            {
                                d = m_transformer->getCameraPos()[i] - portalMax[i];
                                if (d > bestDist)
                                {
                                    bestDist = d;
                                    portalFace = buildFace(i, 0);
                                }
                            }
                        }
                    }
                }

                // portal transform

                Vector4i UMBRA_ATTRIBUTE_ALIGNED16(mnmx);
                VQuad quad;

                if (portalFace != -1)
                {
                    m_transformer->transformPortal(mnmx, quad, mn, mx, portalFace, m_slotPlaneSet.isNearPlaneActive(), cellBoundsSIMD);
                    // exit if empty rectangle
                    if (mnmx.i >= mnmx.k || mnmx.j >= mnmx.l)
                        continue;
                }
                else
                {
                    mnmx = cellBounds;
                }

                UMBRA_ASSERT(rectangleContains(getCellData(currentCell).getBuf().getBounds(), mnmx));

                //-----------------------------------------------------
                // Find target
                //-----------------------------------------------------

                Tile targetTile;
                Cell targetCell = CellData::EMPTY;
                CellState targetState = CellState_Free;
                if (tileInternal)
                {
                    targetCell = findLocalCell(portal.getTargetIndex(), targetState);
                }
                else
                {
                    if (portal.isOutside())
                    {
                        targetCell = m_outsideCell;
                        targetState = CellState_Queued;
                    }
                    else
                    {
                        targetCell = findRemoteCell(targetTile, targetTileIdx, portal.getTargetIndex(), targetState);
                    }
                }
                if (targetCell == CellData::EMPTY)
                    continue;

                //-----------------------------------------------------
                // Grow target buffer to fit current portal
                //-----------------------------------------------------

                Vector4i blockRect = BlockRasterBuffer::boundsToBlockRect(mnmx);
                if (m_bufferAllocator->expandBuffer(getCellData(targetCell).getBuf(), blockRect, tileInternal))
                {
                    UMBRA_QUERYSTAT_INC(PORTALS_PROCESSED);
                    if (!tileInternal)
                        UMBRA_QUERYSTAT_INC(EXT_PORTALS_PROCESSED);

                    bool isVisible;

                    if (portalFace != -1)
                    {
                        isVisible = RasterOps::rasterizePortal(getCellData(targetCell).getBuf(), mnmx, quad, m_axisNormals[portalFace], getCellData(currentCell).getBuf());
                    }
                    else
                    {
                        isVisible = RasterOps::blitOr(getCellData(targetCell).getBuf(), getCellData(currentCell).getBuf());
                    }

                    if (!isVisible)
                    {
                        if (targetState == CellState_Free)
                        {
                            freeCellBuffer(targetCell);
                            freeQueue.insert(targetCell);
                        }
                        continue;
                    }


#if !UMBRA_DISABLE_VISUALIZATIONS
                    if (m_query->debugEnabled(Query::DEBUGFLAG_PORTALS))
                        visualizePortal(portal, !tileInternal, iter.isExternal());
#endif
                }

                // drop into appropriate queue
                if (targetState != CellState_Queued)
                {
                    if (targetState == CellState_Inactive)
                    {
                        UMBRA_ASSERT(tileInternal);
                        UMBRA_QUERYSTAT_INC(CELL_REVISITS);
                        inactiveQueue.remove(targetCell);
                    }

                    if (tileInternal)
                    {
                        setLocalCellState(targetCell, CellState_Queued);
                        m_numCellsQueued++;
                        cellQueue.insertLast(targetCell);
                    }
                    else
                    {
                        getTileCellQueue(targetTile).insert(targetCell);
                        m_tiles.queueTile(targetTile.handle);
                    }
                }
            }

            // inactivate
            inactiveQueue.insert(currentCell);
            setLocalCellState(currentCell, CellState_Inactive);

            // add stats etc for this cell
            UMBRA_QUERYSTAT_INC(CELLS_PROCESSED);
        }

        // finalize inactive
        // would it be better to try finalizing in front to back order?

        while (!inactiveQueue.isEmpty())
        {
            Cell cell = inactiveQueue.removeFirst();
            finalizeCell(cell);
            freeCellBuffer(cell);
            freeQueue.insert(cell);
            setLocalCellState(cell, CellState_Free);
        }

        // move to next slot

        leaveTile();
        m_bufferAllocator->freeTransients();
    }

    // Add outside area to depth buffer
    if (m_result->m_occlusionBuffer && !getCellData(m_outsideCell).getBuf().isEmpty())
    {
        RasterOps::updateDepthBuffer(getCellData(m_outsideCell).getBuf(),
            m_depthBuffer, ImpOcclusionBuffer::getMaxDepth());

        // Combine with input depth
        if (m_result->m_inputDepthBuffer)
        {
            Vector4i blockRect = m_fullyVisible.getBlockRect();
            // translate to raster blocks
            blockRect.i *= 2;
            blockRect.k *= 2;

            DepthBuffer::BlockIterator<2, true, true> iter1 =
                m_depthBuffer.iterateBlocks<2, true, true>(blockRect);
            DepthBuffer::BlockIterator<2, true, false> iter2 =
                m_inputDepth.iterateBlocks<2, true, false>(blockRect);

            while (!iter1.end())
            {
                iter1.blocks().combineMin(iter2.blocks());
                iter1.next();
                iter2.next();
            }
        }
    }

    freeCellBuffer(m_outsideCell);
    getCellFreeList().insert(m_outsideCell);

    UMBRA_ASSERT(getCellFreeList().count() == (m_maxCells - 1));
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Query::ErrorCode PortalCuller::execute (VisibilityResult& result, bool useDepthMaps, bool ignoreCameraPos, const AABB& initialAABB, int objectIdx)
{
    m_result = &result;
    if (m_result->m_occlusionBuffer)
    {
        m_depthBuffer.setBuffer(m_result->m_occlusionBuffer->getDepthBufferPtr(false));
        m_depthBuffer.clear();
    }
    m_inputDepth.setBuffer(m_result->m_inputDepthBuffer);

    if (!init(ignoreCameraPos, useDepthMaps, initialAABB, objectIdx))
    {
        return (Query::ErrorCode)m_query->getError();
    }

    traverse();

    return (Query::ErrorCode)m_query->getError();
}
