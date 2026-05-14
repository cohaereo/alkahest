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
 * \brief   Matrix implementation
 * \todo [wili 310502] A lot of the code could be removed if you just
 *       defined a couple of simple templates/macros for doing the job.
 * \todo [wili 310502] Missing comment boxes
 *
 */

#include "umbraMatrix.hpp"

#include <string.h>
#include <math.h>

namespace Umbra
{

Matrix4x4 MatrixFactory::frustum (float left, float right, float bottom, float top, float zNear, float zFar)
{
    Matrix4x4   m;  //calls ident()

    m.setColumn (0, Vector4(2.f*zNear / (right-left), 0.f, 0.f, 0.f));
    m.setColumn (1, Vector4(0.f, 2.f*zNear / (top-bottom), 0.f, 0.f));
    m.setColumn (2, Vector4((right+left) / (right-left), (top + bottom) / (top - bottom), -(zFar + zNear) / (zFar - zNear), -1.f));
    m.setColumn (3, Vector4(0.f, 0.f, -(2.0f * zFar * zNear) / (zFar - zNear), 0.f));

    return m;
}

Matrix4x4 MatrixFactory::frustumLH (float left, float right, float bottom, float top, float zNear, float zFar)
{
    Matrix4x4   m;  //calls ident()
    m.setColumn (0, Vector4(2.f * zNear / (right-left),      0.f,                             0.f,                                     0.f));
    m.setColumn (1, Vector4(0.f,                             2.f * zNear / (top - bottom),    0.f,                                     0.f));
    m.setColumn (2, Vector4((right + left) / (right - left), (top + bottom) / (top - bottom), zFar / (zNear - zFar),                   -1.f));
    m.setColumn (3, Vector4(0.f,                             0.f,                             (zNear * zFar) / (zNear - zFar),         0.f));
    return m;
}

Matrix4x4 MatrixFactory::frustum (float fov, float aspect, float zNear, float zFar)
{
    Matrix4x4 m;
    float q = 1.f / (zFar - zNear);
    float a = 1.f / (float)tan(fov * 3.14159265f / 360.0f);
    m[0][0] = a;
    m[1][1] = a / aspect;
    m[2][2] = -1.f * (zFar + zNear) * q;
    m[2][3] = -2.f * (zFar * zNear) * q;
    m[3][2] = -1.f;
    m[3][3] = 0.f;
    return m;
}

Matrix4x4 MatrixFactory::frustum (float fov, float zNear, float zFar)
{
    return frustum(fov, 1.f, zNear, zFar);
}

Matrix4x4 MatrixFactory::frustumInf (float fov, float zNear, float zEpsilon)
{
    Matrix4x4 m;
    float a = 1.f / (float)tan(fov * 3.14159265f / 360.0f);
    m[0][0] = a;
    m[1][1] = a;
    m[2][2] = zEpsilon - 1.f;
    m[2][3] = (zEpsilon - 2.f) * zNear;
    m[3][2] = -1.f;
    m[3][3] = 0.f;
    return m;
}

Matrix4x4 MatrixFactory::ortho (float left, float right, float bottom, float top, float zNear, float zFar)
{
    Matrix4x4   m;  //calls ident()

    m.setColumn (0, Vector4(2.f / (right-left), 0.f, 0.f, 0.f));
    m.setColumn (1, Vector4(0.f, 2.f / (top-bottom), 0.f, 0.f));
    m.setColumn (2, Vector4(0.f, 0.f, -2.0f / (zFar - zNear), 0.f));
    m.setColumn (3, Vector4(-(right+left) / (right-left), -(top + bottom) / (top - bottom), -(zFar + zNear) / (zFar - zNear), -1.f));

    return m;
}

Matrix4x4 MatrixFactory::ortho (float fov, float zNear, float zFar)
{
    Matrix4x4   m;  //calls ident()

    float q = 1.f / (zFar - zNear);
    float a = 2.f / fov;
    m[0][0] = a;
    m[1][1] = a;
    m[2][2] = -2.f / (zFar - zNear);
    m[2][3] = -1.f * (zFar + zNear) * q;

    return m;
}

Matrix4x4 MatrixFactory::rotate (float angle, float x, float y, float z)
{
    // angle must be in radians!

    Matrix4x4   m; // identity matrix

    if (!angle || (!x && !y && !z))
        return m;

    Vector3 n(x, y, z);
    n.normalize(1);

    float xy = n.x * n.y;
    float xz = n.x * n.z;
    float yz = n.y * n.z;
    float c = (float)cosf(angle);
    float s = (float)sinf(angle);

    m.setRight      (Vector3(n.x*n.x * (1 - c) + c, xy * (1 - c) + n.z * s, xz * (1 - c) - n.y * s));
    m.setUp         (Vector3(xy * (1 - c) - n.z * s, n.y*n.y * (1 - c) + c, yz * (1 - c) + n.x * s));
    m.setDof        (Vector3(xz * (1 - c) + n.y * s, yz * (1 - c) - n.x * s, n.z*n.z * (1 - c) + c));

    m.setTranslation(Vector3(0, 0, 0));
    m.setColumn     (3, Vector4(0, 0, 0, 1));

    return m;
}


Matrix4x3 MatrixFactory::rotateX    (float angle)
{
    float s = (float)sinf(angle);
    float c = (float)cosf(angle);
    float o = 1.f;
    float z = 0.f;

    Matrix4x3   m;  //calls ident()
    m.setRight  (Vector3(o, z, z));
    m.setUp     (Vector3(z, c,-s));
    m.setDof    (Vector3(z, s, c));
//  srMatrix3T<T> (srVector3T<T>(1.0, 0.0, 0.0),srVector3T<T>(0.0,   c,  -s),srVector3T<T>(0.0,   s,   c));

    return m;
}


Matrix4x3 MatrixFactory::rotateY    (float angle)
{
    float s = (float)sinf(angle);
    float c = (float)cosf(angle);
    float o = 1.f;
    float z = 0.f;

    Matrix4x3   m;  //calls ident()
    m.setRight  (Vector3(c, z, s));
    m.setUp     (Vector3(z, o, z));
    m.setDof    (Vector3(-s,z, c));
//  srMatrix3T<T> (srVector3T<T>(  c, 0.0,   s),srVector3T<T>(0.0, 1.0, 0.0),srVector3T<T>( -s, 0.0,   c));

    return m;
}


Matrix4x3 MatrixFactory::rotateZ    (float angle)
{
    float s = (float)sinf(angle);
    float c = (float)cosf(angle);
    float o = 1.f;
    float z = 0.f;

    Matrix4x3   m;  //calls ident()
    m.setRight  (Vector3(c,-s, z));
    m.setUp     (Vector3(s, c, z));
    m.setDof    (Vector3(z, z, o));
//  srMatrix3T<T>(srVector3T<T>(  c,  -s, 0.0),srVector3T<T>(  s,   c, 0.0),srVector3T<T>(0.0, 0.0, 1.0));

    return m;
}


Matrix4x3 MatrixFactory::scale      (const Vector3& v)
{
    Matrix4x3   m;  //calls ident()
    m.setRight  (Vector3(v.x, 0.f, 0.f));
    m.setUp     (Vector3(0.f, v.y, 0.f));
    m.setDof    (Vector3(0.f, 0.f, v.z));
    return m;
}

Matrix4x3 MatrixFactory::transformBase (const Vector3& origin, const Vector3& forward, const Vector3& right, const Vector3& up)
{
    Matrix4x3   mtx;  //calls ident()

    mtx[0][0] = right.x;
    mtx[0][1] = right.y;
    mtx[0][2] = right.z;
    mtx[0][3] = -dot(right, origin);
    mtx[1][0] = up.x;
    mtx[1][1] = up.y;
    mtx[1][2] = up.z;
    mtx[1][3] = -dot(up, origin);
    mtx[2][0] = forward.x;
    mtx[2][1] = forward.y;
    mtx[2][2] = forward.z;
    mtx[2][3] = -dot(forward, origin);

    return mtx;
}

Matrix4x4 MatrixFactory::orthonormalBasis(const Vector3& dir)
{
    // Build perpendicular vector, see
    // Hughes, J. F., Möller, T., “Building an Orthonormal Basis from a Unit Vector”, Journal of Graphics Tools 4:4 (1999), 33-35.

    Vector3 w = normalize(dir);

    Vector3 a(fabsf(w.x), fabsf(w.y), fabsf(w.z));
    Vector3 v;

    if (a.x <= a.y && a.x <= a.z)
        v = Vector3(0.0f, -w.z, w.y);
    else if (a.y <= a.x && a.y <= a.z)
        v = Vector3(-w.z, 0.0f, w.x);
    else
        v = Vector3(-w.y, w.x, 0.0f);
    v.normalize();

    // Build basis

    Vector3 u = normalize(cross(w,v));

    Matrix4x4 result;
    result.setColumn(0, Vector4(u.x,u.y,u.z,0.0f));
    result.setColumn(1, Vector4(v.x,v.y,v.z,0.0f));
    result.setColumn(2, Vector4(w.x,w.y,w.z,0.0f));
    result.setColumn(3, Vector4(0,0,0,1));

    return result;
}

//===================================================================
//                  Vector-Matrix functions
//===================================================================

// \todo [wili 310502] Why are these non-inline?

Vector2 operator* (const Vector2& v,  const Matrix2x2& m)
{
    return m.transform(v);
}

Vector3 operator* (const Vector3& v,  const Matrix4x3& m)
{
    return m.transform(v);
}

Vector3 operator* (const Vector3& v,  const Matrix3x3& m)
{
    return m.transform(v);
}

Vector4 operator* (const Vector4& v,  const Matrix4x4& m)
{
    return m.transform(v);
}


//===================================================================
//                  Binary Matrix Multiplications
//===================================================================

// \todo [wili 310502] Note that your first two routines operate
// with Matrix4x3, but return 4x4 matrices.. Seems like you have
// a silent Matrix4x3->Matrix4x4 conversion in the 'return m'
// statement. Is this purposeful? If so, then at least show
// it with an addition ctor/cast?

Matrix4x4   operator* (const Matrix4x3& m1, const Matrix4x3& m2)
{
    Matrix4x3 m(m1);
    m*=m2;
    return m;
}

Matrix4x4   operator* (const Matrix4x3& m1, const Matrix4x4& m2)
{
    Matrix4x4 m(m1);        // implicat conversion to 4x4
    m*=m2;
    return m;
}

Matrix4x4   operator* (const Matrix4x4& m1, const Matrix4x3& m2)
{
    Matrix4x4 m(m1);
    m*=m2;
    return m;
}

Matrix4x4   operator* (const Matrix4x4& m1, const Matrix4x4& m2)
{
    Matrix4x4 m(m1);
    m*=m2;
    return m;
}

Matrix3x3   operator* (const Matrix3x3& m1, const Matrix3x3& m2)
{
    Matrix3x3 m(m1);
    m*=m2;
    return m;
}

Matrix2x2   operator* (const Matrix2x2& m1, const Matrix2x2& m2)
{
    Matrix2x2 m(m1);
    m*=m2;
    return m;
}

//===================================================================
//                      Matrix4x3
//===================================================================

bool Matrix4x3::operator== (const Matrix4x3& n) const
{
    return ((*this)[0]==n[0] && (*this)[1]==n[1] && (*this)[2]==n[2]);
}

// \todo [wili 310502] Why use a for loop here when other implementations use memset()?

Matrix4x3& Matrix4x3::clear(void)
{
    m[0][0] = 0.0f; m[0][1] = 0.0f; m[0][2] = 0.0f; m[0][3] = 0.0f;
    m[1][0] = 0.0f; m[1][1] = 0.0f; m[1][2] = 0.0f; m[1][3] = 0.0f;
    m[2][0] = 0.0f; m[2][1] = 0.0f; m[2][2] = 0.0f; m[2][3] = 0.0f;
    return *this;
}

Matrix4x3& Matrix4x3::ident(void)
{
    m[0][0] = 1.0f; m[0][1] = 0.0f; m[0][2] = 0.0f; m[0][3] = 0.0f;
    m[1][0] = 0.0f; m[1][1] = 1.0f; m[1][2] = 0.0f; m[1][3] = 0.0f;
    m[2][0] = 0.0f; m[2][1] = 0.0f; m[2][2] = 1.0f; m[2][3] = 0.0f;
    return *this;
}

bool Matrix4x3::isIdentity() const
{
    return
        m[0][0] == 1.0f && m[0][1] == 0.0f && m[0][2] == 0.0f && m[0][3] == 0.0f &&
        m[1][0] == 0.0f && m[1][1] == 1.0f && m[1][2] == 0.0f && m[1][3] == 0.0f &&
        m[2][0] == 0.0f && m[2][1] == 0.0f && m[2][2] == 1.0f && m[2][3] == 0.0f;
}

float Matrix4x3::getMaxScale (void) const
{
    float xs = (m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]);
    float ys = (m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]);
    float zs = (m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]);

    float maxScale = xs;

    if (ys > maxScale)
        maxScale = ys;
    if (zs > maxScale)
        maxScale = zs;

    return (float)sqrtf(maxScale);
}

