#ifndef __UMBRAVECTOR_HPP
#define __UMBRAVECTOR_HPP

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
 * \brief   Umbra Vector classes
 *
 */

#include "umbraPrivateDefs.hpp"

#undef min
#undef max

//these can go out once flash switches to its own stdcxx and libc

#define _RWSTD_NO_ABS_FLT
#define _RWSTD_NO_ABS_DBL
#include <math.h>

namespace Umbra
{

class Matrix4x4;
class Matrix4x3;
class Matrix3x3;
class Matrix2x2;

/*-------------------------------------------------------------------*//*!
 * \brief   Two-component int32 vector
 *//*-------------------------------------------------------------------*/

class Vector2i
{
public:

    enum Empty
    {
        NO_INIT
    };

    int32 i,j;

UMBRA_FORCE_INLINE                  Vector2i    (Empty)                         {}
UMBRA_FORCE_INLINE                  Vector2i    (void) : i(0),j(0)              {}
UMBRA_FORCE_INLINE                  Vector2i    (const Vector2i& s)             : i(s.i),j(s.j) { }
UMBRA_FORCE_INLINE                  Vector2i    (int ci, int cj)                : i(ci),j(cj)  {}
UMBRA_FORCE_INLINE const int32&     operator[]  (int n) const                   { UMBRA_ASSERT (n>=0 && n < 2);  return (&i)[n]; }
UMBRA_FORCE_INLINE int32&           operator[]  (int n)                         { UMBRA_ASSERT (n>=0 && n < 2);  return (&i)[n]; }
UMBRA_FORCE_INLINE  Vector2i&       clear       (void)                          { i = 0; j = 0; return *this; }
UMBRA_FORCE_INLINE  Vector2i&       set         (int ci, int cj)                { i = ci;  j = cj; return *this;    }
UMBRA_FORCE_INLINE bool             operator==  (const Vector2i& v) const       { return (i == v.i && j == v.j);    }
UMBRA_FORCE_INLINE bool             operator!=  (const Vector2i& v) const       { return !(i == v.i && j == v.j);   }
UMBRA_FORCE_INLINE  Vector2i&       operator*=  (int32 s)                       { i = (i*s), j = (j*s); return *this; }
};

UMBRA_FORCE_INLINE  Vector2i        operator-           (const Vector2i& v1, const Vector2i& v2)      { return Vector2i(v1.i-v2.i, v1.j-v2.j); }
UMBRA_FORCE_INLINE  Vector2i        operator*           (const Vector2i& v,  const int32 s)           { return Vector2i(v.i*s, v.j*s); }

UMBRA_FORCE_INLINE  Vector2i        min                 (const Vector2i& v1, const Vector2i& v2)      { return Vector2i (min2(v1.i,v2.i),min2(v1.j,v2.j)); }
UMBRA_FORCE_INLINE  Vector2i        max                 (const Vector2i& v1, const Vector2i& v2)      { return Vector2i (max2(v1.i,v2.i),max2(v1.j,v2.j)); }

/*-------------------------------------------------------------------*//*!
 * \brief   Three-component int32 vector
 *//*-------------------------------------------------------------------*/

class Vector3i
{
public:
    int32 i,j,k;
UMBRA_FORCE_INLINE                  Vector3i    (void) : i(0),j(0),k(0)         {}
UMBRA_FORCE_INLINE                  Vector3i    (const Vector3i& s)             : i(s.i),j(s.j),k(s.k) {}
UMBRA_FORCE_INLINE                  Vector3i    (int ci, int cj, int ck)        : i(ci),j(cj),k(ck) {}
UMBRA_FORCE_INLINE const int32&     operator[]  (int n) const                   { UMBRA_ASSERT (n>=0 && n < 3);  return (&i)[n]; }
UMBRA_FORCE_INLINE int32&           operator[]  (int n)                         { UMBRA_ASSERT (n>=0 && n < 3);  return (&i)[n]; }
UMBRA_FORCE_INLINE  Vector3i&       clear       (void)                          { i = 0; j = 0; k = 0; return *this; }
UMBRA_FORCE_INLINE  Vector3i&       set         (int ci, int cj, int ck)        { i = ci;  j = cj;  k = ck; return *this;   }
UMBRA_FORCE_INLINE  Vector3i&       operator*=  (int s)                         { i = (i*s),   j = (j*s), k = (k*s); return *this; }
UMBRA_FORCE_INLINE Vector3i&        operator=   (const Vector3i& v)             { i = v.i; j = v.j; k = v.k;  return *this; }
UMBRA_FORCE_INLINE bool             operator==  (const Vector3i& v) const       { return (i == v.i && j == v.j && k == v.k);    }
UMBRA_FORCE_INLINE bool             operator!=  (const Vector3i& v) const       { return !(i == v.i && j == v.j && k == v.k);   }
};


UMBRA_FORCE_INLINE  Vector3i        operator+   (const Vector3i& v1,    const Vector3i& v2)     { return Vector3i(v1.i+v2.i, v1.j+v2.j, v1.k+v2.k); }
UMBRA_FORCE_INLINE  Vector3i        operator-   (const Vector3i& v1,    const Vector3i& v2)     { return Vector3i(v1.i-v2.i, v1.j-v2.j, v1.k-v2.k); }
UMBRA_FORCE_INLINE  Vector3i        operator*   (const Vector3i& v,     const int32 s)          { return Vector3i(v.i*s, v.j*s, v.k*s); }
UMBRA_FORCE_INLINE  Vector3i        operator/   (const Vector3i& v,     const int32 s)          { return Vector3i(v.i/s, v.j/s, v.k/s); }

UMBRA_FORCE_INLINE int              sum             (const Vector3i& v)                             { return v.i + v.j + v.k; }
UMBRA_FORCE_INLINE int              sum2            (const Vector3i& v)                             { return max2(v.i,0) + max2(v.j,0) + max2(v.k,0); }
UMBRA_FORCE_INLINE int              getLongestAxis  (const Vector3i& v)                             { return (v.i >= v.j) ? ((v.i >= v.k) ? 0 : 2) : ((v.j >= v.k) ? 1 : 2); }

UMBRA_FORCE_INLINE  Vector3i         min                 (const Vector3i& v1, const Vector3i& v2)      { return Vector3i (min2(v1.i,v2.i),min2(v1.j,v2.j),min2(v1.k,v2.k)); }
UMBRA_FORCE_INLINE  Vector3i         max                 (const Vector3i& v1, const Vector3i& v2)      { return Vector3i (max2(v1.i,v2.i),max2(v1.j,v2.j),max2(v1.k,v2.k)); }

/*-------------------------------------------------------------------*//*!
 * \brief   Three-component int32 vector, version that doesn't init members!
 *//*-------------------------------------------------------------------*/

class Vector3iRaw
{
public:
    int32 i,j,k;
UMBRA_FORCE_INLINE                  Vector3iRaw (void)                      {}
UMBRA_FORCE_INLINE                  Vector3iRaw (const Vector3i& s)             : i(s.i),j(s.j),k(s.k) {}
UMBRA_FORCE_INLINE                  Vector3iRaw (int ci, int cj, int ck)        : i(ci),j(cj),k(ck) {}
UMBRA_FORCE_INLINE const int32&     operator[]  (int n) const                   { UMBRA_ASSERT (n>=0 && n < 3);  return (&i)[n]; }
UMBRA_FORCE_INLINE int32&           operator[]  (int n)                         { UMBRA_ASSERT (n>=0 && n < 3);  return (&i)[n]; }
UMBRA_FORCE_INLINE  Vector3iRaw&        clear       (void)                          { i = 0; j = 0; k = 0; return *this; }
UMBRA_FORCE_INLINE  Vector3iRaw&        set         (int ci, int cj, int ck)        { i = ci;  j = cj;  k = ck; return *this;   }
UMBRA_FORCE_INLINE  Vector3iRaw&        operator*=  (int s)                         { i = (i*s),   j = (j*s), k = (k*s); return *this; }
UMBRA_FORCE_INLINE Vector3iRaw&     operator=   (const Vector3i& v)             { i = v.i; j = v.j; k = v.k;  return *this; }
UMBRA_FORCE_INLINE bool         operator==  (const Vector3i& v) const       { return (i == v.i && j == v.j && k == v.k);    }
UMBRA_FORCE_INLINE bool         operator!=  (const Vector3i& v) const       { return !(i == v.i && j == v.j && k == v.k);   }
};


UMBRA_FORCE_INLINE  Vector3iRaw     operator+   (const Vector3iRaw& v1, const Vector3i& v2)     { return Vector3iRaw(v1.i+v2.i, v1.j+v2.j, v1.k+v2.k); }
UMBRA_FORCE_INLINE  Vector3iRaw     operator-   (const Vector3iRaw& v1, const Vector3i& v2)     { return Vector3iRaw(v1.i-v2.i, v1.j-v2.j, v1.k-v2.k); }
UMBRA_FORCE_INLINE  Vector3iRaw     operator*   (const Vector3iRaw& v,      const int32 s)          { return Vector3iRaw(v.i*s, v.j*s, v.k*s); }
UMBRA_FORCE_INLINE  Vector3iRaw     operator/   (const Vector3iRaw& v,      const int32 s)          { return Vector3iRaw(v.i/s, v.j/s, v.k/s); }

/*-------------------------------------------------------------------*//*!
 * \brief   Four-component int32 vector
 *//*-------------------------------------------------------------------*/

class Vector4i
{
public:
    int32 i,j,k,l;
UMBRA_FORCE_INLINE                  Vector4i        (void)                          : i(0),j(0),k(0),l(0)   {}
UMBRA_FORCE_INLINE                  Vector4i        (const Vector4i& s)             : i(s.i),j(s.j),k(s.k),l(s.l) {}
UMBRA_FORCE_INLINE                  Vector4i        (int ci, int cj, int ck, int cl): i(ci),j(cj),k(ck),l(cl) {}
UMBRA_FORCE_INLINE                  Vector4i        (const Vector3i& s)             : i(s.i),j(s.j),k(s.k),l(0) {}
UMBRA_FORCE_INLINE const int32&     operator[]      (int n) const                   { UMBRA_ASSERT (n>=0 && n < 4);  return (&i)[n]; }
UMBRA_FORCE_INLINE int32&           operator[]      (int n)                         { UMBRA_ASSERT (n>=0 && n < 4);  return (&i)[n]; }
UMBRA_FORCE_INLINE  Vector4i&       clear           (void)                          { i = 0; j = 0; k = 0; l = 0; return *this; }
UMBRA_FORCE_INLINE  Vector4i&       set             (int ci, int cj, int ck, int cl){ i = ci;  j = cj;  k = ck; l = cl; return *this;   }
UMBRA_FORCE_INLINE bool         operator==      (const Vector4i& v) const       { return (i == v.i && j == v.j && k == v.k && l == v.l);    }
UMBRA_FORCE_INLINE bool         operator!=      (const Vector4i& v) const       { return !(i == v.i && j == v.j && k == v.k && l == v.l);   }
};

UMBRA_FORCE_INLINE  Vector4i     operator+   (const Vector4i& v1, const Vector4i& v2)     { return Vector4i(v1.i+v2.i, v1.j+v2.j, v1.k+v2.k, v1.l+v2.l); }

/*-------------------------------------------------------------------*//*!
 * \brief   Two-component vector
 *//*-------------------------------------------------------------------*/

class Vector2
{
public:
    float x,y;
UMBRA_FORCE_INLINE                  Vector2         (void)                          : x(0.0f),y(0.0f)       {}
UMBRA_FORCE_INLINE                  Vector2         (float cx, float cy)            : x(cx),y(cy) {}
UMBRA_FORCE_INLINE                  Vector2         (const Vector2i& v)             : x((float)v.i),y((float)v.j) {}
UMBRA_FORCE_INLINE  const float&    operator[]      (int i) const                   { UMBRA_ASSERT (i>=0 && i < 2); return (&x)[i]; }
UMBRA_FORCE_INLINE  float&          operator[]      (int i)                         { UMBRA_ASSERT (i>=0 && i < 2); return (&x)[i]; }
UMBRA_FORCE_INLINE  Vector2&        clear           (void)                          { x = 0.0f; y = 0.0f; return *this; }
UMBRA_FORCE_INLINE  Vector2&        set             (float cx, float cy)            { x = cx;  y = cy; return *this;    }
UMBRA_FORCE_INLINE  bool            operator==      (const Vector2& s) const        { return (x==s.x)&&(y==s.y); }
UMBRA_FORCE_INLINE  bool            operator!=      (const Vector2& s) const        { return (x!=s.x)||(y!=s.y); }
UMBRA_FORCE_INLINE  float           length          (void) const                    { return (float)sqrt(x*x+y*y); }
UMBRA_FORCE_INLINE  float           lengthSqr       (void) const                    { return (x*x+y*y); }
UMBRA_FORCE_INLINE  Vector2&        normalize       (float len = 1.0f)              { double l = x*x+y*y; if(l!=0.0) { l = len / sqrt(l); x = (float)(x*l); y = (float)(y*l); } return *this; }
UMBRA_FORCE_INLINE  void            scale           (const Vector2& v)              { x*=v.x, y*=v.y; }
UMBRA_FORCE_INLINE  Vector2&        operator+=      (const Vector2& v)              { x += v.x, y += v.y; return *this; }
UMBRA_FORCE_INLINE  Vector2&        operator-=      (const Vector2& v)              { x -= v.x, y -= v.y; return *this; }
UMBRA_FORCE_INLINE  Vector2&        operator*=      (float s)                       { x = (x*s),   y = (y*s); return *this; }
UMBRA_FORCE_INLINE  Vector2&        operator/=      (float s)                       { s = (1.0f/s); x = (x*s), y = (y*s); return *this; }
UMBRA_FORCE_INLINE  Vector2&        operator*=      (const Matrix2x2& m);
};

typedef Vector2 Vector2f;

UMBRA_FORCE_INLINE  Vector2         operator+           (const Vector2& v1, const Vector2& v2)      { return Vector2(v1.x+v2.x, v1.y+v2.y); }
UMBRA_FORCE_INLINE  Vector2         operator-           (const Vector2& v1, const Vector2& v2)      { return Vector2(v1.x-v2.x, v1.y-v2.y); }
UMBRA_FORCE_INLINE  Vector2         operator*           (const Vector2& v,  const float s)          { return Vector2(v.x*s, v.y*s); }
UMBRA_FORCE_INLINE  Vector2         operator*           (const float s,     const Vector2& v)       { return Vector2(v.x*s, v.y*s); }
UMBRA_FORCE_INLINE  Vector2         operator/           (const Vector2& v,  const float s)          { return v*(1.0f/s); }
UMBRA_FORCE_INLINE  Vector2         operator-           (const Vector2& v)                          { return Vector2(-v.x, -v.y); }
                    Vector2         operator*           (const Vector2& v,  const Matrix2x2& m);
UMBRA_FORCE_INLINE  float           dot                 (const Vector2& v1, const Vector2& v2)      { return (v1.x*v2.x + v1.y*v2.y); }

UMBRA_FORCE_INLINE  Vector2         min                 (const Vector2& v1, const Vector2& v2)      { return Vector2 (min2(v1.x,v2.x),min2(v1.y,v2.y)); }
UMBRA_FORCE_INLINE  Vector2         max                 (const Vector2& v1, const Vector2& v2)      { return Vector2 (max2(v1.x,v2.x),max2(v1.y,v2.y)); }

class Vector3d;

/*-------------------------------------------------------------------*//*!
 * \brief   Three-component vector
 *//*-------------------------------------------------------------------*/

class Vector3
{
public:
                enum Empty
                {
                    NO_INIT
                };

