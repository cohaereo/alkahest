#pragma once
#ifndef __UMBRASIMD_SSE_H
#define __UMBRASIMD_SSE_H

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

// Force enable AVX on platforms that always support it

#ifndef UMRBA_SIMD_AVX
#if (UMBRA_OS == UMBRA_PS4) || (UMBRA_OS == UMBRA_XBOXONE)
#   define UMBRA_SIMD_AVX
#endif
#endif

#ifdef UMBRA_SIMD_AVX
#   include <immintrin.h>
#else
#   include <emmintrin.h>
#endif

namespace Umbra
{

typedef __m128  SIMDRegister;
typedef __m128i SIMDRegister32;
#define SIMDZero()                  _mm_setzero_ps()
#define SIMDOne()                   SIMDLoad(1.f)
#define SIMDMinusOne()              SIMDLoad(-1.f)
#define SIMDAdd(a,b)                _mm_add_ps(a,b)
#define SIMDSub(a,b)                _mm_sub_ps(a,b)
#define SIMDMultiply(a,b)           _mm_mul_ps(a,b)
#define SIMDDivide(a,b)             _mm_div_ps(a,b)
#define SIMDCompareGT(a,b)          _mm_cmpgt_ps(a,b)
#define SIMDCompareGE(a,b)          _mm_cmpge_ps(a,b)
#define SIMDCompareEQ(a,b)          _mm_cmpeq_ps(a,b)
#define SIMDBitwiseAnd(a,b)         _mm_and_ps(a,b)
#define SIMDBitwiseOr(a,b)          _mm_or_ps(a,b)
#define SIMDBitwiseEor(a,b)         _mm_xor_ps(a,b)
#define SIMDBitwiseAndNot(a,b)      _mm_andnot_ps(b,a)
#define SIMDLeftShift32(a,i)        _mm_slli_epi32(a,i)
#define SIMDMin                     _mm_min_ps
#define SIMDMin32(a,b)              SIMDSelect32(a, b, SIMDCompareGT32(a, b))
#define SIMDMax                     _mm_max_ps
#define SIMDSqrt                    _mm_sqrt_ps
#define SIMDMax32(a,b)              SIMDSelect32(b, a, SIMDCompareGT32(a, b))
#ifdef UMBRA_SIMD_AVX
#define SIMDReplicate(v, i)         _mm_permute_ps(v, _MM_SHUFFLE(i,i,i,i))
#else
#define SIMDReplicate(v, i)         _mm_shuffle_ps(v,v, _MM_SHUFFLE(i,i,i,i))
#endif
#define SIMDMaskXYZW()              SIMDBitPatternToFloat(SIMDLoad32(0xFFFFFFFF))
#define SIMDMaskXYZ()               SIMDLoad((int)0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0)
#define SIMDMaskXY()                SIMDLoad((int)0xFFFFFFFF, 0xFFFFFFFF, 0, 0)
#define SIMDMaskW()                 SIMDLoad((int)0, 0, 0, 0xFFFFFFFF)
#define SIMDMultiplyAdd(a,b,c)      _mm_add_ps(_mm_mul_ps(a,b),c)
#define SIMDSelect(a,b,c)           _mm_or_ps(_mm_andnot_ps(c,a), _mm_and_ps(b,c))
#define SIMDReciprocalSqrt(a)       _mm_rsqrt_ps(a)
#define SIMDAdd32(a,b)              _mm_add_epi32(a,b)
#define SIMDStoreAligned32(a,dst)   _mm_store_si128((__m128i*)(dst), (a))
#define SIMDLoadAligned32(p)        _mm_load_si128((const __m128i*)(p))
#define SIMDCompareGT32(a,b)        _mm_cmpgt_epi32(a,b)
#define SIMDCompareEQ32(a,b)        _mm_cmpeq_epi32(a,b)
#define SIMDCompareEQ16(a,b)        _mm_cmpeq_epi16(a,b)
#define SIMDBitwiseOr32(a,b)        _mm_or_si128(a,b)
#define SIMDBitwiseAnd32(a,b)       _mm_and_si128(a,b)
#define SIMDBitwiseAndNot32(a,b)    _mm_andnot_si128(b,a)
#define SIMDZero32()                _mm_setzero_si128()
#define SIMDSelect32(a,b,c)         _mm_or_si128(_mm_andnot_si128(c,a), _mm_and_si128(b,c))
#define SIMDReplicate32(v, i)       _mm_shuffle_epi32(v, _MM_SHUFFLE(i,i,i,i))
#define SIMDIntToFloat(v)           _mm_cvtepi32_ps(v)
#define SIMDFloatToInt(v)           _mm_cvttps_epi32(v)
#define SIMDFloatToBitPattern(v)    _mm_castps_si128(v)
#define SIMDBitPatternToFloat(v)    _mm_castsi128_ps(v)
#define SIMDRestoreState(a)         _MM_SET_EXCEPTION_MASK(a)

#define SIMDMergeLow(a,b)           _mm_unpacklo_ps(a,b)
#define SIMDMergeHigh(a,b)          _mm_unpackhi_ps(a,b)
#define SIMDHighToLow(a)            _mm_movehl_ps(a, a)

#define SIMDMergeLow32(a,b)         _mm_unpacklo_epi32(a,b)
#define SIMDMergeHigh32(a,b)        _mm_unpackhi_epi32(a,b)

#define SIMDShuffle_A0B0A0B0(a,b)   _mm_movelh_ps(_mm_unpacklo_ps(a, b), _mm_unpacklo_ps(a, b))
#define SIMDShuffle_A2B2A2B2(a,b)   _mm_movelh_ps(_mm_unpackhi_ps(a, b), _mm_unpackhi_ps(a, b))
#define SIMDShuffle_A0A0B0B0(a,b)   _mm_shuffle_ps(a, b, _MM_SHUFFLE(0,0,0,0))
#define SIMDShuffle_A1A1B1B1(a,b)   _mm_shuffle_ps(a, b, _MM_SHUFFLE(1,1,1,1))
#define SIMDShuffle_A2A2B2B2(a,b)   _mm_shuffle_ps(a, b, _MM_SHUFFLE(2,2,2,2))
#define SIMDShuffle_A3A3B3B3(a,b)   _mm_shuffle_ps(a, b, _MM_SHUFFLE(3,3,3,3))
#define SIMDShuffle_A0B0A1B1(a,b)   _mm_unpacklo_ps(a, b)
#define SIMDShuffle_A0A2B0B2(a,b)   _mm_shuffle_ps(a, b, _MM_SHUFFLE(2,0,2,0))
#define SIMDShuffle_A0A1B0B1(a,b)   _mm_movelh_ps(a, b)

#define SIMDShuffle32_A0A0B0B0(a,b) _mm_unpacklo_epi64(SIMDReplicate32(a, 0), SIMDReplicate32(b, 0))
#define SIMDShuffle32_A1A1B1B1(a,b) _mm_unpacklo_epi64(SIMDReplicate32(a, 1), SIMDReplicate32(b, 1))
#define SIMDShuffle32_A2A2B2B2(a,b) _mm_unpacklo_epi64(SIMDReplicate32(a, 2), SIMDReplicate32(b, 2))
#define SIMDShuffle32_A3A3B3B3(a,b) _mm_unpacklo_epi64(SIMDReplicate32(a, 3), SIMDReplicate32(b, 3))
#define SIMDShuffle32_A0A2B0B2(a,b) _mm_unpacklo_epi64(_mm_shuffle_epi32(a, _MM_SHUFFLE(2, 0, 2, 0)), _mm_shuffle_epi32(b, _MM_SHUFFLE(2, 0, 2, 0)))

UMBRA_FORCE_INLINE UINT32 SIMDSaveState (void)
{
    UINT32 cur = _MM_GET_EXCEPTION_MASK();
#if 0 && defined(UMBRA_DEBUG)
    _MM_SET_EXCEPTION_MASK(_MM_MASK_INEXACT | _MM_MASK_OVERFLOW);
#else
    _MM_SET_EXCEPTION_MASK(_MM_MASK_MASK);
#endif
    return cur;
}

template <int a, int b, int c, int d>
UMBRA_FORCE_INLINE SIMDRegister     SIMDShuffle(const SIMDRegister& x, const SIMDRegister& y)
{
    SIMDRegister t0 = _mm_shuffle_ps(x, x, _MM_SHUFFLE(d&3,c&3,b&3,a&3));
    SIMDRegister t1 = _mm_shuffle_ps(y, y, _MM_SHUFFLE(d&3,c&3,b&3,a&3));
    SIMDRegister mask = _mm_cmpgt_ps(_mm_setr_ps(a,b,c,d), _mm_set1_ps(3.0f));
    return SIMDSelect(t0,t1,mask);
}
template <int a, int b, int c, int d>
UMBRA_FORCE_INLINE SIMDRegister     SIMDShuffle(const SIMDRegister& x)
{
    return _mm_shuffle_ps(x, x, _MM_SHUFFLE(d&3,c&3,b&3,a&3));
}

UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a)                               { return _mm_set1_epi32(a); }
UMBRA_FORCE_INLINE SIMDRegister32   SIMDLoad32(int a,int b, int c,int d)            { return _mm_set_epi32(d,c,b,a); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float* p)                              { return _mm_loadu_ps(p); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float f)                               { return _mm_set1_ps(f); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAligned(float* p)                       { return _mm_load_ps(p); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAligned(int* p)                         { return _mm_load_ps((float*)p); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoad(float x, float y, float z, float w)    { return _mm_setr_ps(x,y,z,w); } // private
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0 (float v) { return SIMDLoad(v, v, v, 0.f); }
UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadXXYY (float x, float y) { return SIMDLoad(x, x, y, y); }
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
UMBRA_FORCE_INLINE void             SIMDStore32(const SIMDRegister32& r, Vector3i& v)
{
    Vector4i UMBRA_ATTRIBUTE_ALIGNED32(tmp);
    SIMDStoreAligned32(r, &tmp);
    v.i = tmp.i;
    v.j = tmp.j;
    v.k = tmp.k;
}
UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, float* p)
{
    _mm_storeu_ps(p,r);
}
UMBRA_FORCE_INLINE void             SIMDStore(const SIMDRegister& r, float& res)
{
    _mm_store_ss(&res, r);
}
UMBRA_FORCE_INLINE void             SIMDStoreAligned(const SIMDRegister& r, float* p)
{
    _mm_store_ps(p,r);
}
UMBRA_FORCE_INLINE int              SIMDExtractSignBits(const SIMDRegister& a)
{
    return _mm_movemask_ps(a);
}
#define SIMDExtract16Signs(a,b,c,d) ((SIMDExtractSignBits(a) << 0) | \
                                     (SIMDExtractSignBits(b) << 4) | \
                                     (SIMDExtractSignBits(c) << 8) | \
                                     (SIMDExtractSignBits(d) << 12))
enum
{
    FullNegativeMask = 0xF
};
UMBRA_FORCE_INLINE void             SIMDWriteNegativeMask(int& result, const SIMDRegister& a)
{
    result = _mm_movemask_ps(a);
}
UMBRA_FORCE_INLINE void             SIMDWriteAnyMask(int& result, const SIMDRegister& a)
{
    result = _mm_movemask_ps(a);
}
UMBRA_FORCE_INLINE int              SIMDNotZero(const SIMDRegister& a)
{
    return _mm_movemask_ps(SIMDCompareEQ(a, SIMDZero())) != 0xF;
}
UMBRA_FORCE_INLINE int              SIMDNotZero32(const SIMDRegister32& a)
{
    return _mm_movemask_epi8(SIMDCompareEQ32(a, SIMDZero32())) != 0xFFFF;
}
UMBRA_FORCE_INLINE int              SIMDBitwiseOrTestAny(const SIMDRegister& a, const SIMDRegister& b)
{
    return _mm_movemask_ps(SIMDBitwiseOr(a,b));
}
UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny(const SIMDRegister& a, const SIMDRegister& b)
{
    return _mm_movemask_ps(SIMDCompareGT(a,b));
}
UMBRA_FORCE_INLINE int              SIMDCompareGTTestAny32(const SIMDRegister32& a, const SIMDRegister32& b)
{
    return _mm_movemask_epi8(SIMDCompareGT32(a,b));
}
UMBRA_FORCE_INLINE void             SIMDIntFloor(const SIMDRegister& a, int& res)
{
    res = _mm_cvtt_ss2si(a);
}
UMBRA_FORCE_INLINE int              SIMDIntFloor(const SIMDRegister& a)
{
    return _mm_cvtt_ss2si(a);
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
UMBRA_FORCE_INLINE SIMDRegister     SIMDDot3(const SIMDRegister& a, const SIMDRegister& b)
{
    SIMDRegister ab = SIMDMultiply(a,b);
    SIMDRegister r  = SIMDAdd(SIMDReplicate(ab,0), SIMDReplicate(ab,1));
    return SIMDAdd(r, SIMDReplicate(ab,2));
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDDot4(const SIMDRegister& a, const SIMDRegister& b)
{
#ifdef UMBRA_SIMD_AVX
    SIMDRegister ab = SIMDMultiply(a,b);
    ab = _mm_hadd_ps(ab, ab);
    ab = _mm_hadd_ps(ab, ab);
    return ab;
#else
    SIMDRegister ab = SIMDMultiply(a,b);
    SIMDRegister r;
    r = SIMDAdd(SIMDReplicate(ab,0), SIMDReplicate(ab,1));
    r = SIMDAdd(r, SIMDReplicate(ab,2));
    r = SIMDAdd(r, SIMDReplicate(ab,3));
    return r;
#endif
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDNegate(const SIMDRegister& a)
{
    return SIMDBitwiseEor(a, SIMDBitPatternToFloat(SIMDLoad32(0x80000000)));
}
UMBRA_FORCE_INLINE SIMDRegister     SIMDAbs(const SIMDRegister& a)
{
    return SIMDBitwiseAndNot(a, SIMDBitPatternToFloat(SIMDLoad32(0x80000000)));
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

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0(const Vector3& v_)
{
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, v) = Vector4(v_, 0.f);
    return SIMDLoadAligned((float*)&v.x);
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector3& v_)
{
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, v) = Vector4(v_, 1.f);
    return SIMDLoadAligned((float*)&v.x);
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW0(const Vector4& v)
{
    SIMDRegister r = SIMDLoad((float*)&v.x);
#ifdef UMBRA_SIMD_AVX
    r = _mm_insert_ps(r, r, _MM_MK_INSERTPS_NDX(0, 0, 1<<3));
#else
    r = SIMDBitwiseAnd(r, SIMDMaskXYZ());
#endif
    return r;
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadW1(const Vector4& v)
{
    SIMDRegister r = SIMDLoad((float*)&v.x);
#ifdef UMBRA_SIMD_AVX
    r = _mm_insert_ps(r, SIMDOne(), _MM_MK_INSERTPS_NDX(0, 3, 0));
#else
    r = SIMDSelect(r, SIMDOne(), SIMDMaskW());
#endif
    return r;
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW0(const Vector4& v)
{
    SIMDRegister r = SIMDLoadAligned((float*)&v.x);
#ifdef UMBRA_SIMD_AVX
    r = _mm_insert_ps(r, r, _MM_MK_INSERTPS_NDX(0, 0, 1<<3));
#else
    r = SIMDBitwiseAnd(r, SIMDMaskXYZ());
#endif
    return r;
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDLoadAlignedW1(const Vector4& v)
{
    SIMDRegister r = SIMDLoadAligned((float*)&v.x);
#ifdef UMBRA_SIMD_AVX
    r = _mm_insert_ps(r, SIMDOne(), _MM_MK_INSERTPS_NDX(0, 3, 0));
#else
    r = SIMDSelect(r, SIMDOne(), SIMDMaskW());
#endif
    return r;
}

UMBRA_FORCE_INLINE SIMDRegister     SIMDReciprocal(const SIMDRegister& x)
{
    // safe version that works around div-by-zero
    SIMDRegister mask = SIMDCompareEQ(x, SIMDZero());
    return _mm_rcp_ps(SIMDSelect(x, SIMDOne(), mask));
}

UMBRA_FORCE_INLINE SIMDRegister32   SIMDMax16u(const SIMDRegister32& a, const SIMDRegister32& b)
{
#ifdef UMBRA_SIMD_AVX
    return _mm_max_epu16(a,b);
#else
    // SSE2 only has signed max, so only 15-bit values can be used.
    UMBRA_ASSERT(!SIMDNotZero32(SIMDBitwiseAnd32(a, SIMDLoad32(0x80008000))));
    UMBRA_ASSERT(!SIMDNotZero32(SIMDBitwiseAnd32(b, SIMDLoad32(0x80008000))));
    return _mm_max_epi16(a,b);
#endif
}


} // namespace Umbra

#endif // UMBRA_SIMD_CODE
#endif // __UMBRASIMD_H
