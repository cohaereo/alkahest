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
 * \brief   Umbra connectivity query implementations
 *
 */

#include "umbraConnectivity.hpp"
#include "umbraIntersect.hpp"
#include "umbraSIMD.hpp"
#include "runtime/umbraQuery.hpp"

using namespace Umbra;

namespace Umbra
{

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

static float sqrDistanceVecTri(const Vector3& point, const Vector3& triA,
    const Vector3& triB, const Vector3& triC)
{
    Vector3 kDiff = triA - point;

    Vector3 edge0 = triB - triA;
    Vector3 edge1 = triC - triA;

    float fA00 = edge0.lengthSqr();
    float fA01 = dot(edge0, edge1);
    float fA11 = edge1.lengthSqr();

    float fB0 = dot(kDiff,edge0);
    float fB1 = dot(kDiff,edge1);

    float fC = kDiff.lengthSqr();

    float fDet = (float)fabsf(fA00*fA11-fA01*fA01);
    float fS = fA01*fB1-fA11*fB0;
    float fT = fA01*fB0-fA00*fB1;

    static const float EPSILON = 0.0001f; // argh

    if (fDet < EPSILON)
        return FLT_MAX;

    float fSqrDist;

    if ( fS + fT <= fDet )
    {
        if ( fS < 0.0f )
        {
            if ( fT < 0.0f )  // region 4
            {
                if ( fB0 < 0.0f )
                {
                    fT = 0.0f;
                    if ( -fB0 >= fA00 )
                    {
                        fS = 1.0f;
                        fSqrDist = fA00+2.0f*fB0+fC;
                    }
                    else
                    {
                        fS = -fB0/fA00;
                        fSqrDist = fB0*fS+fC;
                    }
                }
                else
                {
                    fS = 0.0f;
                    if ( fB1 >= 0.0f )
                    {
                        fT = 0.0f;
                        fSqrDist = fC;
                    }
                    else if ( -fB1 >= fA11 )
                    {
                        fT = 1.0f;
                        fSqrDist = fA11+2.0f*fB1+fC;
                    }
                    else
                    {
                        fT = -fB1/fA11;
                        fSqrDist = fB1*fT+fC;
                    }
                }
            }
            else  // region 3
            {
                fS = 0.0f;
                if ( fB1 >= 0.0f )
                {
                    fT = 0.0f;
                    fSqrDist = fC;
                }
                else if ( -fB1 >= fA11 )
                {
                    fT = 1.0f;
                    fSqrDist = fA11+2.0f*fB1+fC;
                }
                else
                {
                    fT = -fB1/fA11;
                    fSqrDist = fB1*fT+fC;
                }
            }
        }
        else if ( fT < 0.0f )  // region 5
        {
            fT = 0.0f;
            if ( fB0 >= 0.0f )
            {
                fS = 0.0f;
                fSqrDist = fC;
            }
            else if ( -fB0 >= fA00 )
            {
                fS = 1.0f;
                fSqrDist = fA00+2.0f*fB0+fC;
            }
            else
            {
                fS = -fB0/fA00;
                fSqrDist = fB0*fS+fC;
            }
        }
        else  // region 0
        {
            // minimum at interior point
            float fInvDet = 1.0f/fDet;
            fS *= fInvDet;
            fT *= fInvDet;
            fSqrDist = fS*(fA00*fS+fA01*fT+2.0f*fB0) +
                fT*(fA01*fS+fA11*fT+2.0f*fB1)+fC;
        }
    }
    else
    {
        float fTmp0, fTmp1, fNumer, fDenom;

        if ( fS < 0.0f )  // region 2
        {
            fTmp0 = fA01 + fB0;
            fTmp1 = fA11 + fB1;
            if ( fTmp1 > fTmp0 )
            {
                fNumer = fTmp1 - fTmp0;
                fDenom = fA00-2.0f*fA01+fA11;
                if ( fNumer >= fDenom )
                {
                    fS = 1.0f;
                    fT = 0.0f;
                    fSqrDist = fA00+2.0f*fB0+fC;
                }
                else
                {
                    fS = fNumer/fDenom;
                    fT = 1.0f - fS;
                    fSqrDist = fS*(fA00*fS+fA01*fT+2.0f*fB0) +
                        fT*(fA01*fS+fA11*fT+2.0f*fB1)+fC;
                }
            }
            else
            {
                fS = 0.0f;
                if ( fTmp1 <= 0.0f )
                {
                    fT = 1.0f;
                    fSqrDist = fA11+2.0f*fB1+fC;
                }
                else if ( fB1 >= 0.0f )
                {
                    fT = 0.0f;
                    fSqrDist = fC;
                }
                else
                {
                    fT = -fB1/fA11;
                    fSqrDist = fB1*fT+fC;
                }
            }
        }
        else if ( fT < 0.0f )  // region 6
        {
            fTmp0 = fA01 + fB1;
            fTmp1 = fA00 + fB0;
            if ( fTmp1 > fTmp0 )
            {
                fNumer = fTmp1 - fTmp0;
                fDenom = fA00-2.0f*fA01+fA11;
                if ( fNumer >= fDenom )
                {
                    fT = 1.0f;
                    fS = 0.0f;
                    fSqrDist = fA11+2.0f*fB1+fC;
                }
                else
                {
                    fT = fNumer/fDenom;
                    fS = 1.0f - fT;
                    fSqrDist = fS*(fA00*fS+fA01*fT+2.0f*fB0) +
                        fT*(fA01*fS+fA11*fT+2.0f*fB1)+fC;
                }
            }
            else
            {
                fT = 0.0f;
                if ( fTmp1 <= 0.0f )
                {
                    fS = 1.0f;
                    fSqrDist = fA00+2.0f*fB0+fC;
                }
                else if ( fB0 >= 0.0f )
                {
                    fS = 0.0f;
                    fSqrDist = fC;
                }
                else
                {
                    fS = -fB0/fA00;
                    fSqrDist = fB0*fS+fC;
                }
            }
        }
        else  // region 1
        {
            fNumer = fA11 + fB1 - fA01 - fB0;
            if ( fNumer <= 0.0f )
            {
                fS = 0.0f;
                fT = 1.0f;
                fSqrDist = fA11+2.0f*fB1+fC;
            }
            else
            {
                fDenom = fA00-2.0f*fA01+fA11;
                if ( fNumer >= fDenom )
                {
                    fS = 1.0f;
                    fT = 0.0f;
                    fSqrDist = fA00+2.0f*fB0+fC;
                }
                else
                {
                    fS = fNumer/fDenom;
                    fT = 1.0f - fS;
                    fSqrDist = fS*(fA00*fS+fA01*fT+2.0f*fB0) +
                        fT*(fA01*fS+fA11*fT+2.0f*fB1)+fC;
                }
            }
        }
    }

    return (float)fabsf(fSqrDist);
}


/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

static bool addGateCost (QueryContext* q, const Portal& portal, float& dist, float& modifier)
{
    if (!portal.isUser())
        return true;

    const float* gateCosts = q->getState()->getGateCosts();
    if (!gateCosts)
        return true;

    float gateCost = q->getGateCost(portal);
    if (!q->getState()->isGateCostAdditive())
    {
        if (gateCost < 1.f)
        {
            q->setError(Query::ERROR_INVALID_ARGUMENT);
            return false;
        }
        modifier *= gateCost;
        dist *= gateCost; // multiply new modifier into accumulated distance
    }
    else
    {
        if (gateCost < 0.f)
        {
            q->setError(Query::ERROR_INVALID_ARGUMENT);
            return false;
        }
        // simply add to distance
        dist += gateCost;
    }

    return true;
}
/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

static bool isLinkActive (QueryContext* q, const Portal& link)
{
    if (link.isOutside())
        return false;

    if (link.isUser())
    {
        return q->isGateOpen(link);
    }

    return true;
}

static UMBRA_FORCE_INLINE Vector3 getVec3(const DataArray& data, int idx)
{
    Vector3 tmp;
    data.getElem(tmp, idx);
    return tmp;
}

static bool intersectClusterLink(MappedTome& tome, const Portal& portal, const Vector3& point, 
    float distance, float* confidenceInside, float* confidenceOutside)
{
    float distanceSqr = distance * distance;

    if (portal.isUser())
    {
        int offset = portal.getGeometryOfs();
        int cnt = portal.getVertexCount();
        DataArray gateVertices = tome.getTome()->getGateVertices();
        Vector3 center(getVec3(gateVertices, offset));
        float minRadius = getVec3(gateVertices, offset + 2)[1];
        float maxRadius = getVec3(gateVertices, offset + 2)[2];
        Vector4 pleq(getVec3(gateVertices, offset + 1), getVec3(gateVertices, offset + 2)[0]);
        float distToPlane = dot(pleq, point);

        // query point to center
        Vector3 pointToCenter = center - point;
        // query point projected onto plane
        Vector3 projected = point - distToPlane * pleq.xyz();
        // circle center to projected point unit vector & length
        Vector3 centerToProjected = projected - center;
        float centerToProjectedLen;
        centerToProjected.normalizeAndGetLength(centerToProjectedLen);

        float distanceToMaxCircleSqr = (pointToCenter +
            centerToProjected * min2(maxRadius, centerToProjectedLen)).lengthSqr();

        // does not intersect bounding circle
        if (distanceSqr < distanceToMaxCircleSqr)
        {
            if (confidenceOutside)
                *confidenceOutside = min2(*confidenceOutside, sqrtf(distanceToMaxCircleSqr) - distance);
            return false;
        }

        float distanceToMinCircleSqr = (pointToCenter +
            centerToProjected * min2(minRadius, centerToProjectedLen)).lengthSqr();

        // intersects minimum circle
        if (distanceSqr > distanceToMinCircleSqr)
        {
            if (confidenceInside)
                *confidenceInside = min2(*confidenceInside, distance - sqrtf(distanceToMinCircleSqr));
            return true;
        }

        // do triangle-by-triangle intersection test

        float d = FLT_MAX;

        for (int i = 0; i < cnt - 5; i++)
        {
            Vector3 a(getVec3(gateVertices, offset + 3));
            Vector3 b(getVec3(gateVertices, offset + 4 + i));
            Vector3 c(getVec3(gateVertices, offset + 5 + i));

            float triDistanceSqr = sqrDistanceVecTri(point, a, b, c);
            d = min2(d, triDistanceSqr);

            // early exit intersecting if no inside confidence requested
            if (!confidenceInside && (d <= distanceSqr))
                break;
        }

        bool ret = (d <= distanceSqr);
        if (ret && confidenceInside)
            *confidenceInside = min2(*confidenceInside, distance-sqrtf(d));
        if (!ret && confidenceOutside)
            *confidenceOutside = min2(*confidenceOutside, sqrtf(d)-distance);

        return ret;
    }
    else
    {
        Vector3 sceneMin = tome.getTome()->getTreeMin();
        Vector3 sceneMax = tome.getTome()->getTreeMax();

        Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, pmn);
        Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, pmx);
        portal.getMinMax(sceneMin, sceneMax, 0.f, pmn, pmx);
        SIMDRegister mn = SIMDLoadAligned((float*)&pmn.x);
        SIMDRegister mx = SIMDLoadAligned((float*)&pmx.x);
        SIMDRegister pt = SIMDLoadW0(point);
        float d;
        SIMDStore(distanceAABBPointSqrSIMD(pt, mn, mx), d);
        bool ret = (d <= distanceSqr);
        if (ret && confidenceInside)
            *confidenceInside = min2(*confidenceInside, distance-sqrtf(d));
        if (!ret && confidenceOutside)
            *confidenceOutside = min2(*confidenceOutside, sqrtf(d)-distance);
        return ret;
    }
}

} // namespace Umbra

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

