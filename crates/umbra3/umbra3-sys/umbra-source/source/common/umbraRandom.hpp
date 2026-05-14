#ifndef UMBRARANDOM_HPP
#define UMBRARANDOM_HPP

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
 * \brief   Umbra Random number generation
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief           Random number generator that returns floating point
 *                  random numbers in range [0,1[ or integer random numbers
 *                  in range [0,2^32-1]
 *
 * \note            The code is based on Agner Fog's RANROT A algorithm
 *                  and original C++ implementation. The conditional branches
 *                  of the original code have been replaced by bit arithmetic and
 *                  the code is somewhat cleaned up. For further information about
 *                  the algorithm, please see "http://www.agner.org/random/".
 *
 * \note            The integer version executes in approximately 13.5 Pentium II
 *                  clocks and the floating point version in 17.5.
 *//*-------------------------------------------------------------------*/

class Random
{
public:
                Random      (void);
    float       get         (void);                     //!< return float [0,1[
    uint32      getI        (void);                     //!< return integer [0,2^32-1]
    void        reset       (uint32 seed);              //!< randomize
private:
    enum
    {
        KK = 11,
        JJ =  7,
        RR = 13
    };

    int     m_p1;
    int     m_p2;
    uint32  m_randbuffer[KK];
};

} // namespace Umbra

#endif // UMBRARANDOM_HPP

//--------------------------------------------------------------------