Vector3 Matrix4x3::getScale() const
{
    return Vector3( getColumn(0).length(),  //(m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]),
                    getColumn(1).length(),  //(m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]),
                    getColumn(2).length() );    //(m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]));
}

Matrix4x3& Matrix4x3::scale (const Vector3& v)
{
    setRight( getRight()*v.x );
    setUp   ( getUp()   *v.y );
    setDof  ( getDof()  *v.z );
    return *this;
}

// NOTE: used to be productFromLeft()
// \todo [wili 310502] Why use an assertion? How about
// just falling back to another routine that handles
// the multiplication properly?

Matrix4x3& Matrix4x3::operator*= (const Matrix4x3& n)
{
    UMBRA_ASSERT(&n != this);

    float a,b,c;

    a = m[0][0], b = m[1][0], c = m[2][0];
    m[0][0] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c);
    m[1][0] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c);
    m[2][0] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c);

    a = m[0][1], b = m[1][1], c = m[2][1];
    m[0][1] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c);
    m[1][1] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c);
    m[2][1] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c);

    a = m[0][2], b = m[1][2], c = m[2][2];
    m[0][2] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c);
    m[1][2] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c);
    m[2][2] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c);

    a = m[0][3], b = m[1][3], c = m[2][3];
    m[0][3] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]);
    m[1][3] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]);
    m[2][3] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]);

    return *this;
}

