#pragma once
#ifndef __UMBRAPORTALRAYTRACER_H
#define __UMBRAPORTALRAYTRACER_H

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
 * \brief   Umbra runtime portal ray tracing
 *
 */

#include "umbraQueryContext.hpp"
#include "umbraPortalTraversal.hpp"

#define UMBRA_PORTAL_REFERENCE_RASTER_SIZE 64

namespace Umbra
{

class AABB;
class ImpIndexList;

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

class PortalRayTracer
{
public:
    PortalRayTracer(QueryContext* q, const Vector3& point, const ImpObjectDistanceParams* objDist, Transformer* camera = NULL);
    ~PortalRayTracer(void);

    Query::ErrorCode    execute             (VisibilityResult& result);

private:

    enum
    {
        PRT_STACK_SIZE = 256
    };

    struct StackItem
    {
        float      t;
        PortalNode node;
    };

    Query::ErrorCode    init            (PortalNode& start);
    void                initTraverse    (const PortalNode& start);
    void                trace           (void);
    bool                intersectAABB   (Vector3 mn, Vector3 mx, float& tMinOut, float& tMaxOut, const Vector3& dir);

    QueryContext*       m_query;
    Vector3             m_start;
    Transformer*        m_transformer;
    Matrix4x4           m_clipToWorld;
    UINT32*             m_visitedCells;
    VisibilityResult*   m_result;
    Vector3             m_scaleVector0;
    Vector3             m_scaleVector1;
    Vector4             UMBRA_ATTRIBUTE_ALIGNED16(start4);
    Vector4             UMBRA_ATTRIBUTE_ALIGNED16(dir4);
    Vector4             UMBRA_ATTRIBUTE_ALIGNED16(oneDivDir4);
    Vector4             UMBRA_ATTRIBUTE_ALIGNED16(oneDivDotDirDir4);
    Vector4             UMBRA_ATTRIBUTE_ALIGNED16(scaleVector);
    Vector3             dir;
    Vector3             oneDivDir;
    float               oneDivDotDirDir;
    float               maxT;
    float               m_farClipZ;
    float               m_portalExpand;
    SIMDRegister        m_distanceScaleSqr;
    float               m_minContribution;
    StackItem           m_stack[PRT_STACK_SIZE];
    int                 m_stackSize;
    ArrayMapper         m_objBounds;
    ArrayMapper         m_objDist;
};
}


#endif
