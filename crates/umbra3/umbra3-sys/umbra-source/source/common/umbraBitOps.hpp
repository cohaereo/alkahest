#pragma once
#ifndef __UMBRABITOPS_H
#define __UMBRABITOPS_H

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
 * \brief   Umbra bit math
 *
 */

#include "umbraPrivateDefs.hpp"
#include <standard/BitwiseOps.hpp>
#include <string.h>

#define UMBRA_ROUND_TO_32(x)        (((x) + 31) & ~31)
#define UMBRA_BIT_IDX(x)            ((x) & 0x1F)
#define UMBRA_BIT_MASK(x)           (1 << UMBRA_BIT_IDX(x))
#define UMBRA_BIT_DWORD(x)          ((x) >> 5)

#define UMBRA_SIGN_EXTEND(x)        ((-(int)(x)) >> 31)   // if (x <= 0) return 0; else return 0xffffffff;
#define UMBRA_SIGN_EXTEND64(x)      ((-(INT64)(x)) >> 63)

#define UMBRA_BITVECTOR_DWORDS(x)   (((x) + 31) >> 5)
#define UMBRA_BITVECTOR_SIZE(x)     (UMBRA_BITVECTOR_DWORDS(x) * sizeof(Umbra::UINT32))

#define UMBRA_FIRSTDWORD_MASK(x)    (~(UMBRA_BIT_MASK(x) - 1))
#define UMBRA_LASTDWORD_MASK(x)     (UMBRA_BIT_MASK(x) - 1)

#define UMBRA_BITFIELD_MASK(x)      (((1 << x) - 1) | ((1 << ((x) - 1)) >> 31))