Matrix4x3& Matrix4x3::operator*= (float f)
{
    for (int j = 0; j < 3; j++)
    for (int i = 0; i < 4; i++)
        m[j][i] *= f;

    return *this;
}

void Matrix4x3::flushToZero (void)
{
    const double EPSILON = 1e-15;

    for (int j = 0; j < 3; j++)
    for (int i = 0; i < 4; i++)
    if (fabsf(m[j][i]) <= EPSILON)
        m[j][i] = 0.0f;
}

bool Matrix4x3::isUniform (void) const
{
    double xs = sqrtf(m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]);
    double ys = sqrtf(m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]);
    double zs = sqrtf(m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]);
    return (fabs(xs-ys) <= xs*0.0001 && fabs(xs-zs) <= xs*0.0001);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 * \note            TIME-CRITICAL
 * \note            m may be equal to src...
 * \review
 *//*----------------------------------------------------------------------*/

Matrix4x3& Matrix4x3::invert    (const Matrix4x3& src)
{
    float a1 = src[0][0];
    float b1 = src[0][1];
    float c1 = src[0][2];
    float d1 = src[0][3];
    float a2 = src[1][0];
    float b2 = src[1][1];
    float c2 = src[1][2];
    float d2 = src[1][3];
    float a3 = src[2][0];
    float b3 = src[2][1];
    float c3 = src[2][2];
    float d3 = src[2][3];

    float b2c3_b3c2 = b2 * c3 - b3 * c2;
    float a3c2_a2c3 = a3 * c2 - a2 * c3;
    float a2b3_a3b2 = a2 * b3 - a3 * b2;

    float rDet  = (a1 * (b2c3_b3c2) + b1 * (a3c2_a2c3) + c1 * (a2b3_a3b2));
    UMBRA_ASSERT(rDet != 0.0f); //tried to invert a singular matrix
    rDet = 1.0f / rDet;

    b2c3_b3c2   *= rDet;
    a3c2_a2c3   *= rDet;
    a2b3_a3b2   *= rDet;
    a1          *= rDet;
    b1          *= rDet;
    c1          *= rDet;

    float c1b3_b1c3 = c1 * b3 - b1 * c3;
    float b1c2_c1b2 = b1 * c2 - c1 * b2;

    m[0][0] = b2c3_b3c2;
    m[0][1] = c1b3_b1c3;
    m[0][2] = b1c2_c1b2;
    m[0][3] =-(d1 * (b2c3_b3c2) + d2 * (c1b3_b1c3) + d3 * (b1c2_c1b2));

    float c1a2_a1c2 = c1 * a2 - a1 * c2;
    float a1c3_c1a3 = a1 * c3 - c1 * a3;

    m[1][0] = a3c2_a2c3;
    m[1][1] = a1c3_c1a3;
    m[1][2] = c1a2_a1c2;
    m[1][3] =-(d1 * (a3c2_a2c3) + d2 * (a1c3_c1a3) + d3 * (c1a2_a1c2));

    float b1a3_a1b3 = b1 * a3 - a1 * b3;
    float a1b2_b1a2 = a1 * b2 - b1 * a2;

    m[2][0] = a2b3_a3b2;
    m[2][1] = b1a3_a1b3;
    m[2][2] = a1b2_b1a2;
    m[2][3] =-(d1 * (a2b3_a3b2) + d2 * (b1a3_a1b3) + d3 * (a1b2_b1a2));

    return *this;
}

