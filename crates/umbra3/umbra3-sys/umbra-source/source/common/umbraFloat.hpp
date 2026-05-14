#ifndef UMBRAFLOAT_HPP
#define UMBRAFLOAT_HPP

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
 * \brief   Umbra strict float wrapper
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{

class Float
{
public:
    UMBRA_FORCE_INLINE                  Float           (void)                              { UMBRA_DEBUG_CODE(union {float f; uint32 i;} x; x.i = 0xdeadbeefu; m_value = x.f;) }
    UMBRA_FORCE_INLINE                  Float           (float f)                           : m_value(f) { UMBRA_ASSERT(isFinite()); }
    explicit                            Float           (int i);
    explicit                            Float           (unsigned int i);
    explicit                            Float           (const double& d);
    UMBRA_FORCE_INLINE                  Float           (const Float& f)                    : m_value(f.m_value) { UMBRA_ASSERT(isFinite()); }
    UMBRA_FORCE_INLINE                  ~Float          (void)                              { /* empty */ }

    UMBRA_FORCE_INLINE float                get             (void) const                        { return m_value; }

    UMBRA_FORCE_INLINE Float&           operator=       (float f)                           { m_value = f; UMBRA_ASSERT(isFinite()); return *this; }
    Float&                              operator+=      (float f);
    Float&                              operator-=      (float f);
    Float&                              operator*=      (float f);
    Float&                              operator/=      (float f);

    UMBRA_FORCE_INLINE Float&           operator=       (Float f)                           { return operator=(f.m_value); }
    UMBRA_FORCE_INLINE Float&           operator+=      (Float f)                           { return operator+=(f.m_value); }
    UMBRA_FORCE_INLINE Float&           operator-=      (Float f)                           { return operator-=(f.m_value); }
    UMBRA_FORCE_INLINE Float&           operator*=      (Float f)                           { return operator*=(f.m_value); }
    UMBRA_FORCE_INLINE Float&           operator/=      (Float f)                           { return operator/=(f.m_value); }

    UMBRA_FORCE_INLINE const Float      operator+       (float f) const                     { Float temp(*this); temp += f; return temp; }
    UMBRA_FORCE_INLINE const Float      operator-       (float f) const                     { Float temp(*this); temp -= f; return temp; }
    UMBRA_FORCE_INLINE const Float      operator*       (float f) const                     { Float temp(*this); temp *= f; return temp; }
    UMBRA_FORCE_INLINE const Float      operator/       (float f) const                     { Float temp(*this); temp /= f; return temp; }

    UMBRA_FORCE_INLINE const Float      operator+       (Float f) const                     { return operator+(f.m_value); }
    UMBRA_FORCE_INLINE const Float      operator-       (Float f) const                     { return operator-(f.m_value); }
    UMBRA_FORCE_INLINE const Float      operator*       (Float f) const                     { return operator*(f.m_value); }
    UMBRA_FORCE_INLINE const Float      operator/       (Float f) const                     { return operator/(f.m_value); }

    UMBRA_FORCE_INLINE const Float      operator-       (void) const                        { return Float(-m_value); }
    UMBRA_FORCE_INLINE const Float&     operator+       (void) const                        { return *this; }

    UMBRA_FORCE_INLINE bool             operator==      (float f) const                     { UMBRA_ASSERT(isFinite(f)); return (m_value == f); }
    UMBRA_FORCE_INLINE bool             operator!=      (float f) const                     { UMBRA_ASSERT(isFinite(f)); return (m_value != f); }
    UMBRA_FORCE_INLINE bool             operator<       (float f) const                     { UMBRA_ASSERT(isFinite(f)); return (m_value < f); }
    UMBRA_FORCE_INLINE bool             operator<=      (float f) const                     { UMBRA_ASSERT(isFinite(f)); return (m_value <= f); }
    UMBRA_FORCE_INLINE bool             operator>       (float f) const                     { UMBRA_ASSERT(isFinite(f)); return (m_value > f); }
    UMBRA_FORCE_INLINE bool             operator>=      (float f) const                     { UMBRA_ASSERT(isFinite(f)); return (m_value >= f); }

    UMBRA_FORCE_INLINE bool             operator==      (Float f) const                     { return operator==(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator!=      (Float f) const                     { return operator!=(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator<       (Float f) const                     { return operator<(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator<=      (Float f) const                     { return operator<=(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator>       (Float f) const                     { return operator>(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator>=      (Float f) const                     { return operator>=(f.m_value); }

    UMBRA_FORCE_INLINE bool             isFinite        (void) const                        { return isFinite(m_value); }
    static bool                         isFinite        (float f);

private:
    float                               m_value;
};

UMBRA_FORCE_INLINE const Float          operator+   (float a, Float b)                  { return b + a; }
UMBRA_FORCE_INLINE const Float          operator-   (float a, Float b)                  { return Float(a) - b; }
UMBRA_FORCE_INLINE const Float          operator*   (float a, Float b)                  { return b * a; }
UMBRA_FORCE_INLINE const Float          operator/   (float a, Float b)                  { return Float(a) / b; }
UMBRA_FORCE_INLINE bool                 operator==  (float a, Float b)                  { return b == a; }
UMBRA_FORCE_INLINE bool                 operator!=  (float a, Float b)                  { return b != a; }
UMBRA_FORCE_INLINE bool                 operator<   (float a, Float b)                  { return b > a; }
UMBRA_FORCE_INLINE bool                 operator<=  (float a, Float b)                  { return b >= a; }
UMBRA_FORCE_INLINE bool                 operator>   (float a, Float b)                  { return b < a; }
UMBRA_FORCE_INLINE bool                 operator>=  (float a, Float b)                  { return b <= a; }

namespace Math
{

Float floor(Float f);
Float ceil(Float f);
Float abs(Float f);
Float sqrt(Float f);
Float pow(Float a, Float b);
Float exp(Float f);
Float log(Float f);
Float sin(Float f);
Float cos(Float f);
Float tan(Float f);
Float asin(Float f);
Float acos(Float f);
Float atan(Float f);
Float atan2(Float a, Float b);

UMBRA_FORCE_INLINE Float                    fabs        (Float f)                           { return abs(f); }
UMBRA_FORCE_INLINE Float                    pow         (Float a, float b)                  { return pow(a, Float(b)); }
UMBRA_FORCE_INLINE Float                    pow         (float a, Float b)                  { return pow(Float(a), b); }

} // namespace Math

} // namespace Umbra

#endif // UMBRAFLOAT_HPP

//--------------------------------------------------------------------
