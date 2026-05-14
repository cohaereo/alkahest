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
 * \brief   Umbra bit operations
 *
 */

#include "umbraBitOps.hpp"

namespace Umbra
{

Umbra::BitRangeResult testBitRangeFull (const Umbra::UINT32* vector, const int start, const int end)
{
    const UINT32* p = vector + UMBRA_BIT_DWORD(start);
    int num = end - start;
    UINT32 curMask = UMBRA_FIRSTDWORD_MASK(start);
    UINT32 testAll = 0;
    UINT32 testAny = 0;

    if ((curMask != 0xFFFFFFFF) && ((start >> 5) != (end >> 5)))
    {
        UINT32 val = (*p++ & curMask);
        testAny = val;
        testAll = val ^ curMask;
        num -= (32 - (start & 0x1F));
        curMask = 0xFFFFFFFF;
    }
    while (num >= 32 && (!testAny || !testAll))
    {
        UINT32 val = *p++;
        testAny |= val;
        testAll |= ~val;
        num -= 32;
    }
    if (num)
    {
        curMask &= UMBRA_LASTDWORD_MASK(end);
        UINT32 val = (*p & curMask);
        testAny |= val;
        testAll |= val ^ curMask;
    }

    if (!testAny)
        return BitRange_None;
    if (!testAll)
        return BitRange_All;
    return BitRange_Some;
}

bool testAndSetBitRange (Umbra::UINT32* vector, int start, int end)
{
    UINT32* p = vector + UMBRA_BIT_DWORD(start);
    int num = end - start;
    UINT32 curMask = UMBRA_FIRSTDWORD_MASK(start);
    UINT32 test = 0;

    if ((curMask != 0xFFFFFFFF) && ((start >> 5) != (end >> 5)))
    {
        UINT32 old = *p;
        *p++ |= curMask;
        test = (old & curMask);
        num -= (32 - (start & 0x1F));
        curMask = 0xFFFFFFFF;
    }
    while (num >= 32)
    {
        UINT32 old = *p;
        *p = 0xFFFFFFFF;
        test |= old;
        num -= 32;
    }
    if (num)
    {
        UINT32 old = *p;
        curMask &= UMBRA_LASTDWORD_MASK(end);
        *p |= curMask;
        test |= (old & curMask);
    }

    return test != 0;
}

bool testAllAndSetBitRange (Umbra::UINT32* vector, int start, int end)
{
    UINT32* p = vector + UMBRA_BIT_DWORD(start);
    int num = end - start;
    UINT32 curMask = UMBRA_FIRSTDWORD_MASK(start);
    UINT32 test = 0;

    if ((curMask != 0xFFFFFFFF) && ((start >> 5) != (end >> 5)))
    {
        UINT32 old = *p;
        *p++ |= curMask;
        test = (old & curMask) ^ curMask;
        num -= (32 - (start & 0x1F));
        curMask = 0xFFFFFFFF;
    }
    while (num >= 32)
    {
        UINT32 old = *p;
        *p++ = 0xFFFFFFFF;
        test |= ~old;
        num -= 32;
    }
    if (num)
    {
        UINT32 old = *p;
        curMask &= UMBRA_LASTDWORD_MASK(end);
        *p |= curMask;
        test |= (old & curMask) ^ curMask;
    }

    return test == 0;
}

void bitPackIntArray (const UINT32* src, int num, UINT32* dst, int width)
{
    int ofs = 0;
    for (int i = 0; i < num; i++)
    {
        UINT32 val = src[i];
        UMBRA_ASSERT(bitsForValue(val) <= width);
        copyBitRange(dst, ofs, &val, 0, width);
        ofs += width;
    }
}

void bitOpsTest (void)
{
    // Test countLeadingZeros().
    //UMBRA_ASSERT(countLeadingZeros(0) == 32);
    UMBRA_ASSERT(countLeadingZeros(0x1543) == 19);
    UMBRA_ASSERT(countLeadingZeros(0x237F58A0) == 2);
    UMBRA_ASSERT(countLeadingZeros(0xFFFFFFFF) == 0);
    for (int i = 0; i < 31; i++)
    {
        UMBRA_ASSERT(countLeadingZeros(1<<i) == 31-i);
        UMBRA_ASSERT(countLeadingZeros((1<<i) + ((1<<i)>>1)) == 31-i);
    }

    // Test countTrailingZeros().
    //UMBRA_ASSERT(countTrailingZeros(0) == 32);
    UMBRA_ASSERT(countTrailingZeros(0x7321) == 0);
    UMBRA_ASSERT(countTrailingZeros(0x787AB20) == 5);
    UMBRA_ASSERT(countTrailingZeros(0xFFFFFFFF) == 0);
    for (int i = 0; i < 31; i++)
    {
        UMBRA_ASSERT(countTrailingZeros(1<<i) == i);
        UMBRA_ASSERT(countTrailingZeros((1<<i) + (1<<(i+1))) == i);
        UMBRA_ASSERT(countTrailingZeros(0xFFFFFFFFu << i) == i);
    }
}

} // namespace Umbra