//===================================================================
//                      Matrix4x4
//===================================================================

void Matrix4x4::make        (const Matrix4x3& src)
{
    Matrix4x3& mm = reinterpret_cast<Matrix4x3&>(*this);
    mm = src;       // copies 4x3
    m[3][0] = 0.f;  // make the fourth row identity
    m[3][1] = 0.f;
    m[3][2] = 0.f;
    m[3][3] = 1.f;
}


// avoid underflow in any of the components...
void Matrix4x4::flushToZero (void)
{
    const double EPSILON = 1e-15;

    for (int j = 0; j < 4; j++)
    for (int i = 0; i < 4; i++)
    if (fabsf(m[j][i]) <= EPSILON)
        m[j][i] = 0.0f;
}

void Matrix4x4::transpose (const Matrix4x4& src)
{
    if (&src == this)
    {
        this->transpose();
        return;
    }

    // note that transposition is _not_ the same as matrix inversion
    // with generic 4x4 matrices (see Matrix4x4::invert)

    m[0][0] = src[0][0];
    m[0][1] = src[1][0];
    m[0][2] = src[2][0];
    m[0][3] = src[3][0];
    m[1][0] = src[0][1];
    m[1][1] = src[1][1];
    m[1][2] = src[2][1];
    m[1][3] = src[3][1];
    m[2][0] = src[0][2];
    m[2][1] = src[1][2];
    m[2][2] = src[2][2];
    m[2][3] = src[3][2];
    m[3][0] = src[0][3];
    m[3][1] = src[1][3];
    m[3][2] = src[2][3];
    m[3][3] = src[3][3];
}

void Matrix4x4::transpose (void)
{
    float tmp;
    tmp = m[0][1]; m[0][1] = m[1][0]; m[1][0] = tmp;
    tmp = m[0][2]; m[0][2] = m[2][0]; m[2][0] = tmp;
    tmp = m[0][3]; m[0][3] = m[3][0]; m[3][0] = tmp;
    tmp = m[1][2]; m[1][2] = m[2][1]; m[2][1] = tmp;
    tmp = m[1][3]; m[1][3] = m[3][1]; m[3][1] = tmp;
    tmp = m[2][3]; m[2][3] = m[3][2]; m[3][2] = tmp;
}

// \review
UMBRA_INLINE float Matrix4x4::det2x2( float a, float b, float c, float d )
{
    return a * d - b * c;
}

// \review
UMBRA_INLINE float Matrix4x4::det3x3( float a1, float a2, float a3, float b1, float b2, float b3, float c1, float c2, float c3 )
{
    return a1 * det2x2 (b2, b3, c2, c3) - b1 * det2x2 (a2, a3, c2, c3) + c1 * det2x2 (a2, a3, b2, b3);
}

// \review
float Matrix4x4::det() const
{
    return m[0][0] * det3x3( m[1][1], m[2][1], m[3][1], m[1][2], m[2][2], m[3][2], m[1][3], m[2][3], m[3][3] ) -
           m[0][1] * det3x3( m[1][0], m[2][0], m[3][0], m[1][2], m[2][2], m[3][2], m[1][3], m[2][3], m[3][3] ) +
           m[0][2] * det3x3( m[1][0], m[2][0], m[3][0], m[1][1], m[2][1], m[3][1], m[1][3], m[2][3], m[3][3] ) -
           m[0][3] * det3x3( m[1][0], m[2][0], m[3][0], m[1][1], m[2][1], m[3][1], m[1][2], m[2][2], m[3][2] );
}

bool Matrix4x4::operator== (const Matrix4x4& n) const
{
    const float* d = (const float*)&(m[0][0]);
    const float* s = (const float*)&(n[0][0]);
    for (int i = 0; i < 16; i++)
    if (d[i]!=s[i])
        return false;
    return true;
}

Matrix4x4& Matrix4x4::operator*= (const Matrix4x3& n43)
{
    const Matrix4x4& n = reinterpret_cast<const Matrix4x4&>(n43);   // Avoid protection problem
    float a,b,c,d;

    a = m[0][0], b = m[1][0], c = m[2][0], d = m[3][0];
    m[0][0] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][0] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][0] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
