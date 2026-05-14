#pragma once
#ifndef __UMBRASIMD_PS3PPU_H
#define __UMBRASIMD_PS3PPU_H

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

#include <spu2vmx.h>

typedef vec_float4  SIMDRegister;
typedef vec_int4    SIMDRegister32;
#define SIMDZero()                  spu_splats(0.0f)
#define SIMDZero32()                spu_splats(0)
#define SIMDOne()                   spu_splats(1.0f)
#define SIMDMinusOne()              spu_splats(-1.0f)
#define SIMDAdd(a,b)                spu_add(a,b)
#define SIMDSub(a,b)                spu_sub(a,b)
#define SIMDMultiply(a,b)           spu_mul(a,b)
#define SIMDCompareGT(a,b)          (vec_float4)spu_cmpgt(a,b)
#define SIMDCompareEQ(a,b)          (vec_float4)spu_cmpeq(a,b)
#define SIMDCompareGE(a,b)          (vec_float4)spu_or(spu_cmpgt(a,b),spu_cmpeq(a,b))
#define SIMDCompareGT32(a,b)        (vec_int4)SIMDCompareGT(a,b)
#define SIMDCompareEQ32(a,b)        (vec_int4)SIMDCompareEQ(a,b)
#define SIMDComparGE32(a,b)         (vec_int4)SIMDCompareGE(a,b)
#define SIMDCompareEQ16(a,b)        (vec_int4)spu_cmpeq((vec_short8)a, (vec_short8)b)
#define SIMDBitwiseAnd(a,b)         spu_and(a,b)
#define SIMDBitwiseOr(a,b)          spu_or(a,b)
#define SIMDBitwiseEor(a,b)         spu_xor(a,b)
#define SIMDBitwiseAndNot(a,b)      spu_andc(a,b)
#define SIMDReciprocal(a)           spu_re(a)
#define SIMDReciprocalSqrt(a)       spu_rsqrte(a)
#define SIMDSqrt(a)                 SIMDReciprocal(SIMDReciprocalSqrt(a)) // \todo
#define SIMDAdd32(a,b)              spu_add((vec_int4)a,(vec_int4)b)
#define SIMDSub32(a,b)              spu_sub((vec_int4)a,(vec_int4)b)
#define SIMDMin32(a,b)              (vec_int4)spu_sel(b,a,spu_cmpgt(b,a))
#define SIMDMax32(a,b)              (vec_int4)spu_sel(b,a,spu_cmpgt(a,b))
#define SIMDMax16u(a,b)             (vec_int4)spu_sel((vec_ushort8)b,(vec_ushort8)a,spu_cmpgt((vec_ushort8)a,(vec_ushort8)b))
#define SIMDSelect(a,b,c)           spu_sel(a,b,(vector unsigned int)c)
#define SIMDSelect32(a,b,c)         (vec_int4)spu_sel(a,b,(vector unsigned int)c)
#define SIMDMultiplyAdd(a,b,c)      (vec_float4)si_fma((qword)a,(qword)b,(qword)c)
#define SIMDBitwiseOr32(a,b)        spu_or(a,b)
#define SIMDBitwiseAnd32(a,b)       spu_and(a,b)
#define SIMDBitwiseAndNot32(a,b)    spu_andc(a,b)
#define SIMDLeftShift32(a,n)        spu_sl(a,n)
#define SIMDMaskW()                 ((vec_float4)SIMDLoad32(0,0,0,-1))
#define SIMDMaskXYZW()              ((vec_float4)SIMDLoad32(-1,-1,-1,-1))
#define SIMDMaskXYZ()               ((vec_float4)SIMDLoad32(-1,-1,-1,0))
#define SIMDMaskXY()                ((vec_float4)SIMDLoad32(-1,-1,0,0))
#define SIMDFloatToBitPattern(v)    ((vec_int4)v)
#define SIMDBitPatternToFloat(v)    ((vec_float4)v)
#define SIMDSaveState()             0
#define SIMDRestoreState(a)         ((void)a)

/* \todo [antti 7.11.2011]: I'm sure there are direct insns for these! */
#define SIMDMergeLow32(a,b) SIMDShuffle32<0,4,1,5>(a,b)
#define SIMDMergeHigh32(a,b) SIMDShuffle32<2,6,3,7>(a,b)
#define SIMDMergeLow(a,b) SIMDShuffle<0,4,1,5>(a,b)
#define SIMDMergeHigh(a,b) SIMDShuffle<2,6,3,7>(a,b)
#define SIMDHighToLow(a) SIMDShuffle<2,3,2,3>(a)

