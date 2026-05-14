#ifndef UMBRAMATRIX_HPP
#define UMBRAMATRIX_HPP

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra Matrix classes
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief           Constructs 4x4 matrices of specified type.
 *//*-------------------------------------------------------------------*/

class Matrix4x4;

class MatrixFactory
{
public:

    /* coordinate system base transform matrix */
    static Matrix4x3    transformBase (const Vector3& origin, const Vector3& forward, const Vector3& right, const Vector3& up);

    /* projection matrices, following OpenGL convention (right handed) */
    static Matrix4x4    frustum     (float fov, float zNear, float zFar);
    static Matrix4x4    frustum     (float fov, float aspect, float zNear, float zFar);
    static Matrix4x4    frustum     (float left, float right, float bottom, float top, float zNear, float zFar);
    static Matrix4x4    ortho       (float left, float right, float bottom, float top, float zNear, float zFar);
    static Matrix4x4    ortho       (float fov, float zNear, float zFar);
    static Matrix4x4    frustumInf  (float fov, float zNear, float zEpsilon);

    /* projection matrices (left handed) */
    static Matrix4x4    frustumLH   (float left, float right, float bottom, float top, float zNear, float zFar);

    /* rotation matrices */
    static Matrix4x3    rotateX (float angle);
    static Matrix4x3    rotateY (float angle);
    static Matrix4x3    rotateZ (float angle);
    static Matrix4x4    rotate  (float angle, float x, float y, float z);

    /* scale matrices */
    static Matrix4x3    scale   (const Vector3& v);

    /* utils */

    static Matrix4x4    orthonormalBasis(const Vector3& dof);

    static UMBRA_INLINE Matrix4x4 subFrustum(const Matrix4x4& transform, float x, float y, float sx, float sy);
};

/*-------------------------------------------------------------------*//*!
 * \brief           4x3 Matrix class used for passing data into some classes.
 *//*-------------------------------------------------------------------*/

class Matrix4x3
{
private:
    float m[3][4];
public:
                enum Empty
                {
                    NO_INIT
                };

UMBRA_FORCE_INLINE  const Vector4&      operator[]      (int i) const                           { UMBRA_ASSERT (i>=0 && i < 3);  return *(const Vector4*)(&m[i][0]); }
UMBRA_FORCE_INLINE  Vector4&            operator[]      (int i)                                 { UMBRA_ASSERT (i>=0 && i < 3);  return *(Vector4*)(&m[i][0]); }

UMBRA_FORCE_INLINE                      Matrix4x3       (void)                                  { ident(); }
UMBRA_FORCE_INLINE                      Matrix4x3       (Empty)                                 { /*nada*/ }
UMBRA_FORCE_INLINE                      Matrix4x3       (const Vector3& right, const Vector3& up, const Vector3& dof, const Vector3& translation)   { setRight(right); setUp(up); setDof(dof), setTranslation(translation); }
UMBRA_FORCE_INLINE                      Matrix4x3       (const Matrix4x3& n)                    { *this=n; }
UMBRA_FORCE_INLINE                      Matrix4x3       (float e00, float e01, float e02, float e03,
                                                     float e10, float e11, float e12, float e13,
                                                     float e20, float e21, float e22, float e23)    { m[0][0] = e00; m[0][1] = e01; m[0][2] = e02; m[0][3] = e03; m[1][0] = e10; m[1][1] = e11; m[1][2] = e12; m[1][3] = e13; m[2][0] = e20; m[2][1] = e21; m[2][2] = e22; m[2][3] = e23; }
UMBRA_FORCE_INLINE  Matrix4x3&          operator=       (const Matrix4x3&);

                Matrix4x3&          clear           (void);
                void                flushToZero     (void);
                float               getMaxScale     (void) const;
                Matrix4x3&          ident           (void);
                bool                isUniform       (void) const;
                bool                isIdentity      (void) const;

