#ifndef UMBRAVECTORT_HPP
#define UMBRAVECTORT_HPP

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
 * \brief   Umbra template Vector classes
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "umbraFloat.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   Two-component vector template class
 * \todo    Check that scalar operations don't have aliasing issues
 *//*-------------------------------------------------------------------*/

template <class T> class Vector2T
{
public:
    T x, y; //lint !e1925 public data member
UMBRA_FORCE_INLINE                  Vector2T        (void)                          : x(0.0f),y(0.0f)       {}
UMBRA_FORCE_INLINE                  Vector2T        (const T& cx, const T& cy)      : x(cx),y(cy) {}
UMBRA_FORCE_INLINE  const T&        operator[]      (int i) const                   { UMBRA_ASSERT (i>=0 && i < 2); return ((const T*)this)[i]; }
UMBRA_FORCE_INLINE  T&              operator[]      (int i)                         { UMBRA_ASSERT (i>=0 && i < 2); return ((T*)this)[i]; }
UMBRA_FORCE_INLINE  Vector2T&       clear           (void)                          { x = 0.0; y = 0.0; return *this; }
UMBRA_FORCE_INLINE  Vector2T&       set             (const T& cx, const T& cy)      { x = cx;  y = cy; return *this;    }
UMBRA_FORCE_INLINE  Vector2T&       operator=       (const Vector2T& s)             { x = s.x; y = s.y; return *this; }
UMBRA_FORCE_INLINE  bool            operator==      (const Vector2T& s) const       { return (x==s.x)&&(y==s.y); }
UMBRA_FORCE_INLINE  bool            operator!=      (const Vector2T& s) const       { return (x!=s.x)||(y!=s.y); }
UMBRA_FORCE_INLINE  T               lengthSqr       (void) const                    { return (x*x+y*y); }
// Didn't compile => I removed it. -Hannu
#if 0
UMBRA_FORCE_INLINE  Vector2T&       normalize       (T len = 1.0)                   { T l = x*x+y*y; if(l!=0.0) { l = len / Math::sqrt(l); x = (T)(x*l); y = (T)(y*l); } return *this; }
#endif
UMBRA_FORCE_INLINE  Vector2T&       operator+=      (const Vector2T& v)             { x += v.x, y += v.y; return *this; }
UMBRA_FORCE_INLINE  Vector2T&       operator-=      (const Vector2T& v)             { x -= v.x, y -= v.y; return *this; }
UMBRA_FORCE_INLINE  Vector2T&       operator*=      (const T s)                     { x = (x*s),   y = (y*s); return *this; }
UMBRA_FORCE_INLINE  Vector2T&       operator/=      (const T t)                     { T s = (1.0/t); x = (x*s), y = (y*s); return *this; }

};

template <class T> UMBRA_FORCE_INLINE   Vector2T<T>     operator+       (const Vector2T<T>& v1, const Vector2T<T>& v2)  { return Vector2T<T>(v1.x+v2.x, v1.y+v2.y); }
template <class T> UMBRA_FORCE_INLINE   Vector2T<T>     operator-       (const Vector2T<T>& v1, const Vector2T<T>& v2)  { return Vector2T<T>(v1.x-v2.x, v1.y-v2.y); }
template <class T> UMBRA_FORCE_INLINE   Vector2T<T>     operator*       (const Vector2T<T>& v,  const T& s)             { return Vector2T<T>(v.x*s, v.y*s); }
template <class T> UMBRA_FORCE_INLINE   Vector2T<T>     operator*       (const T& s,            const Vector2T<T>& v)   { return Vector2T<T>(v.x*s, v.y*s); }
template <class T> UMBRA_FORCE_INLINE   Vector2T<T>     operator/       (const Vector2T<T>& v,  const T& s)             { return v*(1.0f/s); }
template <class T> UMBRA_FORCE_INLINE   Vector2T<T>     operator-       (const Vector2T<T>& v)                          { return Vector2T<T>(-v.x, -v.y); }
template <class T> UMBRA_FORCE_INLINE   T               dot             (const Vector2T<T>& v1, const Vector2T<T>& v2)  { return (v1.x*v2.x + v1.y*v2.y); }

