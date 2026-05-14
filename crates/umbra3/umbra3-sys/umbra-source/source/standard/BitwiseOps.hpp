// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Base.hpp>
#if UMBRA_IS_WIN32
#if UMBRA_OS == UMBRA_XBOX360
#   include <ppcintrinsics.h>
#else
#   include <intrin.h>
#endif
#endif

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   Count leading zeros in 32-bit word, result undefined for
 *          input value v == 0 (asserted in debug builds).
 *//*-------------------------------------------------------------------*/

static UMBRA_FORCE_INLINE int countLeadingZeros (uint32_t v)
{
    UMBRA_ASSERT(v != 0);
#if UMBRA_OS == UMBRA_XBOX360
    return _CountLeadingZeros(v);
#elif UMBRA_GCC_INTRINSICS && (UMBRA_COMPILER != UMBRA_SNC)
    return __builtin_clz(v);
#elif UMBRA_IS_WIN32
    unsigned long pos;
    _BitScanReverse(&pos, v);
    return 31 - (int)pos;
#else
    // simple multiply + lookup from Hacker's delight
    static const uint8_t MultiplyDeBruijnBitPosition[32] =
    {
        0, 9, 1, 10, 13, 21, 2, 29, 11, 14, 16, 18, 22, 25, 3, 30,
        8, 12, 20, 28, 15, 17, 24, 7, 19, 27, 23, 6, 26, 5, 4, 31
    };

    v |= v >> 1; v |= v >> 2; v |= v >> 4; v |= v >> 8; v |= v >> 16;

    // note: returns 31 for input value 0
    return 31 - (int)MultiplyDeBruijnBitPosition[(uint32_t)(v * 0x07C4ACDDU) >> 27];
#endif
}

/*-------------------------------------------------------------------*//*!
 * \brief   Count trailing zeros in 32-bit word, result undefined for
 *          input value v == 0 (asserted in debug builds).
 *//*-------------------------------------------------------------------*/

static UMBRA_FORCE_INLINE int countTrailingZeros (uint32_t v)
{
    UMBRA_ASSERT(v != 0);
#if UMBRA_GCC_INTRINSICS && (UMBRA_COMPILER != UMBRA_SNC)
    return __builtin_ctz(v);
#elif UMBRA_IS_WIN32 && (UMBRA_OS != UMBRA_XBOX360)
    unsigned long pos;
    _BitScanForward(&pos, v);
    return (int)pos;
#else
    return 31 - countLeadingZeros(v & (-(int32_t)v));
#endif
}

#define highestBitSet(v) (31 - countLeadingZeros(v))
#define lowestBitSet(v) countTrailingZeros(v)
#define log2Base(v) highestBitSet(v)

}