#pragma once

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2006-2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Ray tracer interface
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "umbraMemory.hpp"
#include "umbraArray.hpp"

namespace Umbra
{

class GeometryBlock;

//------------------------------------------------------------------------

class RayTracerDefs
{
public:

    enum RayTraceResult
    {
        NO_HIT          = 0,
        HIT_BACKFACE    = 1,
        HIT_FRONTFACE   = 2
    };

    struct Triangle
    {
        Triangle(void)
        :   UserData    (0)
        {
        }

        Triangle(const Vector3i& InVertex, UINT32 InUserData = 0)
        :   Vertex      (InVertex)
        ,   UserData    (InUserData)
        {
        }

        Vector3i    Vertex;
        UINT32      UserData;
    };
};

//------------------------------------------------------------------------

class RayTracer : public RayTracerDefs
{
public:

                        RayTracer           (const PlatformServices& services);
                        ~RayTracer          (void);
                        RayTracer           (const RayTracer&);
    RayTracer&          operator=           (const RayTracer&);

    void                buildBVH            (const GeometryBlock& tg);
    void                buildBVH            (const Vector3* vertices, const Triangle* triangles, int numVertices, int numTriangles);

private:

    friend class RayTracerTraversal;
    class ImpRayTracer* m_imp;
};

//------------------------------------------------------------------------

class RayTracerTraversal : public RayTracerDefs
{
public:

                        RayTracerTraversal  (void);
                        RayTracerTraversal  (const RayTracer& tracer);
                        ~RayTracerTraversal (void);

    void                init                (const RayTracer& tracer);
    RayTraceResult      rayTrace            (const Vector3& origin, const Vector3& dir, float maxDist, float& dist, Vector3* vert) const;
    bool                rayCastFirst        (const Vector3& origin, const Vector3& dir, Triangle& outTriangle) const;

private:

    class ImpRayTracerTraversal* m_imp;
};

} // namespace Umbra
