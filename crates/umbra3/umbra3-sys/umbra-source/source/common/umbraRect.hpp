#ifndef UMBRARECT_HPP
#define UMBRARECT_HPP

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
#include <float.h>
#include <limits.h>

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   Class for storing axis-aligned bounding boxes.
 *//*-------------------------------------------------------------------*/

template <typename VectorT, typename Elem>
class RectBase
{
private:
    VectorT m_min;                          // min XYZ coordinates
    VectorT m_max;                          // max XYZ coordinates
public:

    enum Corner                             // corner enumeration
    {
        MINX_MINY = 0,
        MAXX_MINY = 1,
        MINX_MAXY = 2,
        MAXX_MAXY = 3
    };

    enum Empty
    {
        NO_INIT
    };

    UMBRA_FORCE_INLINE                  RectBase            (Empty) : m_min(VectorT::NO_INIT), m_max(VectorT::NO_INIT) {}
    UMBRA_FORCE_INLINE                  RectBase            (void);
    UMBRA_FORCE_INLINE                  RectBase            (const RectBase<VectorT, Elem>& s) : m_min(s.m_min),m_max(s.m_max) {}
    UMBRA_FORCE_INLINE                  RectBase            (const VectorT& mn, const VectorT& mx)  { set(mn, mx); }
    UMBRA_FORCE_INLINE                  RectBase            (const RectBase<VectorT, Elem>& A, const RectBase<VectorT, Elem>& B);
    UMBRA_FORCE_INLINE                  RectBase            (const Vector4& R);
    UMBRA_FORCE_INLINE  RectBase<VectorT, Elem>&  operator= (const RectBase<VectorT, Elem>& s)         { m_min = s.m_min; m_max = s.m_max; return *this; }
    UMBRA_FORCE_INLINE  void            set                 (const VectorT& mn, const VectorT& mx);
    UMBRA_FORCE_INLINE  void            setMin              (int i, Elem f)        { UMBRA_ASSERT(i>=0 && i<3); m_min[i] = f; }
    UMBRA_FORCE_INLINE  void            setMax              (int i, Elem f)        { UMBRA_ASSERT(i>=0 && i<3); m_max[i] = f; }
    UMBRA_FORCE_INLINE  void            setMin              (const VectorT& mn)     { m_min = mn; }
    UMBRA_FORCE_INLINE  void            setMax              (const VectorT& mx)     { m_max = mx; }
    UMBRA_FORCE_INLINE  const VectorT&  getMin              (void) const            { return m_min; }
    UMBRA_FORCE_INLINE  const VectorT&  getMax              (void) const            { return m_max; }
    UMBRA_FORCE_INLINE  void            grow                (const RectBase<VectorT, Elem>& s);
    UMBRA_FORCE_INLINE  void            grow                (const VectorT&);
    UMBRA_FORCE_INLINE  void            inflate             (const VectorT& s);
    UMBRA_FORCE_INLINE  void            inflate             (Elem e);
    UMBRA_FORCE_INLINE  void            translate           (const VectorT&);
    UMBRA_FORCE_INLINE  void            scale               (const VectorT&);
    UMBRA_FORCE_INLINE  void            scaleFloat          (const Vector2&);
    UMBRA_FORCE_INLINE  RectBase<VectorT, Elem> inflated            (const VectorT& s) const;
    UMBRA_FORCE_INLINE  RectBase<VectorT, Elem> inflated            (Elem e) const;
    UMBRA_FORCE_INLINE  void            clamp               (const RectBase<VectorT, Elem>& s);
    UMBRA_FORCE_INLINE  Elem            getAxisLength       (int axis) const;
    UMBRA_FORCE_INLINE  Elem            getArea             (void) const;
    UMBRA_FORCE_INLINE  VectorT         getCenter           (void) const;
    UMBRA_FORCE_INLINE  VectorT         getDimensions       (void) const;
    UMBRA_FORCE_INLINE  Elem            getDiagonalLength   (void) const;
    UMBRA_FORCE_INLINE  Elem            getDiagonalLengthSqr(void) const;
    UMBRA_FORCE_INLINE  Elem            getMinAxisLength    (void) const;
    UMBRA_FORCE_INLINE  Elem            getMaxAxisLength    (void) const;
    UMBRA_FORCE_INLINE  int             getMaxAxis          (void) const;
    UMBRA_FORCE_INLINE  int             getMinAxis          (void) const;
    UMBRA_FORCE_INLINE  int             getLongestAxis      (void) const;
    UMBRA_FORCE_INLINE  bool            contains            (const VectorT&) const;
    UMBRA_FORCE_INLINE  bool            contains            (const RectBase<VectorT, Elem>& s) const;
    UMBRA_FORCE_INLINE  bool            containsFully       (const RectBase<VectorT, Elem>& s) const;
    UMBRA_FORCE_INLINE  bool            touches             (const RectBase<VectorT, Elem>& s) const;
    UMBRA_FORCE_INLINE  bool            intersects          (const RectBase<VectorT, Elem>& s) const;
    UMBRA_FORCE_INLINE  bool            intersectsWithArea  (const RectBase<VectorT, Elem>& s) const;
    UMBRA_FORCE_INLINE  bool            operator==          (const RectBase<VectorT, Elem>& s) const   { return m_min == s.m_min && m_max == s.m_max; }
    UMBRA_FORCE_INLINE  bool            operator!=          (const RectBase<VectorT, Elem>& s) const   { return m_min != s.m_min || m_max != s.m_max; }
    UMBRA_FORCE_INLINE  VectorT         getCorner           (Corner corner) const;
    UMBRA_FORCE_INLINE  const Elem*     getFloatPtr         (void) const            { return &m_min[0]; }
    UMBRA_FORCE_INLINE  bool            isOK                (void) const;

