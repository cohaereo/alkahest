#ifndef UMBRADOUBLE_HPP
#define UMBRADOUBLE_HPP

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
 * \brief   Umbra strict double wrapper
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{

class Double
{
public:
    UMBRA_FORCE_INLINE                  Double          (void)                              { UMBRA_DEBUG_CODE(union {float f; uint64 i;} x; x.i = 0xdeadbeefdeadbeefull; m_value = x.f;) }
    UMBRA_FORCE_INLINE                  Double          (const double& d)                   : m_value(d) { UMBRA_ASSERT(isFinite()); }
    explicit                            Double          (int i);
    explicit                            Double          (unsigned int i);
    UMBRA_FORCE_INLINE explicit         Double          (float f)                           : m_value(f) { UMBRA_ASSERT(isFinite()); }
    UMBRA_FORCE_INLINE                  Double          (const Double& f)                   : m_value(f.m_value) { UMBRA_ASSERT(isFinite()); }
    UMBRA_FORCE_INLINE                  ~Double         (void)                              { /* empty */ }

    UMBRA_FORCE_INLINE const double&        get             (void) const                        { return m_value; }

    Double&                             operator=       (const double& f)                   { m_value = f; UMBRA_ASSERT(isFinite()); return *this; }
    Double&                             operator+=      (const double& f);
    Double&                             operator-=      (const double& f);
    Double&                             operator*=      (const double& f);
    Double&                             operator/=      (const double& f);

    UMBRA_FORCE_INLINE Double&          operator=       (const Double& f)                   { return operator=(f.m_value); }
    UMBRA_FORCE_INLINE Double&          operator+=      (const Double& f)                   { return operator+=(f.m_value); }
    UMBRA_FORCE_INLINE Double&          operator-=      (const Double& f)                   { return operator-=(f.m_value); }
    UMBRA_FORCE_INLINE Double&          operator*=      (const Double& f)                   { return operator*=(f.m_value); }
    UMBRA_FORCE_INLINE Double&          operator/=      (const Double& f)                   { return operator/=(f.m_value); }

    UMBRA_FORCE_INLINE const Double     operator+       (const double& f) const             { Double temp(*this); temp += f; return temp; }
    UMBRA_FORCE_INLINE const Double     operator-       (const double& f) const             { Double temp(*this); temp -= f; return temp; }
    UMBRA_FORCE_INLINE const Double     operator*       (const double& f) const             { Double temp(*this); temp *= f; return temp; }
    UMBRA_FORCE_INLINE const Double     operator/       (const double& f) const             { Double temp(*this); temp /= f; return temp; }

    UMBRA_FORCE_INLINE const Double     operator+       (const Double& f) const             { return operator+(f.m_value); }
    UMBRA_FORCE_INLINE const Double     operator-       (const Double& f) const             { return operator-(f.m_value); }
    UMBRA_FORCE_INLINE const Double     operator*       (const Double& f) const             { return operator*(f.m_value); }
    UMBRA_FORCE_INLINE const Double     operator/       (const Double& f) const             { return operator/(f.m_value); }

    UMBRA_FORCE_INLINE const Double     operator-       (void) const                        { return Double(-m_value); }
    UMBRA_FORCE_INLINE const Double&        operator+       (void) const                        { return *this; }

    UMBRA_FORCE_INLINE bool             operator==      (const double& f) const             { UMBRA_ASSERT(isFinite(f)); return (m_value == f); }
    UMBRA_FORCE_INLINE bool             operator!=      (const double& f) const             { UMBRA_ASSERT(isFinite(f)); return (m_value != f); }
    UMBRA_FORCE_INLINE bool             operator<       (const double& f) const             { UMBRA_ASSERT(isFinite(f)); return (m_value < f); }
    UMBRA_FORCE_INLINE bool             operator<=      (const double& f) const             { UMBRA_ASSERT(isFinite(f)); return (m_value <= f); }
    UMBRA_FORCE_INLINE bool             operator>       (const double& f) const             { UMBRA_ASSERT(isFinite(f)); return (m_value > f); }
    UMBRA_FORCE_INLINE bool             operator>=      (const double& f) const             { UMBRA_ASSERT(isFinite(f)); return (m_value >= f); }

    UMBRA_FORCE_INLINE bool             operator==      (const Double& f) const             { return operator==(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator!=      (const Double& f) const             { return operator!=(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator<       (const Double& f) const             { return operator<(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator<=      (const Double& f) const             { return operator<=(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator>       (const Double& f) const             { return operator>(f.m_value); }
    UMBRA_FORCE_INLINE bool             operator>=      (const Double& f) const             { return operator>=(f.m_value); }

    UMBRA_FORCE_INLINE bool             isFinite        (void) const                        { return isFinite(m_value); }
    static bool                         isFinite        (const double& d);

private:
    double                              m_value;
};

UMBRA_FORCE_INLINE const Double         operator+   (const double& a, const Double& b)  { return b + a; }
UMBRA_FORCE_INLINE const Double         operator-   (const double& a, const Double& b)  { return Double(a) - b; }
UMBRA_FORCE_INLINE const Double         operator*   (const double& a, const Double& b)  { return b * a; }
UMBRA_FORCE_INLINE const Double         operator/   (const double& a, const Double& b)  { return Double(a) / b; }
UMBRA_FORCE_INLINE bool                 operator==  (const double& a, const Double& b)  { return b == a; }
UMBRA_FORCE_INLINE bool                 operator!=  (const double& a, const Double& b)  { return b != a; }
UMBRA_FORCE_INLINE bool                 operator<   (const double& a, const Double& b)  { return b > a; }
UMBRA_FORCE_INLINE bool                 operator<=  (const double& a, const Double& b)  { return b >= a; }
UMBRA_FORCE_INLINE bool                 operator>   (const double& a, const Double& b)  { return b < a; }
UMBRA_FORCE_INLINE bool                 operator>=  (const double& a, const Double& b)  { return b <= a; }

namespace Math
{

Double floor(const Double& f);
Double ceil(const Double& f);
Double abs(const Double& f);
Double sqrt(const Double& f);
Double pow(const Double& a, const Double& b);
Double exp(const Double& f);
Double log(const Double& f);
Double sin(const Double& f);
Double cos(const Double& f);
Double tan(const Double& f);
Double asin(const Double& f);
Double acos(const Double& f);
Double atan(const Double& f);
Double atan2(const Double& a, const Double& b);

UMBRA_FORCE_INLINE Double               fabs        (const Double& f)                   { return abs(f); }
UMBRA_FORCE_INLINE Double               pow         (const Double& a, const double& b)  { return pow(a, Double(b)); }
UMBRA_FORCE_INLINE Double               pow         (const double& a, const Double& b)  { return pow(Double(a), b); }

} // namespace Math

} // namespace Umbra

#endif // UMBRADOUBLE_HPP

//--------------------------------------------------------------------
