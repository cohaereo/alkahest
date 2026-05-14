
#pragma once
#ifndef __UMBRASIMD_XBOX360_H
#define __UMBRASIMD_XBOX360_H

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

#if defined(UMBRA_SIMD_CODE)

#include <vectorintrinsics.h>

namespace Umbra
{

typedef __vector4    SIMDRegister;
typedef SIMDRegister SIMDRegister32;
#define SIMDZero()                  __vspltisw(0)
#define SIMDOne()                   __vspltw(UMBRA_SIMD_CONST_XBOX360, 1)
#define SIMDMinusOne()              __vspltw(UMBRA_SIMD_CONST_XBOX360, 2)
#define SIMDAdd(a,b)                __vaddfp(a,b)
#define SIMDSub(a,b)                __vsubfp(a,b)
#define SIMDMultiply(a,b)           __vmulfp(a,b)
#define SIMDCompareGT(a,b)          __vcmpgtfp(a,b)
#define SIMDCompareLT(a,b)          SIMDCompareGT(b,a)
#define SIMDCompareGE(a,b)          __vcmpgefp(a,b)
#define SIMDCompareEQ(a,b)          __vcmpeqfp(a,b)
#define SIMDBitwiseAnd(a,b)         __vand(a,b)
#define SIMDBitwiseOr(a,b)          __vor(a,b)
#define SIMDBitwiseEor(a,b)         __vxor(a,b)
#define SIMDBitwiseAndNot(a,b)      __vandc(a,b)
#define SIMDReciprocal(a)           __vrefp(a)
#define SIMDMin                     __vminfp
#define SIMDMin32(a,b)              __vminsw(a,b)
#define SIMDMax                     __vmaxfp
#define SIMDMax16u(a,b)             __vmaxuh(a,b)
#define SIMDMax32(a,b)              __vmaxsw(a,b)
#define SIMDReplicate(a, i)         __vspltw(a,i)
#define SIMDMaskXYZW()              __vspltisw(-1)
#define SIMDMaskXYZ()               __vrlimi(SIMDZero(), SIMDMaskXYZW(), 0xE, 0)
#define SIMDMaskXY()                __vrlimi(SIMDZero(), SIMDMaskXYZW(), 0xC, 0)
#define SIMDMaskW()                 __vrlimi(SIMDZero(), SIMDMaskXYZW(), 0x1, 0)
#define SIMDSignMask()              __vspltw(UMBRA_SIMD_CONST_XBOX360, 0)
#define SIMDShufflePattern()        __vspltw(UMBRA_SIMD_CONST_XBOX360, 3)
#define SIMDMultiplyAdd(a,b,c)      __vmaddfp(a,b,c)
#define SIMDDot3(a,b)               __vmsum3fp(a,b)
#define SIMDDot4(a,b)               __vmsum4fp(a,b)
#define SIMDSelect(a,b,c)           __vsel(a,b,c)
#define SIMDReciprocalSqrt(a)       __vrsqrtefp(a)
#define SIMDAdd32(a,b)              __vaddsws(a,b)
#define SIMDStoreAligned32(a,dst)   __stvx((a),(dst),0)
#define SIMDLoadAligned32(p)        SIMDLoadAligned(p)
#define SIMDCompareGT32(a,b)        __vcmpgtsw(a,b)
#define SIMDCompareEQ32(a,b)        __vcmpequw(a,b)
#define SIMDCompareEQ16(a,b)        __vcmpequh(a,b)
#define SIMDBitwiseOr32(a,b)        SIMDBitwiseOr(a,b)
#define SIMDBitwiseAnd32(a,b)       SIMDBitwiseAnd(a,b)
#define SIMDBitwiseEor32(a,b)       SIMDBitwiseEor(a,b)
#define SIMDBitwiseAndNot32(a,b)    SIMDBitwiseAndNot(a,b)
#define SIMDLeftShift32(a,n)        __vslw(a,SIMDLoad32(n))
#define SIMDZero32()                SIMDZero()
#define SIMDSelect32(a,b,c)         SIMDSelect(a,b,c)
#define SIMDReplicate32(v, i)       SIMDReplicate(v,i)
#define SIMDReplicate16(v, i)       __vsplth((v), i)
#define SIMDNotZero32(a)            SIMDNotZero(a)
#define SIMDIntToFloat(a)           __vcfsx(a, 0)
#define SIMDFloatToInt(a)           __vctsxs(a, 0)
#define SIMDFloatToBitPattern(a)    a
#define SIMDBitPatternToFloat(a)    a
#define SIMDSaveState()             0
#define SIMDRestoreState(a)         ((void)a)

#define SIMDMergeLow32(a,b)         __vmrghw(a,b)
#define SIMDMergeHigh32(a,b)        __vmrglw(a,b)
#define SIMDMergeLow(a,b)           __vmrghw(a,b)
#define SIMDMergeHigh(a,b)          __vmrglw(a,b)
#define SIMDHighToLow(a)            SIMDShuffle<2,3,2,3>(a)

#define SIMDShuffle_A0A2B0B2(a,b)   SIMDShuffle<0,2,4,6>(a,b)
#define SIMDShuffle_A0A1B0B1(a,b)   SIMDShuffle<0,1,4,5>(a,b)
#define SIMDShuffle_A0B0A0B0(a,b)   __vmrglw(__vspltw(a, 0), __vspltw(b, 0))
#define SIMDShuffle_A2B2A2B2(a,b)   __vmrglw(__vspltw(a, 2), __vspltw(b, 2))
#define SIMDShuffle_A0B0A1B1(a,b)   __vmrghw(a,b)

// const values: -0, 1, -1, 0x0004080c
extern const SIMDRegister UMBRA_SIMD_CONST_XBOX360;
// pixel masks for rasterizer
extern const SIMDRegister UMBRA_SIMD_PIXEL_MASKS_XBOX360;

template <int x, int y, int z, int w>
UMBRA_FORCE_INLINE SIMDRegister     SIMDShuffle(const SIMDRegister& a, const SIMDRegister& b)
{
    struct AlignedPattern
    {
        int UMBRA_ATTRIBUTE_ALIGNED16(x); int y,z,w;
    };

    static const AlignedPattern pattern = {
        0x00010203 + x*0x04040404,
        0x00010203 + y*0x04040404,
        0x00010203 + z*0x04040404,
        0x00010203 + w*0x04040404
    };

    return  __vperm(a, b, __lvx((void*)&pattern, 0));
}
template <int x, int y, int z, int w>
UMBRA_FORCE_INLINE SIMDRegister     SIMDShuffle(const SIMDRegister& a)
{
    const int swizzle = (x<<6) | (y<<4) | (z<<2) | w;
    return __vpermwi(a, swizzle);
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float x, float y, float z, float w)
{
    union { SIMDRegister r; float f[4]; } result = { x,y,z,w };
    return result.r;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(int x, int y, int z, int w)
{
    union
    {
        SIMDRegister r;
        int i[4];
    } result;

    result.i[0] = x;
    result.i[1] = y;
    result.i[2] = z;
    result.i[3] = w;

    return result.r;
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float* p)                              { return __vor(__lvlx(p, 0), __lvrx(p, 16)); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(const Vector4& v)                      { return SIMDLoad((float*)&v.x); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAligned(const float* p)                 { return __lvx(p, 0); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAligned(const int* p)                   { return __lvx((void*)p, 0); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0(const Vector3& v_)                   { Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, v) = Vector4(v_, 0.f); return SIMDLoadAligned((float*)&v.x); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector3& v_)                   { Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, v) = Vector4(v_, 1.f); return SIMDLoadAligned((float*)&v.x); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0 (float v) { return SIMDLoad(v, v, v, 0.f); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadXXYY (float x, float y) { return SIMDLoad(x, x, y, y); }
UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a, int b, int c, int d)
{
    return SIMDLoad(a,b,c,d);
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a)
{
    int val = a;
    return __vspltw(__lvlx(&val, 0), 0);
}

UMBRA_FORCE_INLINE SIMDRegister SIMDLoad(float f)
{
    float val = f;
    return __vspltw(__lvlx(&val, 0), 0);
}

UMBRA_FORCE_INLINE void             SIMDStore32(const SIMDRegister32& r, Vector3i& v)
{
    Vector4i UMBRA_ATTRIBUTE_ALIGNED16(tmp);
    __stvx(r, &tmp, 0);
    v.i = tmp.i;
    v.j = tmp.j;
    v.k = tmp.k;
}
UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, float* dst)
{
    __storeunalignedvector(r,dst);
}
UMBRA_FORCE_INLINE void             SIMDStoreAligned(const SIMDRegister& r, float* dst)
{
    __stvx(r,dst,0);
}

UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, float& dst)
{
    // \note [petri] The way this function is used currently, the replicate should not be needed (i.e., the value is already splatted).
    __stvewx(SIMDReplicate(r, 0), &dst, 0);
}

UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, Vector4& v)
{
    SIMDStore(r, &v.x);
}
UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, Vector3& v)
{
    Vector4 tmp;
    SIMDStore(r, tmp);
    v.x = tmp.x;
    v.y = tmp.y;
    v.z = tmp.z;
}
UMBRA_FORCE_INLINE void             SIMDIntFloor(const SIMDRegister& a, int& res)
{
    __stvewx(__vctsxs(a, 0), &res, 0);
}
UMBRA_FORCE_INLINE int              SIMDIntFloor(const SIMDRegister& a)
{
    int res;
    __stvewx(__vctsxs(a, 0), &res, 0);
    return res;
}
UMBRA_FORCE_INLINE int              SIMDBitwiseOrTestAny(const SIMDRegister& a, const SIMDRegister& b)
{
    unsigned int compareResult;
    __vcmpequwR(SIMDBitwiseOr(a,b), SIMDZero(), &compareResult);
    return (compareResult&(1<<7)) == 0;
}
UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny(const SIMDRegister& a, const SIMDRegister& b)
{
    unsigned int compareResult;
    __vcmpgtfpR(a, b, &compareResult);
    return (compareResult&(1<<5)) == 0;
}
UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny32(const SIMDRegister32& a, const SIMDRegister32& b)
{
    unsigned int compareResult;
    __vcmpgtswR(a, b, &compareResult);
    return (compareResult&(1<<5)) == 0;
}
UMBRA_FORCE_INLINE int              SIMDNotZero(const SIMDRegister& a)
{
    unsigned int compareResult;
    __vcmpequwR(a, SIMDZero(), &compareResult);
    return (compareResult&(1<<7)) == 0;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDTransform(const SIMDRegister* matrix, const SIMDRegister& vector)
{
    SIMDRegister r;

    r = SIMDMultiply(matrix[0], SIMDReplicate(vector,0));
    r = SIMDMultiplyAdd(matrix[1], SIMDReplicate(vector,1), r);
    r = SIMDMultiplyAdd(matrix[2], SIMDReplicate(vector,2), r);
    r = SIMDMultiplyAdd(matrix[3], SIMDReplicate(vector,3), r);

    return r;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDNegate(const SIMDRegister& a)
{
    return SIMDBitwiseEor(a, SIMDSignMask());
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDAbs(const SIMDRegister& a)
{
    return SIMDBitwiseAndNot(a, SIMDSignMask());
}

UMBRA_FORCE_INLINE int SIMDExtract16Signs (SIMDRegister scan0Mask,
                                           SIMDRegister scan1Mask,
                                           SIMDRegister scan2Mask,
                                           SIMDRegister scan3Mask)
{
    scan0Mask = __vand(__vsraw(scan0Mask, __vspltisw(-1)), UMBRA_SIMD_PIXEL_MASKS_XBOX360);
    scan1Mask = __vand(__vsraw(scan1Mask, __vspltisw(-1)), __vslw(UMBRA_SIMD_PIXEL_MASKS_XBOX360, __vspltisw(4)));
    scan2Mask = __vand(__vsraw(scan2Mask, __vspltisw(-1)), __vslw(UMBRA_SIMD_PIXEL_MASKS_XBOX360, __vspltisw(8)));
    scan3Mask = __vand(__vsraw(scan3Mask, __vspltisw(-1)), __vslw(UMBRA_SIMD_PIXEL_MASKS_XBOX360, __vspltisw(12)));
    SIMDRegister comb = __vor(__vor(scan0Mask, scan1Mask), __vor(scan2Mask, scan3Mask));
    comb = __vor(comb, __vrlimi(SIMDZero(), comb, 0xF, 1));
    comb = __vor(comb, __vrlimi(SIMDZero(), comb, 0xF, 2));
    int ret;
    __stvewx(comb, &ret, 0);
    return ret;
}

enum { FullNegativeMask = 0xFFFFFFFFu };
static UMBRA_INLINE void SIMDWriteNegativeMask(int& result, const SIMDRegister& a)
{
    SIMDRegister signMask = __vcmpgtsw(__vzero(), a);
    SIMDRegister shuffled = __vperm(signMask, signMask, SIMDShufflePattern());
    __stvewx(shuffled, &result, 0);
}
static UMBRA_INLINE void SIMDWriteAnyMask(int& result, const SIMDRegister& a)
{
    SIMDRegister shuffled = __vperm(a, a, SIMDShufflePattern());
    __stvewx(shuffled, &result, 0);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0(const Vector4& v)
{
    SIMDRegister r = SIMDLoad((float*)&v.x);
    return __vrlimi(r, SIMDZero(), 0x1, 0);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector4& v)
{
    SIMDRegister r = SIMDLoad((float*)&v.x);
    return __vrlimi(r, UMBRA_SIMD_CONST_XBOX360, 0x1, 2);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW0(const Vector4& v)
{
    SIMDRegister r = SIMDLoadAligned((float*)&v.x);
    return __vrlimi(r, SIMDZero(), 0x1, 0);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW1(const Vector4& v)
{
    SIMDRegister r = SIMDLoadAligned((float*)&v.x);
    return __vrlimi(r, UMBRA_SIMD_CONST_XBOX360, 0x1, 2);
}

} // namespace Umbra

#endif // UMBRA_SIMD_CODE
#endif // __UMBRASIMD_XBOX360_H