                Matrix4x3&          invert          (const Matrix4x3& n);
UMBRA_FORCE_INLINE  Matrix4x3&          invert          (void)                                  { return invert(*this); }

UMBRA_FORCE_INLINE  Vector3             getColumn       (int i) const                           { UMBRA_ASSERT (i>=0 && i < 4);  return Vector3(m[0][i], m[1][i], m[2][i]); }
UMBRA_FORCE_INLINE Vector3              getRight        (void) const                            { return Vector3(m[0][0], m[1][0], m[2][0]); }
UMBRA_FORCE_INLINE Vector3              getUp           (void) const                            { return Vector3(m[0][1], m[1][1], m[2][1]); }
UMBRA_FORCE_INLINE  Vector3             getDof          (void) const                            { return Vector3(m[0][2], m[1][2], m[2][2]); }
UMBRA_FORCE_INLINE  Vector3             getTranslation  (void) const                            { return getColumn(3);      }
                Vector3             getScale        (void) const;

UMBRA_FORCE_INLINE  Matrix4x3&          setColumn       (int i, const Vector3& v)               { UMBRA_ASSERT (i>=0 && i < 4);  m[0][i] = v.x; m[1][i] = v.y; m[2][i] = v.z; return *this; }
UMBRA_FORCE_INLINE  Matrix4x3&          setRight        (const Vector3& v)                      { return setColumn(0,v); }
UMBRA_FORCE_INLINE  Matrix4x3&          setUp           (const Vector3& v)                      { return setColumn(1,v); }
UMBRA_FORCE_INLINE  Matrix4x3&          setDof          (const Vector3& v)                      { return setColumn(2,v); }
UMBRA_FORCE_INLINE  Matrix4x3&          setTranslation  (const Vector3& v)                      { return setColumn(3,v); }

                bool                operator==      (const Matrix4x3& n) const;
UMBRA_FORCE_INLINE  bool                operator!=      (const Matrix4x3& n) const              { return !(*this == n); }

                Matrix4x3&          operator*=      (const Matrix4x3& n);
                Matrix4x3&          operator*=      (float f);
UMBRA_FORCE_INLINE  Matrix4x3&          scale           (const Vector3& v);
UMBRA_FORCE_INLINE  Vector3             transform       (const Vector3& v) const;
UMBRA_FORCE_INLINE  Vector3             transformFast   (const Vector3& v) const;
UMBRA_FORCE_INLINE  Vector4             transform       (const Vector4& v) const;
UMBRA_FORCE_INLINE  Vector3             transformNormal (const Vector3& v) const;
};

/*-------------------------------------------------------------------*//*!
 * \brief           4x4 Matrix class used for passing data into the
 *                  public API functions.
 *//*-------------------------------------------------------------------*/

class Matrix4x4
{
private:
    float m[4][4];
public:
                enum Empty
                {
                    NO_INIT
                };

UMBRA_FORCE_INLINE  const Vector4&      operator[]      (int i) const                       { UMBRA_ASSERT (i>=0 && i < 4);  return *(const Vector4*)(&m[i][0]); }
UMBRA_FORCE_INLINE  Vector4&            operator[]      (int i)                             { UMBRA_ASSERT (i>=0 && i < 4);  return *(Vector4*)(&m[i][0]); }


UMBRA_FORCE_INLINE                      Matrix4x4       (void)                              { ident(); }
UMBRA_FORCE_INLINE  explicit            Matrix4x4       (Empty)                             { /*nada*/ }
UMBRA_FORCE_INLINE                      Matrix4x4       (const Matrix4x3& n)                { make(n); }
UMBRA_FORCE_INLINE                      Matrix4x4       (const Matrix4x4& n)                { *this=n; }
UMBRA_FORCE_INLINE                      Matrix4x4       (float e00, float e01, float e02, float e03,
                                                     float e10, float e11, float e12, float e13,
                                                     float e20, float e21, float e22, float e23,
                                                     float e30, float e31, float e32, float e33)    { m[0][0] = e00; m[0][1] = e01; m[0][2] = e02; m[0][3] = e03; m[1][0] = e10; m[1][1] = e11; m[1][2] = e12; m[1][3] = e13; m[2][0] = e20; m[2][1] = e21; m[2][2] = e22; m[2][3] = e23; m[3][0] = e30; m[3][1] = e31; m[3][2] = e32; m[3][3] = e33; }
UMBRA_FORCE_INLINE  Matrix4x4&          operator=       (const Matrix4x4&);

UMBRA_FORCE_INLINE  bool                is4x3Matrix     (void) const                        { return m[3][0] == 0.0f && m[3][1] == 0.0f && m[3][2] == 0.0f && m[3][3] == 1.0f; }
UMBRA_FORCE_INLINE  const Matrix4x3&    get4x3Matrix    (void) const                        { return reinterpret_cast<const Matrix4x3&>(*this); }
                    void                make            (const Matrix4x3& src);

