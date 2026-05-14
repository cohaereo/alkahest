#pragma once
#ifndef __UMBRASIMD_H
#define __UMBRASIMD_H

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   SIMD vector instruction wrapper
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include <math.h>
#include <float.h>

#if !defined(UMBRA_DISABLE_SIMD_CODE)

/* \todo [antti 17.4.2012]: fix SIMD with mingw gcc */
/* \todo [jasin] enable SSE on NaCL once we get compiled with -msse2 */
#ifndef UMBRA_SIMD_SSE
#if UMBRA_ARCH == UMBRA_X86 && !defined(__flash__) && (UMBRA_OS != UMBRA_NACL)
#   define UMBRA_SIMD_SSE 1
#endif
#endif

// \todo [jasin] find out if the MS compiler #defines something like __ARM_NEON__,
// currently just assuming all Windows-ARM platforms support it.
// \todo fix android build
#ifndef UMBRA_SIMD_NEON
#   if (UMBRA_ARCH == UMBRA_ARM &&  \
        !defined(ANDROID)       &&  \
        (defined(__ARM_NEON__) || (UMBRA_OS == UMBRA_WINDOWS) || (UMBRA_OS == UMBRA_METRO)))
#           define UMBRA_SIMD_NEON 1
#endif
#endif


#ifdef UMBRA_SIMD_SSE
#   define UMBRA_SIMD_CODE
#   include "umbraSIMD_SSE.hpp"
#endif

#ifdef UMBRA_SIMD_NEON
#   define UMBRA_SIMD_CODE
#   include "neon/umbraSIMD_NEON.hpp"
#endif


#if UMBRA_OS == UMBRA_XBOX360 // \todo how about UMBRA_ARCH == UMBRA_PPC
#   define UMBRA_SIMD_CODE
#   include "xbox360/umbraSIMD_XBOX360.hpp"
#endif

#if UMBRA_OS == UMBRA_PS3
#if UMBRA_ARCH == UMBRA_PPC
#   define UMBRA_SIMD_CODE
#   include "ps3/umbraSIMD_PS3PPU.hpp"
#endif

#if UMBRA_ARCH == UMBRA_SPU
#   define UMBRA_SIMD_CODE
#   include "ps3/umbraSIMD_PS3SPU.hpp"
#endif
#endif


#endif // !UMBRA_DISABLE_SIMD_CODE

namespace Umbra
{

/*----------------------------------------------------------------------*//*!
 * \brief   Class for disabling FPU exceptions (FPU + SSE units)
 *//*----------------------------------------------------------------------*/

class FPUExceptionMask
{
public:
    FPUExceptionMask()
    {
#if (UMBRA_OS == UMBRA_WINDOWS)
        _clearfp();
        m_oldFPUMask = _controlfp(_EM_INVALID|_EM_OVERFLOW|_EM_UNDERFLOW|_EM_INEXACT|_EM_ZERODIVIDE, _MCW_EM);
#endif

#if defined(UMBRA_SIMD_SSE)
        m_oldSSEMask = _MM_GET_EXCEPTION_MASK();
        _MM_SET_EXCEPTION_MASK(_MM_MASK_MASK);
#endif
    }

    ~FPUExceptionMask()
    {
#if (UMBRA_OS == UMBRA_WINDOWS)
        _clearfp();
        _controlfp(m_oldFPUMask, _MCW_EM);
#endif

#if defined(UMBRA_SIMD_SSE)
        _MM_SET_EXCEPTION_MASK(m_oldSSEMask);
#endif
    }

private:

#if (UMBRA_OS == UMBRA_WINDOWS)
    int m_oldFPUMask;
#endif

#if defined(UMBRA_SIMD_SSE)
    int m_oldSSEMask;
#endif
};

} // namespace Umbra

#define UMBRA_DISABLE_FLOATING_POINT_EXCEPTIONS_TEMPORARY ::Umbra::FPUExceptionMask exceptionMask;

#ifndef UMBRA_SIMD_CODE

// emulated SIMD operations