// /todo
#define SIMDShuffle32_A0B0A0B0(a,b) SIMDShuffle32<0,4,0,4>(a,b)
#define SIMDShuffle32_A2B2A2B2(a,b) SIMDShuffle32<2,6,2,6>(a,b)
#define SIMDShuffle32_A0A0B0B0(a,b) SIMDShuffle32<0,0,4,4>(a,b)
#define SIMDShuffle32_A1A1B1B1(a,b) SIMDShuffle32<1,1,5,5>(a,b)
#define SIMDShuffle32_A2A2B2B2(a,b) SIMDShuffle32<2,2,6,6>(a,b)
#define SIMDShuffle32_A3A3B3B3(a,b) SIMDShuffle32<3,3,7,7>(a,b)
#define SIMDShuffle32_A0A2B0B2(a,b) SIMDShuffle32<0,2,4,6>(a,b)

#define SIMDShuffle_A0B0A0B0(a,b)   SIMDShuffle<0,4,0,4>(a,b)
#define SIMDShuffle_A2B2A2B2(a,b)   SIMDShuffle<2,6,2,6>(a,b)
#define SIMDShuffle_A0A0B0B0(a,b)   SIMDShuffle<0,0,4,4>(a,b)
#define SIMDShuffle_A1A1B1B1(a,b)   SIMDShuffle<1,1,5,5>(a,b)
#define SIMDShuffle_A2A2B2B2(a,b)   SIMDShuffle<2,2,6,6>(a,b)
#define SIMDShuffle_A3A3B3B3(a,b)   SIMDShuffle<3,3,7,7>(a,b)
#define SIMDShuffle_A0A2B0B2(a,b)   SIMDShuffle<0,2,4,6>(a,b)
#define SIMDShuffle_A0B0A1B1(a,b)   SIMDShuffle<0,4,1,5>(a,b)
#define SIMDShuffle_A0A1B0B1(a,b)   SIMDShuffle<0,1,4,5>(a,b)