    float   x,y,z;

UMBRA_FORCE_INLINE  explicit        Vector3     (Empty) {}
UMBRA_FORCE_INLINE                  Vector3     (void) : x(0.0f),y(0.0f),z(0.0f)        {}
UMBRA_FORCE_INLINE                  Vector3     (const Vector3& s)                      { x = s.x; y = s.y; z = s.z; }
UMBRA_FORCE_INLINE                  Vector3     (const Vector3d& s);
UMBRA_FORCE_INLINE                  Vector3     (float cx, float cy, float cz)          : x(cx),y(cy),z(cz) {}
UMBRA_FORCE_INLINE  const float&    operator[]  (int i) const                           { UMBRA_ASSERT (i>=0 && i < 3); return (&x)[i]; }
UMBRA_FORCE_INLINE  float&          operator[]  (int i)                                 { UMBRA_ASSERT (i>=0 && i < 3); return (&x)[i]; }
UMBRA_FORCE_INLINE  Vector3&        clear       (void)                                  { x = 0.0f; y = 0.0f; z = 0.0f; return *this; }
UMBRA_FORCE_INLINE  Vector3&        set         (float cx, float cy, float cz)          { x = cx;  y = cy;  z = cz; return *this;   }
UMBRA_FORCE_INLINE  Vector3&        operator+=  (const Vector3& v)                      { x += v.x, y += v.y, z += v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3&        operator-=  (const Vector3& v)                      { x -= v.x, y -= v.y, z -= v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3&        operator*=  (float s)                               { x = (x*s),   y = (y*s), z = (z*s); return *this; }
UMBRA_FORCE_INLINE  Vector3&        operator/=  (float s)                               { s = (1.0f/s); x = (x*s), y = (y*s), z = (z*s); return *this; }
UMBRA_FORCE_INLINE  Vector3&        operator*=  (const Matrix3x3& m);
UMBRA_FORCE_INLINE  Vector3&        operator*=  (const Matrix4x3& m);
UMBRA_FORCE_INLINE  float           operator|=  (const Vector3& v) const                { return (x*v.x + y*v.y + z*v.z); }
UMBRA_FORCE_INLINE  bool            operator==  (const Vector3& v) const                { return (x == v.x && y == v.y && z == v.z);    }
UMBRA_FORCE_INLINE  bool            operator!=  (const Vector3& v) const                { return !(x == v.x && y == v.y && z == v.z);   }
UMBRA_FORCE_INLINE  float           length      (void) const                            { return (float)sqrt(x*x+y*y+z*z); }
UMBRA_FORCE_INLINE  float           lengthSqr   (void) const                            { return (x*x+y*y+z*z); }
UMBRA_FORCE_INLINE  Vector3&        normalize   (float len = 1.0f)                      { double l = x*x+y*y+z*z; if(l!=0.0) { l = len / sqrt(l); x = (float)(x*l); y = (float)(y*l); z = (float)(z*l); } return *this; }
UMBRA_FORCE_INLINE  Vector3         normalized  (float len = 1.0f) const                { Vector3 n = *this; n.normalize(len); return n; }
UMBRA_FORCE_INLINE  Vector3&        normalizeAndGetLength (float& len)                  { len = length(); if (len != 0.f) { float l = 1.f / len; x = (float)(x*l); y = (float)(y*l); z = (float)(z*l); } return *this; }
UMBRA_FORCE_INLINE  Vector3&        scale       (const Vector3& v)                      { x*=v.x, y*=v.y, z*=v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3&        rotateX     (double s, double c)                    { double ty = c*y-s*z; double tz = s*y + c*z; y = (float)(ty); z = (float)(tz); return *this; }
UMBRA_FORCE_INLINE  Vector3&        rotateY     (double s, double c)                    { double tx = c*x+s*z; double tz =-s*x + c*z; x = (float)(tx); z = (float)(tz); return *this; }
UMBRA_FORCE_INLINE  Vector3&        rotateZ     (double s, double c)                    { double tx = c*x-s*y; double ty = s*x + c*y; x = (float)(tx); y = (float)(ty); return *this; }
UMBRA_FORCE_INLINE  Vector3&        rotateX     (double angle)                          { return rotateX(sin(angle), cos(angle)); }
UMBRA_FORCE_INLINE  Vector3&        rotateY     (double angle)                          { return rotateY(sin(angle), cos(angle)); }
UMBRA_FORCE_INLINE  Vector3&        rotateZ     (double angle)                          { return rotateZ(sin(angle), cos(angle)); }

//UMBRA_FORCE_INLINE    Vector3&        operator*=  (const Vector3& v)                      { x = (x*v.x), y = (y*v.y), z = (z*v.z); return *this; }
//UMBRA_FORCE_INLINE    Vector3&        operator/=  (const Vector3& v)                      { x = (x/v.x), y = (y/v.y), z = (z/v.z); return *this; }
};

typedef Vector3 Vector3f;

UMBRA_FORCE_INLINE  Vector3         min                 (const Vector3& v1, const Vector3& v2)      { return Vector3 (min2(v1.x,v2.x),min2(v1.y,v2.y),min2(v1.z,v2.z)); }
UMBRA_FORCE_INLINE  Vector3         max                 (const Vector3& v1, const Vector3& v2)      { return Vector3 (max2(v1.x,v2.x),max2(v1.y,v2.y),max2(v1.z,v2.z)); }
UMBRA_FORCE_INLINE  Vector3         operator+           (const Vector3& v1, const Vector3& v2)      { return Vector3(v1.x+v2.x, v1.y+v2.y, v1.z+v2.z); }
UMBRA_FORCE_INLINE  Vector3         operator-           (const Vector3& v1, const Vector3& v2)      { return Vector3(v1.x-v2.x, v1.y-v2.y, v1.z-v2.z); }
UMBRA_FORCE_INLINE  Vector3         operator*           (const Vector3& v,  const float s)          { return Vector3(v.x*s, v.y*s, v.z*s); }
UMBRA_FORCE_INLINE  Vector3         operator*           (const float s,     const Vector3& v)       { return Vector3(v.x*s, v.y*s, v.z*s); }
UMBRA_FORCE_INLINE  Vector3         operator/           (const Vector3& v,  const float s)          { return v*(1.0f/s); }
UMBRA_FORCE_INLINE  Vector3         operator-           (const Vector3& v)                          { return Vector3(-v.x, -v.y, -v.z); }
UMBRA_FORCE_INLINE  Vector3         cross               (const Vector3& v1, const Vector3& v2)      { return Vector3 ((v1.y*v2.z)-(v1.z*v2.y), (v1.z*v2.x)-(v1.x*v2.z), (v1.x*v2.y)-(v1.y*v2.x)); }
                    Vector3         operator*           (const Vector3& v,  const Matrix3x3& m);
                    Vector3         operator*           (const Vector3& v,  const Matrix4x3& m);
UMBRA_FORCE_INLINE  Vector3         operator*           (const Vector3& v1, const Vector3& v2)      { return Vector3(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z); }
UMBRA_FORCE_INLINE  float           dot                 (const Vector3& v1, const Vector3& v2)      { return (v1.x*v2.x + v1.y*v2.y + v1.z*v2.z); }
UMBRA_FORCE_INLINE  Vector3         normalize           (const Vector3& v)                          { float scale = 1.0f / sqrt(dot(v,v)); return scale*v; }
UMBRA_FORCE_INLINE  int             getLongestAxis      (const Vector3& v)                          { return (v.x >= v.y) ? ((v.x >= v.z) ? 0 : 2) : ((v.y >= v.z) ? 1 : 2); }
UMBRA_FORCE_INLINE  Vector3         absv                (const Vector3& v)                          { return Vector3(fabs(v.x), fabs(v.y), fabs(v.z)); }

/*-------------------------------------------------------------------*//*!
 * \brief   Three-component double vector
 *//*-------------------------------------------------------------------*/

class Vector3d
{
public:
    double  x,y,z;

UMBRA_FORCE_INLINE                  Vector3d        (void) : x(0.0f),y(0.0f),z(0.0f)        {}
UMBRA_FORCE_INLINE                  Vector3d        (const Vector3d& s)                     { x = s.x; y = s.y; z = s.z; }
UMBRA_FORCE_INLINE                  Vector3d        (const Vector3& s)                      { x = s.x; y = s.y; z = s.z; }
UMBRA_FORCE_INLINE                  Vector3d        (double cx, double cy, double cz)           : x(cx),y(cy),z(cz) {}
UMBRA_FORCE_INLINE  const double&   operator[]  (int i) const                           { UMBRA_ASSERT (i>=0 && i < 3); return (&x)[i]; }
UMBRA_FORCE_INLINE  double&         operator[]  (int i)                                 { UMBRA_ASSERT (i>=0 && i < 3); return (&x)[i]; }
UMBRA_FORCE_INLINE  Vector3d&       clear       (void)                                  { x = 0.0f; y = 0.0f; z = 0.0f; return *this; }
UMBRA_FORCE_INLINE  Vector3d&       set         (double cx, double cy, double cz)           { x = cx;  y = cy;  z = cz; return *this;   }
UMBRA_FORCE_INLINE  Vector3d&       operator+=  (const Vector3d& v)                     { x += v.x, y += v.y, z += v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3d&       operator-=  (const Vector3d& v)                     { x -= v.x, y -= v.y, z -= v.z; return *this; }
UMBRA_FORCE_INLINE  Vector3d&       operator*=  (double s)                              { x = (x*s),   y = (y*s), z = (z*s); return *this; }
UMBRA_FORCE_INLINE  Vector3d&       operator/=  (double s)                              { s = (1.0f/s); x = (x*s), y = (y*s), z = (z*s); return *this; }
UMBRA_FORCE_INLINE  Vector3d&       operator*=  (const Matrix3x3& m);
UMBRA_FORCE_INLINE  Vector3d&       operator*=  (const Matrix4x3& m);
UMBRA_FORCE_INLINE  double          operator|=  (const Vector3d& v) const               { return (x*v.x + y*v.y + z*v.z); }
UMBRA_FORCE_INLINE  bool            operator==  (const Vector3d& v) const               { return (x == v.x && y == v.y && z == v.z);    }
UMBRA_FORCE_INLINE  bool            operator!=  (const Vector3d& v) const               { return !(x == v.x && y == v.y && z == v.z);   }
UMBRA_FORCE_INLINE  double          length      (void) const                            { return (double)sqrt(x*x+y*y+z*z); }
UMBRA_FORCE_INLINE  double          lengthSqr   (void) const                            { return (x*x+y*y+z*z); }
UMBRA_FORCE_INLINE  Vector3d&       normalize   (double len = 1.0f)                     { double l = x*x+y*y+z*z; if(l!=0.0) { l = len / sqrt(l); x = (double)(x*l); y = (double)(y*l); z = (double)(z*l); } return *this; }
UMBRA_FORCE_INLINE  void            scale       (const Vector3d& v)                     { x*=v.x, y*=v.y, z*=v.z; }
UMBRA_FORCE_INLINE  Vector3d&       rotateX     (double s, double c)                    { double ty = c*y-s*z; double tz = s*y + c*z; y = (double)(ty); z = (double)(tz); return *this; }
UMBRA_FORCE_INLINE  Vector3d&       rotateY     (double s, double c)                    { double tx = c*x+s*z; double tz =-s*x + c*z; x = (double)(tx); z = (double)(tz); return *this; }
UMBRA_FORCE_INLINE  Vector3d&       rotateZ     (double s, double c)                    { double tx = c*x-s*y; double ty = s*x + c*y; x = (double)(tx); y = (double)(ty); return *this; }
UMBRA_FORCE_INLINE  Vector3d&       rotateX     (double angle)                          { return rotateX(sin(angle), cos(angle)); }
UMBRA_FORCE_INLINE  Vector3d&       rotateY     (double angle)                          { return rotateY(sin(angle), cos(angle)); }
UMBRA_FORCE_INLINE  Vector3d&       rotateZ     (double angle)                          { return rotateZ(sin(angle), cos(angle)); }


//UMBRA_FORCE_INLINE    Vector3d&       operator*=  (const Vector3d& v)                     { x = (x*v.x), y = (y*v.y), z = (z*v.z); return *this; }
//UMBRA_FORCE_INLINE    Vector3d&       operator/=  (const Vector3d& v)                     { x = (x/v.x), y = (y/v.y), z = (z/v.z); return *this; }
};

UMBRA_FORCE_INLINE  Vector3d            min                 (const Vector3d& v1, const Vector3d& v2)        { return Vector3d (min2(v1.x,v2.x),min2(v1.y,v2.y),min2(v1.z,v2.z)); }
UMBRA_FORCE_INLINE  Vector3d            max                 (const Vector3d& v1, const Vector3d& v2)        { return Vector3d (max2(v1.x,v2.x),max2(v1.y,v2.y),max2(v1.z,v2.z)); }
UMBRA_FORCE_INLINE  Vector3d            operator+           (const Vector3d& v1,    const Vector3d& v2)     { return Vector3d(v1.x+v2.x, v1.y+v2.y, v1.z+v2.z); }
UMBRA_FORCE_INLINE  Vector3d            operator-           (const Vector3d& v1,    const Vector3d& v2)     { return Vector3d(v1.x-v2.x, v1.y-v2.y, v1.z-v2.z); }
UMBRA_FORCE_INLINE  Vector3d            operator*           (const Vector3d& v, const double s)         { return Vector3d(v.x*s, v.y*s, v.z*s); }
UMBRA_FORCE_INLINE  Vector3d            operator*           (const double s,        const Vector3d& v)      { return Vector3d(v.x*s, v.y*s, v.z*s); }
UMBRA_FORCE_INLINE  Vector3d            operator/           (const Vector3d& v, const double s)         { return v*(1.0f/s); }
UMBRA_FORCE_INLINE  Vector3d            operator-           (const Vector3d& v)                         { return Vector3d(-v.x, -v.y, -v.z); }
UMBRA_FORCE_INLINE  Vector3d            cross               (const Vector3d& v1,    const Vector3d& v2)     { return Vector3d ((v1.y*v2.z)-(v1.z*v2.y), (v1.z*v2.x)-(v1.x*v2.z), (v1.x*v2.y)-(v1.y*v2.x)); }
                    Vector3d            operator*           (const Vector3d& v,  const Matrix3x3& m);
                    Vector3d            operator*           (const Vector3d& v,  const Matrix4x3& m);
UMBRA_FORCE_INLINE  double              dot                 (const Vector3d& v1,    const Vector3d& v2)         { return (v1.x*v2.x + v1.y*v2.y + v1.z*v2.z); }

//------------------------------------------------------------------------------------
//

/*-------------------------------------------------------------------*//*!
 * \brief   Conversion operator for converting double vectors to float
 *          vectors
 * \param   s   Double vector.
 *//*-------------------------------------------------------------------*/
//------------------------------------------------------------------------------------
Vector3::Vector3(const Vector3d& s) { x = (float)s.x; y = (float)s.y; z = (float)s.z; }
//------------------------------------------------------------------------------------
//
//------------------------------------------------------------------------------------


/*-------------------------------------------------------------------*//*!
 * \brief   Four-component vector
 *//*-------------------------------------------------------------------*/

class Vector4
{
public:
    float x;
    float y;
    float z;
    float w;

    enum Empty
    {
        NO_INIT
    };

UMBRA_FORCE_INLINE explicit         Vector4     (Empty) {}
UMBRA_FORCE_INLINE                  Vector4     (void)                                          : x(0),y(0),z(0),w(0)           {}
UMBRA_FORCE_INLINE                  Vector4     (float cx, float cy, float cz, float cw)        : x(cx),y(cy),z(cz),w(cw)       {}
UMBRA_FORCE_INLINE explicit         Vector4     (float f)                                       : x(f), y(f), z(f), w(f) {}
UMBRA_FORCE_INLINE                  Vector4     (const Vector3& s, float cw)                    : x(s.x),y(s.y),z(s.z),w(cw)    {}
UMBRA_FORCE_INLINE                  Vector4     (const Vector4& s)                              : x(s.x),y(s.y),z(s.z),w(s.w)   {}
UMBRA_FORCE_INLINE  Vector4&        clear       (void)                                          { x = (0); y = (0); z = (0); w = (0); return *this; }
UMBRA_FORCE_INLINE  Vector4&        set         (float cx, float cy, float cz, float cw)        { x = (cx), y = (cy), z = (cz), w = (cw); return *this; }
UMBRA_FORCE_INLINE  const float&    operator[]  (int i) const                                   { UMBRA_ASSERT (i>=0 && i < 4); return (&x)[i]; }
UMBRA_FORCE_INLINE  float&          operator[]  (int i)                                         { UMBRA_ASSERT (i>=0 && i < 4); return (&x)[i]; }
UMBRA_FORCE_INLINE  bool            operator==  (const Vector4& v) const                        { return (x == v.x && y == v.y && z == v.z && w == v.w);    }
UMBRA_FORCE_INLINE  bool            operator!=  (const Vector4& v) const                        { return !(x == v.x && y == v.y && z == v.z && w == v.w);   }
UMBRA_FORCE_INLINE  Vector4&        operator+=  (const Vector4& v)                              { x += v.x, y += v.y, z += v.z, w += v.w; return *this;     }
UMBRA_FORCE_INLINE  Vector4&        operator-=  (const Vector4& v)                              { x -= v.x, y -= v.y, z -= v.z, w -= v.w; return *this;     }
UMBRA_FORCE_INLINE  Vector4&        operator*=  (float s)                                       { x = (x*s), y = (y*s), z = (z*s), w = (w*s); return *this; }
UMBRA_FORCE_INLINE  Vector4&        operator/=  (float s)                                       { s = (1.0f/s); x = (x*s), y = (y*s), z = (z*s); w = (w*s); return *this; }
UMBRA_FORCE_INLINE  Vector4&        operator*=  (const Matrix4x4& m);
UMBRA_FORCE_INLINE  float           operator|=  (const Vector4& v) const                        { return x*v.x + y*v.y + z*v.z + w*v.w; }
UMBRA_FORCE_INLINE  float           length      (void) const                                    { return (float)sqrt( x*x+y*y+z*z+w*w ); }
UMBRA_FORCE_INLINE  float           lengthSqr   (void) const                                    { return ( x*x+y*y+z*z+w*w ); }
UMBRA_FORCE_INLINE  Vector4&        normalize   (double len = 1.0)                              { float l = length();   if(l!=0.0) *this *= ((float)(len/l)); return *this; }
UMBRA_FORCE_INLINE  friend Vector4  normalize   (const Vector4& v)                              { Vector4 result(v); return result.normalize(); }
UMBRA_FORCE_INLINE  void            scale       (const Vector4& v)                              { x = (x*v.x), y = (y*v.y), z = (z*v.z); w = (w*v.w); }
UMBRA_FORCE_INLINE  Vector3         xyz         (void) const                                    { return Vector3(x, y, z); }

//UMBRA_FORCE_INLINE    Vector4&        operator*=  (const Vector4& v)                              { x = (x*v.x), y = (y*v.y), z = (z*v.z); w = (w*v.w); return *this; }
};

typedef Vector4 Vector4f;

UMBRA_FORCE_INLINE Vector4          min         (const Vector4& v1, const Vector4& v2)  { return Vector4(min2(v1.x, v2.x), min2(v1.y, v2.y), min2(v1.z, v2.z), min2(v1.w, v2.w)); }
UMBRA_FORCE_INLINE Vector4          max         (const Vector4& v1, const Vector4& v2)  { return Vector4(max2(v1.x, v2.x), max2(v1.y, v2.y), max2(v1.z, v2.z), max2(v1.w, v2.w)); }
UMBRA_FORCE_INLINE Vector4          operator+   (const Vector4& v1, const Vector4& v2)  { return Vector4(v1.x+v2.x, v1.y+v2.y, v1.z+v2.z, v1.w+v2.w); }
UMBRA_FORCE_INLINE Vector4          operator-   (const Vector4& v1, const Vector4& v2)  { return Vector4(v1.x-v2.x, v1.y-v2.y, v1.z-v2.z, v1.w-v2.w); }
UMBRA_FORCE_INLINE  Vector4         operator*   (const Vector4& v,  float s)            { return Vector4(v.x*s, v.y*s, v.z*s, v.w*s); }
UMBRA_FORCE_INLINE  Vector4         operator*   (float s,           const Vector4& v)   { return v*s; }
UMBRA_FORCE_INLINE  Vector4         operator/   (const Vector4& v,  float s)            { UMBRA_ASSERT(s!=0.0f); float r = 1.0f/s; return v*r; }
UMBRA_FORCE_INLINE  Vector4         operator-   (const Vector4& v)                      { return Vector4(-v.x, -v.y, -v.z, -v.w); }
                    Vector4         operator*   (const Vector4& v,  const Matrix4x4& m);
UMBRA_FORCE_INLINE  float           dot         (const Vector4& v1, const Vector4& v2)  { return v1.x*v2.x + v1.y*v2.y + v1.z*v2.z + v1.w*v2.w; }
UMBRA_FORCE_INLINE  float           dot         (const Vector4& v1, const Vector3& v2)  { return v1.x*v2.x + v1.y*v2.y + v1.z*v2.z + v1.w; }

UMBRA_FORCE_INLINE Vector4 pseudoNormalize(const Vector4& pleq)
{
    float d = max2(fabs(pleq.x), max2(fabs(pleq.y), fabs(pleq.z)));

    if (d == 0.f)
        return pleq;

    return pleq / d;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Four-component double vector
 *//*-------------------------------------------------------------------*/

class Vector4d
{
public:
    double x,y,z,w;

UMBRA_FORCE_INLINE                  Vector4d        (void)                                          : x(0),y(0),z(0),w(0)           {}
UMBRA_FORCE_INLINE                  Vector4d        (double cx, double cy, double cz, double cw)        : x(cx),y(cy),z(cz),w(cw)       {}
UMBRA_FORCE_INLINE                  Vector4d        (const Vector3& s, double cw)                   : x(s.x),y(s.y),z(s.z),w(cw)    {}
UMBRA_FORCE_INLINE                  Vector4d        (const Vector4d& s)                             : x(s.x),y(s.y),z(s.z),w(s.w)   {}
UMBRA_FORCE_INLINE  Vector4d&       clear       (void)                                          { x = (0); y = (0); z = (0); w = (0); return *this; }
UMBRA_FORCE_INLINE  Vector4d&       set         (double cx, double cy, double cz, double cw)        { x = (cx), y = (cy), z = (cz), w = (cw); return *this; }
UMBRA_FORCE_INLINE  const double&   operator[]  (int i) const                                   { UMBRA_ASSERT (i>=0 && i < 4); return (&x)[i]; }
UMBRA_FORCE_INLINE  double&         operator[]  (int i)                                         { UMBRA_ASSERT (i>=0 && i < 4); return (&x)[i]; }
UMBRA_FORCE_INLINE  bool            operator==  (const Vector4d& v) const                       { return (x == v.x && y == v.y && z == v.z && w == v.w);    }
UMBRA_FORCE_INLINE  bool            operator!=  (const Vector4d& v) const                       { return !(x == v.x && y == v.y && z == v.z && w == v.w);   }
UMBRA_FORCE_INLINE  Vector4d&       operator+=  (const Vector4d& v)                             { x += v.x, y += v.y, z += v.z, w += v.w; return *this;     }
UMBRA_FORCE_INLINE  Vector4d&       operator-=  (const Vector4d& v)                             { x -= v.x, y -= v.y, z -= v.z, w -= v.w; return *this;     }
UMBRA_FORCE_INLINE  Vector4d&       operator*=  (double s)                                      { x = (x*s), y = (y*s), z = (z*s), w = (w*s); return *this; }
UMBRA_FORCE_INLINE  Vector4d&       operator/=  (double s)                                      { s = (1.0f/s); x = (x*s), y = (y*s), z = (z*s); w = (w*s); return *this; }
UMBRA_FORCE_INLINE  Vector4d&       operator*=  (const Matrix4x4& m);
UMBRA_FORCE_INLINE  double          operator|=  (const Vector4d& v) const                       { return x*v.x + y*v.y + z*v.z + w*v.w; }
UMBRA_FORCE_INLINE  double          length      (void) const                                    { return (double)sqrt( x*x+y*y+z*z+w*w ); }
UMBRA_FORCE_INLINE  double          lengthSqr   (void) const                                    { return ( x*x+y*y+z*z+w*w ); }
UMBRA_FORCE_INLINE  Vector4d&       normalize   (double len = 1.0)                              { double l = length();  if(l!=0.0) *this *= ((double)(len/l)); return *this; }
UMBRA_FORCE_INLINE  void            scale       (const Vector4d& v)                             { x = (x*v.x), y = (y*v.y), z = (z*v.z); w = (w*v.w); }

//UMBRA_FORCE_INLINE    Vector4d&       operator*=  (const Vector4d& v)                             { x = (x*v.x), y = (y*v.y), z = (z*v.z); w = (w*v.w); return *this; }
};

UMBRA_FORCE_INLINE Vector4d         operator+   (const Vector4d& v1, const Vector4d& v2)    { return Vector4d(v1.x+v2.x, v1.y+v2.y, v1.z+v2.z, v1.w+v2.w); }
UMBRA_FORCE_INLINE Vector4d         operator-   (const Vector4d& v1, const Vector4d& v2)    { return Vector4d(v1.x-v2.x, v1.y-v2.y, v1.z-v2.z, v1.w-v2.w); }
UMBRA_FORCE_INLINE  Vector4d            operator*   (const Vector4d& v, double s)           { return Vector4d(v.x*s, v.y*s, v.z*s, v.w*s); }
UMBRA_FORCE_INLINE  Vector4d            operator*   (double s,          const Vector4d& v)  { return v*s; }
UMBRA_FORCE_INLINE  Vector4d            operator/   (const Vector4d& v, double s)           { UMBRA_ASSERT(s!=0.0f); double r = 1.0f/s; return v*r; }
UMBRA_FORCE_INLINE  Vector4d            operator-   (const Vector4d& v)                     { return Vector4d(-v.x, -v.y, -v.z, -v.w); }
                Vector4d            operator*   (const Vector4d& v,  const Matrix4x4& m);
UMBRA_FORCE_INLINE  double          dot         (const Vector4d& v1, const Vector4d& v2)    { return v1.x*v2.x + v1.y*v2.y + v1.z*v2.z + v1.w*v2.w; }

template <> inline unsigned int getHashValue (const Vector3iRaw& s)
{
    return (uint32)(s.i + s.j*73 + s.k*1937);
}

template <> inline unsigned int getHashValue (const Vector3i& s)
{
    return (uint32)(s.i + s.j*73 + s.k*1937);
}

template <> inline unsigned int getHashValue (const Vector2i& s)
{
    return (uint32)(s.i + s.j*73);
}

template <> UMBRA_FORCE_INLINE unsigned int getHashValue (const Vector2& s)
{
    return getHashValue(s.x) + getHashValue(s.y)*73;
}

template <> UMBRA_FORCE_INLINE unsigned int getHashValue (const Vector4i& s)
{
    return (uint32)(s.i + s.j*73 + s.k*1937 + s.l*5147);
}

template <> UMBRA_FORCE_INLINE unsigned int getHashValue (const Vector4& s)
{
    return getHashValue(s.x) + getHashValue(s.y)*73 + getHashValue(s.z)*1937 + getHashValue(s.w)*5147;
}

template <> inline unsigned int getHashValue (const Vector3& s)
{
    union
    {
        float   f[3];
        UINT32  ui[3];
    } value;
    value.f[0] = s.x;
    value.f[1] = s.y;
    value.f[2] = s.z;
    if (value.f[0] == -0.f) value.f[0] = 0.f;
    if (value.f[1] == -0.f) value.f[1] = 0.f;
    if (value.f[2] == -0.f) value.f[2] = 0.f;
    // from "Optimized Spatial Hashing for Collision Detection of Deformable Objects"
    return (value.ui[0] * 73856093) ^ (value.ui[1] * 19349663) ^ (value.ui[2] * 83492791);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

inline Vector4 getPlaneEquation (const Vector3& a, const Vector3& b, const Vector3& c)
{
    float   x1 = b.x - a.x;
    float   y1 = b.y - a.y;
    float   z1 = b.z - a.z;
    float   x2 = c.x - a.x;
    float   y2 = c.y - a.y;
    float   z2 = c.z - a.z;
    float   nx = (y1*z2)-(z1*y2);
    float   ny = (z1*x2)-(x1*z2);
    float   nz = (x1*y2)-(y1*x2);

    return Vector4(nx,ny,nz,-(a.x*nx+a.y*ny+a.z*nz));
}

} // namespace Umbra

#endif // __UMBRAVECTOR_HPP

//--------------------------------------------------------------------