namespace Umbra
{

class SIMDRegister
{
public:
    SIMDRegister(void)                              { u.f[0] = 0.f; u.f[1] = 0.f; u.f[2] = 0.f; u.f[3] = 0.f; }
    SIMDRegister(float val)                         { u.f[0] = val; u.f[1] = val; u.f[2] = val; u.f[3] = val; }
    SIMDRegister(int val)                           { u.i[0] = val; u.i[1] = val; u.i[2] = val; u.i[3] = val; }
    SIMDRegister(unsigned short val)                { for (int i = 0; i < 8; i++) u.us[i] = val; }
    SIMDRegister(unsigned val)                      { u.i[0] = val; u.i[1] = val; u.i[2] = val; u.i[3] = val; }
    SIMDRegister(const SIMDRegister& r)             { u.f[0] = r.x(); u.f[1] = r.y(); u.f[2] = r.z(); u.f[3] = r.w(); }
    SIMDRegister(float x, float y, float z, float w) { u.f[0] = x; u.f[1] = y; u.f[2] = z; u.f[3] = w; }
    SIMDRegister(int x, int y, int z, int w)        { u.i[0] = x; u.i[1] = y; u.i[2] = z; u.i[3] = w; }
    SIMDRegister(bool x, bool y, bool z, bool w)    { store(0, x); store(1, y), store(2, z); store(3, w); }
    SIMDRegister(const Vector4& v)                  { u.f[0] = v.x; u.f[1] = v.y; u.f[2] = v.z; u.f[3] = v.w; }
    SIMDRegister(const Vector3& v, float w)         { u.f[0] = v.x; u.f[1] = v.y; u.f[2] = v.z; u.f[3] = w; }
    SIMDRegister(const Vector4& v, float w)         { u.f[0] = v.x; u.f[1] = v.y; u.f[2] = v.z; u.f[3] = w; }

    float   x(void) const   { return u.f[0]; }
    float   y(void) const   { return u.f[1]; }
    float   z(void) const   { return u.f[2]; }
    float   w(void) const   { return u.f[3]; }
    int     ix(void) const  { return u.i[0]; }
    int     iy(void) const  { return u.i[1]; }
    int     iz(void) const  { return u.i[2]; }
    int     iw(void) const  { return u.i[3]; }

    union
    {
        float           f[4];
        int             i[4];
        unsigned short  us[8];
        unsigned char   c[16];
    } u;

private:
    void store (int idx, bool b) { u.i[idx] = b ? 0xFFFFFFFF : 0; }
};

UMBRA_INLINE SIMDRegister operator-  (const SIMDRegister& v)  { return SIMDRegister(-v.x(), -v.y(), -v.z(), -v.w()); }
UMBRA_INLINE SIMDRegister operator~  (const SIMDRegister& v)  { return SIMDRegister(~v.ix(), ~v.iy(), ~v.iz(), ~v.iw()); }
UMBRA_INLINE SIMDRegister operator+  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.x()+v2.x(), v1.y()+v2.y(), v1.z()+v2.z(), v1.w()+v2.w()); }
UMBRA_INLINE SIMDRegister operator-  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.x()-v2.x(), v1.y()-v2.y(), v1.z()-v2.z(), v1.w()-v2.w()); }
UMBRA_INLINE SIMDRegister operator*  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.x()*v2.x(), v1.y()*v2.y(), v1.z()*v2.z(), v1.w()*v2.w()); }
UMBRA_INLINE SIMDRegister operator>  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.x()>v2.x(), v1.y()>v2.y(), v1.z()>v2.z(), v1.w()>v2.w()); }
UMBRA_INLINE SIMDRegister operator<  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.x()<v2.x(), v1.y()<v2.y(), v1.z()<v2.z(), v1.w()<v2.w()); }
UMBRA_INLINE SIMDRegister operator>= (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.x()>=v2.x(), v1.y()>=v2.y(), v1.z()>=v2.z(), v1.w()>=v2.w()); }
UMBRA_INLINE SIMDRegister operator== (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.x()==v2.x(), v1.y()==v2.y(), v1.z()==v2.z(), v1.w()==v2.w()); }
UMBRA_INLINE SIMDRegister operator&  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.ix()&v2.ix(), v1.iy()&v2.iy(), v1.iz()&v2.iz(), v1.iw()&v2.iw()); }
UMBRA_INLINE SIMDRegister operator|  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.ix()|v2.ix(), v1.iy()|v2.iy(), v1.iz()|v2.iz(), v1.iw()|v2.iw()); }
UMBRA_INLINE SIMDRegister operator^  (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(v1.ix()^v2.ix(), v1.iy()^v2.iy(), v1.iz()^v2.iz(), v1.iw()^v2.iw()); }
UMBRA_INLINE SIMDRegister maxf       (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(max2(v1.x(),v2.x()), max2(v1.y(),v2.y()), max2(v1.z(),v2.z()), max2(v1.w(),v2.w())); }
UMBRA_INLINE SIMDRegister minf       (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(min2(v1.x(),v2.x()), min2(v1.y(),v2.y()), min2(v1.z(),v2.z()), min2(v1.w(),v2.w())); }
UMBRA_INLINE SIMDRegister maxi       (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(max2(v1.ix(),v2.ix()), max2(v1.iy(),v2.iy()), max2(v1.iz(),v2.iz()), max2(v1.iw(),v2.iw())); }
UMBRA_INLINE SIMDRegister mini       (const SIMDRegister& v1, const SIMDRegister& v2)  { return SIMDRegister(min2(v1.ix(),v2.ix()), min2(v1.iy(),v2.iy()), min2(v1.iz(),v2.iz()), min2(v1.iw(),v2.iw())); }
UMBRA_INLINE SIMDRegister max16u     (const SIMDRegister& v1, const SIMDRegister& v2)  { SIMDRegister r; for (int i = 0; i < 8; i++) r.u.us[i] = (UINT16)max2(v1.u.us[i], v2.u.us[i]); return r; }
UMBRA_INLINE SIMDRegister equals16   (const SIMDRegister& v1, const SIMDRegister& v2)  { SIMDRegister r; for (int i = 0; i < 8; i++) r.u.us[i] = (v1.u.us[i] == v2.u.us[i]) ? 0xFFFF : 0; return r; }