//  m[3][0] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);
    m[3][0] = (float)(d);

    a = m[0][1], b = m[1][1], c = m[2][1], d = m[3][1];
    m[0][1] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][1] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][1] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
//  m[3][1] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);
    m[3][1] = (float)(d);

    a = m[0][2], b = m[1][2], c = m[2][2], d = m[3][2];
    m[0][2] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][2] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][2] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
//  m[3][2] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);
    m[3][2] = (float)(d);

    a = m[0][3], b = m[1][3], c = m[2][3], d = m[3][3];
    m[0][3] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][3] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][3] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
//  m[3][3] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);
    m[3][3] = (float)(d);

    return *this;
}

// \todo [wili 310502] You should really handle the &n == this case...

Matrix4x4& Matrix4x4::operator*= (const Matrix4x4& n)
{
    UMBRA_ASSERT(&n != this);

    float a,b,c,d;

    a = m[0][0], b = m[1][0], c = m[2][0], d = m[3][0];
    m[0][0] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][0] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][0] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
    m[3][0] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);

    a = m[0][1], b = m[1][1], c = m[2][1], d = m[3][1];
    m[0][1] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][1] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][1] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
    m[3][1] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);

    a = m[0][2], b = m[1][2], c = m[2][2], d = m[3][2];
    m[0][2] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][2] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][2] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
    m[3][2] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);

    a = m[0][3], b = m[1][3], c = m[2][3], d = m[3][3];
    m[0][3] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c + n.m[0][3]*d);
    m[1][3] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c + n.m[1][3]*d);
    m[2][3] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c + n.m[2][3]*d);
    m[3][3] = (float)(n.m[3][0]*a + n.m[3][1]*b + n.m[3][2]*c + n.m[3][3]*d);

    return *this;
}

// \review
Matrix4x4& Matrix4x4::operator*= (float f)
{
    for (int j = 0; j < 4; j++)
    for (int i = 0; i < 4; i++)
        m[j][i] *= f;

    return *this;
}

void Matrix4x4::clear (void)
{
    memset (this, 0, sizeof(Matrix4x4));
}

void Matrix4x4::ident (void)
{
    m[0][0] = 1.0f; m[0][1] = 0.0f; m[0][2] = 0.0f; m[0][3] = 0.0f;
    m[1][0] = 0.0f; m[1][1] = 1.0f; m[1][2] = 0.0f; m[1][3] = 0.0f;
    m[2][0] = 0.0f; m[2][1] = 0.0f; m[2][2] = 1.0f; m[2][3] = 0.0f;
    m[3][0] = 0.0f; m[3][1] = 0.0f; m[3][2] = 0.0f; m[3][3] = 1.0f;
}

// \todo [wili 310502] Why not use Matrix4x3 getMaxScale() routine
// here?

float Matrix4x4::getMaxScale (void) const
{
    float xs = (m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]);
    float ys = (m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]);
    float zs = (m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]);

    float maxScale = xs;

    if (ys > maxScale)
        maxScale = ys;
    if (zs > maxScale)
        maxScale = zs;

    return (float)sqrtf(maxScale);
}

// \todo [wili 310502] Why not use Matrix4x3 isUniform() routine here?
bool Matrix4x4::isUniform (void) const
{
    if (!is4x3Matrix())
        return false;   // not 4x3
    double xs = sqrtf(m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]);
    double ys = sqrtf(m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]);
    double zs = sqrtf(m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]);
    return (fabs(xs-ys) <= xs*0.0001 && fabs(xs-zs) <= xs*0.0001);
}

// \todo [wili 310502] Why not use Matrix4x3 getScale() routine here?

Vector3 Matrix4x4::getScale() const
{
    return Vector3( getColumn(0).length(),  //(m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]),
                    getColumn(1).length(),  //(m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]),
                    getColumn(2).length() );    //(m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]));
}

Matrix4x4& Matrix4x4::scale (const Vector3& v)
{
    setRight( getRight()*v.x );
    setUp   ( getUp()   *v.y );
    setDof  ( getDof()  *v.z );
    return *this;
}


/*----------------------------------------------------------------------*//*!
 * \brief           Matrix4x4 inversion code
 * \note            this == &src is supported
 * \review
 *//*----------------------------------------------------------------------*/

