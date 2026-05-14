// Copyright (c) 2011-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRASIMD_NEON_HPP
#define UMBRASIMD_NEON_HPP

#if defined(UMBRA_SIMD_CODE)

#include <arm_neon.h>

typedef float32x4_t  SIMDRegister;
typedef int32x4_t    SIMDRegister32;
#define SIMDZero()                  vdupq_n_f32(0.0f)
#define SIMDZero32()                vdupq_n_s32(0)
#define SIMDOne()                   vdupq_n_f32(1.0f)
#define SIMDMinusOne()              vdupq_n_f32(-1.0f)
#define SIMDAdd(a,b)                vaddq_f32(a,b)
#define SIMDSub(a,b)                vsubq_f32(a,b)
#define SIMDMultiply(a,b)           vmulq_f32(a,b)
#define SIMDCompareGT(a,b)          vreinterpretq_f32_u32(vcgtq_f32(a,b))
#define SIMDCompareEQ(a,b)          vreinterpretq_f32_u32(vceqq_f32(a,b))
#define SIMDCompareGE(a,b)          vreinterpretq_f32_u32(vcgeq_f32(a,b))
#define SIMDCompareGT32(a,b)        vreinterpretq_s32_u32(vcgtq_s32(a,b))
#define SIMDCompareEQ32(a,b)        vreinterpretq_s32_u32(vceqq_s32(a,b))
#define SIMDComparGE32(a,b)         vreinterpretq_s32_u32(vcgeq_s32(a,b))
#define SIMDBitwiseAnd(a,b)         vreinterpretq_f32_u32(vandq_u32(vreinterpretq_u32_f32(a), vreinterpretq_u32_f32(b)))
#define SIMDBitwiseOr(a,b)          vreinterpretq_f32_u32(vorrq_u32(vreinterpretq_u32_f32(a), vreinterpretq_u32_f32(b)))
#define SIMDBitwiseEor(a,b)         vreinterpretq_f32_u32(veorq_u32(vreinterpretq_u32_f32(a), vreinterpretq_u32_f32(b)))
#define SIMDBitwiseAndNot(a,b)      vreinterpretq_f32_u32(vbicq_u32(vreinterpretq_u32_f32(a), vreinterpretq_u32_f32(b)))
#define SIMDReciprocalSqrt(a)       vrsqrteq_f32(a)
#define SIMDSqrt(a)                 SIMDReciprocal(SIMDReciprocalSqrt(a)) // \todo
#define SIMDAdd32(a,b)              vaddq_s32(a,b)
#define SIMDSub32(a,b)              vsubq_s32(a,b)
#define SIMDMin32(a,b)              vminq_s32(a,b)
#define SIMDMax32(a,b)              vmaxq_s32(a,b)
#define SIMDMax16u(a,b)             vreinterpretq_s32_u16(vmaxq_u16(vreinterpretq_u16_s32(a), vreinterpretq_u16_s32(b)))
#define SIMDSelect(a,b,c)           vbslq_f32(vreinterpretq_u32_f32(c),b,a)
#define SIMDSelect32(a,b,c)         vbslq_s32(vreinterpretq_u32_s32(c),b,a)
#define SIMDMultiplyAdd(a,b,c)      vmlaq_f32(c,a,b)
#define SIMDBitwiseOr32(a,b)        vorrq_s32(a,b)
#define SIMDBitwiseAnd32(a,b)       vandq_s32(a,b)
#define SIMDBitwiseAndNot32(a,b)    vreinterpretq_s32_u32(vbicq_u32(vreinterpretq_u32_s32(a),vreinterpretq_u32_s32(b)))
#define SIMDLeftShift32(a,n)        vshlq_n_s32(a,n)
#define SIMDMaskW()                 SIMDLoad(0,0,0,-1)
#define SIMDMaskXYZW()              vreinterpretq_f32_s32(vdupq_n_s32(-1))
#define SIMDMaskXYZ()               SIMDLoad(-1,-1,-1,0)
#define SIMDMaskXY()                SIMDLoad(-1,-1,0,0)
#define SIMDFloatToBitPattern(v)    vreinterpretq_s32_f32(v)
#define SIMDBitPatternToFloat(v)    vreinterpretq_f32_s32(v)
#define SIMDSaveState()             0
#define SIMDRestoreState(a)         ((void)a)
#define SIMDMin(a, b)               vminq_f32(a,b)
#define SIMDMax(a, b)               vmaxq_f32(a,b)
#define SIMDIntToFloat(a)           vcvtq_f32_s32(a)
#define SIMDFloatToInt(a)           vcvtq_s32_f32(a)
#define SIMDNegate(a)               vnegq_f32(a)
#define SIMDAbs(a)                  vabsq_f32(a)

