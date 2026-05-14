#ifndef UMBRAAABB_HPP
#define UMBRAAABB_HPP

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
 * \brief   Umbra AABB
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "umbraPair.hpp"
#include <float.h>
#include <limits.h>

#undef min
#undef max
namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   Class for storing axis-aligned bounding boxes.
 *//*-------------------------------------------------------------------*/

class AABB
{
private:
    Vector3 m_min;                          // min XYZ coordinates
    Vector3 m_max;                          // max XYZ coordinates
public:

    enum Corner                             // corner enumeration
    {
        MINX_MINY_MINZ = 0,
        MAXX_MINY_MINZ = 1,
        MINX_MAXY_MINZ = 2,
        MAXX_MAXY_MINZ = 3,
        MINX_MINY_MAXZ = 4,
        MAXX_MINY_MAXZ = 5,
        MINX_MAXY_MAXZ = 6,
        MAXX_MAXY_MAXZ = 7
    };

    enum Empty
    {
        NO_INIT
    };

    enum Side
    {
        SIDE_NEGATIVE_X = 0,
        SIDE_NEGATIVE_Y,
        SIDE_NEGATIVE_Z,
        SIDE_POSITIVE_X,
        SIDE_POSITIVE_Y,
        SIDE_POSITIVE_Z
    };

    enum Face
    {
        FACE_NEGATIVE_X = 0,
        FACE_POSITIVE_X,
        FACE_NEGATIVE_Y,
        FACE_POSITIVE_Y,
        FACE_NEGATIVE_Z,
        FACE_POSITIVE_Z
    };

UMBRA_FORCE_INLINE                  AABB                (Empty) : m_min(Vector3::NO_INIT), m_max(Vector3::NO_INIT) {}
UMBRA_FORCE_INLINE                  AABB                (void) : m_min(FLT_MAX,FLT_MAX,FLT_MAX), m_max(-FLT_MAX,-FLT_MAX,-FLT_MAX) {}
UMBRA_FORCE_INLINE                  AABB                (const AABB& s) : m_min(s.m_min),m_max(s.m_max) {}
UMBRA_FORCE_INLINE                  AABB                (const Vector3& mn, const Vector3& mx)  { set(mn, mx); }
UMBRA_FORCE_INLINE                  AABB                (const AABB& A, const AABB& B); 
UMBRA_FORCE_INLINE  AABB&           operator=           (const AABB& s)         { m_min = s.m_min; m_max = s.m_max; return *this; }
UMBRA_FORCE_INLINE  void            set                 (const Vector3& mn, const Vector3& mx);
UMBRA_FORCE_INLINE  void            setMin              (int i, float f)        { UMBRA_ASSERT(i>=0 && i<3); m_min[i] = f; }
UMBRA_FORCE_INLINE  void            setMax              (int i, float f)        { UMBRA_ASSERT(i>=0 && i<3); m_max[i] = f; }
UMBRA_FORCE_INLINE  void            setMin              (const Vector3& mn)     { m_min = mn; }
UMBRA_FORCE_INLINE  void            setMax              (const Vector3& mx)     { m_max = mx; }
UMBRA_FORCE_INLINE  const Vector3&  getMin              (void) const            { return m_min; }
UMBRA_FORCE_INLINE  const Vector3&  getMax              (void) const            { return m_max; }
UMBRA_FORCE_INLINE  void            grow                (const AABB& s);
UMBRA_FORCE_INLINE  void            grow                (const Vector3&);
UMBRA_FORCE_INLINE  void            inflate             (const Vector3& s);
UMBRA_FORCE_INLINE  AABB            inflated            (const Vector3& s) const;
UMBRA_FORCE_INLINE  void            clamp               (const AABB& s);
UMBRA_FORCE_INLINE  float           getAxisLength       (int axis) const;
UMBRA_FORCE_INLINE  float           getVolume           (void) const;
UMBRA_FORCE_INLINE  Vector3         getCenter           (void) const;
UMBRA_FORCE_INLINE  Vector3         getDimensions       (void) const;
UMBRA_FORCE_INLINE  float           getDiagonalLength   (void) const;
UMBRA_FORCE_INLINE  float           getDiagonalLengthSqr(void) const;
UMBRA_FORCE_INLINE  float           getMinAxisLength    (void) const;
UMBRA_FORCE_INLINE  float           getMaxAxisLength    (void) const;
UMBRA_FORCE_INLINE  int             getMaxAxis          (void) const; 
UMBRA_FORCE_INLINE  int             getMinAxis          (void) const; 
UMBRA_FORCE_INLINE  int             getLongestAxis      (void) const;
UMBRA_FORCE_INLINE  bool            contains            (const Vector3&) const;
UMBRA_FORCE_INLINE  bool            contains            (const AABB& s) const;
UMBRA_FORCE_INLINE  bool            touches             (const AABB& s) const;
UMBRA_FORCE_INLINE  bool            intersects          (const AABB& s) const;
UMBRA_FORCE_INLINE  bool            intersectsWithArea  (const AABB& s) const;
UMBRA_FORCE_INLINE  bool            intersectsWithVolume(const AABB& s) const;
UMBRA_FORCE_INLINE  bool            operator==          (const AABB& s) const   { return m_min == s.m_min && m_max == s.m_max; }
UMBRA_FORCE_INLINE  bool            operator!=          (const AABB& s) const   { return m_min != s.m_min || m_max != s.m_max; }
UMBRA_FORCE_INLINE  Vector3         getCorner           (Corner corner) const;
UMBRA_FORCE_INLINE  const float*    getFloatPtr         (void) const            { return &m_min[0]; }
UMBRA_FORCE_INLINE  float*          getFloatPtr         (void)                  { return &m_min[0]; }
UMBRA_FORCE_INLINE  float           getSurfaceArea      (void) const;
UMBRA_FORCE_INLINE  bool            isOK                (void) const;
UMBRA_FORCE_INLINE  float           getDistance         (const AABB& s) const;
UMBRA_FORCE_INLINE  float           getDistanceSqr      (const AABB& s) const;
UMBRA_FORCE_INLINE  float           getDistance         (const Vector3& p) const;
UMBRA_FORCE_INLINE  float           getDistanceSqr      (const Vector3& p) const;
                    void            getPlaneEquations   (Vector4 pleqs[6]) const;   // pleqs point inwards
                    void            getSideQuad         (int side, Vector3 quad[4]) const;
                    void            flattenToSide       (int side);
                    // NOTE: face is interpreted differently than side
UMBRA_FORCE_INLINE  float           getFaceDist         (int face) const;
UMBRA_FORCE_INLINE  void            setFaceDist         (int face, float f);
UMBRA_FORCE_INLINE  Vector4         getFaceRect         (int face) const;
                    Vector4         getPlaneEq          (int face) const;
                    void            validateBounds      (void);