                    void                clear           (void);
                    void                flushToZero     (void);
                    float               getMaxScale     (void) const;
                    void                ident           (void);
                    bool                isUniform       (void) const;

                    Matrix4x4&          invert          (const Matrix4x3& n);
                    Matrix4x4&          invert          (const Matrix4x4& n);
UMBRA_FORCE_INLINE  Matrix4x4&          invert          (void)                              { return invert(*this); }

UMBRA_FORCE_INLINE  Vector4             getColumn       (int i) const                       { UMBRA_ASSERT (i>=0 && i < 4);  return Vector4(m[0][i], m[1][i], m[2][i],m[3][i]); }
UMBRA_FORCE_INLINE  Vector4             getRow          (int i) const                       { UMBRA_ASSERT (i>=0 && i < 4);  return Vector4(m[i][0], m[i][1], m[i][2],m[i][3]); }
UMBRA_FORCE_INLINE  Vector3             getRight        (void) const                        { return Vector3(m[0][0], m[1][0], m[2][0]); }
UMBRA_FORCE_INLINE  Vector3             getUp           (void) const                        { return Vector3(m[0][1], m[1][1], m[2][1]); }
UMBRA_FORCE_INLINE  Vector3             getDof          (void) const                        { return Vector3(m[0][2], m[1][2], m[2][2]); }
UMBRA_FORCE_INLINE  Vector3             getTranslation  (void) const                        { return Vector3(m[0][3], m[1][3], m[2][3]); }
                    Vector3             getScale        (void) const;

UMBRA_FORCE_INLINE  Matrix4x4&          setColumn       (int i, const Vector4& v)           { UMBRA_ASSERT (i>=0 && i < 4);  m[0][i] = v.x; m[1][i] = v.y; m[2][i] = v.z; m[3][i] = v.w; return *this; }
UMBRA_FORCE_INLINE  Matrix4x4&          setRow          (int i, const Vector4& v)           { UMBRA_ASSERT (i>=0 && i < 4);  m[i][0] = v.x; m[i][1] = v.y; m[i][2] = v.z; m[i][3] = v.w; return *this; }
UMBRA_FORCE_INLINE  Matrix4x4&          setRight        (const Vector3& v)                  { m[0][0] = v.x; m[1][0] = v.y; m[2][0] = v.z; return *this;}
UMBRA_FORCE_INLINE  Matrix4x4&          setUp           (const Vector3& v)                  { m[0][1] = v.x; m[1][1] = v.y; m[2][1] = v.z; return *this;}
UMBRA_FORCE_INLINE  Matrix4x4&          setDof          (const Vector3& v)                  { m[0][2] = v.x; m[1][2] = v.y; m[2][2] = v.z; return *this;}
UMBRA_FORCE_INLINE  Matrix4x4&          setTranslation  (const Vector3& v)                  { m[0][3] = v.x; m[1][3] = v.y; m[2][3] = v.z; return *this;}