namespace Umbra
{


template<int i>
UMBRA_FORCE_INLINE SIMDRegister ImplReplicate (SIMDRegister a)
{
    UMBRA_ASSERT(!"Default impl not allowed.");
    (void)i;
    return a;
}

template<>
UMBRA_FORCE_INLINE SIMDRegister ImplReplicate<0> (SIMDRegister a)
{
    return vdupq_lane_f32(vget_low_f32(a),0);
}

template<>
UMBRA_FORCE_INLINE SIMDRegister ImplReplicate<1> (SIMDRegister a)
{
    return vdupq_lane_f32(vget_low_f32(a),1);
}

template<>
UMBRA_FORCE_INLINE SIMDRegister ImplReplicate<2> (SIMDRegister a)
{
    return vdupq_lane_f32(vget_high_f32(a),0);
}

template<>
UMBRA_FORCE_INLINE SIMDRegister ImplReplicate<3> (SIMDRegister a)
{
    return vdupq_lane_f32(vget_high_f32(a),1);
}


template<int i>
UMBRA_FORCE_INLINE SIMDRegister32 ImplReplicate32 (SIMDRegister32 a)
{
    UMBRA_ASSERT(!"Default impl not allowed.");
    return SIMDZero();
}

template<>
UMBRA_FORCE_INLINE SIMDRegister32 ImplReplicate32<0> (SIMDRegister32 a)
{
    return vdupq_lane_s32(vget_low_s32(a),0);
}

template<>
UMBRA_FORCE_INLINE SIMDRegister32 ImplReplicate32<1> (SIMDRegister32 a)
{
    return vdupq_lane_s32(vget_low_s32(a),1);
}

template<>
UMBRA_FORCE_INLINE SIMDRegister32 ImplReplicate32<2> (SIMDRegister32 a)
{
    return vdupq_lane_s32(vget_high_s32(a),0);
}

template<>
UMBRA_FORCE_INLINE SIMDRegister32 ImplReplicate32<3> (SIMDRegister32 a)
{
    return vdupq_lane_s32(vget_high_s32(a),1);
}

#define SIMDReplicate(a, i)     ImplReplicate<i>(a)
#define SIMDReplicate32(a, i)   ImplReplicate32<i>(a)

UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a, int b, int c, int d)
{
    //return vcombine_s32(vcreate_s32((int64_t)a<<32ll | b), vcreate_s32((int64_t)c<<32ll | d));
    int32x4_t result;
	result = vdupq_n_s32(a);
	result = vsetq_lane_s32(b, result, 1);
	result = vsetq_lane_s32(c, result, 2);
	result = vsetq_lane_s32(d, result, 3);
	return result;
}

UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a)
{
    return vdupq_n_s32(a);
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float a, float b, float c, float d)
{
    float32x4_t result;
	result = vdupq_n_f32(a);
	result = vsetq_lane_f32(b, result, 1);
	result = vsetq_lane_f32(c, result, 2);
	result = vsetq_lane_f32(d, result, 3);
	return result;
}
UMBRA_FORCE_INLINE SIMDRegister SIMDLoadW0 (float x)
{
    float32x4_t r = vdupq_n_f32(x);
    return vsetq_lane_f32(0.f, r, 3);
}