// \todo [petri] Potentially unsafe not to parenthesize all macro params, or make these inline funcs.
#define SIMDRegister32              SIMDRegister
#define SIMDZero()                  SIMDRegister()
#define SIMDZero32()                SIMDRegister()
#define SIMDOne()                   SIMDRegister(1.f)
#define SIMDMinusOne()              SIMDRegister(-1.f)
#define SIMDMaskW()                 SIMDRegister(0, 0, 0, 0xFFFFFFFF)
#define SIMDNegate(a)               (-a)
#define SIMDAdd(a,b)                (a + b)
#define SIMDSub(a,b)                (a - b)
#define SIMDAdd32(a,b)              (a + b)
#define SIMDSub32(a,b)              (a - b)
#define SIMDMultiply(a,b)           (a * b)
#define SIMDMultiplyAdd(a,b,c)      ((a * b) + c)
#define SIMDMax                     maxf
#define SIMDMin                     minf
#define SIMDSqrt(v)                 SIMDRegister(sqrtf(v.x()), sqrtf(v.y()), sqrtf(v.z()), sqrtf(v.w()))
#define SIMDMax32(a,b)              maxi(a, b)
#define SIMDMin32(a,b)              mini(a, b)
#define SIMDMax16u(a,b)             max16u(a, b)
#define SIMDCompareGT(a,b)          (a > b)
#define SIMDCompareEQ(a,b)          (a == b)
#define SIMDCompareGE(a,b)          (a >= b)
#define SIMDCompareGT32(a,b)        SIMDCompareGT(a,b)
#define SIMDCompareEQ32(a,b)        SIMDCompareEQ(a,b)
#define SIMDCompareEQ16(a,b)        equals16(a,b)
#define SIMDComparGE32(a,b)         SIMDCompareGE(a,b)
#define SIMDReplicate(a, i)         SIMDRegister(a.u.f[i], a.u.f[i], a.u.f[i], a.u.f[i])
#define SIMDReplicate32(a, i)       SIMDReplicate(a, i)
#define SIMDReplicate16(a, i)       SIMDRegister(a.u.us[i])
#define SIMDBitwiseAnd(a,b)         (a & b)
#define SIMDBitwiseAndNot(a,b)      (a & ~b)
#define SIMDBitwiseAndNot32(a,b)    (a & ~b)
#define SIMDBitwiseOr(a,b)          (a | b)
#define SIMDBitwiseEor(a,b)         (a ^ b)
#define SIMDBitwiseAnd32(a,b)       SIMDBitwiseAnd(a, b)
#define SIMDBitwiseOr32(a,b)        SIMDBitwiseOr(a, b)
#define SIMDBitwiseOrTestAny(a, b)  SIMDNotZero32(a | b)
#define SIMDCompareGTTestAny(a, b)  SIMDNotZero32(a > b)
#define SIMDCompareGTTestAny32(a, b) SIMDNotZero32(a > b)
#define SIMDLoadAlignedW0(v)        SIMDLoadW0(v)
#define SIMDLoadAlignedW1(v)        SIMDLoadW1(v)
#define SIMDLoadAligned32(v)        SIMDLoad32(v)
#define SIMDStoreAligned32(a, v)    SIMDStore32(a, v)
UMBRA_INLINE SIMDRegister SIMDLoadW0 (const Vector3& v) { return SIMDRegister(v, 0.f); }
UMBRA_INLINE SIMDRegister SIMDLoadW1 (const Vector3& v) { return SIMDRegister(v, 1.f); }
UMBRA_INLINE SIMDRegister SIMDLoadW0 (const Vector4& v) { return SIMDRegister(v.xyz(), 0.f); }
UMBRA_INLINE SIMDRegister SIMDLoadW1 (const Vector4& v) { return SIMDRegister(v.xyz(), 1.f); }
#define SIMDIntToFloat(v)           SIMDRegister((float)v.ix(), (float)v.iy(), (float)v.iz(), (float)v.iw())
#define SIMDFloatToInt(v)           SIMDRegister((int)v.x(), (int)v.y(), (int)v.z(), (int)v.w())
#define SIMDFloatToBitPattern(v)    (v)
#define SIMDBitPatternToFloat(v)    (v)
#define SIMDSelect32(a,b,c)         SIMDSelect(a,b,c)
#define SIMDSaveState()             0
#define SIMDRestoreState(a)         ((void)a)
#define SIMDMaskXY()                SIMDRegister(true, true, false, false)
#define SIMDMaskXYZ()               SIMDRegister(true, true, true, false)
#define SIMDLeftShift32(a,n)        SIMDRegister(a.ix() << (n), a.iy() << (n), a.iz() << (n), a.iw() << (n))