Matrix4x4& Matrix4x4::invert    (const Matrix4x4& src)
{
    // perform 4x3 inversion if possible
    if(src.is4x3Matrix())
    {
        Matrix4x3& m = reinterpret_cast<Matrix4x3&>(*this);     // consider 4x3
        m.invert(src.get4x3Matrix());                           // invert 4x3
        this->m[3][0] = 0;                                      // clear 4th row
        this->m[3][1] = 0;
        this->m[3][2] = 0;
        this->m[3][3] = 1.0f;
        return *this;
    }

    // perform full 4x4 inversion
    Matrix4x4& m = *this;

    const float a4 = src[3][0];
    const float b4 = src[3][1];
    const float c4 = src[3][2];
    const float d4 = src[3][3];
    const float a1 = src[0][0];
    const float b1 = src[0][1];
    const float c1 = src[0][2];
    const float d1 = src[0][3];
    const float a2 = src[1][0];
    const float b2 = src[1][1];
    const float c2 = src[1][2];
    const float d2 = src[1][3];
    const float a3 = src[2][0];
    const float b3 = src[2][1];
    const float c3 = src[2][2];
    const float d3 = src[2][3];

    float a3b4_a4b3 = a3 * b4 - a4 * b3 ;
    float a3c4_a4c3 = a3 * c4 - a4 * c3 ;
    float a3d4_a4d3 = a3 * d4 - a4 * d3 ;
    float b3c4_b4c3 = b3 * c4 - b4 * c3 ;
    float b3d4_b4d3 = b3 * d4 - b4 * d3 ;
    float c3d4_c4d3 = c3 * d4 - c4 * d3 ;

    m[0][0]         = (b2 * c3d4_c4d3 - c2 * b3d4_b4d3 + d2 * b3c4_b4c3);
    m[1][0]         =-(a2 * c3d4_c4d3 - c2 * a3d4_a4d3 + d2 * a3c4_a4c3);
    m[2][0]         = (a2 * b3d4_b4d3 - b2 * a3d4_a4d3 + d2 * a3b4_a4b3);
    m[3][0]         =-(a2 * b3c4_b4c3 - b2 * a3c4_a4c3 + c2 * a3b4_a4b3);
    m[0][1]         =-(b1 * c3d4_c4d3 - c1 * b3d4_b4d3 + d1 * b3c4_b4c3);
    m[1][1]         = (a1 * c3d4_c4d3 - c1 * a3d4_a4d3 + d1 * a3c4_a4c3);
    m[2][1]         =-(a1 * b3d4_b4d3 - b1 * a3d4_a4d3 + d1 * a3b4_a4b3);
    m[3][1]         = (a1 * b3c4_b4c3 - b1 * a3c4_a4c3 + c1 * a3b4_a4b3);

    float a2d4_a4d2 = a2 * d4 - a4 * d2;
    float a2b4_a4b2 = a2 * b4 - a4 * b2;
    float a2c4_a4c2 = a2 * c4 - a4 * c2;
    float b2c4_b4c2 = b2 * c4 - b4 * c2;
    float b2d4_b4d2 = b2 * d4 - b4 * d2;
    float c2d4_c4d2 = c2 * d4 - c4 * d2;

    m[0][2]         = (b1 * c2d4_c4d2 - c1 * b2d4_b4d2 + d1 * b2c4_b4c2);
    m[1][2]         =-(a1 * c2d4_c4d2 - c1 * a2d4_a4d2 + d1 * a2c4_a4c2);
    m[2][2]         = (a1 * b2d4_b4d2 - b1 * a2d4_a4d2 + d1 * a2b4_a4b2);
    m[3][2]         =-(a1 * b2c4_b4c2 - b1 * a2c4_a4c2 + c1 * a2b4_a4b2);

    float a2b3_a3b2 = a2 * b3 - a3 * b2;
    float a2c3_a3c2 = a2 * c3 - a3 * c2;
    float a2d3_a3d2 = a2 * d3 - a3 * d2;
    float b2c3_b3c2 = b2 * c3 - b3 * c2;
    float b2d3_b3d2 = b2 * d3 - b3 * d2;
    float c2d3_c3d2 = c2 * d3 - c3 * d2;

    m[0][3]         =-(b1 * c2d3_c3d2 - c1 * b2d3_b3d2 + d1 * b2c3_b3c2);
    m[1][3]         = (a1 * c2d3_c3d2 - c1 * a2d3_a3d2 + d1 * a2c3_a3c2);
    m[2][3]         =-(a1 * b2d3_b3d2 - b1 * a2d3_a3d2 + d1 * a2b3_a3b2);
    m[3][3]         = (a1 * b2c3_b3c2 - b1 * a2c3_a3c2 + c1 * a2b3_a3b2);

    float  det      = a1 * m[0][0] + b1 * m[1][0] + c1 * m[2][0] + d1 * m[3][0];

    if (det != 1.0f)
    {
        UMBRA_ASSERT(det != 0.0f);      //tried to invert a singular matrix

        det = 1.0f/det;
#if UMBRA_ARCH == UMBRA_SPU
        // size-optimized
        for (int i = 0; i < 16; i++)
            ((float*)this->m)[i] *= det;
#else
        m[0][0] = m[0][0]*det;
        m[0][1] = m[0][1]*det;
        m[0][2] = m[0][2]*det;
        m[0][3] = m[0][3]*det;
        m[1][0] = m[1][0]*det;
        m[1][1] = m[1][1]*det;
        m[1][2] = m[1][2]*det;
        m[1][3] = m[1][3]*det;
        m[2][0] = m[2][0]*det;
        m[2][1] = m[2][1]*det;
        m[2][2] = m[2][2]*det;
        m[2][3] = m[2][3]*det;
        m[3][0] = m[3][0]*det;
        m[3][1] = m[3][1]*det;
        m[3][2] = m[3][2]*det;
        m[3][3] = m[3][3]*det;
#endif
    }

    return *this;
}

Matrix4x4& Matrix4x4::invert (const Matrix4x3& src)
{
    Matrix4x3& m = reinterpret_cast<Matrix4x3&>(*this);     // consider 4x3
    m.invert(src);                                          // invert 4x3

    this->m[3][0] = 0;
    this->m[3][1] = 0;
    this->m[3][2] = 0;
    this->m[3][3] = 1.0f;

    return *this;
}


//===================================================================
//                      Matrix3x3
// \review
//===================================================================

// avoid underflow in any of the components...
void Matrix3x3::flushToZero (void)
{
    const double EPSILON = 1e-15;

    for (int j = 0; j < 3; j++)
    for (int i = 0; i < 3; i++)
    if (fabsf(m[j][i]) <= EPSILON)
        m[j][i] = 0.0f;
}