UMBRA_FORCE_INLINE SIMDRegister SIMDLoadXXYY (float x, float y)
{
    float32x2_t xx = vdup_n_f32(x);
    float32x2_t yy = vdup_n_f32(y);
    return vcombine_f32(xx, yy);
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(int a, int b, int c, int d)
{
    int v[] = { a, b, c, d };
    return vreinterpretq_f32_s32(vld1q_s32(v));
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float a)
{
    return vdupq_n_f32(a);
}

template <int x, int y, int z, int w>
UMBRA_FORCE_INLINE SIMDRegister SIMDShuffle(SIMDRegister a)
{
    UMBRA_ASSERT(!"Default impl not allowed on NEON! Specialize.");
    return SIMDZero();
}

// Specialization for the call in rasterizePortal
template <>
UMBRA_FORCE_INLINE SIMDRegister SIMDShuffle<1, 2, 3, 0>(SIMDRegister a)
{
    return vextq_f32(a, a, 1);
}

UMBRA_FORCE_INLINE SIMDRegister SIMDShuffle_A0A1B0B1 (SIMDRegister a, SIMDRegister b)
{
    return vcombine_f32(vget_low_f32(a), vget_low_f32(b));
}

UMBRA_FORCE_INLINE SIMDRegister SIMDShuffle_A0B0A1B1 (SIMDRegister a, SIMDRegister b)
{
    float32x4x2_t r = vzipq_f32(a, b);
    return r.val[0];
}


UMBRA_FORCE_INLINE SIMDRegister SIMDMergeLow (SIMDRegister a, SIMDRegister b)
{
    float32x2_t alo = vget_low_f32(a);
    float32x2_t blo = vget_low_f32(b);

    float32x2x2_t r2 = vtrn_f32(alo, blo);
    float32x4_t r = vcombine_f32(r2.val[0], r2.val[1]);
    return r;
}

UMBRA_FORCE_INLINE SIMDRegister SIMDMergeHigh (SIMDRegister a, SIMDRegister b)
{
    float32x2_t ahi = vget_high_f32(a);
    float32x2_t bhi = vget_high_f32(b);

    float32x2x2_t r2 = vtrn_f32(ahi, bhi);
    float32x4_t r = vcombine_f32(r2.val[0], r2.val[1]);
    return r;
}

UMBRA_FORCE_INLINE SIMDRegister SIMDHighToLow (SIMDRegister a)
{
    float32x2_t ahi = vget_high_f32(a);
    float32x4_t r = vcombine_f32(ahi, ahi);

    return r;
}

UMBRA_FORCE_INLINE float32x2_t SIMDDot4_Partial(SIMDRegister a, SIMDRegister b)
{
    float32x2_t ahi = vget_high_f32(a); // ax, ay
    float32x2_t bhi = vget_high_f32(b); // bx, by
    float32x2_t alo = vget_low_f32(a);  // az, aw
    float32x2_t blo = vget_low_f32(b);  // bz, bw
    float32x2_t r = vmul_f32(ahi, bhi); // ax*bx, ay*by
    return vmla_f32(r, alo, blo);       // ax*bx+az*bz, ay*by+aw*bw
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDDot4(SIMDRegister a, SIMDRegister b)
{
    float32x2_t r = SIMDDot4_Partial(a, b);
    r = vpadd_f32(r, r);
    return vdupq_lane_f32(r, 0);
}

UMBRA_FORCE_INLINE void             SIMDIntFloor(SIMDRegister a, int& i)
{
    SIMDRegister32 d = SIMDFloatToInt(a);
    i = vgetq_lane_s32(d, 0);
}
UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoadAligned32(const int* ptr)
{
    return *((SIMDRegister32*)ptr);
}
UMBRA_FORCE_INLINE void             SIMDStore32(SIMDRegister32 r, Vector3i& v)
{
    v.i = vgetq_lane_s32(r, 0);
    v.j = vgetq_lane_s32(r, 1);
    v.k = vgetq_lane_s32(r, 2);
}
UMBRA_FORCE_INLINE void             SIMDStoreAligned32(SIMDRegister32 r, int* p)
{
    *((SIMDRegister32*)p) = r;
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAligned(const float* ptr)
{
    return *((const SIMDRegister*)ptr);
}
UMBRA_FORCE_INLINE void             SIMDStoreAligned(SIMDRegister r, float* p)
{
    *((SIMDRegister*)p) = r;
}
UMBRA_FORCE_INLINE void             SIMDStore(SIMDRegister r, float& res)
{
    res = vgetq_lane_f32(r,0);
}

UMBRA_FORCE_INLINE int              SIMDExtractSignBits(SIMDRegister a)
{
    // \todo [jasin] no native instruction for this in Neon?

    uint32x4_t extended = vreinterpretq_u32_s32(vshrq_n_s32(vreinterpretq_s32_f32(a), 31)); // 0xffffffff or zeros
    uint32x4_t mask     = vreinterpretq_u32_s32(SIMDLoad32(1, 2, 4, 8));                    // [1, 2, 4, 8]
    uint32x4_t signbits = vandq_u32(extended, mask);

    uint32x2_t hi = vget_high_u32(signbits);
    uint32x2_t lo = vget_low_u32(signbits);

    uint32x2_t b = vorr_u32(hi, lo);
    uint32x2_t res = vpadd_u32(b, b);

    return vget_lane_u32(res, 0);
}

UMBRA_FORCE_INLINE UINT32 SIMDExtract16Signs(SIMDRegister a, SIMDRegister b, SIMDRegister c, SIMDRegister d)
{
    // shift-and-insert sign bits into correct 4-bit slots
    uint32x4_t combined = vshrq_n_u32(vreinterpretq_u32_f32(d), 19);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(c), 23);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(b), 27);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(a), 31);
    // narrow to 16 bits, mask sign bits and apply per-channel shift
    uint16x4_t narrowed = vmovn_u32(combined);
    narrowed = vand_u16(narrowed, vdup_n_u16(0x1111));
    narrowed = vshl_u16(narrowed, vcreate_s16(0x0003000200010000ll));
    // or channels together
    uint32x2_t collapsed = vpaddl_u16(narrowed);
    collapsed = vpadd_u32(collapsed, collapsed);
    return vget_lane_u32(collapsed, 0);
}

