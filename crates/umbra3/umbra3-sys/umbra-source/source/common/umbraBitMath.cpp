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
 * \brief   Bit manipulation source code
 *
 */

#include "umbraBitMath.hpp"
#include <string.h>

namespace Umbra
{

#if !defined (UMBRA_X86_ASSEMBLY) // we don't need the LUT for x86 machines as they can use the bsr operation
namespace BitMath
{
    const signed char s_highestLUT[256] =           // lookup table for 'highest set bit' operation
    {
        -1,0,1,1,2,2,2,2,3,3,3,3,3,3,3,3,
        4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,
        5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
        5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
        6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,
        6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,
        6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,
        6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7
    };
} // Umbra::Common::BitMath
#endif // !UMBRA_X86_ASSEMBLY

BitVector::BitVector(size_t size, Allocator* a)
:   Base        (a),
    m_array     (0),
    m_dwords    (0)
{
    m_dwords = (size + 31) >> 5;
    if (m_dwords)
        m_array = UMBRA_NEW_ARRAY(uint32, m_dwords);
}

BitVector::BitVector(const BitVector& s)
:   Base        (0),
    m_array     (0),
    m_dwords    (0)
{
    setAllocator(s.getAllocator());
    *this = s;
}

BitVector::~BitVector(void)
{
    UMBRA_DELETE_ARRAY(m_array);
}

void BitVector::setRange(size_t start, size_t end)
{
    UINT32* p = m_array + (start >> 5);
    size_t num = end - start;
    UINT32 curMask = ~((1 << start) - 1);

    if ((curMask != 0xFFFFFFFF) && ((start >> 5) != (end >> 5)))
    {
        *p++ |= curMask;
        num -= (32 - (start & 0x1F));
        curMask = 0xFFFFFFFF;
    }

    size_t dwords = (num >> 5);
    fillDWord(p, 0xFFFFFFFF, dwords);

    if (num & 0x1F)
        p[dwords] |= (curMask & ((1 << end) - 1));
}

void BitVector::reset(size_t size)
{
    size = (size_t)(size + 31) >> 5;
    if (size > m_dwords)
    {
        UMBRA_DELETE_ARRAY(m_array);
        m_array = UMBRA_NEW_ARRAY(uint32, size);
    }
    m_dwords = size;
}

void BitVector::resize(size_t size, bool clear, bool value)
{
    size = (size_t)(size + 31) >> 5;
    if (size > m_dwords)
    {
        uint32* old = m_array;
        m_array = UMBRA_NEW_ARRAY(uint32, size);
        memcpy(m_array, old, m_dwords * sizeof(uint32));
        UMBRA_DELETE_ARRAY(old);
        if (clear)
        {
            uint32 v = (value) ? 0xffffffff : 0;
            for (size_t i = m_dwords; i < size; i++)
                m_array[i] = v;
        }
    }
    m_dwords = size;
}

BitVector& BitVector::operator=(const BitVector& s)
{
    if (&s != this)
    {
        reset(s.m_dwords*32);
        memcpy(m_array, s.m_array, m_dwords * sizeof(uint32));
    }
    return *this;
}

void BitVector::_and(const BitVector& s)
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    for (size_t i = 0; i < m_dwords; i++)
        m_array[i] &= s.m_array[i];
}

void BitVector::andNot(const BitVector& s)
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    for (size_t i = 0; i < m_dwords; i++)
        m_array[i] &= ~s.m_array[i];
}

void BitVector::_or(const BitVector& s)
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    for (size_t i = 0; i < m_dwords; i++)
        m_array[i] |= s.m_array[i];
}

void BitVector::orNot(const BitVector& s)
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    for (size_t i = 0; i < m_dwords; i++)
        m_array[i] |= ~s.m_array[i];
}

void BitVector::_xor(const BitVector& s)
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    for (size_t i = 0; i < m_dwords; i++)
        m_array[i] ^= s.m_array[i];
}

bool BitVector::test(const BitVector& s) const
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    for (size_t i = 0; i < m_dwords; i++)
        if ((m_array[i] & s.m_array[i]) != 0)
            return true;
    return false;
}

bool BitVector::testNot(const BitVector& s) const
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    for (size_t i = 0; i < m_dwords; i++)
        if ((m_array[i] & ~s.m_array[i]) != 0)
            return true;
    return false;
}

/*----------------------------------------------------------------------*//*!
 * \brief   returns true if two memory blocks are equal, false otherwise
 * \return  true if two memory blocks are equal, false otherwise
 *//*----------------------------------------------------------------------*/

bool memEqual (const void* s0, const void* s1, size_t bytes)
{
    UMBRA_ASSERT(s0 && s1);
    const uint32* is0 = (const uint32*)s0;
    const uint32* is1 = (const uint32*)s1;
    size_t dwords = bytes>>2;                                               // div by four
    size_t i;
    for (i = 0; i < dwords; i++)                                        // compare as dwords
    if (*is0++ != *is1++)
        return false;
    bytes&=3;                                                           // last 0-3 bytes
    const unsigned char* bs0 = (const unsigned char*)is0;
    const unsigned char* bs1 = (const unsigned char*)is1;
    for (i = 0; i < bytes; i++)
    if (*bs0++ != *bs1++)
        return false;
    return true;                                                        // blocks are equal
}

/*----------------------------------------------------------------------*//*!
 * \brief   essentially same as memset() but performs proper alignment
 *          on compilers that don't handle the alignment
 *//*----------------------------------------------------------------------*/

void fillByte  (void* dest, unsigned char value, size_t bytes)
{
    if (bytes<=0)
        return;
    UMBRA_ASSERT(dest);
    if (bytes >= 8)
    {
        size_t align = reinterpret_cast<size_t>(dest)&(size_t)(7);
        if (align!=0)                                                   // non-aligned data
        {
            size_t b = 8-align;
            memset (dest,value,b);                                    // system library memset
            memset ((unsigned char*)(dest)+b,value,bytes-b);          // system library memset
            return;
        }
        // fallthru
    }

    memset (dest,value,bytes);                                        // system library memset
}

/*-------------------------------------------------------------------*//*!
 * \brief   Fills memory with dword granularity
 * \param   d       Address
 * \param   pattern Pattern used to fill
 * \param   N       Number of dwords
 *//*-------------------------------------------------------------------*/
void fillDWord (uint32* d, uint32 pattern, size_t N)
{
        if (N<=0)
            return;

        size_t blocks = (N>>3);
        while (blocks)
        {
            d[0] = pattern;
            d[1] = pattern;
            d[2] = pattern;
            d[3] = pattern;
            d[4] = pattern;
            d[5] = pattern;
            d[6] = pattern;
            d[7] = pattern;
            d+=8;
            blocks--;
        }

        N&=7;
        while (N)
        {
            *d++ = pattern;
            N--;
        }
}

} // namespace Umbra

//------------------------------------------------------------------------