    UMBRA_FORCE_INLINE void splitHalf (int axis, AABB& left, AABB& right) const
    {
        float c = getCenter()[axis];
        left = *this;
        left.setMax(axis, c);
        right = *this;
        right.setMin(axis, c);
    }

    UMBRA_FORCE_INLINE bool isFlat() const
    {
        return m_min.x == m_max.x || m_min.y == m_max.y || m_min.z == m_max.z;
    }

    UMBRA_FORCE_INLINE void flattenToFace(int face)
    {
        if (face & 1)
            setMin(face>>1, getMax()[face>>1]);
        else
            setMax(face>>1, getMin()[face>>1]);
    }
};

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

UMBRA_FORCE_INLINE AABB::AABB(const AABB& A, const AABB& B) 
{
    m_min.x = min2(A.getMin().x, B.getMin().x); 
    m_min.y = min2(A.getMin().y, B.getMin().y); 
    m_min.z = min2(A.getMin().z, B.getMin().z); 
    m_max.x = max2(A.getMax().x, B.getMax().x);
    m_max.y = max2(A.getMax().y, B.getMax().y);
    m_max.z = max2(A.getMax().z, B.getMax().z);
} 

/*-------------------------------------------------------------------*//*!
 * \brief   Checks consistency of the AABB.
 * \return  True if all lower bounds are smaller or equal than upper
 *          bounds.
 *//*-------------------------------------------------------------------*/