enum { FullNegativeMask = 4 };
// This is actually SIMDCountSignBits(), not a mask
UMBRA_FORCE_INLINE int ImplWriteNegativeMask (SIMDRegister a)
{
    uint32x4_t signs = vshrq_n_u32(vreinterpretq_u32_f32(a), 31);   // could we use vshrn?
    uint32x2_t hi = vget_high_u32(signs);
    uint32x2_t lo = vget_low_u32(signs);

    uint32x2_t sums = vadd_u32(hi, lo);     // 2s or zeros
    uint32x2_t comb = vpadd_u32(sums, sums);// fold
    return vget_lane_s32(vreinterpret_s32_u32(comb), 0);
}

UMBRA_FORCE_INLINE void SIMDWriteNegativeMask(int& result, SIMDRegister a)
{
    result = ImplWriteNegativeMask(a);
}

UMBRA_FORCE_INLINE void SIMDWriteNegativeMask2(int& result1, int& result2, SIMDRegister a, SIMDRegister b)
{
    uint32x4_t signs1 = vshrq_n_u32(vreinterpretq_u32_f32(a), 31);
    uint32x4_t signs2 = vshrq_n_u32(vreinterpretq_u32_f32(b), 31);
    uint32x2_t sums1 = vadd_u32(vget_high_u32(signs1), vget_low_u32(signs1));
    uint32x2_t sums2 = vadd_u32(vget_high_u32(signs2), vget_low_u32(signs2));
    uint32x2_t comb = vpadd_u32(sums1, sums2);
    result1 = vget_lane_s32(vreinterpret_s32_u32(comb), 0);
    result2 = vget_lane_s32(vreinterpret_s32_u32(comb), 1);
}

UMBRA_FORCE_INLINE int ImplWriteAnyMask(SIMDRegister a)
{
    uint32x2_t hi = vget_high_u32(vreinterpretq_u32_f32(a));
    uint32x2_t lo = vget_low_u32(vreinterpretq_u32_f32(a));
    uint32x2_t r = vmax_u32(hi, lo);
    r = vpmax_u32(r, r);
    return vget_lane_s32(vreinterpret_s32_u32(r), 0);
}

// Input a contains either all ones or zeros, choose any bit pos and create mask
UMBRA_FORCE_INLINE void             SIMDWriteAnyMask(int& result, SIMDRegister a)
{
    result = ImplWriteAnyMask(a);
}

UMBRA_FORCE_INLINE int              SIMDBitwiseOrTestAny(SIMDRegister a, SIMDRegister b)
{
    return ImplWriteAnyMask(SIMDBitwiseOr(a,b));
}

UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny(SIMDRegister a, SIMDRegister b)
{
    return ImplWriteAnyMask(SIMDCompareGT(a,b));
}

UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny32(SIMDRegister32 a, SIMDRegister32 b)
{
    return ImplWriteAnyMask(vreinterpretq_f32_s32(SIMDCompareGT32(a,b)));
}

UMBRA_FORCE_INLINE int              SIMDNotZero32(SIMDRegister32 r)
{
    uint32x4_t ur = vreinterpretq_u32_s32(r);
    uint32x2_t hi = vget_high_u32(ur);
    uint32x2_t lo = vget_low_u32(ur);

    uint32x2_t hilo = vpmax_u32(hi, lo);
    uint32x2_t ret = vpmax_u32(hilo, hilo);
    return (int)vget_lane_u32(ret, 0);
}

UMBRA_FORCE_INLINE void             SIMDStore(SIMDRegister r, float* dst)
{
    // \todo [jasin 2011-12-22] verify alignment
    vst1q_f32(reinterpret_cast<float32_t*>(dst), r);
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(const float* src)
{
    // \todo [jasin 2011-12-21] verify alignment
    return vld1q_f32(reinterpret_cast<const float32_t*>(src));
}

UMBRA_FORCE_INLINE void             SIMDStore(SIMDRegister r, Vector4& v)
{
    SIMDStore(r, &v.x);
}
UMBRA_FORCE_INLINE void             SIMDStore(SIMDRegister r, Vector3& v)
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
    return SIMDSelect(SIMDLoad(&v.x), SIMDZero(), SIMDMaskW());
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector3& v)
{
    return SIMDSelect(SIMDLoad(&v.x), SIMDOne(), SIMDMaskW());
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0(const Vector4& v)
{
    return SIMDSelect(SIMDLoad(&v.x), SIMDZero(), SIMDMaskW());
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector4& v)
{
    return SIMDSelect(SIMDLoad(&v.x), SIMDOne(), SIMDMaskW());
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW0(const Vector4& v)
{
    return SIMDSelect(SIMDLoadAligned(&v.x), SIMDZero(), SIMDMaskW());
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW1(const Vector4& v)
{
    return SIMDSelect(SIMDLoadAligned(&v.x), SIMDOne(), SIMDMaskW());
}

UMBRA_FORCE_INLINE SIMDRegister SIMDReciprocal(SIMDRegister a)
{
    float32x4_t reciprocal = vrecpeq_f32(a);
    return vmulq_f32(vrecpsq_f32(a, reciprocal), reciprocal);
}

UMBRA_FORCE_INLINE SIMDRegister32 SIMDClamp32(SIMDRegister32 mnmx, SIMDRegister32 bounds)
{
    int32x2_t mn = vmax_s32(vget_low_s32(mnmx), vget_low_s32(bounds));
    int32x2_t mx = vmin_s32(vget_high_s32(mnmx), vget_high_s32(bounds));
    return vcombine_s32(mn, mx);
}

UMBRA_FORCE_INLINE void SIMDTranspose(
    SIMDRegister& outX, SIMDRegister& outY, SIMDRegister& outZ, SIMDRegister& outW,
    const SIMDRegister& inA, const SIMDRegister& inB, const SIMDRegister& inC, const SIMDRegister& inD)
{
    SIMDRegister acXY = vcombine_f32(vget_low_f32(inA), vget_low_f32(inC));
    SIMDRegister bdXY = vcombine_f32(vget_low_f32(inB), vget_low_f32(inD));
    SIMDRegister acZW = vcombine_f32(vget_high_f32(inA), vget_high_f32(inC));
    SIMDRegister bdZW = vcombine_f32(vget_high_f32(inB), vget_high_f32(inD));
    float32x4x2_t outXY = vtrnq_f32(acXY, bdXY);
    float32x4x2_t outZW = vtrnq_f32(acZW, bdZW);
    outX = outXY.val[0];
    outY = outXY.val[1];
    outZ = outZW.val[0];
    outW = outZW.val[1];
}

} // namespace Umbra

#endif // UMBRA_SIMD_CODE
#endif // UMBRASIMD_NEON_HPP
