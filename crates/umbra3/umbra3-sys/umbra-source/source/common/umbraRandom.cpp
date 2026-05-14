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
 * \brief   Random number generator
 *
 */

#include "umbraRandom.hpp"

using namespace Umbra;

static UMBRA_FORCE_INLINE uint32 rotateLeft (uint32 x, int32 r)  { return (x << r) | (x >> (sizeof(x)*8-r));   }

//------------------------------------------------------------------------
// Union of a float and an integer
//------------------------------------------------------------------------

union FloatInt
{
    float   randp1;
    uint32  randBits;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Gets random integer
 * \return  Random integer in [0,2^32-1]
 *//*-------------------------------------------------------------------*/

uint32 Umbra::Random::getI(void)
{
    UMBRA_ASSERT(m_p1 >= 0 && m_p1 < KK);
    UMBRA_ASSERT(m_p2 >= 0 && m_p2 < KK);
    uint32 x = m_randbuffer[m_p1] = rotateLeft(m_randbuffer[m_p1] + m_randbuffer[m_p2], RR);

    m_p1--;
    m_p1 += (m_p1>>31)&(KK);
    m_p2--;
    m_p2 += (m_p2>>31)&(KK);

    return x;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Gets random float
 * \return  Random float in [0, 1[
 *//*-------------------------------------------------------------------*/

float Umbra::Random::get(void)
{
    UMBRA_ASSERT(m_p1 >= 0 && m_p1 < KK);
    UMBRA_ASSERT(m_p2 >= 0 && m_p2 < KK);
    uint32 x = m_randbuffer[m_p1] = rotateLeft(m_randbuffer[m_p1] + m_randbuffer[m_p2], RR);

    m_p1--;
    m_p1 += (m_p1>>31)&(KK);
    m_p2--;
    m_p2 += (m_p2>>31)&(KK);


    FloatInt p;
    p.randBits = (x & 0x7fffff) | 0x3F800000;       // get 32-bit random number and map into a float in range [1,2[
    return p.randp1-1.0f;                           // subtract 1 using FPU -> maps value to [0,1[
}

/*-------------------------------------------------------------------*//*!
 * \brief   Resets the generator with given value
 * \param   seed    New seed value.
 * \note    Generator will produce always the same sequence for the same
 *          seed.
 *//*-------------------------------------------------------------------*/

void Umbra::Random::reset (uint32 seed)
{
    int i;
    if (seed==0)
        seed--;

    for (i=0; i<KK; i++)
    {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        m_randbuffer[i] = seed;
    }

    m_p1 = 0;
    m_p2 = JJ;
    for (i=0; i<9; i++)
        getI();
}

/*-------------------------------------------------------------------*//*!
 * \brief   Constructor
 *//*-------------------------------------------------------------------*/

Umbra::Random::Random (void)
{
    reset (0xFFFFFFFF);
}

//------------------------------------------------------------------------
