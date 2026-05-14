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
 * \brief   Some intersection routines (move these later to base lib)
 *
 */

#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraIntersectExact.hpp"
#include "umbraVectorT.hpp"
#include "umbraMath.hpp"
#include "umbraBitOps.hpp"
#include "umbraSIMD.hpp"
#include <standard/Predicates.hpp>
#include <standard/MigrateFromCommon.hpp>

namespace Umbra
{

/*----------------------------------------------------------------------*//*!
 * \brief
 * \todo    Docs missing
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
 * \param   box Reference to bounding box
 * \param   v   Reference to point
 * \return  6-bit mask (see enum ClipMask) where bits are set accordingly.
 *          The mask 0 means that the point is inside the box.
 * \note    The box is interpreted to be completely "inclusive", i.e.
 *          if (v.x == box.max.x), then the bit CLIP_MAX_X is _not_ set!
 * \note    The implementation here requires 6 floating point subtractions,
 *          6 memory loads and stores, 6 integer shifts, 6 ands and 5
 *          ORs.
 *//*----------------------------------------------------------------------*/

UMBRA_FORCE_INLINE unsigned int getClipMask (const AABB& box, const Vector3& v)
{
    float f0 = box.getMax().x - v.x;
    float f1 = box.getMax().y - v.y;
    float f2 = box.getMax().z - v.z;
    float f3 = v.x - box.getMin().x;
    float f4 = v.y - box.getMin().y;
    float f5 = v.z - box.getMin().z;

    unsigned int mask =
      ((((INT32)floatBitPattern(f0))>>31) & CLIP_MAX_X) |
      ((((INT32)floatBitPattern(f1))>>31) & CLIP_MAX_Y) |
      ((((INT32)floatBitPattern(f2))>>31) & CLIP_MAX_Z) |
      ((((INT32)floatBitPattern(f3))>>31) & CLIP_MIN_X) |
      ((((INT32)floatBitPattern(f4))>>31) & CLIP_MIN_Y) |
      ((((INT32)floatBitPattern(f5))>>31) & CLIP_MIN_Z);

    return mask;
}

UMBRA_FORCE_INLINE unsigned int getClipMaskSIMD (const SIMDRegister& aabbMin, const SIMDRegister& aabbMax, const Vector3& a)
{
    UMBRA_CT_ASSERT(CLIP_MAX_X == (1<<0) &&
                    CLIP_MAX_Y == (1<<1) &&
                    CLIP_MAX_Z == (1<<2) &&
                    CLIP_MIN_X == (1<<3) &&
                    CLIP_MIN_Y == (1<<4) &&
                    CLIP_MIN_Z == (1<<5));

    SIMDRegister v = SIMDLoadW1(a);

    SIMDRegister dmx = SIMDSub(aabbMax, v);
    SIMDRegister dmn = SIMDSub(v, aabbMin);

    // ignore w (i.e. AND 7), it's trash
    int mxMask = SIMDExtractSignBits(dmx) & 7;
    int mnMask = SIMDExtractSignBits(dmn) & 7;

    unsigned int retMask = mxMask | (mnMask<<3);
    return retMask;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Computes a six-bit clip mask for a point
 * \param   box Reference to bounding box
 * \param   v   Reference to point
 * \return  6-bit mask (see enum ClipMask) where bits are set accordingly.
 *          The mask 0 means that the point is inside the box.
 * \note    The box is interpreted to be completely "inclusive", i.e.
 *          if (v.x == box.max.x), then the bit CLIP_MAX_X is _not_ set!
 * \note    The implementation here requires 6 floating point subtractions,
 *          6 memory loads and stores, 6 integer shifts, 6 ands and 5
 *          ORs.
 *//*----------------------------------------------------------------------*/
template <class T>
static unsigned int getClipMask (const AABB& box, const Vector3T<T>& v)
{
    // we don't want unnecessary temporary copies
    T f[6] = {  box.getMax().x,
                box.getMax().y,
                box.getMax().z,
                v.x,
                v.y,
                v.z };
    f[0] -= v.x;
    f[1] -= v.y;
    f[2] -= v.z;
    f[3] -= box.getMin().x;
    f[4] -= box.getMin().y;
    f[5] -= box.getMin().z;

    //------------------------------------------------------------------------------------
    // DEBUG DEBUG validate!! (was originally using some bitwise tricks)
    //------------------------------------------------------------------------------------
    unsigned int mask = 0;

    if (f[0] < 0.0) mask |= CLIP_MAX_X;
    if (f[1] < 0.0) mask |= CLIP_MAX_Y;
    if (f[2] < 0.0) mask |= CLIP_MAX_Z;
    if (f[3] < 0.0) mask |= CLIP_MIN_X;
    if (f[4] < 0.0) mask |= CLIP_MIN_Y;
    if (f[5] < 0.0) mask |= CLIP_MIN_Z;

    return mask;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Computes a six-bit clip mask for a point
 * \param   box Reference to bounding box
 * \param   v   Reference to point
 * \return  6-bit mask (see enum ClipMask) where bits are set accordingly.
 *          The mask 0 means that the point is inside the box.
 * \note    The implementation here requires 6 floating point subtractions,
 *          6 memory loads and stores, 6 integer shifts, 6 ands and 5
 *          ORs.
 *//*----------------------------------------------------------------------*/

UMBRA_FORCE_INLINE unsigned int getClipMaskExclusive (const AABB& box, const Vector3& v)
{
    float f0 = v.x - box.getMax().x;
    float f1 = v.y - box.getMax().y;
    float f2 = v.z - box.getMax().z;
    float f3 = v.x - box.getMin().x;
    float f4 = v.y - box.getMin().y;
    float f5 = v.z - box.getMin().z;

    unsigned int mask =
      ((((INT32)floatBitPattern(f0))>>31)     & CLIP_MAX_X) |
      ((((INT32)floatBitPattern(f1))>>31)     & CLIP_MAX_Y) |
      ((((INT32)floatBitPattern(f2))>>31)     & CLIP_MAX_Z) |
      ((((INT32)floatBitPattern(f3))>>31)     & CLIP_MIN_X) |
      ((((INT32)floatBitPattern(f4))>>31)     & CLIP_MIN_Y) |
      ((((INT32)floatBitPattern(f5))>>31)     & CLIP_MIN_Z);

    mask ^= (CLIP_MAX_X | CLIP_MAX_Y | CLIP_MAX_Z);
    return mask;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. line segment intersection query
 * \param   box         Reference to AABB
 * \param   p1          Beginning of line segment
 * \param   p2          End of lie segment
 * \param   intercept   Resulting intersection point
 * \return  true if intersection occurs, false otherwise
 * \todo    [wili] This is not the fastest possible routine, but let's
 *          use this until we get a better one..
 * \todo [janne 200502] I'd rather write this using a for-loop on the XYZ
 *                      axes. This would make it a lot shorter. But let's wait
 *                      until we get the fastest possible routine here :).
 *                      This is not the final routine. Also, it's cut-copy-paste,
 *                      so I didn't bother changing its layout.
 * \todo [janne 200502] You have three variants of the same routine here.. Is
 *                      this syntactic sugaring or are there performance issues?
 *                      [wili] there's one inner variant - it just has
 *                      two public APIs (because the code is not inlined
 *                      externally and we are not typically interested in the
 *                      intersection point, it would be stupid to slow down
 *                      the more common case because of a non-const ref (memory
 *                      accesses rather than FP regs).
 *//*-------------------------------------------------------------------*/

UMBRA_FORCE_INLINE bool intersectAABBLineSegmentInternal (
    const AABB&     box,
    const Vector3&  p1,
    const Vector3&  p2,
    Vector3&        intercept)
{

    unsigned int outcode1 = getClipMask(box,p1);

    if (!outcode1)
    {
        intercept = p1;
        return true;
    }

    unsigned int outcode2 = getClipMask(box,p2);

    if(!outcode2)
    {
        intercept = p2;
        return true;
    }

    if (outcode1 & outcode2)
        return false;

    if( outcode1 & (CLIP_MAX_X | CLIP_MIN_X) )
    {
        intercept.x = (outcode1 & CLIP_MAX_X) ? box.getMax().x : box.getMin().x;

        UMBRA_ASSERT (p1.x != p2.x);
        float x1 = (intercept.x - p1.x) / (p2.x - p1.x);

        intercept.y = p1.y + (p2.y - p1.y) * x1;
        intercept.z = p1.z + (p2.z - p1.z) * x1;

        if (!getClipMask(box,intercept))
            return true;
    }

    if( outcode1 & (CLIP_MAX_Y | CLIP_MIN_Y) )
    {
        intercept.y = (outcode1 & CLIP_MAX_Y) ? box.getMax().y : box.getMin().y;

        UMBRA_ASSERT (p1.y != p2.y);
        float y1 = (intercept.y - p1.y) / (p2.y - p1.y);

        intercept.x = p1.x + (p2.x - p1.x) * y1;
        intercept.z = p1.z + (p2.z - p1.z) * y1;

        if (!getClipMask(box,intercept))
            return true;
    }

    if( outcode1 & (CLIP_MIN_Z | CLIP_MAX_Z) )
    {
        intercept.z = (outcode1 & CLIP_MAX_Z) ? box.getMax().z : box.getMin().z;

        UMBRA_ASSERT (p1.z != p2.z);
        float z1 = (intercept.z - p1.z) / (p2.z - p1.z);

        intercept.x = p1.x + (p2.x - p1.x) * z1;
        intercept.y = p1.y + (p2.y - p1.y) * z1;

        if (!getClipMask(box,intercept))
            return true;
    }

    return false;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. line segment intersection query
 * \param   box         Reference to AABB
 * \param   p1          Beginning of line segment
 * \param   p2          End of line segment
 * \param   intercept   Resulting intersection point
 * \return  true if intersection occurs, false otherwise
 *//*-------------------------------------------------------------------*/

bool intersectAABBLineSegment (
    const AABB&     box,
    const Vector3&  p1,
    const Vector3&  p2,
    Vector3&        result)
{
    return intersectAABBLineSegmentInternal(box,p1,p2,result);
}


namespace
{
/*----------------------------------------------------------------------*//*!
 * \brief   Performs 2D line segment vs. rectangle intersection test
 * \param   p1      Beginning of line segment
 * \param   p2      End of line segment
 * \param   box     Reference to AABB
 * \param   axis1   Index of first axis [0,2]
 * \param   axis2   Index of second axis [0,2]
 *//*----------------------------------------------------------------------*/
#if 0
template <class T>
static bool intersectLineSegment2DRectangle (
    const Vector3T<T>&  p1,
    const Vector3T<T>&  p2,
    const AABB&         box,
    int                 axis1,
    int                 axis2)
{
    Vector2T<T> v1(p1[axis1],p1[axis2]);
    Vector2T<T> v2(p2[axis1],p2[axis2]);
    Vector2T<T> b1(box.getMax()[axis1],box.getMin()[axis2]);
    Vector2T<T> b2(box.getMin()[axis1],box.getMax()[axis2]);
    if ( (v2.x-v1.x) * (v2.y-v1.y) < 0.0f)
        swap(b1.x,b2.x);
    return (Geometry::orient2d<T>(v1,v2,b1) * Geometry::orient2d<T>(v1,v2,b2)) <= 0.0;
}
#endif


/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. line segment intersection query
 * \param   box         Reference to AABB
 * \param   p1          Beginning of line segment
 * \param   p2          End of line segment
 * \return  true if intersection occurs, false otherwise
 *//*----------------------------------------------------------------------*/
template <class T>
static bool intersectAABBLineSegment (
    const AABB  &       box,
    const Vector3T<T>&  p1,
    const Vector3T<T>&  p2,
    unsigned int        outcode1,
    unsigned int        outcode2)
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

}

/*----------------------------------------------------------------------*//*!
 * \brief   Performs 2D line segment vs. rectangle intersection test
 * \param   p1      Beginning of line segment
 * \param   p2      End of line segment
 * \param   box     Reference to AABB
 * \param   axis1   Index of first axis [0,2]
 * \param   axis2   Index of second axis [0,2]
 *//*-------------------------------------------------------------------*/

UMBRA_FORCE_INLINE bool intersectLineSegment2DRectangle (
    const Vector3&  p1,
    const Vector3&  p2,
    const AABB&     box,
    int             axis1,
    int             axis2)
{
    Vector2 v1(p1[axis1],p1[axis2]);
    Vector2 v2(p2[axis1],p2[axis2]);
    Vector2 b1(box.getMax()[axis1],box.getMin()[axis2]);
    Vector2 b2(box.getMin()[axis1],box.getMax()[axis2]);
    if ( (v2.x-v1.x) * (v2.y-v1.y) < 0.0f)
        swap(b1.x,b2.x);
    return (orient2dExact(migrate(v1), migrate(v2), migrate(b1)) * orient2dExact(migrate(v1), migrate(v2), migrate(b2))) <= 0.0f;

}

UMBRA_FORCE_INLINE static float orient2d_Fast(const Vector2& a, const Vector2& b, const Vector2& c)
{
    float acx = a[0] - c[0];
    float bcx = b[0] - c[0];
    float acy = a[1] - c[1];
    float bcy = b[1] - c[1];
    float det = acx * bcy - acy * bcx;
    return (det < 0.0f) ? -1.0f : (det > 0.0f) ? 1.0f : 0.0f;
}

UMBRA_FORCE_INLINE static bool intersectLineSegment2DRectangle_Fast (
    const Vector3&  p1,
    const Vector3&  p2,
    const AABB&     box,
    int             axis1,
    int             axis2)
{
    Vector2 v1(p1[axis1],p1[axis2]);
    Vector2 v2(p2[axis1],p2[axis2]);
    Vector2 b1(box.getMax()[axis1],box.getMin()[axis2]);
    Vector2 b2(box.getMin()[axis1],box.getMax()[axis2]);
    if ( (v2.x-v1.x) * (v2.y-v1.y) < 0.0f)
        swap(b1.x,b2.x);
    return (orient2d_Fast(v1,v2,b1) * orient2d_Fast(v1,v2,b2)) <= 0.0f;

}

/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. line segment intersection query
 * \param   box         Reference to AABB
 * \param   p1          Beginning of line segment
 * \param   p2          End of line segment
 * \return  true if intersection occurs, false otherwise
 * \note    This variant is completely robust
 *//*-------------------------------------------------------------------*/

UMBRA_FORCE_INLINE bool intersectAABBLineSegment (
    const AABB&     box,
    const Vector3&  p1,
    const Vector3&  p2,
    unsigned int    outcode1,
    unsigned int    outcode2)
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

UMBRA_FORCE_INLINE static bool intersectAABBLineSegment_Fast(
    const AABB&     box,
    const Vector3&  p1,
    const Vector3&  p2,
    unsigned int    outcode1,
    unsigned int    outcode2)
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
        !intersectLineSegment2DRectangle_Fast(p1,p2,box,0,1))
        return false;

    if (outcode1 & (CLIP_MAX_X | CLIP_MIN_X | CLIP_MAX_Z | CLIP_MIN_Z) &&
        outcode2 & (CLIP_MAX_X | CLIP_MIN_X | CLIP_MAX_Z | CLIP_MIN_Z) &&
        !intersectLineSegment2DRectangle_Fast(p1,p2,box,0,2))
        return false;

    if (outcode1 & (CLIP_MAX_Y | CLIP_MIN_Y | CLIP_MAX_Z | CLIP_MIN_Z) &&
        outcode2 & (CLIP_MAX_Y | CLIP_MIN_Y | CLIP_MAX_Z | CLIP_MIN_Z) &&
        !intersectLineSegment2DRectangle_Fast(p1,p2,box,1,2))
        return false;

    return true;
}

bool intersectAABBLineSegment_Fast (
    const AABB&     box,
    const Vector3&  p1,
    const Vector3&  p2)
{
    unsigned int outcode1 = getClipMask(box,p1);
    unsigned int outcode2 = getClipMask(box,p2);
    return intersectAABBLineSegment_Fast(box,p1,p2,outcode1,outcode2);
}

/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. line segment intersection query
 * \param   box         Reference to AABB
 * \param   p1          Beginning of line segment
 * \param   p2          End of lie segment
 * \return  true if intersection occurs, false otherwise
 * \note    This version is robust (no divisions or other floating
 *          point ops other than comparisons)
 *//*-------------------------------------------------------------------*/

bool intersectAABBLineSegment (
    const AABB&     box,
    const Vector3&  p1,
    const Vector3&  p2)
{
    unsigned int outcode1 = getClipMask(box,p1);
    unsigned int outcode2 = getClipMask(box,p2);
    return intersectAABBLineSegment(box,p1,p2,outcode1,outcode2);
}

/*----------------------------------------------------------------------*//*!
 * \brief   Internal function for computing (robustly) if
 *          a line segment and a triangle intersect
 * \param   lineStart   Origin of the line segment (starting point)
 * \param   lineEnd     End of the line segment
 * \param   vert0       First vertex of the triangle
 * \param   vert1       Second vertex of the triangle
 * \param   vert2       Third vertex of the triangle
 * \return  true if intersection occurs, false otherwise
 * \note    The intersection distance is not computed (see
 *          getLineSegmentTrianglePlaneIntersectionDistance()).
 * \note    If the line segment is _exactly_ on the triangle's plane,
 *          we consider the case to be non-intersecting. For such
 *          cases a 2D line segment/triangle intersection routine
 *          is more suitable.
 * \note    If either end of the line segment touches the triangle (exactly),
 *          we consider the case to be _intersecting_.
 *//*-------------------------------------------------------------------*/

static UMBRA_FORCE_INLINE bool intersectLineSegmentTriangleInternal (
    const Vector3&  lineStart,
    const Vector3&  lineEnd,
    const Vector3&  triangleVertex0,
    const Vector3&  triangleVertex1,
    const Vector3&  triangleVertex2)
{
    // Perform triangle edges vs. line segment tests. The line segment must
    // pass on the same "side" of all triangle edges.
    float a = orient3dExact(migrate(lineStart), migrate(lineEnd), migrate(triangleVertex0), migrate(triangleVertex1));
    float b = orient3dExact(migrate(lineStart), migrate(lineEnd), migrate(triangleVertex1), migrate(triangleVertex2));

    if (!Math::sameSign(a,b))
        return false;

    float c = orient3dExact(migrate(lineStart), migrate(lineEnd), migrate(triangleVertex2), migrate(triangleVertex0));

    if (!Math::sameSign(a,c) || !Math::sameSign(b,c))
        return false;

    // Perform line segment vs. triangle plane test. The line segment end points must
    // be on different sides of the triangle's plane.
    float sd = orient3dExact(migrate(triangleVertex0),migrate(triangleVertex1),migrate(triangleVertex2),migrate(lineStart));
    float se = orient3dExact(migrate(triangleVertex0),migrate(triangleVertex1),migrate(triangleVertex2),migrate(lineEnd));

    if ((double)(sd)*(double)(se) > 0.0f || (sd == 0.0f && se == 0.0f))
        return false;

    return true;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Selects a diagonal of an AABB based on a triangle's
 *          normal and stores the resulting line segment into {mn-mx}
 * \review
 *//*-------------------------------------------------------------------*/

bool intersectDirectedLineSegmentTriangle(
    const Vector3&  lineStart,
    const Vector3&  lineEnd,
    const Vector3&  triangleVertex0,
    const Vector3&  triangleVertex1,
    const Vector3&  triangleVertex2)
{
    // Perform triangle edges vs. line segment tests. The line segment must
    // pass on the positive "side" of all triangle edges.
    float a = orient3dExact(migrate(lineStart), migrate(lineEnd), migrate(triangleVertex0), migrate(triangleVertex1));

    if (a < 0.0)
        return false;

    float b = orient3dExact(migrate(lineStart), migrate(lineEnd), migrate(triangleVertex1), migrate(triangleVertex2));

    if (b < 0.0)
        return false;

    float c = orient3dExact(migrate(lineStart), migrate(lineEnd), migrate(triangleVertex2), migrate(triangleVertex0));

    if (c < 0.0)
        return false;

    // Perform line segment vs. triangle plane test. The line segment must have
    // points on the both sides of the triangle's plane.
    float sd = orient3dExact(migrate(triangleVertex0),migrate(triangleVertex1),migrate(triangleVertex2),migrate(lineStart));
    float se = orient3dExact(migrate(triangleVertex0),migrate(triangleVertex1),migrate(triangleVertex2),migrate(lineEnd));

    if (sd*se >= 0.0f || (sd == 0.0f && se == 0.0f))
        return false;

    return true;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Selects a diagonal of an AABB based on a triangle's
 *          normal and stores the resulting line segment into {mn-mx}
 * \param   mn      Destination line segment start
 * \param   mx      Destination line segment end
 * \param   aabb    Source AABB
 * \param   tri0    First vertex of triangle
 * \param   tri1    First vertex of triangle
 * \param   tri2    First vertex of triangle
 * \return  true if intersection occurs, false otherwise
 * \note    This intersection query is done using exact predicates and
 *          should as such be completely robust.
 * \note    If the triangle lies on one of the faces of the AABB,
 *          we interpret this as intersecting.
 *//*-------------------------------------------------------------------*/

namespace
{
static UMBRA_FORCE_INLINE void getTriangleDiagonal (
    Vector3&        mn,
    Vector3&        mx,
    const AABB&     aabb,
    const Vector3&  tri0,
    const Vector3&  tri1,
    const Vector3&  tri2)
{
    mn = aabb.getMin();
    mx = aabb.getMax();

    double v1x = (double)(tri1.x)-tri0.x;           // note that we *must* use doubles here, otherwise
    double v1y = (double)(tri1.y)-tri0.y;           // the subtraction/multiplication can overflow
    double v1z = (double)(tri1.z)-tri0.z;
    double v2x = (double)(tri2.x)-tri0.x;
    double v2y = (double)(tri2.y)-tri0.y;
    double v2z = (double)(tri2.z)-tri0.z;

    if ((v1y*v2z) < (v1z*v2y))                      // note that if the values would be equal, the order would not matter..
        swap(mn.x,mx.x);

    if ((v1z*v2x) < (v1x*v2z))
        swap(mn.y,mx.y);

    if ((v1x*v2y) < (v1y*v2x))
        swap(mn.z,mx.z);
}

static void getTriangleDiagonalSIMD (
    Vector3&        mn,
    Vector3&        mx,
    const SIMDRegister& aabbMin,
    const SIMDRegister& aabbMax,
    const Vector3& tri0,
    const Vector3& tri1,
    const Vector3& tri2)
{
    SIMDRegister t0 = SIMDLoadW1(tri0);
    SIMDRegister t1 = SIMDLoadW1(tri1);
    SIMDRegister t2 = SIMDLoadW1(tri2);

    SIMDRegister v1 = SIMDSub(t1, t0);  // tr1.x-tri0.x, tri1.y-tri0.y, tri1.z-tri0.z, trash
    SIMDRegister v2 = SIMDSub(t2, t0);  // tri2.x-tri0.x, tri2.y-tri0.y, tri2.z-tri0.z, trash

    SIMDRegister v1s = SIMDShuffle<1, 2, 0, 3>(v1); // v1y, v1z, v1x, trash
    SIMDRegister v2s = SIMDShuffle<1, 2, 0, 3>(v2); // v2y, v2z, v2x, trash

    SIMDRegister a = SIMDMultiply(v1, v2s); // v1x*v2y, v1y*v2z, v1z*v2x, trash
    SIMDRegister b = SIMDMultiply(v1s, v2); // v1y*v2x, v1z*v2y, v1x*v2z, trash

    SIMDRegister cmp = SIMDCompareGT(b, a); // v1y*v2x > v1x*v2y, v1z*v2y > v1y*v2z, v1x*v2z > v1z*v2x, trash
    // shuffle back for select
    SIMDRegister selMask = SIMDShuffle<1, 2, 0, 3>(cmp);

    // This can be optimized
    SIMDRegister smn = SIMDSelect(aabbMin, aabbMax, selMask);
    SIMDRegister smx = SIMDSelect(aabbMax, aabbMin, selMask);

    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, mn4);
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, mx4);

    SIMDStoreAligned(smn, &mn4[0]);
    SIMDStoreAligned(smx, &mx4[0]);

    mn = Vector3(mn4.x, mn4.y, mn4.z);
    mx = Vector3(mx4.x, mx4.y, mx4.z);
}


}


/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. triangle intersection query
 * \param   box         Reference to AABB
 * \param   tri0        Reference to first vertex in the triangle
 * \param   tri1        Reference to second vertex in the triangle
 * \param   tri2        Reference to third vertex in the triangle
 * \return  true if intersection occurs, false otherwise
 * \note    This intersection query is done using exact predicates and
 *          should as such be completely robust.
 * \note    If the triangle lies on one of the faces of the AABB,
 *          we interpret this as intersecting.
 *//*-------------------------------------------------------------------*/

bool intersectAABBTriangle_Fast(
    const AABB&    aabb,
    const Vector3& tri0,
    const Vector3& tri1,
    const Vector3& tri2)
{
    //--------------------------------------------------------------------
    // Start out with the trivial tests (see if AABB and the triangle's
    // AABB don't overlap). Also, if any of the triangle vertices
    // are inside the AABB, we can exit immediately.
    //--------------------------------------------------------------------

    unsigned int c0 = getClipMask(aabb,tri0);
    if (!c0)                                        // vertex inside AABB
        return true;

    unsigned int c1 = getClipMask(aabb,tri1);
    if (!c1)                                        // vertex inside AABB
        return true;

    unsigned int c2 = getClipMask(aabb,tri2);
    if (!c2)                                        // vertex inside AABB
        return true;

    if (c0 & c1 & c2)                               // all vertices outside on the same side?
        return false;

    //--------------------------------------------------------------------
    // Find the main diagonal through the AABB and see if that
    // intersects the triangle -> return INTERSECT.
    //--------------------------------------------------------------------

    Vector3 mn,mx;
    getTriangleDiagonal(mn,mx,aabb,tri0,tri1,tri2);
    Vector3 n(cross(tri1-tri0, tri2-tri0));
    if (dot(mn-tri0, n) * dot(mx-tri0, n) <= 0.0f)
        return true;

    //--------------------------------------------------------------------
    // Check if any of the triangle's edges intersect the AABB (pass
    // in the clip masks computed earlier to speed up the process).
    //--------------------------------------------------------------------

    if (intersectAABBLineSegment_Fast (aabb,tri0,tri1,c0,c1) ||
        intersectAABBLineSegment_Fast (aabb,tri1,tri2,c1,c2) ||
        intersectAABBLineSegment_Fast (aabb,tri2,tri0,c2,c0))
        return true;

    return false;
}


/*----------------------------------------------------------------------*//*!
 * \brief   Performs AABB vs. triangle intersection query
 * \param   box         Reference to AABB
 * \param   tri0        Reference to first vertex in the triangle
 * \param   tri1        Reference to second vertex in the triangle
 * \param   tri2        Reference to third vertex in the triangle
 * \return  true if intersection occurs, false otherwise
 * \note    This intersection query is done using exact predicates and
 *          should as such be completely robust.
 * \note    If the triangle lies on one of the faces of the AABB,
 *          we interpret this as intersecting.
 *//*-------------------------------------------------------------------*/

bool intersectAABBTriangle (
    const AABB&    aabb,
    const Vector3& tri0,
    const Vector3& tri1,
    const Vector3& tri2)
{
    //--------------------------------------------------------------------
    // Start out with the trivial tests (see if AABB and the triangle's
    // AABB don't overlap). Also, if any of the triangle vertices
    // are inside the AABB, we can exit immediately.
    //--------------------------------------------------------------------

    unsigned int c0 = getClipMask(aabb,tri0);
    if (!c0)                                        // vertex inside AABB
        return true;

    unsigned int c1 = getClipMask(aabb,tri1);
    if (!c1)                                        // vertex inside AABB
        return true;

    unsigned int c2 = getClipMask(aabb,tri2);
    if (!c2)                                        // vertex inside AABB
        return true;

    if (c0 & c1 & c2)                               // all vertices outside on the same side?
        return false;

    //--------------------------------------------------------------------
    // Find the main diagonal through the AABB and see if that
    // intersects the triangle -> return INTERSECT.
    //--------------------------------------------------------------------

    Vector3 mn,mx;
    getTriangleDiagonal(mn,mx,aabb,tri0,tri1,tri2);
    if (intersectLineSegmentTriangleInternal(mn,mx,tri0,tri1,tri2))
        return true;

    //--------------------------------------------------------------------
    // Check if any of the triangle's edges intersect the AABB (pass
    // in the clip masks computed earlier to speed up the process).
    //--------------------------------------------------------------------

    if (intersectAABBLineSegment (aabb,tri0,tri1,c0,c1) ||
        intersectAABBLineSegment (aabb,tri1,tri2,c1,c2) ||
        intersectAABBLineSegment (aabb,tri2,tri0,c2,c0))
        return true;

    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMin().y,aabb.getMin().z),
        Vector3(aabb.getMax().x,aabb.getMax().y,aabb.getMax().z),
        tri0,tri1,tri2));
    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMin().y,aabb.getMax().z),
        Vector3(aabb.getMax().x,aabb.getMax().y,aabb.getMin().z),
        tri0,tri1,tri2));
    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMax().y,aabb.getMax().z),
        Vector3(aabb.getMax().x,aabb.getMin().y,aabb.getMin().z),
        tri0,tri1,tri2));

    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMax().y,aabb.getMin().z),
        Vector3(aabb.getMax().x,aabb.getMin().y,aabb.getMax().z),
        tri0,tri1,tri2));

    return false;
}