    UMBRA_FORCE_INLINE void splitHalf (int axis, RectBase<VectorT, Elem>& left, RectBase<VectorT, Elem>& right) const
    {
        Elem c = getCenter()[axis];
        left = *this;
        left.setMax(axis, c);
        right = *this;
        right.setMin(axis, c);
    }

    UMBRA_FORCE_INLINE void split (int axis, Elem split, RectBase<VectorT, Elem>& left, RectBase<VectorT, Elem>& right) const
    {
        left = *this;
        left.setMax(axis, split);
        right = *this;
        right.setMin(axis, split);
    }
};

typedef RectBase<Vector2i, int>   Recti;

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE RectBase<VectorT, Elem>::RectBase(const RectBase<VectorT, Elem>& A, const RectBase<VectorT, Elem>& B)
{
    m_min.i = min2(A.getMin().i, B.getMin().i);
    m_min.j = min2(A.getMin().j, B.getMin().j);
    m_max.i = max2(A.getMax().i, B.getMax().i);
    m_max.j = max2(A.getMax().j, B.getMax().j);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE RectBase<VectorT, Elem>::RectBase(void) :
    m_min( INT_MAX,  INT_MAX),
    m_max(-INT_MAX, -INT_MAX)
{
}

/*-------------------------------------------------------------------*//*!
 * \brief   Creates a rect from Vector4.
 *//*-------------------------------------------------------------------*/

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE RectBase<VectorT, Elem>:: RectBase(const Vector4& R)
{
    m_min.i = (Elem)R.x;
    m_min.j = (Elem)R.y;
    m_max.i = (Elem)R.z;
    m_max.j = (Elem)R.w;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Checks consistency of the RectBase.
 * \return  True if all lower bounds are smaller or equal than upper
 *          bounds.
 *//*-------------------------------------------------------------------*/

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE bool RectBase<VectorT, Elem>::isOK (void) const
{
    return (m_min.i <= m_max.i && m_min.j <= m_max.j);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Gets a vector containing the dimensions of the RectBase.
 * \return  Vector containing the dimensions.
 *//*-------------------------------------------------------------------*/

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE VectorT RectBase<VectorT, Elem>::getDimensions (void) const
{
    return m_max - m_min;
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE VectorT RectBase<VectorT, Elem>::getCorner (Corner corner) const
{
    UMBRA_ASSERT (corner >= 0 && corner <= 7);
    return VectorT((corner&1) ? m_max.i : m_min.i, (corner&2) ? m_max.j : m_min.j);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::set (const VectorT& mn, const VectorT& mx)
{
    setMin(mn);
    setMax(mx);
    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::grow (const VectorT& s)
{
    m_min = min(m_min,s);
    m_max = max(m_max,s);

    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::grow (const RectBase<VectorT, Elem>& s)
{
    m_min = min(m_min,s.m_min);
    m_max = max(m_max,s.m_max);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::clamp (const RectBase<VectorT, Elem>& s)
{
    VectorT temp = m_min;
    m_min = max(m_min,s.m_min);
    m_max = min(m_max,s.m_max);
    if (m_min[0] > m_max[0] || m_min[1] > m_max[1])
        set(temp, temp);
    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE VectorT RectBase<VectorT, Elem>::getCenter (void) const
{
    return VectorT((m_max.i+m_min.i)*0.5f,(m_max.j+m_min.j)*0.5f);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE Elem RectBase<VectorT, Elem>::getArea (void) const
{
    if (!isOK())
        return 0.f;
    return ((m_max.i-m_min.i) * (m_max.j-m_min.j));
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE Elem RectBase<VectorT, Elem>::getAxisLength (int axis) const
{
    UMBRA_ASSERT(axis>=0 && axis<=1);
    return (m_max[axis] - m_min[axis]);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE Elem RectBase<VectorT, Elem>::getMinAxisLength (void) const
{
    Elem x = m_max[0] - m_min[0];
    Elem y = m_max[1] - m_min[1];
    return min2(x,y);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE Elem RectBase<VectorT, Elem>::getMaxAxisLength (void) const
{
    Elem x = m_max[0] - m_min[0];
    Elem y = m_max[1] - m_min[1];
    return max2(x,y);
}

template <typename VectorT, typename Elem>
UMBRA_INLINE int RectBase<VectorT, Elem>::getMaxAxis(void) const
{
    Elem x = m_max[0] - m_min[0];
    Elem y = m_max[1] - m_min[1];

    if (x >= y)
        return 0;
    return 1;
}

template <typename VectorT, typename Elem>
UMBRA_INLINE int RectBase<VectorT, Elem>::getMinAxis(void) const
{
    Elem x = m_max[0] - m_min[0];
    Elem y = m_max[1] - m_min[1];

    if (x <= y)
        return 0;
    return 1;
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE int RectBase<VectorT, Elem>::getLongestAxis (void) const
{
    return getMaxAxis();
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE bool RectBase<VectorT, Elem>::contains (const VectorT& v) const
{
    // DEBUG DEBUG TODO IMPROVE
    return (v.i >= m_min.i &&
            v.i <= m_max.i &&
            v.j >= m_min.j &&
            v.j <= m_max.j);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE bool RectBase<VectorT, Elem>::contains (const RectBase<VectorT, Elem>& s) const
{
    // DEBUG DEBUG TODO IMPROVE
    return (s.m_min.i >= m_min.i &&
            s.m_max.i <= m_max.i &&
            s.m_min.j >= m_min.j &&
            s.m_max.j <= m_max.j);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE bool RectBase<VectorT, Elem>::containsFully (const RectBase<VectorT, Elem>& s) const
{
    // DEBUG DEBUG TODO IMPROVE
    if (s.m_min.i < m_min.i ||
        s.m_max.i > m_max.i ||
        s.m_min.j < m_min.j ||
        s.m_max.j > m_max.j)
        return false;

    return true;
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE bool RectBase<VectorT, Elem>::touches(const RectBase<VectorT, Elem>& s) const
{
    if (!intersects(s))
        return false;

    return m_min.i == s.m_min.i || m_max.i == s.m_max.i ||
           m_min.j == s.m_min.j || m_max.j == s.m_max.j;
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::inflate(const VectorT& v)
{
    UMBRA_ASSERT(isOK());
    m_min.i -= v.i;
    m_min.j -= v.j;
    m_max.i += v.i;
    m_max.j += v.j;
    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::inflate(Elem e)
{
    UMBRA_ASSERT(isOK());
    m_min.i -= e;
    m_min.j -= e;
    m_max.i += e;
    m_max.j += e;
    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::translate(const VectorT& t)
{
    UMBRA_ASSERT(isOK());
    m_min.i += t.i;
    m_min.j += t.j;
    m_max.i += t.i;
    m_max.j += t.j;
    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::scale(const VectorT& t)
{
    UMBRA_ASSERT(isOK());
    m_min.i *= t.i;
    m_min.j *= t.j;
    m_max.i *= t.i;
    m_max.j *= t.j;
    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE void RectBase<VectorT, Elem>::scaleFloat(const Vector2& t)
{
    UMBRA_ASSERT(isOK());
    m_min.i = (Elem)((float)m_min.i * t.x);
    m_min.j = (Elem)((float)m_min.j * t.y);
    m_max.i = (Elem)((float)m_max.i * t.x);
    m_max.j = (Elem)((float)m_max.j * t.y);
    UMBRA_ASSERT(isOK());
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE RectBase<VectorT, Elem> RectBase<VectorT, Elem>::inflated(const VectorT& v) const
{
    RectBase<VectorT, Elem> aabb2 = *this;
    aabb2.inflate(v);
    return aabb2;
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE RectBase<VectorT, Elem> RectBase<VectorT, Elem>::inflated(Elem e) const
{
    RectBase<VectorT, Elem> aabb2 = *this;
    aabb2.inflate(e);
    return aabb2;
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE bool RectBase<VectorT, Elem>::intersects(const RectBase<VectorT, Elem>& s) const
{
    return (m_min.i <= s.m_max.i && m_min.j <= s.m_max.j &&
            m_max.i >= s.m_min.i && m_max.j >= s.m_min.j);
}

template <typename VectorT, typename Elem>
UMBRA_FORCE_INLINE bool RectBase<VectorT, Elem>::intersectsWithArea(const RectBase<VectorT, Elem>& s) const
{
    return (m_min.i < s.m_max.i && m_min.j < s.m_max.j &&
            m_max.i > s.m_min.i && m_max.j > s.m_min.j);
}

} // namespace Umbra

#endif // UMBRAAABB_HPP

//--------------------------------------------------------------------