namespace Umbra
{

typedef enum
{
    BitRange_None = 0,
    BitRange_Some,
    BitRange_All
} BitRangeResult;

UMBRA_INLINE int bitsForValue (UINT32 value)
{
    return value ? highestBitSet(value) + 1 : 0;
}

UMBRA_INLINE UINT32 roundPowerOfTwo(UINT32 value)
{
    if (value > 1)
        return 1 << (32 - countLeadingZeros ((INT32)value - 1));
    else return 1;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Hamming weight (number of set bits, population count) for
 *          a 32-bit word
 *//*-------------------------------------------------------------------*/

static UMBRA_INLINE int countOnes (UINT32 v)
{
#if UMBRA_GCC_INTRINSICS && (UMBRA_COMPILER != UMBRA_SNC)
    return __builtin_popcount(v);
#else
   v = v - ((v >> 1) & 0x55555555u);
   v = (v & 0x33333333u) + ((v >> 2) & 0x33333333u);
   v += (v >> 4);
   v &= 0xF0F0F0Fu;
   v += (v >> 8);
   v += (v >> 16);
   return v & 0x3Fu;
#endif
}

/*----------------------------------------------------------------------*//*!
 * \brief Helper for iterating bitvectors.
 *
 * Returns index of next set bit starting from idx + 1.
 * Returns -1 when there are no more set bits.
 *
 * Example:
 *
 * int idx = -1;
 * do
 * {
 *    idx = nextSetBit(bitvector, idx, bitvectorSize);
 *    printf("%d\n", idx);
 * } while(idx != -1);
 *
 *//*----------------------------------------------------------------------*/

static UMBRA_FORCE_INLINE int nextSetBit(const UINT32* vector, int idx, const int size)
{
    idx++;
    int dword = UMBRA_BIT_DWORD(idx);
    UINT32 x = vector[dword] >> (idx & 0x1F);
    while(!x)
    {
        idx = (idx + 32) & ~31;

        if (UMBRA_EXPECT(idx >= size, 0))
            return -1;
        x = vector[++dword];
    }

    idx = idx + lowestBitSet(x);
    if (idx >= size)
        return -1;
    return idx;
}

static UMBRA_FORCE_INLINE bool testBit (const UINT32* vector, const int idx)
{
    return (vector[UMBRA_BIT_DWORD(idx)] & UMBRA_BIT_MASK(idx)) != 0;
}

static UMBRA_FORCE_INLINE void setBit (UINT32* vector, const int idx)
{
    vector[UMBRA_BIT_DWORD(idx)] |= UMBRA_BIT_MASK(idx);
}

static UMBRA_FORCE_INLINE void set2BitValue (UINT32* vector, const int idx, UINT32 value)
{
    // must be even position
    UMBRA_ASSERT((idx & 1) == 0);

    int dword = UMBRA_BIT_DWORD(idx);
    vector[dword] &= ~(3 << (idx & 31));
    vector[dword] |= value << (idx & 31);
}

static UMBRA_FORCE_INLINE void flipBit (UINT32* vector, const int idx)
{
    vector[UMBRA_BIT_DWORD(idx)] ^= UMBRA_BIT_MASK(idx);
}

static UMBRA_FORCE_INLINE void clearBit (UINT32* vector, const int idx)
{
    vector[UMBRA_BIT_DWORD(idx)] &= ~UMBRA_BIT_MASK(idx);
}

static UMBRA_FORCE_INLINE bool testAndSetBit (UINT32* vector, const int idx)
{
    UINT32* p = vector + UMBRA_BIT_DWORD(idx);
    UINT32 mask = UMBRA_BIT_MASK(idx);
    UINT32 old = *p;
    *p = old | mask;
    return (old & mask) != 0;
}

static UMBRA_FORCE_INLINE bool testAndClearBit (UINT32* vector, const int idx)
{
    UINT32* p = vector + UMBRA_BIT_DWORD(idx);
    UINT32 mask = UMBRA_BIT_MASK(idx);
    UINT32 old = *p;
    *p = old & ~mask;
    return (old & mask) != 0;
}

static UMBRA_FORCE_INLINE void bitVectorOr (UINT32* bv, const UINT32* left, const UINT32* right, int num)
{
    num >>= 5;
    while (num--)
        *bv++ = *left++ | *right++;
}

static UMBRA_FORCE_INLINE UINT32 unpackElem (const UINT32* bv, int idx, UINT32 width)
{
    // doesn't work with 32
    UMBRA_ASSERT(width < 32);
    UINT32 dword = UMBRA_BIT_DWORD(idx);
    UINT32 shift = idx & 0x1F;
    UINT32 left = bv[dword] >> shift;
    UINT32 right = (bv[dword + 1] << (32 - shift)) & ~((1 << (31 - shift)) - 1);
    UINT32 mask = (1u << (UINT32)width) - 1u; // undefined behaviour with width == 32, sometimes
                                              // interpreted as (1u << (width % 32))
    return (left | right) & mask;
}

static UMBRA_FORCE_INLINE UINT32 unpackElem32 (const UINT32* bv, int idx)
{
    UINT32 dword = UMBRA_BIT_DWORD(idx);
    int shift = (UINT32)(idx & 0x1F);
    UINT32 left = bv[dword] >> shift;
    UINT32 right = ((bv[dword + 1] << (32 - shift)) & UMBRA_SIGN_EXTEND(shift)) & ~((1 << (32 - shift)) - 1);
    return (left | right);
}

static UMBRA_FORCE_INLINE UINT64 unpackElem64 (const UINT32* bv, int idx)
{
    // \todo optimize
    UINT64 ret = unpackElem32(bv, idx);
    ret |= ((UINT64)unpackElem32(bv, idx+32)<<32);
    return ret;
}

static UMBRA_FORCE_INLINE void packElem (UINT32* bv, int bit, UINT64 x, int width)
{
    // \todo optimize
    while (width--)
    {
        if (x & 1)
            setBit(bv, bit);
        else
            clearBit(bv, bit);

        bit++;
        x >>= 1;
    }
}

// bvSize must be power of two
static UMBRA_FORCE_INLINE UINT32 unpackElem32WrapClear (UINT32* bv, int bvSize, int idx)
{
    UINT32 dword = UMBRA_BIT_DWORD(idx);
    int shift = (UINT32)(idx & 0x1F);
    UINT32 left = bv[dword] >> shift;
    UINT32 right = ((bv[(dword + 1) & (bvSize - 1)] << (32 - shift)) & UMBRA_SIGN_EXTEND(shift)) & ~((1 << (32 - shift)) - 1);
    bv[dword] &= ((1 << shift) - 1);
    bv[(dword + 1) & (bvSize - 1)] &= ~((1 << shift) - 1);
    return (left | right);
}

static UMBRA_INLINE
void setBitRange (Umbra::UINT32* vector, const int start, const int end)
{
    UINT32* p = vector + UMBRA_BIT_DWORD(start);
    int num = end - start;
    UINT32 curMask = UMBRA_FIRSTDWORD_MASK(start);

    if ((curMask != 0xFFFFFFFF) && ((start >> 5) != (end >> 5)))
    {
        *p++ |= curMask;
        num -= (32 - (start & 0x1F));
        curMask = 0xFFFFFFFF;
    }
    UMBRA_ASSERT(num >= 0);

    UINT32 dwords = (num >> 5);
    memset(p, 0xFF, dwords * 4);

    if (num & 0x1F)
        p[dwords] |= (curMask & UMBRA_LASTDWORD_MASK(end));
}

static UMBRA_INLINE void
clearBitRange (Umbra::UINT32* vector, const int start, const int end)
{
    UINT32* p = vector + UMBRA_BIT_DWORD(start);
    int num = end - start;
    UINT32 curMask = UMBRA_FIRSTDWORD_MASK(start);

    if ((curMask != 0xFFFFFFFF) && ((start >> 5) != (end >> 5)))
    {
        *p++ &= ~curMask;
        num -= (32 - (start & 0x1F));
        curMask = 0xFFFFFFFF;
    }
    while (num >= 32)
    {
        *p++ = 0;
        num -= 32;
    }
    if (num)
        *p &= ~(curMask & UMBRA_LASTDWORD_MASK(end));
}

static UMBRA_INLINE bool
testBitRange (const Umbra::UINT32* vector, const int start, const int end)
{
    const UINT32* p = vector + UMBRA_BIT_DWORD(start);
    int num = end - start;
    UINT32 curMask = UMBRA_FIRSTDWORD_MASK(start);
    UINT32 test = 0;

    if ((curMask != 0xFFFFFFFF) && ((start >> 5) != (end >> 5)))
    {
        test = (*p++ & curMask);
        num -= (32 - (start & 0x1F));
        curMask = 0xFFFFFFFF;
    }
    while (!test && (num >= 32))
    {
        test = *p++;
        num -= 32;
    }
    if (!test && num)
        test = (*p & curMask & UMBRA_LASTDWORD_MASK(end));

    return test != 0;
}

static UMBRA_INLINE void copyBitRange (Umbra::UINT32* dst, int dstOfs, const Umbra::UINT32* src, int srcOfs, int num)
{
    // optimize
    while (num--)
    {
        if (testBit(src, srcOfs++))
            setBit(dst, dstOfs++);
        else
            clearBit(dst, dstOfs++);
    }
}

BitRangeResult  testBitRangeFull        (const UINT32* vector, const int start, const int end);
bool            testAndSetBitRange      (UINT32* vector, int start, int end);
bool            testAllAndSetBitRange   (UINT32* vector, int start, int end);
void            bitOpsTest              (void);
void            bitPackIntArray         (const UINT32* src, int num, UINT32* dst, int width);


} // namespace Umbra

#endif