/*-------------------------------------------------------------------*//*!
 * \brief   Three-component vector template class
 * \todo    Check that scalar operations don't have aliasing issues
 *//*-------------------------------------------------------------------*/

template <class T> class Vector3T
{
public:
    T   x,y,z; //lint !e1925 public data member

UMBRA_FORCE_INLINE                  Vector3T    (void) throw(): x(0.0f),y(0.0f),z(0.0f)     {}
UMBRA_FORCE_INLINE                  Vector3T    (const Vector3& s) throw()                  { x = s.x; y = s.y; z = s.z; }
template <class K> UMBRA_FORCE_INLINE   Vector3T(const Vector3T<K>& s) throw()              { x = s.x; y = s.y; z = s.z; }
UMBRA_FORCE_INLINE                  Vector3T    (const T& cx, const T& cy, const T& cz) throw() : x(cx),y(cy),z(cz) {}
UMBRA_FORCE_INLINE  const T&        operator[]  (int i) const                           { UMBRA_ASSERT (i>=0 && i < 3); return ((const T*)this)[i]; }
UMBRA_FORCE_INLINE  T&              operator[]  (int i)                                 { UMBRA_ASSERT (i>=0 && i < 3); return ((T*)this)[i]; }
UMBRA_FORCE_INLINE  Vector3T&       clear       (void)                                  { x = 0.0f; y = 0.0f; z = 0.0f; return *this; }
UMBRA_FORCE_INLINE  Vector3T&       set         (const T& cx, const T& cy, const T& cz) { x = cx;  y = cy;  z = cz; return *this;   }
UMBRA_FORCE_INLINE  Vector3T&       operator=   (const Vector3T& v) throw()             { x = v.x; y = v.y; z = v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3T&       operator+=  (const Vector3T& v)                     { x += v.x, y += v.y, z += v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3T&       operator-=  (const Vector3T& v)                     { x -= v.x, y -= v.y, z -= v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3T&       operator*=  (const T s)                             { x = (x*s),   y = (y*s), z = (z*s); return *this; }
UMBRA_FORCE_INLINE  Vector3T&       operator/=  (const T& t)                            { T s = (1.0f/t); x = (x*s), y = (y*s), z = (z*s); return *this; }
UMBRA_FORCE_INLINE  T               operator|=  (const Vector3T& v) const               { return (x*v.x + y*v.y + z*v.z); }
UMBRA_FORCE_INLINE  bool            operator==  (const Vector3T& v) const               { return (x == v.x && y == v.y && z == v.z);    }
UMBRA_FORCE_INLINE  bool            operator!=  (const Vector3T& v) const               { return !(x == v.x && y == v.y && z == v.z);   }
UMBRA_FORCE_INLINE  T               length      (void) const                            { return (T)Math::sqrt( x*x+y*y+z*z ); }
UMBRA_FORCE_INLINE  T               lengthSqr   (void) const                            { return (x*x+y*y+z*z); }
UMBRA_FORCE_INLINE  void            scale       (const Vector3T& v)                     { x*=v.x, y*=v.y, z*=v.z; }
UMBRA_FORCE_INLINE  Vector3T&       normalize   (T len = 1.0)                           { T l = x*x+y*y+z*z; if(l!=0.0) { l = len / Math::sqrt(l); x = (T)(x*l); y = (T)(y*l); z = (T)(z*l); } return *this; }

};

template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     min             (const Vector3T<T>& v1, const Vector3T<T>& v2)  { return Vector3T<T>(min2(v1.x,v2.x),min2(v1.y,v2.y),min2(v1.z,v2.z)); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     max             (const Vector3T<T>& v1, const Vector3T<T>& v2)  { return Vector3T<T>(max2(v1.x,v2.x),max2(v1.y,v2.y),max2(v1.z,v2.z)); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     operator+       (const Vector3T<T>& v1, const Vector3T<T>& v2)  { return Vector3T<T>(v1.x+v2.x, v1.y+v2.y, v1.z+v2.z); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     operator-       (const Vector3T<T>& v1, const Vector3T<T>& v2)  { return Vector3T<T>(v1.x-v2.x, v1.y-v2.y, v1.z-v2.z); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     operator*       (const Vector3T<T>& v,  const T& s)             { return Vector3T<T>(v.x*s, v.y*s, v.z*s); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     operator*       (const T& s,            const Vector3T<T>& v)   { return Vector3T<T>(v.x*s, v.y*s, v.z*s); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     operator/       (const Vector3T<T>& v,  const T& s)             { return v*(1.0f/s); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     operator-       (const Vector3T<T>& v)                          { return Vector3T<T>(-v.x, -v.y, -v.z); }
template <class T> UMBRA_FORCE_INLINE   Vector3T<T>     cross           (const Vector3T<T>& v1, const Vector3T<T>& v2)  { return Vector3T<T>((v1.y*v2.z)-(v1.z*v2.y), (v1.z*v2.x)-(v1.x*v2.z), (v1.x*v2.y)-(v1.y*v2.x)); }
template <class T> UMBRA_FORCE_INLINE   T               dot             (const Vector3T<T>& v1, const Vector3T<T>& v2)  { return (v1.x*v2.x + v1.y*v2.y + v1.z*v2.z); }


/*-------------------------------------------------------------------*//*!
 * \brief   Four-component vector template class
 * \todo    Check that scalar operations don't have aliasing issues
 *//*-------------------------------------------------------------------*/

template <class T> class Vector4T
{
public:
    T x,y,z,w; //lint !e1925 public data member

UMBRA_FORCE_INLINE                  Vector4T        (void)                                          : x(0.0),y(0.0),z(0.0),w(0.0)           {}
UMBRA_FORCE_INLINE                  Vector4T        (const T& cx, const T& cy, const T& cz, const T& cw)        : x(cx),y(cy),z(cz),w(cw)       {}
template <class K> UMBRA_FORCE_INLINE Vector4T      (const Vector3T<K>& s, const T& cw)                 : x(s.x),y(s.y),z(s.z),w(cw)    {}
template <class K> UMBRA_FORCE_INLINE Vector4T      (const Vector4T<K>& s)                          : x(s.x),y(s.y),z(s.z),w(s.w)   {}
UMBRA_FORCE_INLINE                  Vector4T        (const Vector4& s)                              : x(s.x),y(s.y),z(s.z),w(s.w)   {}
UMBRA_FORCE_INLINE  Vector4T&       clear           (void)                                          { x = (0); y = (0); z = (0); w = (0); return *this; }
UMBRA_FORCE_INLINE  Vector4T&       set             (const T& cx, const T& cy, const T& cz, const T& cw)        { x = (cx), y = (cy), z = (cz), w = (cw); return *this; }
UMBRA_FORCE_INLINE  const T&        operator[]      (int i) const                                   { UMBRA_ASSERT (i>=0 && i < 4); return ((const T*)this)[i]; }
UMBRA_FORCE_INLINE  T&              operator[]      (int i)                                         { UMBRA_ASSERT (i>=0 && i < 4); return ((T*)this)[i]; }
UMBRA_FORCE_INLINE  Vector4T&       operator=       (const Vector4T& v)                             { x = v.x; y = v.y; z = v.z; w = v.w; return *this; }
UMBRA_FORCE_INLINE  bool            operator==      (const Vector4T& v) const                       { return (x == v.x && y == v.y && z == v.z && w == v.w);    }
UMBRA_FORCE_INLINE  bool            operator!=      (const Vector4T& v) const                       { return !(x == v.x && y == v.y && z == v.z && w == v.w);   }
UMBRA_FORCE_INLINE  Vector4T&       operator+=      (const Vector4T& v)                             { x += v.x, y += v.y, z += v.z, w += v.w; return *this;     }
UMBRA_FORCE_INLINE  Vector4T&       operator-=      (const Vector4T& v)                             { x -= v.x, y -= v.y, z -= v.z, w -= v.w; return *this;     }
UMBRA_FORCE_INLINE  Vector4T&       operator*=      (const T s)                                     { x = (x*s), y = (y*s), z = (z*s), w = (w*s); return *this; }
UMBRA_FORCE_INLINE  Vector4T&       operator/=      (const T& t)                                    { T s = (1.0f/t); x = (x*s), y = (y*s), z = (z*s); w = (w*s); return *this; }
UMBRA_FORCE_INLINE  Vector4T&       operator*=      (const Matrix4x4& m);
UMBRA_FORCE_INLINE  T               operator|=      (const Vector4T& v) const                       { return x*v.x + y*v.y + z*v.z + w*v.w; }
UMBRA_FORCE_INLINE  T               length          (void) const                                    { return (T)Math::sqrt( x*x+y*y+z*z+w*w ); }
UMBRA_FORCE_INLINE  T               lengthSqr       (void) const                                    { return ( x*x+y*y+z*z+w*w ); }
UMBRA_FORCE_INLINE  Vector4T&       normalize       (double len = 1.0)                              { T l = length();   if(l!=0.0) *this *= ((T)(len/l)); return *this; }
UMBRA_FORCE_INLINE  Vector4T&       pseudoNormalize (void)                                          { T divisor = 0.0f; for (int i = 3; i >= 0; i--) {if ((*this)[i] != 0.0f) {divisor = (*this)[i];} } if (divisor != 0.0f) {*this /= divisor;} return *this; }
};

template <class T> UMBRA_FORCE_INLINE   Vector4T<T> operator+   (const Vector4T<T>& v1, const Vector4T<T>& v2)  { return Vector4T<T>(v1.x+v2.x, v1.y+v2.y, v1.z+v2.z, v1.w+v2.w); }
template <class T> UMBRA_FORCE_INLINE   Vector4T<T> operator-   (const Vector4T<T>& v1, const Vector4T<T>& v2)  { return Vector4T<T>(v1.x-v2.x, v1.y-v2.y, v1.z-v2.z, v1.w-v2.w); }
template <class T> UMBRA_FORCE_INLINE   Vector4T<T> operator*   (const Vector4T<T>& v,  const T& s)             { return Vector4T<T>(v.x*s, v.y*s, v.z*s, v.w*s); }
template <class T> UMBRA_FORCE_INLINE   Vector4T<T> operator*   (const T s,             const Vector4T<T>& v)   { return v*s; }
template <class T> UMBRA_FORCE_INLINE   Vector4T<T> operator/   (const Vector4T<T>& v,  const T& s)             { UMBRA_ASSERT(s!=0.0f); T r = 1.0/s; return v*r; }
template <class T> UMBRA_FORCE_INLINE   Vector4T<T> operator-   (const Vector4T<T>& v)                          { return Vector4T<T>(-v.x, -v.y, -v.z, -v.w); }
template <class T> UMBRA_FORCE_INLINE   T           dot         (const Vector4T<T>& v1, const Vector3T<T>& v2)  { return v1.x*v2.x + v1.y*v2.y + v1.z*v2.z + v1.w; }
template <class T> UMBRA_FORCE_INLINE   T           dot         (const Vector4T<T>& v1, const Vector4T<T>& v2)  { return v1.x*v2.x + v1.y*v2.y + v1.z*v2.z + v1.w*v2.w; }

typedef Vector4T<Float> Vector4F;
typedef Vector3T<Float> Vector3F;

bool    testVectorT(void);

} // namespace Umbra

#endif // UMBRAVECTORT_HPP

//--------------------------------------------------------------------