#define SIMDMergeLow32(a,b)         SIMDRegister(a.ix(), b.ix(), a.iy(), b.iy())
#define SIMDMergeHigh32(a,b)        SIMDRegister(a.iz(), b.iz(), a.iw(), b.iw())
#define SIMDMergeLow(a,b)           SIMDRegister(a.ix(), b.ix(), a.iy(), b.iy())
#define SIMDMergeHigh(a,b)          SIMDRegister(a.iz(), b.iz(), a.iw(), b.iw())
#define SIMDHighToLow(a)            SIMDRegister(a.iz(), a.iw(), a.iz(), a.iw())

#define SIMDShuffle32_A0B0A0B0(a,b) SIMDRegister(a.x(), b.x(), a.x(), b.x())
#define SIMDShuffle32_A2B2A2B2(a,b) SIMDRegister(a.z(), b.z(), a.z(), b.z())
#define SIMDShuffle32_A0A0B0B0(a,b) SIMDRegister(a.x(), a.x(), b.x(), b.x())
#define SIMDShuffle32_A1A1B1B1(a,b) SIMDRegister(a.y(), a.y(), b.y(), b.y())
#define SIMDShuffle32_A2A2B2B2(a,b) SIMDRegister(a.z(), a.z(), b.z(), b.z())
#define SIMDShuffle32_A3A3B3B3(a,b) SIMDRegister(a.w(), a.w(), b.w(), b.w())
#define SIMDShuffle32_A0A2B0B2(a,b) SIMDRegister(a.x(), a.z(), b.x(), b.z())
#define SIMDShuffle_A0B0A0B0(a,b)   SIMDRegister(a.x(), b.x(), a.x(), b.x())
#define SIMDShuffle_A2B2A2B2(a,b)   SIMDRegister(a.z(), b.z(), a.z(), b.z())
#define SIMDShuffle_A0A0B0B0(a,b)   SIMDRegister(a.x(), a.x(), b.x(), b.x())
#define SIMDShuffle_A1A1B1B1(a,b)   SIMDRegister(a.y(), a.y(), b.y(), b.y())
#define SIMDShuffle_A2A2B2B2(a,b)   SIMDRegister(a.z(), a.z(), b.z(), b.z())
#define SIMDShuffle_A3A3B3B3(a,b)   SIMDRegister(a.w(), a.w(), b.w(), b.w())
#define SIMDShuffle_A0A2B0B2(a,b)   SIMDRegister(a.x(), a.z(), b.x(), b.z())
#define SIMDShuffle_A0B0A1B1(a,b)   SIMDRegister(a.x(), b.x(), a.y(), b.y())
#define SIMDShuffle_A0A1B0B1(a,b)   SIMDRegister(a.x(), a.y(), b.x(), b.y())