                    Matrix4x4&          scale           (const Vector3& v);
UMBRA_FORCE_INLINE  Vector4             transform       (const Vector3& src) const;
UMBRA_FORCE_INLINE  Vector4             transform       (const Vector4& src) const;
UMBRA_FORCE_INLINE  Vector3             transformProjectToXYZ(const Vector3& src) const;
UMBRA_FORCE_INLINE  Vector3             transformDivByW (const Vector3& src) const;

                    bool                operator==      (const Matrix4x4& n) const;
UMBRA_FORCE_INLINE  bool                operator!=      (const Matrix4x4& n) const          { return !(*this == n); }

                    Matrix4x4&          operator*=      (const Matrix4x3& n);
                    Matrix4x4&          operator*=      (const Matrix4x4& n);
                    Matrix4x4&          operator*=      (float f);
                    void                transpose       (void);
                    void                transpose       (const Matrix4x4& src);
                    float               det             (void) const;

private:
static              float               det3x3          (float a1, float a2, float a3, float b1, float b2, float b3, float c1, float c2, float c3);
static              float               det2x2          (float a, float b, float c, float d);

};

/*-------------------------------------------------------------------*//*!
 * \brief           3x3 Matrix class used for passing data into the
 *                  public API functions.
 *
 * \review
 *//*-------------------------------------------------------------------*/

class Matrix3x3
{
private:
    float m[3][3];
public:
                enum Empty
                {
                    NO_INIT
                };

UMBRA_FORCE_INLINE  const Vector3&      operator[]      (int i) const                       { UMBRA_ASSERT (i>=0 && i < 3);  return *(const Vector3*)(&m[i][0]); }
UMBRA_FORCE_INLINE  Vector3&            operator[]      (int i)                             { UMBRA_ASSERT (i>=0 && i < 3);  return *(Vector3*)(&m[i][0]); }


UMBRA_FORCE_INLINE                      Matrix3x3       (void)                              { ident(); }
UMBRA_FORCE_INLINE                      Matrix3x3       (Empty)                             { /*nada*/ }
UMBRA_FORCE_INLINE                      Matrix3x3       (float e00, float e01, float e02,
                                                     float e10, float e11, float e12,
                                                     float e20, float e21, float e22)   { m[0][0] = e00; m[0][1] = e01; m[0][2] = e02; m[1][0] = e10; m[1][1] = e11; m[1][2] = e12; m[2][0] = e20; m[2][1] = e21; m[2][2] = e22; }
UMBRA_FORCE_INLINE  Matrix3x3&          operator=       (const Matrix3x3&);

                void                clear           (void);
                void                flushToZero     (void);
                float               getMaxScale     (void) const;
                void                ident           (void);
                bool                isUniform       (void) const;

                Matrix3x3&          invert          (const Matrix3x3& n);
UMBRA_FORCE_INLINE  Matrix3x3&          invert          (void)                              { return invert(*this); }

UMBRA_FORCE_INLINE  Vector3             getColumn       (int i) const                       { UMBRA_ASSERT (i>=0 && i < 3);  return Vector3(m[0][i], m[1][i], m[2][i]); }
UMBRA_FORCE_INLINE  Vector3             getRight        (void) const                        { return Vector3(m[0][0], m[1][0], m[2][0]); }
UMBRA_FORCE_INLINE  Vector3             getUp           (void) const                        { return Vector3(m[0][1], m[1][1], m[2][1]); }
UMBRA_FORCE_INLINE  Vector3             getDof          (void) const                        { return Vector3(m[0][2], m[1][2], m[2][2]); }
                Vector3             getScale        (void) const;

UMBRA_FORCE_INLINE  Matrix3x3&          setColumn       (int i, const Vector3& v)           { UMBRA_ASSERT (i>=0 && i < 3);  m[0][i] = v.x; m[1][i] = v.y; m[2][i] = v.z; return *this; }
UMBRA_FORCE_INLINE  Matrix3x3&          setRight        (const Vector3& v)                  { m[0][0] = v.x; m[1][0] = v.y; m[2][0] = v.z; return *this;}
UMBRA_FORCE_INLINE  Matrix3x3&          setUp           (const Vector3& v)                  { m[0][1] = v.x; m[1][1] = v.y; m[2][1] = v.z; return *this;}
UMBRA_FORCE_INLINE  Matrix3x3&          setDof          (const Vector3& v)                  { m[0][2] = v.x; m[1][2] = v.y; m[2][2] = v.z; return *this;}

UMBRA_FORCE_INLINE  Matrix3x3&          scale           (const Vector3& v);
UMBRA_FORCE_INLINE Vector3              transform       (const Vector3& src) const;