void Matrix3x3::transpose (const Matrix3x3& src)
{
    if (&src == this)
    {
        this->transpose();
        return;
    }

    // note that transposition is _not_ the same as matrix inversion
    // with generic 3x3 matrices (see Matrix3x3::invert)

    m[0][0] = src[0][0];
    m[0][1] = src[1][0];
    m[0][2] = src[2][0];
    m[1][0] = src[0][1];
    m[1][1] = src[1][1];
    m[1][2] = src[2][1];
    m[2][0] = src[0][2];
    m[2][1] = src[1][2];
    m[2][2] = src[2][2];
}

void Matrix3x3::transpose (void)
{
    float tmp;
    tmp = m[0][1]; m[0][1] = m[1][0]; m[1][0] = tmp;
    tmp = m[0][2]; m[0][2] = m[2][0]; m[2][0] = tmp;
    tmp = m[1][2]; m[1][2] = m[2][1]; m[2][1] = tmp;
}

// \review
float Matrix3x3::det (void)
{
    return m[0][0] * (m[1][1]*m[2][2] - m[2][1]*m[1][2]) +
           m[0][1] * (m[2][0]*m[1][2] - m[1][0]*m[2][2]) +
           m[0][2] * (m[1][0]*m[2][1] - m[2][0]*m[1][1]);
}


bool Matrix3x3::operator== (const Matrix3x3& n) const
{
    const float* d = (const float*)&(m[0][0]);
    const float* s = (const float*)&(n[0][0]);
    for (int i = 0; i < 9; i++)
    if (d[i]!=s[i])
        return false;
    return true;
}
// \todo [wili 310502] Should handle &n == this case
Matrix3x3& Matrix3x3::operator*= (const Matrix3x3& n)
{
    UMBRA_ASSERT(&n != this);

    float a,b,c;

    a = m[0][0], b = m[1][0], c = m[2][0];
    m[0][0] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c);
    m[1][0] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c);
    m[2][0] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c);

    a = m[0][1], b = m[1][1], c = m[2][1];
    m[0][1] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c);
    m[1][1] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c);
    m[2][1] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c);

    a = m[0][2], b = m[1][2], c = m[2][2];
    m[0][2] = (float)(n.m[0][0]*a + n.m[0][1]*b + n.m[0][2]*c);
    m[1][2] = (float)(n.m[1][0]*a + n.m[1][1]*b + n.m[1][2]*c);
    m[2][2] = (float)(n.m[2][0]*a + n.m[2][1]*b + n.m[2][2]*c);

    return *this;
}

Matrix3x3& Matrix3x3::operator*= (float f)
{
    for (int j = 0; j < 3; j++)
    for (int i = 0; i < 3; i++)
        m[j][i] *= f;

    return *this;
}


void Matrix3x3::clear (void)
{
    memset (this, 0, sizeof(Matrix3x3));
}

void Matrix3x3::ident (void)
{
    m[0][0] = 1.0f; m[0][1] = 0.0f; m[0][2] = 0.0f;
    m[1][0] = 0.0f; m[1][1] = 1.0f; m[1][2] = 0.0f;
    m[2][0] = 0.0f; m[2][1] = 0.0f; m[2][2] = 1.0f;
}

// \todo [wili 310502] Again, share 4x3 routine?

float Matrix3x3::getMaxScale (void) const
{
    float xs = (m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]);
    float ys = (m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]);
    float zs = (m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]);

    float maxScale = xs;

    if (ys > maxScale)
        maxScale = ys;
    if (zs > maxScale)
        maxScale = zs;

    return (float)sqrtf(maxScale);
}
// \todo [wili 310502] Again, share 4x3 routine?
bool Matrix3x3::isUniform (void) const
{
    double xs = sqrtf(m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]);
    double ys = sqrtf(m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]);
    double zs = sqrtf(m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]);
    return (fabs(xs-ys) <= xs*0.0001 && fabs(xs-zs) <= xs*0.0001);
}

Vector3 Matrix3x3::getScale() const
{
    return Vector3( getColumn(0).length(),  //(m[0][0]*m[0][0] + m[1][0]*m[1][0] + m[2][0]*m[2][0]),
                    getColumn(1).length(),  //(m[0][1]*m[0][1] + m[1][1]*m[1][1] + m[2][1]*m[2][1]),
                    getColumn(2).length() );    //(m[0][2]*m[0][2] + m[1][2]*m[1][2] + m[2][2]*m[2][2]));
}

Matrix3x3& Matrix3x3::scale (const Vector3& v)
{
    setRight( getRight()*v.x );
    setUp   ( getUp()   *v.y );
    setDof  ( getDof()  *v.z );
    return *this;
}


/*----------------------------------------------------------------------*//*!
 * \brief           Matrix3x3 inversion code
 * \note            this == &src is supported
 *//*----------------------------------------------------------------------*/

