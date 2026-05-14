#ifndef UMBRAMATH_HPP
#define UMBRAMATH_HPP

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
 * \brief   Umbra Array
 * \todo [wili] Where did you get sqrtf() from?? You are not including <math.h>
 *       etc. Also; it is a non-portable construct so be prepared for a different
 *       implementation on some platforms.
 *
 */

#include "umbraPrivateDefs.hpp"

#include <math.h>

namespace Umbra
{

namespace Math
{
    typedef union FloatInt_u
    {
        float f;
        int32 i;
    } FloatInt;

    typedef union FloatUInt_u
    {
        float  f;
        uint32 i;
    } FloatUInt;

    //------------------------------------------------------------------------------------
    // TODO optimize..
    //------------------------------------------------------------------------------------
    UMBRA_FORCE_INLINE  double  pi              (void)                                  { return 3.1415926535897932384626433832795; }
    UMBRA_FORCE_INLINE  float   fabs            (float f)                               { return ::fabsf(f);  }
    UMBRA_FORCE_INLINE  float   sqr             (float f)                               { return f*f;   }
    UMBRA_FORCE_INLINE  float   sqrt            (float f)                               { return ::sqrtf(f);  }
    UMBRA_FORCE_INLINE  double  sqrt            (double f)                              { return ::sqrt(f); }
    UMBRA_FORCE_INLINE  float   reciprocalSqrt  (float f)                               { UMBRA_ASSERT(f>0.0f);  return 1.0f / (::sqrt(f)); }
    UMBRA_FORCE_INLINE  float   radToDeg        (float rad)                             { return (rad*180.0f)/(float)pi();}
    UMBRA_FORCE_INLINE  float   degToRad        (float deg)                             { return (deg*(float)pi())/180.0f;}
    UMBRA_FORCE_INLINE  double  sin             (double f)                              { return sin(f); }
    UMBRA_FORCE_INLINE  double  cos             (double f)                              { return cos(f); }
    UMBRA_FORCE_INLINE  double  tan             (double f)                              { return tan(f); }

    template <class T> UMBRA_FORCE_INLINE void sort2    (T& a, T& b)                    { if (a>b) swap(a,b); }
    template <class T> UMBRA_FORCE_INLINE void sort3    (T& a, T& b, T& c)              { sort2(a,b); sort2(b,c); sort2(a,b); }         // sorts values a,b,c so that a<=b<=c
    template <class T> UMBRA_FORCE_INLINE void sort4    (T& a, T& b, T& c, T& d)        { sort3(a,b,c);   sort3(b,c,d);   sort2(a,b); } // NOTE: not optimal

    UMBRA_FORCE_INLINE  int     intChop         (const float& f)
    {
        int32 a         = *reinterpret_cast<const int32*>(&f);          // take bit pattern of float into a register
        int32 sign      = (a>>31);                                      // sign = 0xFFFFFFFF if original value is negative, 0 if positive
        int32 mantissa  = (a&((1<<23)-1))|(1<<23);                      // extract mantissa and add the hidden bit
        int32 exponent  = ((a&0x7fffffff)>>23)-127;                     // extract the exponent
        int32 r         = ((uint32)(mantissa)<<8)>>(31-exponent);       // ((1<<exponent)*mantissa)>>24 -- (we know that mantissa > (1<<24))
        return ((r ^ (sign)) - sign ) &~ (exponent>>31);                // add original sign. If exponent was negative, make return value 0.
    }

    UMBRA_FORCE_INLINE  float   incrementFloat  (const float& f)
    {
        uint32 i = *reinterpret_cast<const uint32*>(&f);
        if (i == 0x80000000u)
            i = 0;
        // the code below effectively subtracts one if value is negative,
        // or adds one if value is positive
        i -= ((i & 0x80000000)>>30);
        i++;

        FloatUInt fi;
        fi.i = i;
        return fi.f;    // *(float*)(&i);
    }

    template <class T> bool sameSign (const T& a, const T& b)
    {
        return (a*b >= 0.0f);
    }


} // namespace Math
} // namespace Umbra

#endif // UMBRAMATH_HPP