// Note, that this is still accurate. In case of TileGrid, nearly
// all triangles can be classified without doing the actual
// accurate line segment intersections, so re-writing the first
// steps halved the amount of time spent in intersectAABBTriangle.
// This could most likely be optimized a lot more, as this is
// currently a straight-forward adaptation of the non-SIMD version,
// and no real magic is involved.

bool intersectAABBTriangleSIMD (
    const SIMDRegister& aabbMin,
    const SIMDRegister& aabbMax,
    const Vector3& tri0,
    const Vector3& tri1,
    const Vector3& tri2)
{
    //--------------------------------------------------------------------
    // Start out with the trivial tests (see if AABB and the triangle's
    // AABB don't overlap). Also, if any of the triangle vertices
    // are inside the AABB, we can exit immediately.
    //--------------------------------------------------------------------

    unsigned int c0 = getClipMaskSIMD(aabbMin, aabbMax, tri0);
    if (!c0)                                        // vertex inside AABB
        return true;

    unsigned int c1 = getClipMaskSIMD(aabbMin, aabbMax, tri1);
    if (!c1)                                        // vertex inside AABB
        return true;

    unsigned int c2 = getClipMaskSIMD(aabbMin, aabbMax, tri2);
    if (!c2)                                        // vertex inside AABB
        return true;

    if (c0 & c1 & c2)                               // all vertices outside on the same side?
        return false;

    //--------------------------------------------------------------------
    // Find the main diagonal through the AABB and see if that
    // intersects the triangle -> return INTERSECT.
    //--------------------------------------------------------------------

    Vector3 mn,mx;
    getTriangleDiagonalSIMD(mn, mx, aabbMin, aabbMax, tri0, tri1, tri2);
    if (intersectLineSegmentTriangleInternal(mn,mx,tri0,tri1,tri2))
        return true;

    //--------------------------------------------------------------------
    // Check if any of the triangle's edges intersect the AABB (pass
    // in the clip masks computed earlier to speed up the process).
    //--------------------------------------------------------------------

    // \todo this is horrible
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, aabbMn);
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, aabbMx);

    SIMDStoreAligned(aabbMin, &aabbMn.x);
    SIMDStoreAligned(aabbMax, &aabbMx.x);

    AABB aabb(aabbMn.xyz(), aabbMx.xyz());

    if (intersectAABBLineSegment (aabb,tri0,tri1,c0,c1) ||
        intersectAABBLineSegment (aabb,tri1,tri2,c1,c2) ||
        intersectAABBLineSegment (aabb,tri2,tri0,c2,c0))
        return true;

    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMin().y,aabb.getMin().z),
        Vector3(aabb.getMax().x,aabb.getMax().y,aabb.getMax().z),
        tri0,tri1,tri2));
    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMin().y,aabb.getMax().z),
        Vector3(aabb.getMax().x,aabb.getMax().y,aabb.getMin().z),
        tri0,tri1,tri2));
    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMax().y,aabb.getMax().z),
        Vector3(aabb.getMax().x,aabb.getMin().y,aabb.getMin().z),
        tri0,tri1,tri2));

    UMBRA_ASSERT(!intersectLineSegmentTriangleInternal(
        Vector3(aabb.getMin().x,aabb.getMax().y,aabb.getMin().z),
        Vector3(aabb.getMax().x,aabb.getMin().y,aabb.getMax().z),
        tri0,tri1,tri2));

    return false;
}