                bool                operator==      (const Matrix3x3& n) const;
UMBRA_FORCE_INLINE  bool                operator!=      (const Matrix3x3& n) const          { return !(*this == n); }

                Matrix3x3&          operator*=      (const Matrix3x3& n);
                Matrix3x3&          operator*=      (float f);
                void                transpose       (void);
                void                transpose       (const Matrix3x3& src);
                float               det             (void);
};

/*-------------------------------------------------------------------*//*!
 * \brief           2x2 Matrix class used for passing data into the
 *                  public API functions.
 *
 * \review
 *//*-------------------------------------------------------------------*/

class Matrix2x2
{
private:
    float m[2][2];
public:
                enum Empty
                {
                    NO_INIT
                };

UMBRA_FORCE_INLINE  const Vector2&      operator[]      (int i) const                       { UMBRA_ASSERT (i>=0 && i < 2);  return *(const Vector2*)(&m[i][0]); }
UMBRA_FORCE_INLINE  Vector2&            operator[]      (int i)                             { UMBRA_ASSERT (i>=0 && i < 2);  return *(Vector2*)(&m[i][0]); }


UMBRA_FORCE_INLINE                      Matrix2x2       (void)                              { ident(); }
UMBRA_FORCE_INLINE                      Matrix2x2       (Empty)                             { /*nada*/ }
UMBRA_FORCE_INLINE                      Matrix2x2       (float e00, float e01,
                                                     float e10, float e11)              { m[0][0] = e00; m[0][1] = e01; m[1][0] = e10; m[1][1] = e11; }
UMBRA_FORCE_INLINE  Matrix2x2&          operator=       (const Matrix2x2&);

                void                clear           (void);
                void                flushToZero     (void);
                float               getMaxScale     (void) const;
                void                ident           (void);
                bool                isUniform       (void) const;

                Matrix2x2&          invert          (const Matrix2x2& n);
UMBRA_FORCE_INLINE  Matrix2x2&          invert          (void)                              { return invert(*this); }

UMBRA_FORCE_INLINE  Vector2             getColumn       (int i) const                       { UMBRA_ASSERT (i>=0 && i < 2);  return Vector2(m[0][i], m[1][i]);  }
UMBRA_FORCE_INLINE  Vector2             getRight        (void) const                        { return Vector2(m[0][0], m[1][0]); }
UMBRA_FORCE_INLINE  Vector2             getUp           (void) const                        { return Vector2(m[0][1], m[1][1]); }
                Vector2             getScale        (void) const;

UMBRA_FORCE_INLINE  Matrix2x2&          setColumn       (int i, const Vector2& v)           { UMBRA_ASSERT (i>=0 && i < 2);  m[0][i] = v.x; m[1][i] = v.y; return *this; }
UMBRA_FORCE_INLINE  Matrix2x2&          setRight        (const Vector2& v)                  { m[0][0] = v.x; m[1][0] = v.y; return *this;}
UMBRA_FORCE_INLINE  Matrix2x2&          setUp           (const Vector2& v)                  { m[0][1] = v.x; m[1][1] = v.y; return *this;}

UMBRA_FORCE_INLINE  Matrix2x2&          scale           (const Vector2& v);
UMBRA_FORCE_INLINE Vector2              transform       (const Vector2& src) const;