Matrix3x3& Matrix3x3::invert    (const Matrix3x3& src)
{
    Matrix3x3 t = src;

    float t1122_2112, t2012_1022, t1021_2011;
    float det;

    t1122_2112 = t.m[1][1]*t.m[2][2] - t.m[2][1]*t.m[1][2];
    t2012_1022 = t.m[2][0]*t.m[1][2] - t.m[1][0]*t.m[2][2];
    t1021_2011 = t.m[1][0]*t.m[2][1] - t.m[2][0]*t.m[1][1];

    det = t.m[0][0]*t1122_2112 + t.m[0][1]*t2012_1022 + t.m[0][2]*t1021_2011;
    UMBRA_ASSERT(det != 0.0f);  //tried to invert a singular matrix

    det = 1.0f / det;
    m[0][0] = det * t1122_2112;
    m[1][0] = det * t2012_1022;
    m[2][0] = det * t1021_2011;
    m[0][1] = det * (t.m[2][1]*t.m[0][2] - t.m[0][1]*t.m[2][2]);
    m[1][1] = det * (t.m[0][0]*t.m[2][2] - t.m[2][0]*t.m[0][2]);
    m[2][1] = det * (t.m[2][0]*t.m[0][1] - t.m[0][0]*t.m[2][1]);
    m[0][2] = det * (t.m[0][1]*t.m[1][2] - t.m[1][1]*t.m[0][2]);
    m[1][2] = det * (t.m[1][0]*t.m[0][2] - t.m[0][0]*t.m[1][2]);
    m[2][2] = det * (t.m[0][0]*t.m[1][1] - t.m[1][0]*t.m[0][1]);

    return *this;
}


//===================================================================
//                      Matrix2x2
// \review
//===================================================================

// avoid underflow in any of the components...
void Matrix2x2::flushToZero (void)
{
    const double EPSILON = 1e-15;

    for (int j = 0; j < 2; j++)
    for (int i = 0; i < 2; i++)
    if (fabsf(m[j][i]) <= EPSILON)
        m[j][i] = 0.0f;
}

void Matrix2x2::transpose (const Matrix2x2& src)
{
    if (&src == this)
    {
        this->transpose();
        return;
    }

    // note that transposition is _not_ the same as matrix inversion
    // with generic 2x2 matrices (see Matrix2x2::invert)

    m[0][0] = src[0][0];
    m[0][1] = src[1][0];
    m[1][0] = src[0][1];
    m[1][1] = src[1][1];
}

// \todo [wili 310502] -> inline
void Matrix2x2::transpose (void)
{
    float tmp;
    tmp = m[0][1]; m[0][1] = m[1][0]; m[1][0] = tmp;
}

// \todo [wili 310502] -> inline
float Matrix2x2::det (void)
{
    return m[0][0] * m[1][1] - m[0][1]*m[1][0];
}

bool Matrix2x2::operator== (const Matrix2x2& n) const
{
    const float* d = (const float*)&(m[0][0]);
    const float* s = (const float*)&(n[0][0]);
    for (int i = 0; i < 4; i++)
    if (d[i]!=s[i])
        return false;
    return true;
}

Matrix2x2& Matrix2x2::operator*= (const Matrix2x2& n)
{
    UMBRA_ASSERT(&n != this);

    float a,b;

    a = m[0][0], b = m[1][0];
    m[0][0] = (float)(n.m[0][0]*a + n.m[0][1]*b);
    m[1][0] = (float)(n.m[1][0]*a + n.m[1][1]*b);

    a = m[0][1], b = m[1][1];
    m[0][1] = (float)(n.m[0][0]*a + n.m[0][1]*b);
    m[1][1] = (float)(n.m[1][0]*a + n.m[1][1]*b);

    return *this;
}

// \todo [wili 310502] -> loops generate more code than just unrolling it (then it could also
// be inlined?)
Matrix2x2& Matrix2x2::operator*= (float f)
{
    for (int j = 0; j < 2; j++)
    for (int i = 0; i < 2; i++)
        m[j][i] *= f;

    return *this;
}

void Matrix2x2::clear (void)
{
    memset (this, 0, sizeof(Matrix2x2));
}

void Matrix2x2::ident (void)
{
    m[0][0] = 1.0f; m[0][1] = 0.0f;
    m[1][0] = 0.0f; m[1][1] = 1.0f;
}

float Matrix2x2::getMaxScale (void) const
{
    float xs = (m[0][0]*m[0][0] + m[1][0]*m[1][0]);
    float ys = (m[0][1]*m[0][1] + m[1][1]*m[1][1]);

    float maxScale = xs;

    if (ys > maxScale)
        maxScale = ys;

    return (float)sqrtf(maxScale);
}

bool Matrix2x2::isUniform (void) const
{
    double xs = sqrtf(m[0][0]*m[0][0] + m[1][0]*m[1][0]);
    double ys = sqrtf(m[0][1]*m[0][1] + m[1][1]*m[1][1]);
    return (fabs(xs-ys) <= xs*0.0001);
}

Vector2 Matrix2x2::getScale() const
{
    return Vector2( getColumn(0).length(),  //(m[0][0]*m[0][0] + m[1][0]*m[1][0]),
                    getColumn(1).length()); //(m[0][1]*m[0][1] + m[1][1]*m[1][1]),
}

Matrix2x2& Matrix2x2::scale (const Vector2& v)
{
    setRight( getRight()*v.x );
    setUp   ( getUp()   *v.y );
    return *this;
}


/*----------------------------------------------------------------------*//*!
 * \brief           Matrix2x2 inversion code
 * \note            this == &src is supported
 * \todo [wili 310502] Maybe deal with d == 0 case otherwise?
 *//*----------------------------------------------------------------------*/

Matrix2x2& Matrix2x2::invert    (const Matrix2x2& src)
{
    Matrix2x2 t = src;

    float d = t.det();
    UMBRA_ASSERT(d != 0.0f);    //tried to invert a singular matrix
    d = 1.0f / d;

    m[0][0] = t.m[1][1] * d;
    m[0][1] = -t.m[0][1] * d;
    m[1][0] = -t.m[1][0] * d;
    m[1][1] = t.m[0][0] * d;

    return *this;
}

}