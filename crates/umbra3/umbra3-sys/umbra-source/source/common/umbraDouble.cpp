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

#include "umbraDouble.hpp"
#include "umbraMath.hpp"

#include <float.h>

namespace Umbra
{

//--------------------------------------------------------------------

bool Double::isFinite(const double& d)
{
    return (d == d && d >= -DBL_MAX && d <= DBL_MAX);
}

Double::Double(int i)
{
    m_value = (double)i;
    UMBRA_ASSERT(isFinite());
}

Double::Double(unsigned int i)
{
    m_value = (double)i;
    UMBRA_ASSERT(isFinite());
}

Double& Double::operator+=(const double& f)
{
    m_value += f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Double& Double::operator-=(const double& f)
{
    m_value -= f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Double& Double::operator*=(const double& f)
{
    m_value *= f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Double& Double::operator/=(const double& f)
{
    m_value /= f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Double Math::floor(const Double& f)
{
    Double ff(::floor(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::ceil(const Double& f)
{
    Double ff(::ceil(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::abs(const Double& f)
{
    Double ff(::fabs(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::sqrt(const Double& f)
{
    Double ff(::sqrt(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::pow(const Double& a, const Double& b)
{
    Double ff(::pow(a.get(), b.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::exp(const Double& f)
{
    Double ff(::exp(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::log(const Double& f)
{
    Double ff(::log(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::sin(const Double& f)
{
    Double ff(::sin(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::cos(const Double& f)
{
    Double ff(::cos(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::tan(const Double& f)
{
    Double ff(::tan(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::asin(const Double& f)
{
    Double ff(::asin(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::acos(const Double& f)
{
    Double ff(::acos(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::atan(const Double& f)
{
    Double ff(::atan(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Double Math::atan2(const Double& a, const Double& b)
{
    Double ff(::atan2(a.get(), b.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

}
//--------------------------------------------------------------------
