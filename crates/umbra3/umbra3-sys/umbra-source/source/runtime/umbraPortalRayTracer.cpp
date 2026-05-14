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

#include "umbraPortalRayTracer.hpp"
#include "umbraAABB.hpp"
#include "umbraIntersect.hpp"
#include "runtime/umbraQuery.hpp"
#include "umbraPortalRaster.hpp"
#include "umbraDepthBuffer.hpp"

using namespace Umbra;

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

PortalRayTracer::PortalRayTracer(QueryContext* q, const Vector3& pt, const ImpObjectDistanceParams* objDist, Transformer* camera)
    :   m_query(q), m_start(pt), m_transformer(camera),
        m_objBounds(q, q->getTome()->getObjectBounds()), m_objDist(q, q->getTome()->getObjectDistances())
{
    m_visitedCells = (UINT32*)m_query->allocWorkMem(UMBRA_BITVECTOR_SIZE(q->getTome()->getNumCells()), false);
    m_distanceScaleSqr = SIMDLoad(ImpObjectDistanceParams::getEffectiveScaleSqr(objDist));
    m_minContribution = ImpObjectDistanceParams::getEffectiveMinContribution(objDist);

    m_farClipZ = 1.f;

    if (m_transformer)
    {
        m_clipToWorld = m_transformer->getWorldToClip();
        m_clipToWorld.invert();

        if (!m_transformer->hasFarPlane())
        {
            AABB bounds = q->getTome()->getAABB();

            m_farClipZ = 0.f;
            for (int i = 0; i < 8; i++)
            {
                Vector3 v = bounds.getCorner((AABB::Corner)i);
                Vector4 v2 = m_transformer->getWorldToClip().transform(Vector4(v, 1.f));
                if (v2.z >= 0.f)
                    m_farClipZ = max2(m_farClipZ, v2.z / v2.w);
            }

            if (m_farClipZ == 0.f)
                m_farClipZ = 0.99f;
        }

        Vector4 p1(0.f, 0.f, m_farClipZ, 1.f);
        Vector4 p2(1.f, 1.f, m_farClipZ, 1.f);
        Vector4 p3(-1.f, 1.f, m_farClipZ, 1.f);

        Vector4 p1t = m_clipToWorld.transform(p1);
        p1t.x /= p1t.w;
        p1t.y /= p1t.w;
        p1t.z /= p1t.w;

        Vector4 p2t = m_clipToWorld.transform(p2);
        p2t.x /= p2t.w;
        p2t.y /= p2t.w;
        p2t.z /= p2t.w;

        Vector4 p3t = m_clipToWorld.transform(p3);
        p3t.x /= p3t.w;
        p3t.y /= p3t.w;
        p3t.z /= p3t.w;

        p2t = p2t - p1t;
        p3t = p3t - p1t;

        m_scaleVector0 = Vector3(p2t.x, p2t.y, p2t.z) * (1.0f / UMBRA_PORTAL_REFERENCE_RASTER_SIZE);
        m_scaleVector1 = Vector3(p3t.x, p3t.y, p3t.z) * (1.0f / UMBRA_PORTAL_REFERENCE_RASTER_SIZE);

        for (int a = 0; a < 3; a++)
            scaleVector[a] = max2(fabsf(m_scaleVector0[a]), fabsf(m_scaleVector1[a]));
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

PortalRayTracer::~PortalRayTracer (void)
{
    m_query->freeWorkMem(m_visitedCells);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Query::ErrorCode PortalRayTracer::init (PortalNode& startNode)
{
    Cell startCell = m_query->findCell(m_start);
    if (!startCell.valid())
        return Query::ERROR_OUTSIDE_SCENE;

    MappedTile mappedTile;
    m_query->mapTile(mappedTile, startCell.slotIdx);
    const ImpTile* tile = mappedTile.getTile();
    if (!tile->getCellNodes())
    {
        m_query->unmapTile(mappedTile);
        return Query::ERROR_INVALID_ARGUMENT; // no portal data available \todo better error message
    }
    
    m_portalExpand = tile->getPortalExpand();
    
    startNode.slot = startCell.slotIdx;
    startNode.local = startCell.index;
    startNode.global = startCell.index + m_query->getTome()->getCellStart(startNode.slot);
    m_query->unmapTile(mappedTile);

    // todo: ray tracer currently broken with UMBRA_REMOTE_MEMORY
    if (m_result->m_occlusionBuffer)
    {
        DepthBuffer depth(m_query);
        depth.setBuffer(m_result->m_occlusionBuffer->getDepthBufferPtr(false));
        depth.clear();
    }

    return Query::ERROR_OK;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void PortalRayTracer::initTraverse (const PortalNode& start)
{
    m_stackSize = 1;
    m_stack[0].node = start;
    m_stack[0].t = 0.f;
    memset(m_visitedCells, 0, UMBRA_BITVECTOR_SIZE(m_query->getTome()->getNumCells()));
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Query::ErrorCode PortalRayTracer::execute (VisibilityResult& result)
{
    if (m_transformer->isOrtho())
        return Query::ERROR_UNSUPPORTED_OPERATION;

    m_result = &result;
    UMBRA_ASSERT(m_transformer);
    PortalNode start;

    Query::ErrorCode err = init(start);
    if (err == Query::ERROR_OK)
    {
        for (int y = 0; y < UMBRA_PORTAL_REFERENCE_RASTER_SIZE; y++)
        for (int x = 0; x < UMBRA_PORTAL_REFERENCE_RASTER_SIZE; x++)
        {
            Vector4 pf((x + 0.5f) / UMBRA_PORTAL_REFERENCE_RASTER_SIZE * 2.f - 1.f,
                       (y + 0.5f) / UMBRA_PORTAL_REFERENCE_RASTER_SIZE * 2.f - 1.f,
                       m_farClipZ,
                       1.f);

            Vector4 pf2 = m_clipToWorld.transform(pf);
            pf2.x /= pf2.w;
            pf2.y /= pf2.w;
            pf2.z /= pf2.w;

            dir = Vector3(pf2.x, pf2.y, pf2.z) - m_start;
            oneDivDir = Vector3(1.f/dir.x, 1.f/dir.y, 1.f/dir.z);
            oneDivDotDirDir = 1.f/dot(dir, dir);
            maxT = 0.f;

            dir4.set(dir.x, dir.y, dir.z, 1.f);
            start4.set(m_start.x, m_start.y, m_start.z, 1.f);
            oneDivDir4.set(oneDivDir.x, oneDivDir.y, oneDivDir.z, 1.f);
            oneDivDotDirDir4.set(oneDivDotDirDir, oneDivDotDirDir, oneDivDotDirDir, 1.f);

            initTraverse(start);
            trace();

            if (m_result->m_occlusionBuffer)
            {
                float depth;
                if (maxT >= 1.f)
                     depth = ImpOcclusionBuffer::getMaxDepth();
                else
                {
                    Vector4 v = m_transformer->getWorldToClip().transform(Vector4(m_start + maxT*dir, 1.f));
                    depth = max2(0.f, v.z / v.w);
                }
                m_result->m_occlusionBuffer->writeDepth(depth, x, y);
            }

            if (m_query->debugEnabled(Query::DEBUGFLAG_VISIBILITY_LINES))
            {
                Vector3 end = m_start + dir * maxT;
                m_query->addQueryDebugLine(m_start, end, Vector4(1,1,1,1));
                m_query->addQueryDebugLine(end + m_scaleVector0 * maxT, end + m_scaleVector1 * maxT, Vector4(0.5,1,0.5,1));
                m_query->addQueryDebugLine(end - m_scaleVector0 * maxT, end - m_scaleVector1 * maxT, Vector4(0.5,1,0.5,1));
                m_query->addQueryDebugLine(end + m_scaleVector0 * maxT, end - m_scaleVector1 * maxT, Vector4(0.5,1,0.5,1));
                m_query->addQueryDebugLine(end - m_scaleVector0 * maxT, end + m_scaleVector1 * maxT, Vector4(0.5,1,0.5,1));
            }
        }
    }

    return err;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool PortalRayTracer::intersectAABB(Vector3 mn, Vector3 mx, float& tMinOut, float& tMaxOut, const Vector3& dir)
{
    // Grow AABB to make things conservative.

    Vector3 cornerPoint;

    cornerPoint[0] = (dir.x < 0.f) ? mn[0] : mx[0];
    cornerPoint[1] = (dir.y < 0.f) ? mn[1] : mx[1];
    cornerPoint[2] = (dir.z < 0.f) ? mn[2] : mx[2];

    const Vector3 start = m_transformer->getCameraPos();
    float growT = dot(dir, cornerPoint - start) * oneDivDotDirDir;
    growT = max2(growT, 0.f);

    Vector3 scale(scaleVector.x * growT, scaleVector.y * growT, scaleVector.z * growT);
    mn -= scale;
    mx += scale;

    // Intersect.

    Vector3 t0 = (mn - start);
    t0.x *= oneDivDir.x;
    t0.y *= oneDivDir.y;
    t0.z *= oneDivDir.z;

    Vector3 t1 = (mx - start);
    t1.x *= oneDivDir.x;
    t1.y *= oneDivDir.y;
    t1.z *= oneDivDir.z;

    Vector3 minT( min2(t0.x, t1.x), min2(t0.y, t1.y), min2(t0.z, t1.z) );
    Vector3 maxT( max2(t0.x, t1.x), max2(t0.y, t1.y), max2(t0.z, t1.z) );

    float tMin = tMinOut;
    float tMax = FLT_MAX;

    tMin = max2(tMin, minT[0]);
    tMax = min2(tMax, maxT[0]);

    tMin = max2(tMin, minT[1]);
    tMax = min2(tMax, maxT[1]);

    tMin = max2(tMin, minT[2]);
    tMax = min2(tMax, maxT[2]);

    tMinOut = tMin; // output t value
    tMaxOut = tMax;

    return (tMin <= tMax);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void PortalRayTracer::trace (void)
{
    CellGraphTraversal traverse(m_query, m_start, m_visitedCells);
    SIMDRegister cameraPos = SIMDLoadW1(m_start);

    while (m_stackSize)
    {
        StackItem si = m_stack[--m_stackSize];
        PortalNode& node = si.node;
        float t = si.t;

        traverse.prefetchNode(node);
        if (!traverse.enterNode(node))
            continue;

        if (m_result->m_clusters)
        {
            int cluster = traverse.getCluster(node);
            if (!testAndSetBit(m_result->m_clusterVector, cluster))
                m_result->m_clusters->pushBack(cluster);
        }

        if (m_result->hasObjectVisibility())
        {
            while (traverse.getObjects().hasMore())
            {
                int obj = traverse.getTile().getMappedTome().mapLocalObject(traverse.getObjects().next());

                //-----------------------------------------------------
                // Check for already visible
                //-----------------------------------------------------

                if (testBit(m_result->m_processedObjectVector, obj))
                    continue;

                //-----------------------------------------------------
                // Cull
                //-----------------------------------------------------

                float tMin = 0.f;
                float tMax = 0.f;
                ObjectBounds bound;
                m_objBounds.get(bound, obj);
                ObjectDistance dist;
                if (m_objDist.getOriginal())
                    m_objDist.get(dist, obj);

                SIMDRegister mnSIMD = SIMDLoad(Vector4(bound.mn, 1.f));
                SIMDRegister mxSIMD = SIMDLoad(Vector4(bound.mx, 1.f));

                float contribution;
                Vector4i UMBRA_ATTRIBUTE_ALIGNED16(mnmx);
                m_transformer->transformBox(mnmx, mnSIMD, mxSIMD, true, contribution);
                
                if (contribution >= m_minContribution && 
                    intersectAABB(bound.mn, bound.mx, tMin, tMax, dir) &&
                    distanceInRange(cameraPos, dist, m_distanceScaleSqr) &&
                    m_transformer->frustumTestBounds(mnSIMD, mxSIMD))
                {
                    setBit(m_result->m_processedObjectVector, obj);
                    if (m_result->m_objects)
                        m_result->m_objects->pushBack(obj);
                    if (m_result->m_visibleObjectVector)
                        setBit(m_result->m_visibleObjectVector, obj);
                    if (m_result->m_objectDistances.getBuf())
                    {
                        SIMDRegister distMn = SIMDLoad(m_objDist.getOriginal() ? (float*)&dist.boundMin : (float*)&bound.mn);
                        SIMDRegister distMx = SIMDLoad(m_objDist.getOriginal() ? (float*)&dist.boundMax : (float*)&bound.mx);
                        float d;
                        SIMDStore(distanceAABBPointSqrSIMD(cameraPos, distMn, distMx), d);
                        m_result->m_objectDistances.pushBack(d);
                    }

                    if (m_result->m_objectContributions.getBuf())
                        m_result->m_objectContributions.pushBack(contribution);
                }
            }
        }

        // compute depth to cell AABB far face
        Vector3 mn, mx;
        traverse.cellBounds(mn, mx, node);
        AABB aabb(mn, mx);
        float tMin = 0.f;
        float tMax = 0.f;

        intersectAABB(mn, mx, tMin, tMax, dir);

        if (tMax > maxT)
            maxT = tMax;

        while (traverse.getPortals().hasMore())
        {
            const Portal& portal = traverse.getPortals().next();

            //-----------------------------------------------------
            // Backface cull
            //-----------------------------------------------------

            if (!portal.isUser())
            {
                const UINT32 s = getFaceDirection(portal.getFace());
                const UINT32 a = getFaceAxis(portal.getFace());
                const int z = (portal.idx_z & 0xFFFF);

                if (z != traverse.m_cameraPos[a] && (z < traverse.m_cameraPos[a]) != (s == 0))
                    continue;
            }

            //-----------------------------------------------------
            // Don't enter lod levels
            //-----------------------------------------------------

            if (portal.isHierarchy())
                continue;

            //-----------------------------------------------------
            // Test
            //-----------------------------------------------------

            float tMin = t;
            float tMax = 0.f;

            if (!portal.isUser())
            {
                Vector3 mn, mx;
                portal.getMinMax(traverse.m_slotMin, traverse.m_slotMax, traverse.m_portalExpand, mn, mx);

                // Intersect beam with portal

                if (!intersectAABB(mn, mx, tMin, tMax, dir))
                    continue;
            }

            //-----------------------------------------------------
            // Exit portal
            //-----------------------------------------------------

            if (portal.isOutside())
            {
                if ((1 << portal.getFace()) & traverse.getTile().getExitPortalMask())
                    maxT = 1.f;
                continue;
            }

            //-----------------------------------------------------
            // Enter
            //-----------------------------------------------------

            PortalNode target;
            if (!enterPortal(m_query, traverse.getTile(), target, portal, traverse.getPortals().isExternal(),
                traverse.getElemStartMap(), traverse.getVisited()))
                continue;

            //-----------------------------------------------------
            // Push
            //-----------------------------------------------------

            UMBRA_ASSERT(m_stackSize < PRT_STACK_SIZE);
            if (m_stackSize < PRT_STACK_SIZE)
            {
                StackItem& targetSI = m_stack[m_stackSize++];
                targetSI.node = target;
                targetSI.t = tMin;
            }
        }
    }
}
