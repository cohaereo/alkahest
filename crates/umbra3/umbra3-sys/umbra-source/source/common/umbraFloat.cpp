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

#include "umbraFloat.hpp"

#include "umbraMath.hpp"

#include <float.h>

namespace Umbra
{

//--------------------------------------------------------------------

bool Float::isFinite(float f)
{
    return (f == f && f >= -FLT_MAX && f <= FLT_MAX);
}

Float::Float(const double& d)
{
    m_value = (float)d;
    UMBRA_ASSERT(isFinite());
}

Float::Float(int i)
{
    m_value = (float)i;
    UMBRA_ASSERT(isFinite());
}

Float::Float(unsigned int i)
{
    m_value = (float)i;
    UMBRA_ASSERT(isFinite());
}

Float& Float::operator+=(float f)
{
    m_value += f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Float& Float::operator-=(float f)
{
    m_value -= f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Float& Float::operator*=(float f)
{
    m_value *= f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Float& Float::operator/=(float f)
{
    m_value /= f;
    UMBRA_ASSERT(isFinite());
    return *this;
}

Float Math::floor(Float f)
{
    Float ff(floorf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::ceil(Float f)
{
    Float ff(ceilf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::abs(Float f)
{
    Float ff(fabsf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::sqrt(Float f)
{
    Float ff(sqrtf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::pow(Float a, Float b)
{
    Float ff(powf(a.get(), b.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::exp(Float f)
{
    Float ff(expf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::log(Float f)
{
    Float ff(logf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::sin(Float f)
{
    Float ff(sinf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::cos(Float f)
{
    Float ff(cosf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::tan(Float f)
{
    Float ff(tanf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::asin(Float f)
{
    Float ff(asinf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::acos(Float f)
{
    Float ff(acosf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::atan(Float f)
{
    Float ff(atanf(f.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

Float Math::atan2(Float a, Float b)
{
    Float ff(atan2f(a.get(), b.get()));
    UMBRA_ASSERT(ff.isFinite());
    return ff;
}

}
//--------------------------------------------------------------------
