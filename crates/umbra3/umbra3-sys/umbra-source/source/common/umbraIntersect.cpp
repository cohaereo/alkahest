// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraIntersect.hpp"
#include <math.h>

using namespace Umbra;

#define ispositive(f) (((floatBitPattern(f)) & 0x80000000) == 0)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

enum ClipMask
{
    CLIP_MAX_X   = (1<<0),
    CLIP_MAX_Y   = (1<<1),
    CLIP_MAX_Z   = (1<<2),
    CLIP_MIN_X   = (1<<3),
    CLIP_MIN_Y   = (1<<4),
    CLIP_MIN_Z   = (1<<5)
};

/*----------------------------------------------------------------------*//*!
 * \brief   Computes a six-bit clip mask for a point
 *//*----------------------------------------------------------------------*/

UMBRA_FORCE_INLINE Umbra::UINT32 getClipMask (const AABB& box, const Vector3& v)
{
    float f0 = box.getMax().x - v.x;
    float f1 = box.getMax().y - v.y;
    float f2 = box.getMax().z - v.z;
    float f3 = v.x - box.getMin().x;
    float f4 = v.y - box.getMin().y;
    float f5 = v.z - box.getMin().z;

    Umbra::UINT32 mask =
      ((((Umbra::INT32)floatBitPattern(f0))>>31) & CLIP_MAX_X) |
      ((((Umbra::INT32)floatBitPattern(f1))>>31) & CLIP_MAX_Y) |
      ((((Umbra::INT32)floatBitPattern(f2))>>31) & CLIP_MAX_Z) |
      ((((Umbra::INT32)floatBitPattern(f3))>>31) & CLIP_MIN_X) |
      ((((Umbra::INT32)floatBitPattern(f4))>>31) & CLIP_MIN_Y) |
      ((((Umbra::INT32)floatBitPattern(f5))>>31) & CLIP_MIN_Z);

    return mask;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

static UMBRA_FORCE_INLINE LineSegment selectDiagonal(
    const AABB& aabb, const Vector3& dir)
{
    Vector3 nearv, farv;
    for (int i = 0; i < 3; i++)
    {
        if (ispositive(dir[i]))
        {
            nearv[i] = aabb.getMin()[i];
            farv[i] = aabb.getMax()[i];
        }
        else
        {
            farv[i] = aabb.getMin()[i];
            nearv[i] = aabb.getMax()[i];
        }
    }

    return LineSegment(nearv, farv);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool Umbra::intersect (const Vector4& pleq, const Triangle& tri)
{
    float d0 = dot(pleq, tri.a);
    float d1 = dot(pleq, tri.b);
    float d2 = dot(pleq, tri.c);

    if (ispositive(d0) != ispositive(d1))
        return true;
    if (ispositive(d0) != ispositive(d2))
        return true;
    return false;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool Umbra::intersect (const Vector4& plane, const AABB& box)
{
    LineSegment diagonal(selectDiagonal(box, plane.xyz()));
    float d1 = dot(plane, diagonal.a);
    float d2 = dot(plane, diagonal.b);
    return ispositive(d1) != ispositive(d2);
}


/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool Umbra::intersect (const AABB& a, const AABB& b)
{
    return a.intersects(b);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool Umbra::intersect (const AABB& box, const Sphere& sphere)
{
    float d = 0.f;
    Vector3 e1(sphere.center - box.getMin());
    Vector3 e2(sphere.center - box.getMax());

    for (int i = 0; i < 3; i++)
    {
        if (e1[i] < 0)
        {
            if (e1[i] < -sphere.radius)
                return false;
            d += e1[i] * e1[i];
        }
        else if (e2[i] > 0)
        {
            if (e2[i] > sphere.radius)
                return false;
            d += e2[i] * e2[i];
        }
    }

    if (d <= sphere.radius * sphere.radius)
        return true;

    return false;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. line segment intersection query
 *//*-------------------------------------------------------------------*/

bool Umbra::intersect (const AABB& box, const LineSegment& line)
{
#if 1
    Vector3 d = (line.b - line.a) * 0.5f;
    Vector3 e = box.getDimensions() * 0.5f;
    Vector3 c = line.a + d - box.getCenter();
    Vector3 ad = absv(d);

    if (fabsf(c[0]) > e[0] + ad[0])
        return false;
    if (fabsf(c[1]) > e[1] + ad[1])
        return false;
    if (fabsf(c[2]) > e[2] + ad[2])
        return false;
    if (fabsf(d[1] * c[2] - d[2] * c[1]) > e[1] * ad[2] + e[2] * ad[1])
        return false;
    if (fabsf(d[2] * c[0] - d[0] * c[2]) > e[2] * ad[0] + e[0] * ad[2])
        return false;
    if (fabsf(d[0] * c[1] - d[1] * c[0]) > e[0] * ad[1] + e[1] * ad[0])
        return false;

    return true;

#else
    // probably faster and easier to write in SIMD, but needs
    // rethinking for the dir component == 0 case
    Vector3 dir = line.b - line.a;
    Vector3 invDir(1.f / dir.x, 1.f / dir.y, 1.f / dir.z);
    Vector3 t0 = (box.getMin() - line.a) * invDir;
    Vector3 t1 = (box.getMax() - line.a) * invDir;
    Vector3 minT = min(t0, t1);
    Vector3 maxT = max(t0, t1);
    float mn = 0.f;
    float mx = 1.f;
    for (int i = 0; i < 3; i++)
    {
        mn = max2(mn, minT[i]);
        mx = min2(mx, maxT[i]);
    }
    return (mn <= mx);
#endif
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool Umbra::intersect (const AABB& box, const Quad& quad)
{
    //--------------------------------------------------------------------
    // Start out with the trivial tests (see if AABB and the quad's
    // AABB don't overlap). Also, if any of the quad vertices
    // are inside the AABB, we can exit immediately.
    //--------------------------------------------------------------------

    UINT32 c0 = getClipMask(box, quad.a);
    if (!c0) // vertex inside AABB
        return true;
    UINT32 c1 = getClipMask(box, quad.b);
    if (!c1) // vertex inside AABB
        return true;
    UINT32 c2 = getClipMask(box, quad.c);
    if (!c2) // vertex inside AABB
        return true;
    UINT32 c3 = getClipMask(box, quad.d);
    if (!c3) // vertex inside AABB
        return true;
    if (c0 & c1 & c2 & c3) // all vertices outside on the same side
        return false;

    //--------------------------------------------------------------------
    // Check if any of the quad's edges intersect the AABB
    //--------------------------------------------------------------------

    if (intersect(box, quad.getEdge(0)) ||
        intersect(box, quad.getEdge(1)) ||
        intersect(box, quad.getEdge(2)) ||
        intersect(box, quad.getEdge(3)))
        return true;

    //--------------------------------------------------------------------
    // Select main diagonal and see if that intersects quad
    //--------------------------------------------------------------------

    Vector3 normal(cross(quad.b - quad.a, quad.c - quad.a));
    LineSegment diagonal(selectDiagonal(box, normal));

    return intersect(diagonal, quad);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool Umbra::intersect (const Sphere& sphere, const Triangle& triangle)
{
    Vector3 A = triangle.a - sphere.center;
    Vector3 B = triangle.b - sphere.center;
    Vector3 C = triangle.c - sphere.center;
    float rr = sphere.radius*sphere.radius;

    // sphere outside of triangle plane
    Vector3 V = cross(B - A, C - A);
    float d = dot(A, V);
    float e = dot(V, V);
    if (d * d > rr * e)
        return false;

    // sphere outside of a triangle vertex
    float aa = dot(A, A);
    float ab = dot(A, B);
    float ac = dot(A, C);
    float bb = dot(B, B);
    float bc = dot(B, C);
    float cc = dot(C, C);
    if (((aa > rr) && (ab > aa) && (ac > aa)) ||
        ((bb > rr) && (ab > bb) && (bc > bb)) ||
        ((cc > rr) && (ac > cc) && (bc > cc)))
        return false;

    // sphere outside of triangle edge
    Vector3 AB = B - A;
    Vector3 BC = C - B;
    Vector3 CA = A - C;
    float d1 = ab - aa;
    float d2 = bc - bb;
    float d3 = ac - cc;
    float e1 = dot(AB, AB);
    float e2 = dot(BC, BC);
    float e3 = dot(CA, CA);
    Vector3 Q1 = A * e1 - d1 * AB;
    Vector3 Q2 = B * e2 - d2 * BC;
    Vector3 Q3 = C * e3 - d3 * CA;
    Vector3 QC = C * e1 - Q1;
    Vector3 QA = A * e2 - Q2;
    Vector3 QB = B * e3 - Q3;
    if (((dot(Q1, Q1) > rr * e1 * e1) && (dot(Q1, QC) > 0)) ||
        ((dot(Q2, Q2) > rr * e2 * e2) && (dot(Q2, QA) > 0)) ||
        ((dot(Q3, Q3) > rr * e3 * e3) && (dot(Q3, QB) > 0)))
        return false;
    return true;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

static UMBRA_INLINE float orient3d (const Vector3& a, const Vector3& b,
    const Vector3& c, const Vector3& d)
{
    Vector3 ad(a - d);
    Vector3 bd(b - d);
    Vector3 cd(c - d);

    return ad.x * (bd.y * cd.z - bd.z * cd.y)
         + bd.x * (cd.y * ad.z - cd.z * ad.y)
         + cd.x * (ad.y * bd.z - ad.z * bd.y);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool Umbra::intersect (const LineSegment& line, const Quad& quad)
{
    // line segment must have end points on different sides of plane
    float oa = orient3d(quad.a, quad.b, quad.c, line.a);
    float ob = orient3d(quad.a, quad.b, quad.c, line.b);
    if (ispositive(oa) == ispositive(ob))
        return false;

    // must pass on same side of quad edges
    bool a = ispositive(orient3d(line.a, line.b, quad.a, quad.b));
    bool b = ispositive(orient3d(line.a, line.b, quad.b, quad.c));
    bool c = ispositive(orient3d(line.a, line.b, quad.c, quad.d));
    bool d = ispositive(orient3d(line.a, line.b, quad.d, quad.a));
    if (a != b || a != c || a != d)
        return false;
    return true;

}
