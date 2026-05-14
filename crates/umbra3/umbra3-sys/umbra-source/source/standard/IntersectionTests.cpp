// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

// TODO: this is the old "exact" intersection test code

#include <standard/IntersectionTests.hpp>
#include <standard/Base.hpp>
#include <standard/Predicates.hpp>

namespace Umbra
{

typedef union FloatUInt_u
{
    float  f;
    uint32_t i;
} FloatUInt;

static UMBRA_INLINE uint32_t floatBitPattern(float f)
{
    FloatUInt fi;
    fi.f = f;
    return fi.i;
}

static UMBRA_INLINE int sameSign(float a, float b)
{
    return (floatBitPattern(a) & 0x80000000) == (floatBitPattern(b) & 0x80000000);
}

enum ClipMask
{
    CLIP_MAX_X   = (1<<0),
    CLIP_MAX_Y   = (1<<1),
    CLIP_MAX_Z   = (1<<2),
    CLIP_MIN_X   = (1<<3),
    CLIP_MIN_Y   = (1<<4),
    CLIP_MIN_Z   = (1<<5)
};

static UMBRA_INLINE unsigned int getClipMask (const AABoxf& box, const Vec3f& v)
{
    float f0 = box.max().x() - v.x();
    float f1 = box.max().y() - v.y();
    float f2 = box.max().z() - v.z();
    float f3 = v.x() - box.min().x();
    float f4 = v.y() - box.min().y();
    float f5 = v.z() - box.min().z();

    unsigned int mask =
      ((((int32_t)floatBitPattern(f0))>>31) & CLIP_MAX_X) |
      ((((int32_t)floatBitPattern(f1))>>31) & CLIP_MAX_Y) |
      ((((int32_t)floatBitPattern(f2))>>31) & CLIP_MAX_Z) |
      ((((int32_t)floatBitPattern(f3))>>31) & CLIP_MIN_X) |
      ((((int32_t)floatBitPattern(f4))>>31) & CLIP_MIN_Y) |
      ((((int32_t)floatBitPattern(f5))>>31) & CLIP_MIN_Z);

    return mask;
}

bool lineIntersectsTriangle(Line3f line, Tri3f tri)
{
    // Perform triangle edges vs. line segment tests. The line segment must
    // pass on the same "side" of all triangle edges.

    Vec3f o = line.origin();
    Vec3f e = line.pointAt(1.f);

    float a = orient3dExact(o, e, tri[0], tri[1]);
    float b = orient3dExact(o, e, tri[1], tri[2]);

    if (!sameSign(a, b))
        return false;

    float c = orient3dExact(o, e, tri[2], tri[0]);

    if (!sameSign(a, c) || !sameSign(b, c))
        return false;

    return true;
}


UMBRA_INLINE bool intersectLineSegmentTriangle(Vec3f lineStart, Vec3f lineEnd, Tri3f tri)
{
    // Perform triangle edges vs. line segment tests. The line segment must
    // pass on the same "side" of all triangle edges.
    float a = orient3dExact(lineStart, lineEnd, tri[0], tri[1]);
    float b = orient3dExact(lineStart, lineEnd, tri[1], tri[2]);

    if (!sameSign(a,b))
        return false;

    float c = orient3dExact(lineStart, lineEnd, tri[2], tri[0]);

    if (!sameSign(a,c) || !sameSign(b,c))
        return false;

    // Perform line segment vs. triangle plane test. The line segment end points must
    // be on different sides of the triangle's plane.
    float sd = orient3dExact(tri[0],tri[1],tri[2],lineStart);
    float se = orient3dExact(tri[0],tri[1],tri[2],lineEnd);

    if (sameSign(sd, se) || (sd == 0.0f && se == 0.0f))
        return false;

    return true;
}

static UMBRA_INLINE void getTriangleDiagonal(
    Vec3f& mn,
    Vec3f& mx,
    AABoxf aabb,
    Tri3f tri)
{
    mn = aabb.min();
    mx = aabb.max();

    double v1x = (double)(tri[1].x())-tri[0].x();           // note that we *must* use doubles here, otherwise
    double v1y = (double)(tri[1].y())-tri[0].y();           // the subtraction/multiplication can overflow
    double v1z = (double)(tri[1].z())-tri[0].z();
    double v2x = (double)(tri[2].x())-tri[0].x();
    double v2y = (double)(tri[2].y())-tri[0].y();
    double v2z = (double)(tri[2].z())-tri[0].z();

    if ((v1y*v2z) < (v1z*v2y)) // note that if the values would be equal, the order would not matter..
        swap2(mn.x(),mx.x());

    if ((v1z*v2x) < (v1x*v2z))
        swap2(mn.y(),mx.y());

    if ((v1x*v2y) < (v1y*v2x))
        swap2(mn.z(),mx.z());
}

UMBRA_INLINE bool intersectLineSegment2DRectangle(
    Vec3f  p1,
    Vec3f  p2,
    AABoxf box,
    int    axis1,
    int    axis2)
{
    Vec2f v1(p1[axis1],p1[axis2]);
    Vec2f v2(p2[axis1],p2[axis2]);
    Vec2f b1(box.max()[axis1],box.min()[axis2]);
    Vec2f b2(box.min()[axis1],box.max()[axis2]);
    if ( (v2.x()-v1.x()) * (v2.y()-v1.y()) < 0.0f)
        swap2(b1.x(),b2.x());
    return (orient2dExact(v1,v2,b1) * orient2dExact(v1,v2,b2)) <= 0.0f;
}

UMBRA_FORCE_INLINE bool intersectAABBLineSegment(
    AABoxf box,
    Vec3f  p1,
    Vec3f  p2,
    unsigned int outcode1,
    unsigned int outcode2)
{
    if (!outcode1 || !outcode2)                     // p1 is inside the box -> intersect
        return true;
    if (outcode1 & outcode2)                        // p1 and p2 are behind one of the six clip planes -> no intersection
        return false;

    //--------------------------------------------------------------------
    // Then we perform three 2D intersection tests. An intersecting line
    // segment must pass all of them.
    //--------------------------------------------------------------------

    if (outcode1 & (CLIP_MAX_X | CLIP_MIN_X | CLIP_MAX_Y | CLIP_MIN_Y) &&
        outcode2 & (CLIP_MAX_X | CLIP_MIN_X | CLIP_MAX_Y | CLIP_MIN_Y) &&
        !intersectLineSegment2DRectangle(p1,p2,box,0,1))
        return false;

    if (outcode1 & (CLIP_MAX_X | CLIP_MIN_X | CLIP_MAX_Z | CLIP_MIN_Z) &&
        outcode2 & (CLIP_MAX_X | CLIP_MIN_X | CLIP_MAX_Z | CLIP_MIN_Z) &&
        !intersectLineSegment2DRectangle(p1,p2,box,0,2))
        return false;

    if (outcode1 & (CLIP_MAX_Y | CLIP_MIN_Y | CLIP_MAX_Z | CLIP_MIN_Z) &&
        outcode2 & (CLIP_MAX_Y | CLIP_MIN_Y | CLIP_MAX_Z | CLIP_MIN_Z) &&
        !intersectLineSegment2DRectangle(p1,p2,box,1,2))
        return false;

    return true;
}

template<>
bool polygonIntersectsAABox(Tri3f tri, AABoxf box)
{
    //--------------------------------------------------------------------
    // Start out with the trivial tests (see if AABB and the triangle's
    // AABB don't overlap). Also, if any of the triangle vertices
    // are inside the AABB, we can exit immediately.
    //--------------------------------------------------------------------

    unsigned int c0 = getClipMask(box,tri[0]);
    if (!c0)                                        // vertex inside AABB
        return true;

    unsigned int c1 = getClipMask(box,tri[1]);
    if (!c1)                                        // vertex inside AABB
        return true;

    unsigned int c2 = getClipMask(box,tri[2]);
    if (!c2)                                        // vertex inside AABB
        return true;

    if (c0 & c1 & c2)                               // all vertices outside on the same side?
        return false;

    //--------------------------------------------------------------------
    // Find the main diagonal through the AABB and see if that
    // intersects the triangle -> return INTERSECT.
    //--------------------------------------------------------------------

    Vec3f mn,mx;
    getTriangleDiagonal(mn,mx,box,tri);
    if (intersectLineSegmentTriangle(mn,mx,tri))
        return true;

    //--------------------------------------------------------------------
    // Check if any of the triangle's edges intersect the AABB (pass
    // in the clip masks computed earlier to speed up the process).
    //--------------------------------------------------------------------

    if (intersectAABBLineSegment (box,tri[0],tri[1],c0,c1) ||
        intersectAABBLineSegment (box,tri[1],tri[2],c1,c2) ||
        intersectAABBLineSegment (box,tri[2],tri[0],c2,c0))
        return true;

    UMBRA_ASSERT(!intersectLineSegmentTriangle(
        Vec3f(box.min().x(),box.min().y(),box.min().z()),
        Vec3f(box.max().x(),box.max().y(),box.max().z()),
        tri));
    UMBRA_ASSERT(!intersectLineSegmentTriangle(
        Vec3f(box.min().x(),box.min().y(),box.max().z()),
        Vec3f(box.max().x(),box.max().y(),box.min().z()),
        tri));
    UMBRA_ASSERT(!intersectLineSegmentTriangle(
        Vec3f(box.min().x(),box.max().y(),box.max().z()),
        Vec3f(box.max().x(),box.min().y(),box.min().z()),
        tri));
    UMBRA_ASSERT(!intersectLineSegmentTriangle(
        Vec3f(box.min().x(),box.max().y(),box.min().z()),
        Vec3f(box.max().x(),box.min().y(),box.max().z()),
        tri));

    return false;
}

}