                bool                operator==      (const Matrix2x2& n) const;
UMBRA_FORCE_INLINE  bool                operator!=      (const Matrix2x2& n) const          { return !(*this == n); }

                Matrix2x2&          operator*=      (const Matrix2x2& n);
                Matrix2x2&          operator*=      (float f);
                void                transpose       (void);
                void                transpose       (const Matrix2x2& src);
                float               det             (void);
};

Matrix4x4   operator* (const Matrix4x3& m1, const Matrix4x3& m2);
Matrix4x4   operator* (const Matrix4x3& m1, const Matrix4x4& m2);
Matrix4x4   operator* (const Matrix4x4& m1, const Matrix4x3& m2);
Matrix4x4   operator* (const Matrix4x4& m1, const Matrix4x4& m2);
Matrix3x3   operator* (const Matrix3x3& m1, const Matrix3x3& m2);
Matrix2x2   operator* (const Matrix2x2& m1, const Matrix2x2& m2);

//===================================================================
//      Matrix4x3 performance critical implementation
//===================================================================

UMBRA_FORCE_INLINE Matrix4x3& Matrix4x3::operator= (const Matrix4x3& ss)
{
/*    float*          d = &(m[0][0]);
    const float*    s = &ss[0][0];
    for (int i = 0 ; i < 4*3; i++)
        d[i] = s[i];

 \todo [jasin] gcc issues a "non-initialized" warning, so assign explicitly.
               maybe we should fix this better, but this will do for now.
 */
    m[0][0] = ss.m[0][0];
    m[0][1] = ss.m[0][1];
    m[0][2] = ss.m[0][2];
    m[0][3] = ss.m[0][3];

    m[1][0] = ss.m[1][0];
    m[1][1] = ss.m[1][1];
    m[1][2] = ss.m[1][2];
    m[1][3] = ss.m[1][3];

    m[2][0] = ss.m[2][0];
    m[2][1] = ss.m[2][1];
    m[2][2] = ss.m[2][2];
    m[2][3] = ss.m[2][3];
    return *this;
}

UMBRA_FORCE_INLINE  Vector3& Vector3::operator*= (const Matrix4x3& m)
{
    *this = m.transform(*this);
    return *this;
}

UMBRA_FORCE_INLINE Vector3 Matrix4x3::transform (const Vector3& s)  const
{
    return Vector3(
    s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2] + m[0][3],
    s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2] + m[1][3],
    s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2] + m[2][3]

    );
}

UMBRA_FORCE_INLINE Vector4 Matrix4x3::transform (const Vector4& s)  const
{
    return Vector4(
    s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2] + s.w * m[0][3],
    s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2] + s.w * m[1][3],
    s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2] + s.w * m[2][3],
    s.w);
}

UMBRA_FORCE_INLINE Vector3 Matrix4x3::transformNormal (const Vector3& s)  const
{
    // Transformation is not taken into account
    return Vector3(
    s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2],
    s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2],
    s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2]);
}

//===================================================================
//      Matrix4x4 performance critical implementation
//===================================================================

UMBRA_FORCE_INLINE  Vector4& Vector4::operator*= (const Matrix4x4& m)
{
    *this = m.transform(*this);
    return *this;
}

UMBRA_FORCE_INLINE Matrix4x4& Matrix4x4::operator= (const Matrix4x4& ss)
{
    float*          d = &(*this)[0][0];
    const float*    s = &ss[0][0];
    for (int i = 0 ; i < 4*4; i++)
        d[i] = s[i];
    return *this;
}