namespace Umbra
{

UMBRA_FORCE_INLINE SIMDRegister     SIMDMin(const SIMDRegister& a, const SIMDRegister& b) { return spu_sel(b,a,spu_cmpgt(b,a)); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDMax(const SIMDRegister& a, const SIMDRegister& b) { return spu_sel(b,a,spu_cmpgt(a,b)); }


UMBRA_FORCE_INLINE SIMDRegister     SIMDIntToFloat(const SIMDRegister32& a)
{
    return spu_convtf(a,0);
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDFloatToInt(const SIMDRegister& a)
{
    return spu_convts(a,0);
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a, int b, int c, int d)
{
    vector signed int v = { a, b, c, d };
    return v;
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a)
{
    vector signed int v = { a, a, a, a };
    return v;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float a, float b, float c, float d)
{
    vector float v = { a, b, c, d };
    return v;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(int a, int b, int c, int d)
{
    vector signed int v = { a, b, c, d };
    return (vector float)v;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float a)
{
    return spu_splats(a);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0 (float v) { return SIMDLoad(v, v, v, 0.f); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadXXYY (float x, float y) { return SIMDLoad(x, x, y, y); }

UMBRA_FORCE_INLINE SIMDRegister     SIMDReplicate(const SIMDRegister& a, int i)
{
    return vec_splat(a, i);
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDReplicate32(const SIMDRegister32& a, int i)
{
    return vec_vspltw(a, i);
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDReplicate16(const SIMDRegister32& a, int i)
{
    return (vec_int4)vec_vsplth((vec_short8)a, i);
}
template <int x, int y, int z, int w>
UMBRA_FORCE_INLINE SIMDRegister     SIMDShuffle(const SIMDRegister& a, const SIMDRegister& b)
{
    vector unsigned int pattern = {
        0x00010203 + x*0x04040404,
        0x00010203 + y*0x04040404,
        0x00010203 + z*0x04040404,
        0x00010203 + w*0x04040404,
    };
    return (vec_float4)si_shufb((vec_uchar16)a,(vec_uchar16)b,(vec_uchar16)pattern);
}
template <int x, int y, int z, int w>
UMBRA_FORCE_INLINE SIMDRegister     SIMDShuffle(const SIMDRegister& a)
{
    // \todo [petri] implement using vperm?
    vector unsigned int pattern = {
        0x00010203 + x*0x04040404,
        0x00010203 + y*0x04040404,
        0x00010203 + z*0x04040404,
        0x00010203 + w*0x04040404,
    };
    return (vec_float4)si_shufb((vec_uchar16)a,(vec_uchar16)a,(vec_uchar16)pattern);
}
template <int x, int y, int z, int w>
UMBRA_FORCE_INLINE SIMDRegister32   SIMDShuffle32(const SIMDRegister32& a, const SIMDRegister32& b)
{
    vector unsigned int pattern = {
        0x00010203 + x*0x04040404,
        0x00010203 + y*0x04040404,
        0x00010203 + z*0x04040404,
        0x00010203 + w*0x04040404,
    };
    return (vec_int4)si_shufb((vec_uchar16)a,(vec_uchar16)b,(vec_uchar16)pattern);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDDot4(const SIMDRegister& a, const SIMDRegister& b)
{
    SIMDRegister ab = SIMDMultiply(a,b);
    SIMDRegister r;
    r = SIMDAdd(SIMDReplicate(ab,0), SIMDReplicate(ab,1));
    r = SIMDAdd(r, SIMDReplicate(ab,2));
    r = SIMDAdd(r, SIMDReplicate(ab,3));
    return r;
}
UMBRA_FORCE_INLINE void             SIMDIntFloor(const SIMDRegister& a, int& i)
{
    vec_int4 d = spu_convts(a, 0);
    i = spu_extract(d, 0);
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoadAligned32(const int* ptr)
{
    return *(vec_int4*)ptr;
}
UMBRA_FORCE_INLINE void             SIMDStore32(const SIMDRegister32& r, Vector3i& v)
{
    v.i = spu_extract(r, 0);
    v.j = spu_extract(r, 1);
    v.k = spu_extract(r, 2);
}
UMBRA_FORCE_INLINE void             SIMDStoreAligned32(const SIMDRegister32& r, int* p)
{
    vector signed int* ptr = (vector signed int*)p;
    *ptr = r;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAligned(const float* ptr)
{
    return *(vec_float4*)ptr;
}
UMBRA_FORCE_INLINE void             SIMDStoreAligned(const SIMDRegister& r, float* p)
{
    vector float* ptr = (vector float*)p;
    *ptr = r;
}
UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, float& res)
{
    res = spu_extract(r,0);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float* ptr)
{
#if 0
    /* \todo [antti 18.12.2012]: this seems to be broken somehow */
    vector float qw0, qw1;
    int shift;
    qw0 = *(vector float*)ptr;
    qw1 = *((vector float*)(ptr+1));
    shift = (unsigned) ptr & 15;
    return spu_or(
    spu_slqwbyte(qw0, shift),
    spu_rlmaskqwbyte(qw1, shift-16));
#else
    return SIMDLoad(ptr[0], ptr[1], ptr[2], ptr[3]);
#endif
}
UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, float *p)
{
    /* \todo [antti 18.12.2012]: this is also highly likely broken! */
    vector float* ptr = (vector float*)p;
    vector float qw0, qw1;
    vector unsigned int mask;
    int shift;
    qw0 = *ptr;
    qw1 = *(ptr+1);
    shift = (unsigned)(ptr) & 15;
    mask = (vector unsigned int)
    spu_rlmaskqwbyte((vector unsigned char)(0xFF), -shift);
    SIMDRegister flt = spu_rlqwbyte(r, -shift);
    *ptr = spu_sel(qw0, flt, mask);
    *(ptr+1) = spu_sel(flt, qw1, mask);
}
enum { FullNegativeMask = 0xFu };
UMBRA_FORCE_INLINE void             SIMDWriteNegativeMask(int& result, const SIMDRegister& a)
{
    vec_uint4 mask = spu_rlmask((vec_uint4)a, -31);
    vec_uint4 res  = (vec_uint4)si_gb((vector unsigned char)mask);
    result = (int)spu_extract(res,0);
}
UMBRA_FORCE_INLINE void             SIMDWriteAnyMask(int& result, const SIMDRegister& a)
{
    /* \todo [antti 28.11.2011]: this can be optimized */
    vec_uint4 mask = spu_rlmask((vec_uint4)a, -31);
    vec_uint4 res  = (vec_uint4)si_gb((vector unsigned char)mask);
    result = (int)spu_extract(res,0);
}
UMBRA_FORCE_INLINE int              SIMDExtractSignBits(const SIMDRegister& a)
{
    // \todo [petri] Untested!
    vec_uint4  mask = spu_rlmask((vec_uint4)a, -31);
    vec_uint4  res  = (vec_uint4)si_gb((vector unsigned char)mask);
    return (int)spu_extract(res,0);
}
#define SIMDExtract16Signs(a,b,c,d) ((SIMDExtractSignBits(a) << 0) | \
                                     (SIMDExtractSignBits(b) << 4) | \
                                     (SIMDExtractSignBits(c) << 8) | \
                                     (SIMDExtractSignBits(d) << 12))


UMBRA_FORCE_INLINE int              SIMDNotZero32(const SIMDRegister32& r)
{
    vec_uint4 mask = (vec_uint4)SIMDCompareEQ32(r, SIMDZero32());
    vec_uint4 res  = (vec_uint4)spu_gather((vector unsigned char)mask);
    if ((int)spu_extract(res,0) != 0xF)
        return 1;
    return 0;
}
UMBRA_FORCE_INLINE int              SIMDBitwiseOrTestAny(const SIMDRegister& a, const SIMDRegister& b)
{
    vec_uint4 mask = (vec_uint4)spu_or(a,b);
    vec_uint4 res  = (vec_uint4)spu_gather((vector unsigned int)mask);
    return (int)spu_extract(res,0);
}
UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny(const SIMDRegister& a, const SIMDRegister& b)
{
    vec_uint4 mask = (vec_uint4)SIMDCompareGT(a,b);
    vec_uint4 res  = (vec_uint4)spu_gather((vector unsigned int)mask);
    return (int)spu_extract(res,0);
}
UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny32(const SIMDRegister32& a, const SIMDRegister32& b)
{
    vec_uint4 mask = (vec_uint4)SIMDCompareGT32(a,b);
    vec_uint4 res  = (vec_uint4)spu_gather((vector unsigned int)mask);
    return (int)spu_extract(res,0);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDNegate(const SIMDRegister& a)
{
    vector signed int mask = { 0x80000000, 0x80000000, 0x80000000, 0x80000000 };
    return SIMDBitwiseEor(a, (vec_float4)mask);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDAbs(const SIMDRegister& a)
{
    /* \todo [petri] This implementation ought to be better.
    vector signed int mask = { 0x80000000, 0x80000000, 0x80000000, 0x80000000 };
    return SIMDBitwiseAnd(a, (vec_float4)mask); */
    SIMDRegister c = SIMDCompareGE(a, SIMDZero());
    SIMDRegister t0 = SIMDBitwiseAnd(a,c);
    SIMDRegister t1 = SIMDBitwiseAndNot(SIMDSub(SIMDZero(), a), c);
    return SIMDBitwiseOr(t0,t1);
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
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(const Vector4& v)
{
    return SIMDLoad((float*)&v.x);
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0(const Vector3& v)
{
    return SIMDLoad(v.x, v.y, v.z, 0.f);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector3& v)
{
    return SIMDLoad(v.x, v.y, v.z, 1.f);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW1(const Vector3& v)
{
    SIMDRegister r = SIMDLoadAligned((float*)&v.x);
    return SIMDSelect(r, SIMDOne(), SIMDMaskW());
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0(const Vector4& v)
{
    return SIMDLoad(v.x, v.y, v.z, 0.f);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector4& v)
{
    return SIMDLoad(v.x, v.y, v.z, 1.f);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW0(const Vector4& v)
{
    SIMDRegister r = SIMDLoadAligned((float*)&v.x);
    return SIMDBitwiseAnd(r, SIMDMaskXYZ());
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW1(const Vector4& v)
{
    SIMDRegister r = SIMDLoadAligned((float*)&v.x);
    return SIMDSelect(r, SIMDOne(), SIMDMaskW());
}

} // namespace Umbra

#endif // UMBRA_SIMD_CODE
#endif // __UMBRASIMD_PS3SPU_H