template<class NODE> bool ConnectTraversal<NODE>::updateNode (NODE node, float dist)
{
    UMBRA_ASSERT(!isNodeClosed(node));

    if (isNodeOpen(node))
    {
        return m_openSet.decreaseKey(dist, node);
    }
    else
    {
        m_openSet.insert(dist, node);
        setBit(m_open, node.getIndex());
        return true;
    }
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

template<class NODE> NODE ConnectTraversal<NODE>::removeFirst (float& distance)
{
    distance = m_openSet.getKey(0);
    NODE node = m_openSet.getValue(0);
    m_openSet.removeFirst();
    setBit(m_closed, node.getIndex());
    return node;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool DepthFirstRegionFinder::execute(
    UserList<int>* clustersOut,
    const Umbra::UINT32* clustersToFind,
    int startCluster,
    const Vector3& center,
    float radius,
    float* confidenceBound)
{
    UMBRA_ASSERT(startCluster != -1);

    // TODO: relying on default tome here is very dodgy
    MappedTome currentTome = m_query->getDefaultTome();
    int startGlobal = currentTome.mapLocalCluster(startCluster);

    if (clustersToFind && testBit(clustersToFind, startGlobal))
    {
        return true;
    }
    if (clustersOut && !clustersOut->pushBack(startGlobal))
    {
        m_query->setError(Query::ERROR_OUT_OF_MEMORY);
        return true;
    }

    int clustersTotal = m_query->getState()->getRootTome()->getNumClusters();
    memset(m_visited, 0, UMBRA_BITVECTOR_SIZE(clustersTotal));
    setBit(m_visited, startGlobal);

    m_clusters.setArray(currentTome.getTome()->getClusterNodes());
    m_extClusters.setArray(currentTome.getExtClusterNodes());

    m_stack[0].tome = currentTome.getIndex();
    m_stack[0].cluster = startCluster;

    float confidenceInValue = FLT_MAX;
    float confidenceOutValue = FLT_MAX;
    float* confidenceIn = NULL;
    float* confidenceOut = NULL;
    if (confidenceBound)
    {
        UINT32 f = m_flags & (QueryExt::QUERYFLAG_CONFIDENCE_INTERSECTING | QueryExt::QUERYFLAG_CONFIDENCE_INTERSECTING);
        if (f == QueryExt::QUERYFLAG_CONFIDENCE_INTERSECTING)
        {
            confidenceIn = &confidenceInValue;
        }
        else if (f == QueryExt::QUERYFLAG_CONFIDENCE_NONINTERSECTING)
        {
            confidenceOut = &confidenceOutValue;
        }
        else
        {
            confidenceIn = &confidenceInValue;
            confidenceOut = &confidenceOutValue;
        }
    }

    int stackHead = 0;
    while (stackHead >= 0)
    {
        ClusterNode cur;
        ExtClusterNode curExt;

        int nextTome = m_stack[stackHead].tome;
        int curIdx = m_stack[stackHead].cluster;
        stackHead--;

        if (nextTome != currentTome.getIndex())
        {
            m_query->getState()->mapTome(currentTome, nextTome);
            m_clusters.setArray(currentTome.getTome()->getClusterNodes());
            m_extClusters.setArray(currentTome.getExtClusterNodes());
        }

        m_clusters.get(cur, curIdx);
        if (m_extClusters.getCount())
            m_extClusters.get(curExt, curIdx);

        if (cur.getPortalCount() + curExt.getPortalCount() > 0)
        {
            m_portals.init(currentTome, cur, curExt);

            while (m_portals.hasMore())
            {
                const Portal& portal = m_portals.next();
                if (!isLinkActive(m_query, portal))
                    continue;

                int tomeIdx = m_portals.isExternal() ? portal.getTargetIndex() : currentTome.getIndex();
                int target = portal.getTargetCluster();
                int global = m_query->getState()->mapLocalCluster(tomeIdx, target);

                if (testBit(m_visited, global))
                    continue;

                if (!(radius <= 0.f) &&
                    !intersectClusterLink(currentTome, portal, center, radius, confidenceIn, confidenceOut))
                    continue;

                if (clustersToFind && testBit(clustersToFind, global))
                {
                    return true;
                }
                if (clustersOut && !clustersOut->pushBack(global))
                {
                    m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                    return true;
                }

                if (stackHead >= UMBRA_MAX_CELLS_PER_TILE)
                {
                    m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                    return true;
                }

                setBit(m_visited, global);
                m_stack[++stackHead].cluster = target;
                m_stack[stackHead].tome = tomeIdx;
            }
        }
    }

    if (confidenceBound)
    {
        *confidenceBound = FLT_MAX;
        if (confidenceIn)
            *confidenceBound = min2(*confidenceBound, *confidenceIn);
        if (confidenceOut)
            *confidenceBound = min2(*confidenceBound, *confidenceOut);
    }

    return false;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

RegionFinder::RegionFinder(QueryContext* q, int cluster, const Vector3& point, float limit, bool skipFirstDistance)
:   m_clusters(q, q->getDefaultTome().getTome()->getClusterNodes()),
    m_extClusters(q, q->getDefaultTome().getExtClusterNodes()),
    m_clusterPortals(q, q->getDefaultTome().getTome()->getClusterPortals()),
    m_extClusterPortals(q, q->getDefaultTome().getExtPortals()),
    m_query(q),
    m_traverse(q, q->getState()->getRootTome()->getNumClusterPortals() + 1),
    m_startCluster(cluster),
    m_startPoint(point),
    m_limit(limit),
    m_clusterBV(NULL),
    m_skipFirstDistance(skipFirstDistance)
{
    if (m_query->getError() == Query::ERROR_OUT_OF_MEMORY ||
        !m_clusters.isInitialized() ||
        !m_extClusters.isInitialized() ||
        !m_clusterPortals.isInitialized() ||
        !m_extClusterPortals.isInitialized())
    {
        m_query->setError(Query::ERROR_OUT_OF_MEMORY);
        return;
    }

    m_startNodeIdx = m_traverse.getNumNodes() - 1;
    m_clusterBV = UMBRA_HEAP_NEW_ARRAY(q->getAllocator(), UINT32, UMBRA_BITVECTOR_DWORDS(m_query->getState()->getRootTome()->getNumClusters()));
    if (!m_clusterBV)
    {
        m_query->setError(Query::ERROR_OUT_OF_MEMORY);
        return;
    }
    memset(m_clusterBV, 0, UMBRA_BITVECTOR_SIZE(m_query->getState()->getRootTome()->getNumClusters()));
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

RegionFinder::~RegionFinder(void)
{
    UMBRA_HEAP_DELETE_ARRAY(m_query->getAllocator(), m_clusterBV);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void RegionFinder::execute (UserList<int>* clusters, UserList<float>* clusterPathDistances, UserList<float>* clusterPathModifiers, UserList<int>* clusterEntryPortals)
{
    if (m_startCluster == -1)
    {
        m_query->setError(Query::ERROR_OUTSIDE_SCENE);
        return;
    }
    m_traverse.updateNode(ClusterPathNode(m_startNodeIdx, 1.f), 0.f);

    MappedTome mappedTome = m_query->getDefaultTome();

    while (!m_traverse.isEmpty())
    {
        float accumulatedDist;
        ClusterPathNode n = m_traverse.removeFirst(accumulatedDist);

        Vector3 source;
        int currentCluster;
        int currentPortalIndex;

        if (n.index == m_startNodeIdx)
        {
            source = m_startPoint;
            currentCluster = m_startCluster;
            currentPortalIndex = -1;
        }
        else
        {
            Portal currentPortal;

            int tomeIdx    = m_query->getState()->findTomeByClusterPortal(n.index);

            int oldIdx = mappedTome.getIndex();
            if (tomeIdx != mappedTome.getIndex())
            {
                m_query->getState()->mapTome(mappedTome, tomeIdx);
                m_clusterPortals.setArray(mappedTome.getTome()->getClusterPortals());
                m_extClusterPortals.setArray(mappedTome.getExtPortals());
            }

            int  localIdx   = mappedTome.mapGlobalClusterPortal(n.index);
            bool isExternal = localIdx >= mappedTome.getTome()->getNumClusterPortals();

            if (isExternal)
                m_extClusterPortals.get(currentPortal, localIdx - mappedTome.getTome()->getNumClusterPortals());
            else
                m_clusterPortals.get(currentPortal, localIdx);

            tomeIdx         = isExternal ? currentPortal.getTargetIndex() : mappedTome.getIndex();
            currentCluster  = currentPortal.getTargetCluster();

            if (tomeIdx != mappedTome.getIndex())
            {
                m_query->getState()->mapTome(mappedTome, tomeIdx);
                m_clusterPortals.setArray(mappedTome.getTome()->getClusterPortals());
                m_extClusterPortals.setArray(mappedTome.getExtPortals());
            }

            if (mappedTome.getIndex() != oldIdx)
            {
                m_clusters.setArray(mappedTome.getTome()->getClusterNodes());
                m_extClusters.setArray(mappedTome.getExtClusterNodes());
            }

            source = mappedTome.getTome()->getClusterPortalCenter(currentPortal);
            currentPortalIndex = n.index;
        }

        ClusterNode clusterNode;
        ExtClusterNode extClusterNode;
        m_clusters.get(clusterNode, currentCluster);
        if (m_extClusters.getCount())
            m_extClusters.get(extClusterNode, currentCluster);

        //m_portals.init(m_query->getDefaultTome(), clusterNode, extClusterNode);

        int globalCluster = mappedTome.mapLocalCluster(currentCluster);
        if (clusters && !testAndSetBit(m_clusterBV, globalCluster))
        {
            if (!clusters->pushBack(globalCluster))
            {
                m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                return;
            }

            if (clusterPathDistances && !clusterPathDistances->pushBack(accumulatedDist))
            {
                m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                return;
            }

            if (clusterPathModifiers && !clusterPathModifiers->pushBack(n.modifier))
            {
                m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                return;
            }

            if (clusterEntryPortals && !clusterEntryPortals->pushBack(currentPortalIndex))
            {
                m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                return;
            }
        }

        for (int i = 0; i < clusterNode.getPortalCount() + extClusterNode.getPortalCount(); i++)
        {
            //bool isExternal            = m_portals.isExternal();
            //const Portal& targetPortal = m_portals.next();

            Portal targetPortal;
            ClusterPathNode target;
            if (i >= clusterNode.getPortalCount())
            {
                int idx = i - clusterNode.getPortalCount() + extClusterNode.getPortalIndex();
                m_extClusterPortals.get(targetPortal, idx);
                target.index = mappedTome.mapLocalClusterPortal(idx + mappedTome.getTome()->getNumClusterPortals());
            } else
            {
                int idx = i + clusterNode.getPortalIndex();
                m_clusterPortals.get(targetPortal, idx);
                target.index = mappedTome.mapLocalClusterPortal(idx);
            }

            target.modifier  = n.modifier;

            if (m_traverse.isNodeClosed(target))
                continue;

            if (!isLinkActive(m_query, targetPortal))
                continue;

            Vector3 center = mappedTome.getTome()->getClusterPortalCenter(targetPortal);

            if (n.index == m_startNodeIdx && m_skipFirstDistance)
                source = center;    // don't take first distance into account

            // first get the distance to the portal
            float dist = accumulatedDist + target.modifier * (center - source).length();

            // then add the gate cost (fails if modified distance would be smaller)
            if (!addGateCost(m_query, targetPortal, dist, target.modifier))
                return;

            if (dist > m_limit)
                continue;

            m_traverse.updateNode(target, dist);
            if (m_traverse.getSize() >= UMBRA_OPENSET_LIMIT)
            {
                UMBRA_ASSERT(!"Open set limit reached");
                m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                return;
            }
        }
    }
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

size_t PathFinder::getMemoryRequirement(const ImpTome* tome)
{
    int numNodes = tome->getNumClusterPortals() + 2;

    StatsAlloc stack((size_t)0x7fffffff);
    QueryContext::estimateSize(&stack);

#ifdef UMBRA_REMOTE_MEMORY
    ArrayMapper::estimateSize(&stack, tome->getClusterNodes());
    ArrayMapper::estimateSize(&stack, tome->getClusterPortals());
#endif

    // pathfinder
    UMBRA_HEAP_ALLOC(&stack, sizeof(PathFinder));
    // open+closed sets allocated by ConnectTraversal
    //UMBRA_HEAP_NEW_ARRAY_NOINIT(&stack, UINT32, UMBRA_BITVECTOR_DWORDS(numNodes)*2);
    ConnectTraversal<ClusterPathNode>::estimateSize(&stack, numNodes);
    // NodeLocator used by PathFinder::find
    UMBRA_HEAP_ALLOC(&stack, sizeof(NodeLocator));
    // m_sources array
    UMBRA_HEAP_NEW_ARRAY_NOINIT(&stack, UINT16, numNodes);

    return stack.allocated();
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

PathFinder::PathFinder(QueryContext& q)
:   QueryRunner(q),
    m_clusters(&q, sizeof(ClusterNode)),
    m_extClusters(&q, sizeof(ExtClusterNode)),
    m_clusterPortals(&q, sizeof(Portal)),
    m_extClusterPortals(&q, sizeof(Portal)),
    m_traverse(&q, q.getState()->getRootTome()->getNumClusterPortals() + 2)
{
    if (m_traverse.getNumNodes() >= 0xFFFF)
        return;

    if (m_query->getError() == Query::ERROR_OUT_OF_MEMORY ||
        !m_clusters.isInitialized() ||
        !m_extClusters.isInitialized() ||
        !m_clusterPortals.isInitialized() ||
        !m_extClusterPortals.isInitialized())
    {
        m_query->setError(Query::ERROR_OUT_OF_MEMORY);
        return;
    }

    m_sources = UMBRA_NEW_ARRAY(UINT16, m_traverse.getNumNodes());
    if (!m_sources)
    {
        m_query->setError(Query::ERROR_OUT_OF_MEMORY);
        return;
    }

    m_startNodeIdx = (UINT16)(m_traverse.getNumNodes() - 1);
    m_endNodeIdx = (UINT16)(m_traverse.getNumNodes() - 2);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

PathFinder::~PathFinder (void)
{
    UMBRA_DELETE_ARRAY(m_sources);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void PathFinder::reversePath (void)
{
    UINT16 cur = m_endNodeIdx;
    UINT16 prev = m_sources[cur];
    while (prev != INVALID_NODE)
    {
        UINT16 next = m_sources[prev];
        m_sources[prev] = cur;
        cur = prev;
        prev = next;
    }
    m_sources[m_endNodeIdx] = INVALID_NODE;
    UMBRA_ASSERT(cur == m_startNodeIdx);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void PathFinder::outputPath (ImpPath& p, float mod, const Vector3& start, const Vector3& end)
{
    int curNode = m_startNodeIdx;
    Vector3 curCoord = start;
    float dist = 0.f;

    p.pushNode(start, -1, dist);

    MappedTome mappedTome = m_query->getDefaultTome();

    int nextNode = m_sources[curNode];
    while (nextNode != m_endNodeIdx)
    {
        int tomeIdx = m_query->getState()->findTomeByClusterPortal(nextNode);
        if (tomeIdx != mappedTome.getIndex())
        {
            m_query->getState()->mapTome(mappedTome, tomeIdx);
            m_clusterPortals.setArray(mappedTome.getTome()->getClusterPortals());
            m_extClusterPortals.setArray(mappedTome.getExtPortals());
        }

        int  localIdx   = mappedTome.mapGlobalClusterPortal(nextNode);
        bool isExternal = localIdx >= mappedTome.getTome()->getNumClusterPortals();

        Portal currentPortal;
        if (isExternal)
            m_extClusterPortals.get(currentPortal, localIdx - mappedTome.getTome()->getNumClusterPortals());
        else
            m_clusterPortals.get(currentPortal, localIdx);

        Vector3 nextCoord = mappedTome.getTome()->getClusterPortalCenter(currentPortal);
        dist += (nextCoord - curCoord).length() * mod;
        if (currentPortal.isUser() && m_query->getState()->getGateCosts() && m_query->getState()->isGateCostAdditive())
            dist += m_query->getGateCost(currentPortal);
        p.pushNode(nextCoord, nextNode, dist);
        curNode = nextNode;
        nextNode = m_sources[curNode];
        curCoord = nextCoord;
    }

    dist += ((end - curCoord).length() * mod);
    p.pushNode(end, -1, dist);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void PathFinder::find (ImpPath& p, const Vector3& start, const Vector3& end)
{
    int endCluster   = m_query->findCluster(end);
    int endTome      = m_query->getDefaultTome().getIndex();
    int startCluster = m_query->findCluster(start);

    if (startCluster == -1 || endCluster == -1)
    {
        m_query->setError(Query::ERROR_OUTSIDE_SCENE);
        return;
    }

    MappedTome mappedTome = m_query->getDefaultTome();

    m_traverse.reset();
    m_traverse.updateNode(ClusterPathNode(m_startNodeIdx, 1.f), 0.f);
    m_sources[m_startNodeIdx] = INVALID_NODE;

    while (!m_traverse.isEmpty())
    {
        float accumulatedDist;
        ClusterPathNode n = m_traverse.removeFirst(accumulatedDist);

        // shortest path found
        if (n.index == m_endNodeIdx)
        {
            reversePath();
            outputPath(p, n.modifier, start, end);
            return;
        }

        Vector3 source;
        int currentCluster;

        if (n.index == m_startNodeIdx)
        {
            source = start;
            currentCluster = startCluster;
            m_clusters.setArray(mappedTome.getTome()->getClusterNodes());
            m_extClusters.setArray(mappedTome.getExtClusterNodes());
            m_clusterPortals.setArray(mappedTome.getTome()->getClusterPortals());
            m_extClusterPortals.setArray(mappedTome.getExtPortals());
        }
        else
        {
            Portal currentPortal;

            int tomeIdx    = m_query->getState()->findTomeByClusterPortal(n.index);

            int oldIdx = mappedTome.getIndex();
            if (tomeIdx != mappedTome.getIndex())
            {
                m_query->getState()->mapTome(mappedTome, tomeIdx);
                m_clusterPortals.setArray(mappedTome.getTome()->getClusterPortals());
                m_extClusterPortals.setArray(mappedTome.getExtPortals());
            }

            int  localIdx   = mappedTome.mapGlobalClusterPortal(n.index);
            bool isExternal = localIdx >= mappedTome.getTome()->getNumClusterPortals();

            if (isExternal)
                m_extClusterPortals.get(currentPortal, localIdx - mappedTome.getTome()->getNumClusterPortals());
            else
                m_clusterPortals.get(currentPortal, localIdx);

            tomeIdx         = isExternal ? currentPortal.getTargetIndex() : mappedTome.getIndex();
            currentCluster  = currentPortal.getTargetCluster();

            source = mappedTome.getTome()->getClusterPortalCenter(currentPortal);

            if (tomeIdx != mappedTome.getIndex())
            {
                m_query->getState()->mapTome(mappedTome, tomeIdx);
                m_clusterPortals.setArray(mappedTome.getTome()->getClusterPortals());
                m_extClusterPortals.setArray(mappedTome.getExtPortals());
            }

            if (mappedTome.getIndex() != oldIdx)
            {
                m_clusters.setArray(mappedTome.getTome()->getClusterNodes());
                m_extClusters.setArray(mappedTome.getExtClusterNodes());
            }
        }

        if (currentCluster == endCluster && mappedTome.getIndex() == endTome)
        {
            ClusterPathNode target;
            target.index = m_endNodeIdx;
            target.modifier = n.modifier;
            float dist = accumulatedDist + target.modifier * (end - source).length();

            if (m_traverse.updateNode(target, dist))
                m_sources[target.index] = (UINT16)n.index;

            if (m_traverse.getSize() >= UMBRA_OPENSET_LIMIT)
            {
                UMBRA_ASSERT(!"Open set limit reached");
                m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                return;
            }
        }
        else
        {
            ClusterNode clusterNode;
            ExtClusterNode extClusterNode;
            m_clusters.get(clusterNode, currentCluster);
            if (m_extClusters.getCount())
                m_extClusters.get(extClusterNode, currentCluster);

            for (int i = 0; i < clusterNode.getPortalCount() + extClusterNode.getPortalCount(); i++)
            {
                Portal targetPortal;
                ClusterPathNode target;
                target.modifier = n.modifier;
                if (i >= clusterNode.getPortalCount())
                {
                    int idx = i - clusterNode.getPortalCount() + extClusterNode.getPortalIndex();
                    m_extClusterPortals.get(targetPortal, idx);
                    target.index = mappedTome.mapLocalClusterPortal(idx + mappedTome.getTome()->getNumClusterPortals());
                } else
                {
                    int idx = i + clusterNode.getPortalIndex();
                    m_clusterPortals.get(targetPortal, idx);
                    target.index = mappedTome.mapLocalClusterPortal(idx);
                }

                if (m_traverse.isNodeClosed(target))
                    continue;

                if (!isLinkActive(m_query, targetPortal))
                    continue;

                Vector3 center = mappedTome.getTome()->getClusterPortalCenter(targetPortal);

                // first get the distance to the portal
                float dist = accumulatedDist + target.modifier * (center - source).length();

                // then add the gate cost (fails if modified distance would be smaller)
                if (!addGateCost(m_query, targetPortal, dist, target.modifier))
                    return;

                if (m_traverse.updateNode(target, dist))
                    m_sources[target.index] = (UINT16)n.index;

                if (m_traverse.getSize() >= UMBRA_OPENSET_LIMIT)
                {
                    UMBRA_ASSERT(!"Open set limit reached");
                    m_query->setError(Query::ERROR_OUT_OF_MEMORY);
                    return;
                }
            }
        }
    }

    m_query->setError(Query::ERROR_NO_PATH);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

inline bool LineSegmentFinder::intersectRayAABB(const SIMDRegister& min, const SIMDRegister& max)
{
    // slab method

    SIMDRegister t0 = SIMDSub(min, m_startSIMDLocal);
    t0 = SIMDMultiply(t0, m_invdir);

    SIMDRegister t1 = SIMDSub(max, m_startSIMDLocal);
    t1 = SIMDMultiply(t1, m_invdir);

    SIMDRegister mn = SIMDMin(t0, t1);
    SIMDRegister mx = SIMDMax(t0, t1);

    SIMDRegister tMin = SIMDReplicate(mn, 0);
    tMin = SIMDMax(tMin, SIMDReplicate(mn, 1));
    tMin = SIMDMax(tMin, SIMDReplicate(mn, 2));

    // all values in mx must be GE zero
    SIMDRegister out = SIMDCompareGT(SIMDZero(), mx);
    // all values in mn must be LE one
    out = SIMDBitwiseOr(out, SIMDCompareGT(mn, SIMDOne()));
    // max value of mn must be less than min value of mx
    out = SIMDBitwiseOr(out, SIMDCompareGT(tMin, mx));

#if UMBRA_OS == UMBRA_XBOX360
    if (SIMDNotZero(SIMDSelect(out, SIMDZero(), SIMDMaskW())))
        return false;
#elif UMBRA_OS == UMBRA_PS3
    // The order of SIMDExtractSignBits is reversed on PS3 compared to SSE
    // todo fix properly
    if (SIMDExtractSignBits(out) & 0xE)
        return false;
#else
    if (SIMDExtractSignBits(out) & 0x7)
        return false;
#endif
    return true;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

inline void LineSegmentFinder::findHitPoints(const Vector4& vMin, const Vector4& vMax, SIMDRegister& tMin, SIMDRegister& tMax)
{
    // calculates [0, 1] intersection point when we already know it hits
    // slab method

    SIMDRegister min = SIMDLoadAligned((float*)&vMin);
    SIMDRegister max = SIMDLoadAligned((float*)&vMax);

    SIMDRegister t0 = SIMDSub(min, m_startSIMD);
    t0 = SIMDMultiply(t0, m_invdir);

    SIMDRegister t1 = SIMDSub(max, m_startSIMD);
    t1 = SIMDMultiply(t1, m_invdir);

    SIMDRegister mn = SIMDMin(t0, t1);
    SIMDRegister mx = SIMDMax(t0, t1);

    tMin = SIMDZero();
    tMin = SIMDMax(tMin, SIMDReplicate(mn, 0));
    tMin = SIMDMax(tMin, SIMDReplicate(mn, 1));
    tMin = SIMDMax(tMin, SIMDReplicate(mn, 2));

    tMax = SIMDOne();
    tMax = SIMDMin(tMax, SIMDReplicate(mx, 0));
    tMax = SIMDMin(tMax, SIMDReplicate(mx, 1));
    tMax = SIMDMin(tMax, SIMDReplicate(mx, 2));
}

namespace Umbra
{
#if UMBRA_LINESEGMENT_DEBUG
enum LineSegmentStat
{
    LSS_TILE,
    LSS_CELL,
    LSS_PORTALS,
    LSS_CELLS,
    LSS_STEP_PORTALS
};
int lineSegmentDebugStep     = 0;              // which step to debug
int lineSegmentDebugStats[5] = {-1,-1,0,0,0}; // stats for current step
#endif
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void LineSegmentFinder::execute(ImpLineSegmentQuery* queries, int count)
{
    if (!count)
        return;

    bool sameStart = true;
    for (int i = 1; i < count; i++)
    {
        if (queries[i].q.m_start != queries[0].q.m_start)
        {
            sameStart = false;
            break;
        }
    }

    IndexList*  prevObjectSet   = NULL;

    Vector3* batchA     = (Vector3*)m_query->allocWorkMem(sizeof(Vector3) * UMBRA_FINDCELL_BATCH);
    Vector3* batchB     = (Vector3*)m_query->allocWorkMem(sizeof(Vector3) * UMBRA_FINDCELL_BATCH);
    Cell*    startCell  = (Cell*)   m_query->allocWorkMem(sizeof(Cell)    * UMBRA_FINDCELL_BATCH);
    Cell*    endCell    = (Cell*)   m_query->allocWorkMem(sizeof(Cell)    * UMBRA_FINDCELL_BATCH);

    Cell startCellDefault;
    if (sameStart)
        startCellDefault = m_query->findCell(queries[0].q.m_start);

    while (count)
    {
        int batchSize = min2(count, UMBRA_FINDCELL_BATCH);

        for (int i = 0; i < batchSize; i++)
        {
            batchA[i] = queries[i].q.m_start;
            batchB[i] = queries[i].q.m_end;
            startCell[i] = startCellDefault;
            endCell[i] = Cell();
        }
        
        /* \todo [antti 25.6.2012]: proper hashing, include end points too */
        if (!sameStart)
            m_query->findMultipleCells(batchA, startCell, batchSize);
        
        if (UMBRA_LINESEGMENT_ALWAYS_FIND_END)
            m_query->findMultipleCells(batchB, endCell, batchSize);

        for (int i = 0; i < batchSize; i++)
        {
            ImpLineSegmentQuery::Data& qd = queries[i].q;

            if (UMBRA_EXPECT(!startCell[i].valid(), 0))
            {
                qd.m_result = LineSegmentQuery::RESULT_OUTSIDE_SCENE;
                continue;
            }
        
            UserList<int>* objects = NULL;
#if defined(UMBRA_REMOTE_MEMORY)
            UserList<int> UMBRA_ATTRIBUTE_ALIGNED(16, userListLocal);
            if (qd.m_objectSet)
            {
                MemoryAccess::alignedRead(&userListLocal, qd.m_objectSet, sizeof(UserList<int>));
                objects = &userListLocal;
            }
#else
            objects = (UserList<int>*)qd.m_objectSet;
#endif

            if (qd.m_objectSet && qd.m_objectSet != prevObjectSet)
            {
                m_query->freeWorkMem(m_foundObjects);
                objects->clear();
                m_foundObjects = (UINT32*)m_query->allocWorkMem(UMBRA_BITVECTOR_SIZE(m_query->getTome()->getNumObjects()), true);
                prevObjectSet = qd.m_objectSet;
            }

            bool result = false;
            result = queryInternal<true>(batchA[i], batchB[i], startCell[i], endCell[i], objects);
            /*if (objects)
                result = queryInternal<true>(batchA[i], batchB[i], startCell[i], endCell[i], objects);
            else
                result = queryInternal<false>(batchA[i], batchB[i], startCell[i], endCell[i], objects); */

            if (result)
                qd.m_result = LineSegmentQuery::RESULT_NO_INTERSECTION;
            else
                qd.m_result = LineSegmentQuery::RESULT_INTERSECTION;

#if defined(UMBRA_REMOTE_MEMORY)
            if (objects)
                userListLocal.updateRemote((UserList<int>*)qd.m_objectSet);
#endif
        }
        
        count -= batchSize;
        queries += batchSize;
    }

    m_query->freeWorkMem(m_foundObjects);
    m_query->freeWorkMem(endCell);
    m_query->freeWorkMem(startCell);
    m_query->freeWorkMem(batchB);
    m_query->freeWorkMem(batchA);
    
    m_foundObjects = NULL;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

template<bool findObjects>
bool LineSegmentFinder::queryInternal(const Vector3& start, const Vector3& end, Cell& startCell, Cell& endCell, UserList<int>* objects)
{
    #if UMBRA_LINESEGMENT_ALWAYS_FIND_END
    if (UMBRA_EXPECT(startCell == endCell && !objects, 0))
        return true;
    #endif

    ArrayMapper             objBounds(m_query, sizeof(ObjectBounds));

    m_start                 = start;
    m_end                   = end;

    Vector3 dir             = m_end - m_start;

    Vector3                 slotMin, slotMax;
    SIMDRegister scale      = SIMDLoad(65535.0f);            
    m_startSIMD             = SIMDLoadW1(m_start);
    m_endSIMD               = SIMDLoadW1(m_end);

    #if UMBRA_LINESEGMENT_ALWAYS_FIND_END
    if (UMBRA_EXPECT(startCell == endCell, 0))
    {
        if (findObjects && objects)
        {
            MappedTile mappedTile;
            m_query->mapTile(mappedTile, startCell.slotIdx);
            const ImpTile* tile = mappedTile.getTile();
            m_cellNodeMap.setArray(tile->getCellNodes());
            m_extCellNodeMap.setArray(mappedTile.getExtCellNodes());
            
            slotMin = tile->getTreeMin();
            slotMax = tile->getTreeMax();
            m_slotMinSIMD = SIMDLoadW1(slotMin);
            m_slotMaxSIMD = SIMDLoadW1(slotMax);
            
            m_portalExpand     = tile->getPortalExpand();
            m_portalExpandSIMD = SIMDLoadW0(Vector3(m_portalExpand, m_portalExpand, m_portalExpand));

            // Figure out scale for this tile
            m_localScale      = SIMDMultiply(scale, SIMDReciprocalAccurate(SIMDSub(m_slotMaxSIMD, m_slotMinSIMD)));

            // Scale start end end coordinate to tile local space
            m_startSIMDLocal  = SIMDMultiply(SIMDSub(m_startSIMD, m_slotMinSIMD), m_localScale);
            m_endSIMDLocal    = SIMDMultiply(SIMDSub(m_endSIMD,   m_slotMinSIMD), m_localScale);

            // Compute new invdir for this tile
            SIMDRegister diff = SIMDSub(m_endSIMDLocal, m_startSIMDLocal);
            SIMDRegister mask = SIMDCompareEQ(diff, SIMDZero());
            m_invdir          = SIMDReciprocalAccurate(SIMDSelect(diff, SIMDOne(), mask));
            
            {
                objBounds.setArray(m_query->getTome()->getObjBounds());

                CellNode cell;
                tile->getCellNodes().getElem(cell, startCell.index);

                m_objectIter.setArray(tile->getObjects(cell));

                while (!objects->isMaxed() && m_objectIter.hasMore())
                {
                    int object = m_objectIter.next();
                    ObjectBounds mnmx;
                    objBounds.get(mnmx, object);
                    SIMDRegister obMin = SIMDMultiply(SIMDSub(SIMDLoadW1(mnmx.mn), m_slotMinSIMD), m_localScale);
                    SIMDRegister obMax = SIMDMultiply(SIMDSub(SIMDLoadW1(mnmx.mx), m_slotMinSIMD), m_localScale);

                    // Intersect ray with object, test/report each object only once
                    if (testAndSetBit(m_foundObjects, object) ||
                        !intersectRayAABB(obMin, obMax))
                        continue; 

                    objects->pushBack(object);
                }
            }

            m_query->unmapTile(mappedTile);
        }
        return true;
    }
    #endif

    // Generate a mask of accepted faces

#if UMBRA_OS == UMBRA_XBOX360
    // avoid variable bitshifts
    UINT32 faces[6];
    for (int face = 0; face < 6; face++)
    {
        int    axis = getFaceAxis(face);
        UINT32 fdir = getFaceDirection(face);
        UINT32 sign = floatSignBit(dir[axis]);
        faces[face] = (sign ^ fdir);
    }
#else
    UINT32 portalFaceMask = 0;
    for (int face = 0; face < 6; face++)
    {
        int    axis = getFaceAxis(face);
        UINT32 fdir = getFaceDirection(face);
        UINT32 sign = floatSignBit(dir[axis]);
        portalFaceMask |= ((sign ^ fdir) << face);
    }
#endif
    
    // Insert first cell into stack

    int cellStart = m_query->getTome()->getCellStart(startCell.slotIdx);
    PortalNode startNode(startCell.slotIdx, startCell.index, cellStart + startCell.index);
    
    m_historyStart = 0;
    m_historyPos   = 0;

    m_stackStart = 0;
    m_stackEnd   = 0;
    m_stack[m_stackEnd].m_node      = startNode;
    #if UMBRA_LINESEGMENT_CALCULATE_MINMAX
    m_stack[m_stackEnd].m_enterMin  = m_start;
    m_stack[m_stackEnd].m_enterMax  = m_start;
    #endif
    m_stackEnd++;

    if (findObjects && objects)
        objBounds.setArray(m_query->getTome()->getObjectBounds());

    //int numCells = m_query->getTome()->getNumCells();
    //memset(m_visitedCells, 0, UMBRA_BITVECTOR_SIZE(numCells));

    #ifdef UMBRA_DEBUG
    // the m_visitedCells bitvector should always be initially cleared
    int numCells = m_query->getTome()->getNumCells();
    for (int i = 0; i < UMBRA_BITVECTOR_DWORDS(numCells); i++)
    {
        UMBRA_ASSERT(m_visitedCells[i] == 0);
    }
    #endif

    int                     currentSlot = -1;
    MappedTile              mappedTile;
    const ImpTile*          tile = NULL;
    CellNode                currentCell;
    ExtCellNode             currentExtCell;
    PortalIterator          portalIter(m_query->getAllocator(), m_query->getTagManager());

    // Debugging

    #if UMBRA_LINESEGMENT_DEBUG
    int step = 0;
    lineSegmentDebugStats[LSS_TILE] = -1;
    lineSegmentDebugStats[LSS_CELL] = -1;
    lineSegmentDebugStats[LSS_PORTALS] = 0;
    lineSegmentDebugStats[LSS_CELLS] = 0;
    lineSegmentDebugStats[LSS_STEP_PORTALS] = 0;
    #define UMBRA_LINESEGMENT_DEBUG_CODE(X) X
    #define UMBRA_LINESEGMENT_DEBUG_PORTAL(color) \
        do { \
            if (step == lineSegmentDebugStep) \
            { \
                portal.getMinMax(slotMin, slotMax, mn3, mx3); \
                m_query->addQueryDebugAABB(mn3, mx3, color);  \
            } \
        } while(0)
    #else
    #define UMBRA_LINESEGMENT_DEBUG_CODE(X)
    #define UMBRA_LINESEGMENT_DEBUG_PORTAL(color)
    #endif

    Vector4i        UMBRA_ATTRIBUTE_ALIGNED(16, mn);
    Vector4i        UMBRA_ATTRIBUTE_ALIGNED(16, mx);
    Vector3         UMBRA_ATTRIBUTE_ALIGNED(16, mn3);
    Vector3         UMBRA_ATTRIBUTE_ALIGNED(16, mx3);
    SIMDRegister    portalMinSIMD = SIMDZero();
    SIMDRegister    portalMaxSIMD = SIMDZero();

    bool            result = false;

    // Traverse loop

    while (m_stackEnd != m_stackStart)
    {
        //-----------------------------------------------------
        // Get next cell from stack
        //-----------------------------------------------------

        UINT32 stackPos = (m_stackEnd - 1) & (g_stackCapacity - 1);
        m_stackEnd = stackPos;

        // Get node from top of stack
        PortalNode currentNode = m_stack[stackPos].m_node;

        UMBRA_LINESEGMENT_DEBUG_CODE(
            if (step == lineSegmentDebugStep + 1)
            {
                lineSegmentDebugStats[0] = currentNode.slot;
                lineSegmentDebugStats[1] = currentNode.local;
            }
        );

        if (currentNode.slot != currentSlot)
        {
            m_query->unmapTile(mappedTile);
            m_query->mapTile(mappedTile, currentNode.slot);
            tile = mappedTile.getTile();
            m_cellNodeMap.setArray(tile->getCellNodes());
            m_extCellNodeMap.setArray(mappedTile.getExtCellNodes());
            
            slotMin = tile->getTreeMin();
            slotMax = tile->getTreeMax();
            m_slotMinSIMD = SIMDLoadW1(slotMin);
            m_slotMaxSIMD = SIMDLoadW1(slotMax);

            m_portalExpand     = tile->getPortalExpand();
            m_portalExpandSIMD = SIMDLoadW0(Vector3(m_portalExpand, m_portalExpand, m_portalExpand));

            // Figure out scale for this tile
            m_localScale      = SIMDMultiply(scale, SIMDReciprocalAccurate(SIMDSub(m_slotMaxSIMD, m_slotMinSIMD)));

            // Scale start end end coordinate to tile local space
            m_startSIMDLocal  = SIMDMultiply(SIMDSub(m_startSIMD, m_slotMinSIMD), m_localScale);
            m_endSIMDLocal    = SIMDMultiply(SIMDSub(m_endSIMD,   m_slotMinSIMD), m_localScale);

            // Compute new invdir for this tile
            SIMDRegister diff = SIMDSub(m_endSIMDLocal, m_startSIMDLocal);
            SIMDRegister mask = SIMDCompareEQ(diff, SIMDZero());
            m_invdir          = SIMDReciprocalAccurate(SIMDSelect(diff, SIMDOne(), mask));
            
            currentSlot = currentNode.slot;
        }

        m_cellNodeMap.prefetch(currentNode.local);

        setBit(m_visitedCells, currentNode.global);

        // update node history, keeping a history avoids
        // memsets of the full array for batched queries
        m_history[m_historyPos] = currentNode.global;
        m_historyPos = (m_historyPos + 1) & (g_historySize - 1);
        if (m_historyPos == m_historyStart)
        {
            // clear old entries if array wraps
            clearBit(m_visitedCells, m_history[m_historyPos]);
            m_historyStart = (m_historyStart + 1) & (g_historySize - 1);
        }
        
        m_cellNodeMap.get(currentCell, currentNode.local);
        if (mappedTile.hasExternalPortals())
            m_extCellNodeMap.get(currentExtCell, currentNode.local);
        portalIter.init(mappedTile, currentCell, &currentExtCell);

        UMBRA_LINESEGMENT_DEBUG_CODE(lineSegmentDebugStats[LSS_CELLS]++);

        //-----------------------------------------------------
        // Find object candidates for intersection
        //-----------------------------------------------------

        if (findObjects && objects)
        {
            m_objectIter.setArray(mappedTile.getMappedTome().getTome()->getObjectLists(),
                mappedTile.getMappedTome().getTome()->getObjectListElemWidth(),
                mappedTile.getMappedTome().getTome()->getObjectListCountWidth(),
                currentCell.getObjectIndex(), currentCell.getObjectCount());

            while (!objects->isMaxed() && m_objectIter.hasMore())
            {
                int object =  mappedTile.getMappedTome().mapLocalObject(m_objectIter.next());

                ObjectBounds bound;
                objBounds.get(bound, object);

                SIMDRegister obMin = SIMDMultiply(SIMDSub(SIMDLoadW1(bound.mn), m_slotMinSIMD), m_localScale);
                SIMDRegister obMax = SIMDMultiply(SIMDSub(SIMDLoadW1(bound.mx), m_slotMinSIMD), m_localScale);

                // Intersect ray with object, test/report each object only once
                if (testAndSetBit(m_foundObjects, object) ||
                    !intersectRayAABB(obMin, obMax))
                    continue;

                objects->pushBack(object);
            }
        }

        int numPortals = 0;

        //ArrayIterator<Portal>& portalIter = m_portalIter;
        int count = portalIter.hasMore();

        //while (m_portalIter.hasMore())
        for (int p = 0; p < count; p++)
        {
            const Portal& portal = portalIter.next();
            numPortals++;

            if (portal.isHierarchy())
                continue;

            if (UMBRA_EXPECT(portal.isUser(), 0))
            {
                //-----------------------------------------------------
                // Reject closed user portals
                //-----------------------------------------------------

                if (!m_query->isGateOpen(mappedTile, portal))
                    continue;

                //-----------------------------------------------------
                // Get portal AABB
                //-----------------------------------------------------

                mappedTile.getMappedTome().getTome()->getGateBounds(portal, Vector3(m_portalExpand, m_portalExpand, m_portalExpand), mn3, mx3);
                portalMinSIMD = SIMDMultiply(SIMDSub(SIMDLoadW0(mn3), m_slotMinSIMD), m_localScale);
                portalMaxSIMD = SIMDMultiply(SIMDSub(SIMDLoadW0(mx3), m_slotMinSIMD), m_localScale);
            }
            else if (UMBRA_EXPECT(portal.isOutside(), 0))
            {
                //-----------------------------------------------------
                // Reject exit portals, for now
                //-----------------------------------------------------

                continue;
            }
            else
            {
                //-----------------------------------------------------
                // Reject backfacing portals
                //-----------------------------------------------------

                #if UMBRA_OS == UMBRA_XBOX360                
                if (!faces[portal.getFace()]) // avoid variable bitshifts
                #else
                if (!((1 << portal.getFace()) & portalFaceMask))
                #endif
                    continue;

                //-----------------------------------------------------
                // Get portal AABB
                //-----------------------------------------------------

                portal.getIntMinMax(mn, mx);
            }

            // create PortalNode

            PortalNode target;
            target.slot  = portal.getTargetTileIdx();
            target.local = portal.getTargetIndex();
            if (!portalIter.isExternal())
                target.slot = mappedTile.getMappedTome().mapLocalTile(target.slot);

            m_cellStartMap.get(target.global, target.slot);
            target.global += target.local;

            // ignore visited cells

            if (testBit(m_visitedCells, target.global))
                continue;

            UMBRA_LINESEGMENT_DEBUG_CODE(
                lineSegmentDebugStats[LSS_PORTALS]++;
                if (step == lineSegmentDebugStep)
                    lineSegmentDebugStats[LSS_STEP_PORTALS]++;
            );
            
            if (UMBRA_EXPECT(!portal.isUser(), 1))
            {
                portalMinSIMD = SIMDIntToFloat(SIMDLoadAligned32((int*)&mn));
                portalMaxSIMD = SIMDIntToFloat(SIMDLoadAligned32((int*)&mx));
                
                SIMDRegister locPortalExpand = SIMDMultiply(m_portalExpandSIMD, m_localScale);
                portalMinSIMD = SIMDSub(portalMinSIMD, locPortalExpand);
                portalMaxSIMD = SIMDAdd(portalMaxSIMD, locPortalExpand);
            }

            //-----------------------------------------------------
            // Intersect portal with ray
            //-----------------------------------------------------

            if (!intersectRayAABB(portalMinSIMD, portalMaxSIMD))
            {
                UMBRA_LINESEGMENT_DEBUG_PORTAL(Vector4(1.0f, 0.0f, 0.0f, 1.0f));
                continue;
            }

            // entered portal
            UMBRA_LINESEGMENT_DEBUG_PORTAL(Vector4(0.0f, 1.0f, 0.0f, 1.0f));

            //-----------------------------------------------------
            // Push to stack
            //-----------------------------------------------------

            m_stack[m_stackEnd].m_node = target;
            #if UMBRA_LINESEGMENT_CALCULATE_MINMAX
            m_stack[m_stackEnd].m_enterMin = mn.xyz();
            m_stack[m_stackEnd].m_enterMax = mx.xyz();
            #endif

            m_stackEnd = (m_stackEnd + 1) & (g_stackCapacity - 1);
            if (m_stackEnd == m_stackStart)
                m_stackStart = (m_stackStart + 1) & (g_stackCapacity - 1);
        }

        // If we didn't open any new portals
        //if (m_stackEnd == stackPos)
        {
            // this is a cell where traverse ends
            // (we might continue from an earlier portal though)

            //-----------------------------------------------------
            // End query succesfully if end point inside this cell
            //-----------------------------------------------------

            #if UMBRA_LINESEGMENT_ALWAYS_FIND_END
            if (currentNode.slot == endCell.slotIdx &&
                currentNode.local == endCell.index)
            {
                result = true;
                break;
            }
            #else
            CellNode cn;
            tile->getCellNodes().getElem(cn, currentNode.local);
            const PackedAABB& aabb = cn.getBounds();

            SIMDRegister32 mnSIMDInt = SIMDLoad32(aabb.getMnx(), aabb.getMny(), aabb.getMnz(), 0);
            SIMDRegister32 mxSIMDInt = SIMDLoad32(aabb.getMxx(), aabb.getMxy(), aabb.getMxz(), 0);
            SIMDRegister32 end       = SIMDFloatToInt(m_endSIMDLocal);

            if (!SIMDCompareGTTestAny32(mnSIMDInt, end) && !SIMDCompareGTTestAny32(end, mxSIMDInt))
            {
                if (!endCell.valid())
                    endCell = m_query->findCell(m_end);

                if (currentNode.slot == endCell.slotIdx &&
                    currentNode.local == endCell.index)
                {
                    result = true;
                    break;
                }
            }
            #endif

            #if UMBRA_LINESEGMENT_CALCULATE_MINMAX

            SIMDRegister tMin, tMax;

            mn = Vector4(m_stack[stackPos].m_enterMin, 1.f);
            mx = Vector4(m_stack[stackPos].m_enterMax, 1.f);

            // Calculate intersection point with latest portal
            findHitPoints(mn, mx, tMin, tMax);

            // range min is maximum of these minimums
            m_hitMin = SIMDMax(m_hitMin, tMin);

            // Similiarly, figure out max range value from cell

            Vector3 cellMin, cellMax;
            traversal.cellBounds(cellMin, cellMax, currentNode);

            //m_query->addQueryDebugAABB(cellMin, cellMax, Vector4(1,1,1,1));

            mn = Vector4(cellMin, 1.f);
            mx = Vector4(cellMax, 1.f);

            // Calculate intersection point with current cell
            findHitPoints(mn, mx, tMin, tMax);

            m_hitMin = SIMDMax(m_hitMin, tMin);
            m_hitMax = SIMDMax(m_hitMax, tMax);
            #endif
        }

        UMBRA_LINESEGMENT_DEBUG_CODE(step++);
    }

    // history cleanup
    UINT32 idx = m_historyStart;
    while (idx != m_historyPos)
    {
        clearBit(m_visitedCells, m_history[idx]);
        idx = (idx + 1) & (g_historySize - 1);
    }

    // cleanup
    m_query->unmapTile(mappedTile);
    return result;
}