template <int a, int b, int c, int d>
UMBRA_INLINE SIMDRegister SIMDShuffle(const SIMDRegister& x) { return SIMDRegister(x.u.f[a], x.u.f[b], x.u.f[c], x.u.f[d]); }
UMBRA_INLINE int SIMDExtractSignBits(const SIMDRegister& a) { return (a.ix()<0?1:0) | (a.iy()<0?2:0) | (a.iz()<0?4:0) | (a.iw()<0?8:0); }
#define SIMDExtract16Signs(a,b,c,d) ((SIMDExtractSignBits(a) << 0) | \
                                     (SIMDExtractSignBits(b) << 4) | \
                                     (SIMDExtractSignBits(c) << 8) | \
                                     (SIMDExtractSignBits(d) << 12))

enum { FullNegativeMask = 0xFu };
UMBRA_INLINE void SIMDWriteNegativeMask(int& result, const SIMDRegister& a) { result = SIMDExtractSignBits(a); }
UMBRA_INLINE void SIMDWriteAnyMask(int& result, const SIMDRegister& a) { result = SIMDExtractSignBits(a); }

UMBRA_INLINE SIMDRegister SIMDLoad(const Vector4& v) { return SIMDRegister(v.x, v.y, v.z, v.w); }
UMBRA_INLINE SIMDRegister SIMDLoad(float x, float y, float z, float w) { return SIMDRegister(x, y, z, w); }
UMBRA_INLINE SIMDRegister SIMDLoadW0 (float v) { return SIMDLoad(v, v, v, 0.f); }
UMBRA_INLINE SIMDRegister SIMDLoadXXYY (float x, float y) { return SIMDLoad(x, x, y, y); }
UMBRA_INLINE SIMDRegister SIMDLoad(int x, int y, int z, int w) { return SIMDRegister(x, y, z, w); }
UMBRA_INLINE SIMDRegister SIMDLoad(float v) { return SIMDRegister(v); }
UMBRA_INLINE SIMDRegister SIMDLoad(const float* a) { return SIMDRegister(a[0], a[1], a[2], a[3]); }
UMBRA_INLINE SIMDRegister SIMDLoad32(int x, int y, int z, int w) { return SIMDRegister(x, y, z, w); }
UMBRA_INLINE SIMDRegister SIMDLoad32(int v) { return SIMDRegister(v); }
UMBRA_INLINE SIMDRegister SIMDLoad32(const int* a) { return SIMDRegister(a[0], a[1], a[2], a[3]); }
UMBRA_INLINE SIMDRegister SIMDLoadAligned (const float* ptr) { return SIMDLoad(ptr); }
UMBRA_INLINE void SIMDStore32 (const SIMDRegister& a, int* v) { v[0] = a.ix(); v[1] = a.iy(); v[2] = a.iz(); v[3] = a.iw(); }
UMBRA_INLINE void SIMDStore32 (const SIMDRegister& a, Vector3i& v) { v.i = a.ix(); v.j = a.iy(); v.k = a.iz(); }
UMBRA_INLINE void SIMDStore (const SIMDRegister& a, float& f) { f = a.x(); }
UMBRA_INLINE void SIMDStore(const SIMDRegister& r, float* p) { p[0] = r.x(); p[1] = r.y(); p[2] = r.z(); p[3] = r.w(); }
UMBRA_INLINE void SIMDStore(const SIMDRegister& r, Vector3& v) { v[0] = r.x(); v[1] = r.y(); v[2] = r.z(); }
UMBRA_INLINE void SIMDStoreAligned(const SIMDRegister& r, float* p) { UMBRA_ASSERT(is128Aligned(p)); SIMDStore(r, p); }

UMBRA_INLINE SIMDRegister SIMDAbs (const SIMDRegister& a)
{
    return SIMDRegister(fabsf(a.x()), fabsf(a.y()), fabsf(a.z()), fabsf(a.w()));
}

UMBRA_INLINE void SIMDIntFloor (const SIMDRegister& a, int& res)
{
    res = (int)a.x();
}