UMBRA_FORCE_INLINE Vector4 Matrix4x4::transform (const Vector4& s)  const
{
    return Vector4(
    s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2] + s.w * m[0][3],
    s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2] + s.w * m[1][3],
    s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2] + s.w * m[2][3],
    s.x * m[3][0] + s.y * m[3][1] + s.z * m[3][2] + s.w * m[3][3]);
}

UMBRA_FORCE_INLINE Vector4 Matrix4x4::transform (const Vector3& s)  const
{
    // the non-existing W component of the source vector is regarded to be 1.0
    return Vector4(
    s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2] + m[0][3],
    s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2] + m[1][3],
    s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2] + m[2][3],
    s.x * m[3][0] + s.y * m[3][1] + s.z * m[3][2] + m[3][3]);
}

UMBRA_FORCE_INLINE Vector3 Matrix4x4::transformProjectToXYZ(const Vector3& s) const
{
    // the non-existing W component of the source vector is regarded to be 1.0
    return Vector3(
    s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2] + m[0][3],
    s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2] + m[1][3],
    s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2] + m[2][3]);
}

UMBRA_FORCE_INLINE Vector3 Matrix4x4::transformDivByW (const Vector3& s)  const
{
    double w = s.x * m[3][0] + s.y * m[3][1] + s.z * m[3][2] + m[3][3];
    double oow = 1.0 / w;
    // the non-existing W component of the source vector is regarded to be 1.0
    return Vector3(
    (float)((s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2] + m[0][3])*oow),
    (float)((s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2] + m[1][3])*oow),
    (float)((s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2] + m[2][3])*oow));
}

//===================================================================
//      Matrix3x3 performance critical implementation
// \review
//===================================================================

UMBRA_FORCE_INLINE  Vector3& Vector3::operator*= (const Matrix3x3& m)
{
    *this = m.transform(*this);
    return *this;
}

UMBRA_FORCE_INLINE Matrix3x3& Matrix3x3::operator= (const Matrix3x3& ss)
{
    float*          d = &(*this)[0][0];
    const float*    s = &ss[0][0];
    for (int i = 0 ; i < 3*3; i++)
        d[i] = s[i];
    return *this;
}

UMBRA_FORCE_INLINE Vector3 Matrix3x3::transform (const Vector3& s)  const
{
    return Vector3(
    s.x * m[0][0] + s.y * m[0][1] + s.z * m[0][2],
    s.x * m[1][0] + s.y * m[1][1] + s.z * m[1][2],
    s.x * m[2][0] + s.y * m[2][1] + s.z * m[2][2]);
}

//===================================================================
//      Matrix2x2 performance critical implementation
// \review
//===================================================================

UMBRA_FORCE_INLINE  Vector2& Vector2::operator*= (const Matrix2x2& m)
{
    *this = m.transform(*this);
    return *this;
}

UMBRA_FORCE_INLINE Matrix2x2& Matrix2x2::operator= (const Matrix2x2& ss)
{
    float*          d = &(*this)[0][0];
    const float*    s = &ss[0][0];
    for (int i = 0 ; i < 2*2; i++)
        d[i] = s[i];
    return *this;
}

UMBRA_FORCE_INLINE Vector2 Matrix2x2::transform (const Vector2& s)  const
{
    return Vector2(
    s.x * m[0][0] + s.y * m[0][1],
    s.x * m[1][0] + s.y * m[1][1]);
}

Matrix4x4 MatrixFactory::subFrustum(const Matrix4x4& transform, float xOfs, float yOfs, float scaleX, float scaleY)
{
    Matrix4x4 m;
    m[0][0] = scaleX;
    m[1][1] = scaleY;
    m[0][3] = scaleX - 1.f - 2.f * xOfs * scaleX;
    m[1][3] = scaleY - 1.f - 2.f * yOfs * scaleY;
    return transform * m;
}


} // namespace Umbra

#endif // UMBRAMATRIX_HPP

//--------------------------------------------------------------------