UMBRA_FORCE_INLINE bool AABB::isOK (void) const
{
    return (m_min.x <= m_max.x && m_min.y <= m_max.y && m_min.z <= m_max.z);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Gets a vector containing the dimensions of the AABB.
 * \return  Vector containing the dimensions.
 *//*-------------------------------------------------------------------*/

UMBRA_FORCE_INLINE Vector3 AABB::getDimensions (void) const
{
    return m_max - m_min;
}

UMBRA_FORCE_INLINE float AABB::getSurfaceArea (void) const
{
    Vector3 d = getDimensions();
    return 2.0f*(d.x*d.y + d.x*d.z + d.y*d.z);
}

UMBRA_FORCE_INLINE Vector3 AABB::getCorner (Corner corner) const
{
    UMBRA_ASSERT (corner >= 0 && corner <= 7);
    return Vector3((corner&1) ? m_max.x : m_min.x, (corner&2) ? m_max.y : m_min.y,(corner&4) ? m_max.z : m_min.z);
}

UMBRA_FORCE_INLINE void AABB::set (const Vector3& mn, const Vector3& mx)
{
    setMin(mn);
    setMax(mx);
}

UMBRA_FORCE_INLINE void AABB::grow (const Vector3& s)
{
    m_min = min(m_min,s);
    m_max = max(m_max,s);

    UMBRA_ASSERT(isOK());
}


UMBRA_FORCE_INLINE void AABB::grow (const AABB& s)
{
    m_min = min(m_min,s.m_min);
    m_max = max(m_max,s.m_max);
}

UMBRA_FORCE_INLINE void AABB::clamp (const AABB& s)
{
    Vector3 temp = m_min;
    m_min = max(m_min,s.m_min);
    m_max = min(m_max,s.m_max);
    if (m_min[0] > m_max[0] || m_min[1] > m_max[1] || m_min[2] > m_max[2])
        set(temp, temp);
    UMBRA_ASSERT(isOK());
}

UMBRA_FORCE_INLINE float AABB::getDiagonalLength (void) const
{
    return (m_max - m_min).length();
}

UMBRA_FORCE_INLINE float AABB::getDiagonalLengthSqr (void) const
{
    return (m_max - m_min).lengthSqr();
}

UMBRA_FORCE_INLINE Vector3 AABB::getCenter (void) const
{
    return Vector3((m_max.x+m_min.x)*0.5f,(m_max.y+m_min.y)*0.5f,(m_max.z+m_min.z)*0.5f);
}

UMBRA_FORCE_INLINE float AABB::getVolume (void) const
{
    if (!isOK())
        return -1.f;
    return ((m_max.x-m_min.x) * (m_max.y-m_min.y) * (m_max.z - m_min.z));
}

UMBRA_FORCE_INLINE float AABB::getAxisLength (int axis) const
{
    UMBRA_ASSERT(axis>=0 && axis<=2);
    return (m_max[axis] - m_min[axis]);
}

UMBRA_FORCE_INLINE float AABB::getMinAxisLength (void) const
{
    float x = m_max[0] - m_min[0];
    float y = m_max[1] - m_min[1];
    float z = m_max[2] - m_min[2];
    return min2(min2(x,y),z);
}

UMBRA_FORCE_INLINE float AABB::getMaxAxisLength (void) const
{
    float x = m_max[0] - m_min[0];
    float y = m_max[1] - m_min[1];
    float z = m_max[2] - m_min[2];
    return max2(max2(x,y),z);
}

UMBRA_INLINE int AABB::getMaxAxis(void) const
{
	float x = m_max[0] - m_min[0];
	float y = m_max[1] - m_min[1];
	float z = m_max[2] - m_min[2];

	if (x >= y && x >= z) 
		return 0; 
	if (y >= x && y >= z)
		return 1; 

	return 2; 
} 

UMBRA_INLINE int AABB::getMinAxis(void) const 
{
	float x = m_max[0] - m_min[0];
	float y = m_max[1] - m_min[1];
	float z = m_max[2] - m_min[2];

	if (x <= y && x <= z) 
		return 0; 
	if (y <= x && y <= z)
		return 1; 

	return 2; 
} 

UMBRA_FORCE_INLINE int AABB::getLongestAxis (void) const
{
    int axis = 0;
    for (int i = 0; i < 3; i++)
    {
        if (getAxisLength(i) >= getAxisLength(axis))
            axis = i;
    }
    return axis;
}

UMBRA_FORCE_INLINE bool AABB::contains (const Vector3& v) const
{
    // DEBUG DEBUG TODO IMPROVE
    return (v.x >= m_min.x &&
            v.x <= m_max.x &&
            v.y >= m_min.y &&
            v.y <= m_max.y &&
            v.z >= m_min.z &&
            v.z <= m_max.z);
}

UMBRA_FORCE_INLINE bool AABB::contains (const AABB& s) const
{
    // DEBUG DEBUG TODO IMPROVE
    return (s.m_min.x >= m_min.x &&
            s.m_max.x <= m_max.x &&
            s.m_min.y >= m_min.y &&
            s.m_max.y <= m_max.y &&
            s.m_min.z >= m_min.z &&
            s.m_max.z <= m_max.z);
}

UMBRA_FORCE_INLINE bool AABB::touches(const AABB& s) const
{
    if (!intersects(s))
        return false;

    return m_min.x == s.m_min.x || m_max.x == s.m_max.x ||
           m_min.y == s.m_min.y || m_max.y == s.m_max.y ||
           m_min.z == s.m_min.z || m_max.z == s.m_max.z;
}

UMBRA_FORCE_INLINE void AABB::inflate(const Vector3& v)
{
    UMBRA_ASSERT(isOK());
    m_min.x -= v.x;
    m_min.y -= v.y;
    m_min.z -= v.z;
    m_max.x += v.x;
    m_max.y += v.y;
    m_max.z += v.z;
    UMBRA_ASSERT(isOK());
}

UMBRA_FORCE_INLINE AABB AABB::inflated(const Vector3& v) const
{
    AABB aabb2 = *this;
    aabb2.inflate(v);
    return aabb2;
}

UMBRA_FORCE_INLINE bool AABB::intersects(const AABB& s) const
{
    return (m_min.x <= s.m_max.x && m_min.y <= s.m_max.y && m_min.z <= s.m_max.z &&
            m_max.x >= s.m_min.x && m_max.y >= s.m_min.y && m_max.z >= s.m_min.z);
}

UMBRA_FORCE_INLINE bool AABB::intersectsWithArea(const AABB& s) const
{
    int touch = 0;
    int tarea = 0;
    for (int i=0; i < 3; i++)
    {
        if (m_min[i] <= s.m_max[i] && m_max[i] >= s.m_min[i])
            touch++;
        if (m_min[i] <  s.m_max[i] && m_max[i] >  s.m_min[i])
            tarea++;
    }

    return (touch == 3 && tarea >= 2);
}

UMBRA_FORCE_INLINE bool AABB::intersectsWithVolume(const AABB& s) const
{
    float mnx = max2(m_min.x, s.m_min.x);
    float mny = max2(m_min.y, s.m_min.y);
    float mnz = max2(m_min.z, s.m_min.z);

    float mxx = min2(m_max.x, s.m_max.x);
    float mxy = min2(m_max.y, s.m_max.y);
    float mxz = min2(m_max.z, s.m_max.z);

    return mnx < mxx && mny < mxy && mnz < mxz;
}

UMBRA_FORCE_INLINE float AABB::getDistanceSqr(const AABB& s) const
{
    float l = 0.f;

    for (int axis = 0; axis < 3; axis++)
    {
        float ll;
        if (m_max[axis] < s.m_min[axis])
            ll = s.m_min[axis] - m_max[axis];
        else if (m_min[axis] > s.m_max[axis])
            ll = m_min[axis] - s.m_max[axis];
        else
            ll = 0.f;

        l += ll * ll;
    }

    return l; 
}

UMBRA_FORCE_INLINE float AABB::getDistance(const AABB& s) const
{
    float l = 0.f;

    for (int axis = 0; axis < 3; axis++)
    {
        float ll;
        if (m_max[axis] < s.m_min[axis])
            ll = s.m_min[axis] - m_max[axis];
        else if (m_min[axis] > s.m_max[axis])
            ll = m_min[axis] - s.m_max[axis];
        else
            ll = 0.f;

        l += ll * ll;
    }

    return sqrt(l);
}

UMBRA_FORCE_INLINE float AABB::getDistanceSqr(const Vector3& p) const
{
    float l = 0.f;

    for (int axis = 0; axis < 3; axis++)
    {
        float ll;
        if (m_max[axis] < p[axis])
            ll = p[axis] - m_max[axis];
        else if (m_min[axis] > p[axis])
            ll = m_min[axis] - p[axis];
        else
            ll = 0.f;

        l += ll * ll;
    }

    return l;
}

UMBRA_FORCE_INLINE float AABB::getDistance(const Vector3& p) const
{
    return sqrt(getDistanceSqr(p));
}

UMBRA_FORCE_INLINE float AABB::getFaceDist(int face) const
{
    UMBRA_ASSERT(face >= 0 && face < 6);
    return
        (face & 1) ? m_max[face >> 1] : m_min[face >> 1];
}

UMBRA_FORCE_INLINE void AABB::setFaceDist(int face, float f)
{
    UMBRA_ASSERT(face >= 0 && face < 6);
    if (face & 1)
        m_max[face >> 1] = f;
    else
        m_min[face >> 1] = f;
}

UMBRA_FORCE_INLINE Vector4 AABB::getFaceRect(int face) const
{
    UMBRA_ASSERT(face >= 0 && face < 6);

    int axis = face >> 1;
    Vector4 r;
    r.x = m_min[(axis+1)%3];
    r.y = m_min[(axis+2)%3];
    r.z = m_max[(axis+1)%3];
    r.w = m_max[(axis+2)%3];

    return r;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

static inline void convexHullOfTwoAABBs2(const AABB& a, const AABB& b, Vector4* planes, int& np)
{
// #ifdef UMBRA_DEBUG
//     Vector3 ta = a.getCenter();
//     Vector3 tb = b.getCenter();
// #endif

    const Vector3& amin = a.getMin();
    const Vector3& amax = a.getMax();
    const Vector3& bmin = b.getMin();
    const Vector3& bmax = b.getMax();

#define PLANE(ax, ay, az, bx, by, bz, cx, cy, cz) \
    { planes[np++] = getPlaneEquation(Vector3(ax,ay,az), Vector3(bx,by,bz), Vector3(cx,cy,cz)); \
      /*
      // todo Turkka [5.12.2011] Figure out corner cases, userportalregression1 crash
      UMBRA_ASSERT(dot(planes[np-1], ta) >= 0.f); \
      UMBRA_ASSERT(dot(planes[np-1], tb) >= 0.f);*/ }

    if (amin.x < bmin.x)
    {
        if (amin.y > bmin.y)
            PLANE(amin.x, amin.y, amin.z,
                  amin.x, amin.y, amax.z,
                  bmin.x, bmin.y, bmax.z);
        if (amax.y < bmax.y)
            PLANE(amin.x, amax.y, amin.z,
                  bmin.x, bmax.y, bmax.z,
                  amin.x, amax.y, amax.z);
        if (amin.z > bmin.z)
            PLANE(amin.x, amin.y, amin.z,
                  bmin.x, bmax.y, bmin.z,
                  amin.x, amax.y, amin.z);
        if (amax.z < bmax.z)
            PLANE(amin.x, amin.y, amax.z,
                  amin.x, amax.y, amax.z,
                  bmin.x, bmax.y, bmax.z);
    }

    if (amin.y < bmin.y)
    {
        if (amin.z > bmin.z)
            PLANE(amin.x, amin.y, amin.z,
                  amax.x, amin.y, amin.z,
                  bmax.x, bmin.y, bmin.z);
        if (amax.z < bmax.z)
            PLANE(amin.x, amin.y, amax.z,
                  bmax.x, bmin.y, bmax.z,
                  amax.x, amin.y, amax.z);
    }

    if (amax.x > bmax.x)
    {
        if (amin.y > bmin.y)
            PLANE(amax.x, amin.y, amin.z,
                  bmax.x, bmin.y, bmax.z,
                  amax.x, amin.y, amax.z);
        if (amax.y < bmax.y)
            PLANE(amax.x, amax.y, amin.z,
                  amax.x, amax.y, amax.z,
                  bmax.x, bmax.y, bmax.z);
        if (amin.z > bmin.z)
            PLANE(amax.x, amin.y, amin.z,
                  amax.x, amax.y, amin.z,
                  bmax.x, bmax.y, bmin.z);
        if (amax.z < bmax.z)
            PLANE(amax.x, amax.y, amax.z,
                  amax.x, amin.y, amax.z,
                  bmax.x, bmin.y, bmax.z);
    }

    if (amax.y > bmax.y)
    {
        if (amin.z > bmin.z)
            PLANE(amin.x, amax.y, amin.z,
                  bmax.x, bmax.y, bmin.z,
                  amax.x, amax.y, amin.z);
        if (amax.z < bmax.z)
            PLANE(amin.x, amax.y, amax.z,
                  amax.x, amax.y, amax.z,
                  bmax.x, bmax.y, bmax.z);
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

static inline void convexHullOfTwoAABBs(const AABB& a, const AABB& b, Vector4* planes, int& np)
{
    UMBRA_ASSERT(a.isOK() && b.isOK());

    convexHullOfTwoAABBs2(a, b, planes, np);
    convexHullOfTwoAABBs2(b, a, planes, np);

    AABB aabb = a;
    aabb.grow(b);
    aabb.getPlaneEquations(&planes[np]);
    np += 6;

#if defined(UMBRA_DEBUG) && 0
    // todo Turkka [5.12.2011] Figure out corner cases, userportalregression1 crash
    Vector3 ta = a.getCenter();
    Vector3 tb = b.getCenter();
    for (int i = 0; i < np; i++)
    {
        UMBRA_ASSERT(dot(planes[i], ta) >= 0.f);
        UMBRA_ASSERT(dot(planes[i], tb) >= 0.f);
    }
#endif
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

static inline bool intersectAABBPlanes(const AABB& a, const Vector4* planes, int numPlanes)
{
    Vector3 m(a.getCenter());   // center of AABB
    Vector3 d(a.getMax() - m);  // half-diagonal

    for (int p = 0; p < numPlanes; p++)
    {
        const Vector4& plane = planes[p];

        float NP = d.x*fabs(plane.x)+d.y*fabs(plane.y)+d.z*fabs(plane.z);
        float MP = m.x*plane.x + m.y*plane.y + m.z*plane.z + plane.w;

        if ((MP+NP) < 0.0f)
            return false;   // behind clip plane
    }
    return true;            // AABB intersects frustum
}

/*-------------------------------------------------------------------*//*!
 * \brief   Integer axis-aligned bounding box.
 *//*-------------------------------------------------------------------*/

class AABBi
{
public:
    Vector3i m_min;                         // min XYZ coordinates
    Vector3i m_max;                         // max XYZ coordinates

    UMBRA_FORCE_INLINE                  AABBi               (void)  : m_min(INT_MAX,INT_MAX,INT_MAX), m_max(-INT_MAX-1,-INT_MAX-1,-INT_MAX-1) {}
    UMBRA_FORCE_INLINE                  AABBi               (const AABBi& s) : m_min(s.m_min),m_max(s.m_max) {}
    UMBRA_FORCE_INLINE                  AABBi               (const Vector3i& mn, const Vector3i& mx) { set(mn, mx); }
    UMBRA_FORCE_INLINE                  AABBi               (const AABB& s)  : m_min((int)s.getMin().x, (int)s.getMin().y, (int)s.getMin().z),
                                                                               m_max((int)s.getMax().x, (int)s.getMax().y, (int)s.getMax().z) {}

    UMBRA_FORCE_INLINE  void            set                 (const Vector3i& mn, const Vector3i& mx) { m_min = mn; m_max = mx; }
    UMBRA_FORCE_INLINE  void            setMin              (int i, int f)          { UMBRA_ASSERT(i>=0 && i<3); m_min[i] = f; }
    UMBRA_FORCE_INLINE  void            setMax              (int i, int f)          { UMBRA_ASSERT(i>=0 && i<3); m_max[i] = f; }
    UMBRA_FORCE_INLINE  void            setMin              (const Vector3i& mn)    { m_min = mn; }
    UMBRA_FORCE_INLINE  void            setMax              (const Vector3i& mx)    { m_max = mx; }
    UMBRA_FORCE_INLINE  const Vector3i& getMin              (void) const            { return m_min; }
    UMBRA_FORCE_INLINE  const Vector3i& getMax              (void) const            { return m_max; }

    UMBRA_FORCE_INLINE  AABBi&          operator=           (const AABBi& s)        { m_min = s.m_min; m_max = s.m_max; return *this; }
    UMBRA_FORCE_INLINE  bool            operator==          (const AABBi& s) const  { return m_min == s.m_min && m_max == s.m_max; }
    UMBRA_FORCE_INLINE  bool            operator!=          (const AABBi& s) const  { return m_min != s.m_min || m_max != s.m_max; }

    Vector3i getCenter() const
    {
        return (m_min + m_max) / 2;
    }

    Vector3i getSize() const
    {
        return m_max - m_min;
    }

    int getVolume() const
    {
        // TODO: assert that the result fits
        return getSize().i * getSize().j * getSize().k;
    }

    UMBRA_FORCE_INLINE bool intersects(const AABBi& s) const
    {
        return (m_min.i <= s.m_max.i && m_min.j <= s.m_max.j && m_min.k <= s.m_max.k &&
                m_max.i >= s.m_min.i && m_max.j >= s.m_min.j && m_max.k >= s.m_min.k);
    }

    UMBRA_FORCE_INLINE bool intersectsWithVolume(const AABBi& s) const
    {
        return (m_min.i < s.m_max.i && m_min.j < s.m_max.j && m_min.k < s.m_max.k &&
                m_max.i > s.m_min.i && m_max.j > s.m_min.j && m_max.k > s.m_min.k);
    }

    UMBRA_FORCE_INLINE bool intersectsWithArea(const AABBi& s) const
    {
        int touch = 0;
        int tarea = 0;
        for (int i=0; i < 3; i++)
        {
            if (m_min[i] <= s.m_max[i] && m_max[i] >= s.m_min[i])
                touch++;
            if (m_min[i] <  s.m_max[i] && m_max[i] >  s.m_min[i])
                tarea++;
        }

        return (touch == 3 && tarea >= 2);
    }

    UMBRA_FORCE_INLINE void clamp (const AABBi& s)
    {
        m_min = max(m_min,s.m_min);
        m_max = min(m_max,s.m_max);
    }

    UMBRA_FORCE_INLINE void grow (const AABBi& s)
    {
        m_min = min(m_min,s.m_min);
        m_max = max(m_max,s.m_max);
    }

    UMBRA_FORCE_INLINE void grow (const Vector3i& s)
    {
        m_min = min(m_min,s);
        m_max = max(m_max,s);
    }

    UMBRA_FORCE_INLINE int split (int axis, AABBi& left, AABBi& right) const
    {
        int c = getCenter()[axis];

        left = *this;
        left.setMax(axis, c);

        right = *this;
        right.setMin(axis, c);

        UMBRA_ASSERT(left.getSize()[axis] > 0);
        UMBRA_ASSERT(right.getSize()[axis] > 0);

        return axis;
    }

    UMBRA_FORCE_INLINE int split (AABBi& left, AABBi& right) const
    {
        int axis = getLongestAxis(getSize());
        return split(axis, left, right);
    }

    UMBRA_FORCE_INLINE bool isOK (void) const
    {
        return (m_min.i <= m_max.i && m_min.j <= m_max.j && m_min.k <= m_max.k);
    }

    UMBRA_FORCE_INLINE bool hasVolume (void) const
    {
        return (m_min.i < m_max.i && m_min.j < m_max.j && m_min.k < m_max.k);
    }

    UMBRA_FORCE_INLINE int getMinAxisLength (void) const
    {
        int x = m_max[0] - m_min[0];
        int y = m_max[1] - m_min[1];
        int z = m_max[2] - m_min[2];
        return min2(min2(x,y),z);
    }

    UMBRA_FORCE_INLINE int getMaxAxisLength (void) const
    {
        int x = m_max[0] - m_min[0];
        int y = m_max[1] - m_min[1];
        int z = m_max[2] - m_min[2];
        return max2(max2(x,y),z);
    }

    bool contains (const AABBi& s) const
    {
        // DEBUG DEBUG TODO IMPROVE
        return (s.m_min.i >= m_min.i &&
                s.m_max.i <= m_max.i &&
                s.m_min.j >= m_min.j &&
                s.m_max.j <= m_max.j &&
                s.m_min.k >= m_min.k &&
                s.m_max.k <= m_max.k);
    }

    AABB toFloat(float s) const
    {
        UMBRA_ASSERT(isOK());

        AABB aabb;
        aabb.setMin(0, m_min.i * s);
        aabb.setMin(1, m_min.j * s);
        aabb.setMin(2, m_min.k * s);
        aabb.setMax(0, m_max.i * s);
        aabb.setMax(1, m_max.j * s);
        aabb.setMax(2, m_max.k * s);

        UMBRA_ASSERT(aabb.isOK());

        return aabb;
    }
};

template <> inline unsigned int getHashValue (const AABB& aabb)
{
    return getHashValue(Pair<Vector3, Vector3>(aabb.getMin(), aabb.getMax()));
}

template <> inline unsigned int getHashValue (const AABBi& aabbi)
{
    return getHashValue(Pair<Vector3i, Vector3i>(aabbi.getMin(), aabbi.getMax()));
}


} // namespace Umbra

#endif // UMBRAAABB_HPP

//--------------------------------------------------------------------