UMBRA_INLINE bool SIMDNotZero32 (const SIMDRegister& a)
{
    for (int i = 0; i < 4; i++)
    {
        if (a.u.i[i] != 0)
            return true;
    }
    return false;
}
#define SIMDNotZero(x) SIMDNotZero32(x)

UMBRA_INLINE SIMDRegister SIMDSelect (const SIMDRegister& a, const SIMDRegister& b, const SIMDRegister& c)
{
    SIMDRegister res;
    for (int i = 0; i < 4; i++)
        res.u.i[i] = (c.u.i[i] & b.u.i[i]) | (~c.u.i[i] & a.u.i[i]);
    return res;
}

UMBRA_INLINE SIMDRegister SIMDDot4 (const SIMDRegister& a, const SIMDRegister& b)
{
    SIMDRegister ab = SIMDMultiply(a,b);
    SIMDRegister r;
    r = SIMDAdd(SIMDReplicate(ab,0), SIMDReplicate(ab,1));
    r = SIMDAdd(r, SIMDReplicate(ab,2));
    r = SIMDAdd(r, SIMDReplicate(ab,3));
    return r;
}

UMBRA_INLINE SIMDRegister SIMDReciprocal (const SIMDRegister& a)
{
    return SIMDRegister(
        a.x() == 0.f ? 1.f : 1 / a.x(),
        a.y() == 0.f ? 1.f : 1 / a.y(),
        a.z() == 0.f ? 1.f : 1 / a.z(),
        a.w() == 0.f ? 1.f : 1 / a.w());
}

} // Umbra

#endif

namespace Umbra
{

UMBRA_FORCE_INLINE SIMDRegister     SIMDReciprocalAccurate(const SIMDRegister& x)
{
    // perform one newton-raphson iteration to get 23-bit accurate result
    SIMDRegister rcp  = SIMDReciprocal(x);
    SIMDRegister tmp1 = SIMDMultiply(SIMDMultiply(x, rcp), rcp);    // x * rcp(x)^2
    SIMDRegister rcp2 = SIMDAdd(rcp, rcp);                          // 2.0 * rcp(x)
    return SIMDSub(rcp2, tmp1);                                     // 2*rcp(x) - x*rcp(x)^2
}

#if !defined(UMBRA_SIMD_NEON)
UMBRA_FORCE_INLINE SIMDRegister32 SIMDClamp32 (SIMDRegister32 minmax, SIMDRegister32 bounds)
{
    // max operation for first 2 elements, min for the latter

    SIMDRegister32 clampedMin = SIMDMax32(minmax, bounds);
    SIMDRegister32 clampedMax = SIMDMin32(minmax, bounds);
    return SIMDFloatToBitPattern(SIMDSelect(SIMDBitPatternToFloat(clampedMax), SIMDBitPatternToFloat(clampedMin), SIMDMaskXY()));
}

UMBRA_FORCE_INLINE void SIMDTranspose(
    SIMDRegister& outX, SIMDRegister& outY, SIMDRegister& outZ, SIMDRegister& outW,
    const SIMDRegister& inA, const SIMDRegister& inB, const SIMDRegister& inC, const SIMDRegister& inD)
{
    SIMDRegister acXY = SIMDMergeLow (inA, inC);
    SIMDRegister bdXY = SIMDMergeLow (inB, inD);
    SIMDRegister acZW = SIMDMergeHigh(inA, inC);
    SIMDRegister bdZW = SIMDMergeHigh(inB, inD);
    outX = SIMDMergeLow (acXY, bdXY);
    outY = SIMDMergeHigh(acXY, bdXY);
    outZ = SIMDMergeLow(acZW, bdZW);
    outW = SIMDMergeHigh(acZW, bdZW);
}

UMBRA_FORCE_INLINE void SIMDWriteNegativeMask2(int& result1, int& result2, SIMDRegister a, SIMDRegister b)
{
    SIMDWriteNegativeMask(result1, a);
    SIMDWriteNegativeMask(result2, b);
}

#endif

UMBRA_FORCE_INLINE SIMDRegister SIMDLoadAligned(const Vector4& vec)
{
    return *((SIMDRegister*)&vec);
}

UMBRA_FORCE_INLINE void SIMDStoreAligned(SIMDRegister r, const Vector4& vec)
{
    *((SIMDRegister*)&vec) = r;
}


} // Umbra

#endif // __UMBRASIMD_H