bool intersectLineSegmentTriangle (
    const Vector3&  lineStart,
    const Vector3&  lineEnd,
    const Vector3&  triangleVertex0,
    const Vector3&  triangleVertex1,
    const Vector3&  triangleVertex2)
{
    return (intersectLineSegmentTriangleInternal(lineStart,lineEnd,triangleVertex0,triangleVertex1,triangleVertex2));
}

/*-------------------------------------------------------------------*//*!
 * \brief   Tests if a line segment and a triangle intersect in 2D.
 *
 *//*-------------------------------------------------------------------*/
template <class T> bool intersectLineSegmentTriangle2d(
    const Vector2T<T>&  lineStart,
    const Vector2T<T>&  lineEnd,
    const Vector2T<T>&  triangleVertex0,
    const Vector2T<T>&  triangleVertex1,
    const Vector2T<T>&  triangleVertex2)
{
    // Is the lineStart inside the triangle?
    if (intersectPointTriangle2d(lineStart, triangleVertex0, triangleVertex1, triangleVertex2))
        return true;

    // Is the lineEnd inside the triangle?
        if (intersectPointTriangle2d(lineEnd, triangleVertex0, triangleVertex1, triangleVertex2))
        return true;

    // Intersection occurs if the line segment intersects at least one of the triangle's edges.
    if (intersectLineSegmentLineSegment2d(lineStart, lineEnd, triangleVertex0, triangleVertex1) ||
        intersectLineSegmentLineSegment2d(lineStart, lineEnd, triangleVertex1, triangleVertex2) ||
        intersectLineSegmentLineSegment2d(lineStart, lineEnd, triangleVertex2, triangleVertex0))
        return true;

    return false;
}

UINT8 triangleOctants (const AABB& aabb, const Vector3& a, const Vector3& b, const Vector3& c)
{
    // \todo [Hannu] optimize, this is the stupidest way to do this (but should be correct)

    Vector3 p = aabb.getCenter();
    UINT32 m = 0;

    for (int i = 0; i < 8; i++)
    {
        AABB aabb2 = aabb;

        for (int axis = 0; axis < 3; axis++)
            if (i & (1 << axis))
                aabb2.setMin(axis, p[axis]);
            else
                aabb2.setMax(axis, p[axis]);

        if (intersectAABBTriangle(aabb2, a, b, c))
            m |= 1 << i;
    }

    return (UINT8)m;
}

} // namespace Umbra

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)

//------------------------------------------------------------------------
