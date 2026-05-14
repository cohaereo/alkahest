// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Assert.hpp>
#include <standard/Vector.hpp>

namespace Umbra
{

template<typename _Scalar, int Dim>
class AxisExtents
{
public:
    typedef _Scalar Scalar;
    typedef NumTraits<Scalar> ScalarTraits;
    typedef Matrix<Scalar, Dim, 1> CoordinateType;

    AxisExtents() { setInvalid(); }

    template<typename OtherVectorType1, typename OtherVectorType2>
    AxisExtents(const OtherVectorType1& _min, const OtherVectorType2& _max) : m_min(_min), m_max(_max) {}

    template<typename OtherScalarType>
    inline explicit AxisExtents(const AxisExtents<OtherScalarType, Dim>& other)
    {
        m_min = other.min().template cast<Scalar>();
        m_max = other.max().template cast<Scalar>();
    }

    const CoordinateType& min() const { return m_min; }
    CoordinateType& min() { return m_min; }
    const CoordinateType& max() const { return m_max; }
    CoordinateType& max() { return m_max; }

    void setInvalid()
    {
        m_max.setConstant(ScalarTraits::lowest());
        m_min.setConstant(ScalarTraits::highest());
    }

    bool isValid() const
    {
        return !(m_min.array() > m_max.array()).any();
    }

    bool contains (const AxisExtents& b) const
    {
        return (m_min.array() <= b.min().array()).all() &&
               (m_max.array() >= b.max().array()).all();
    }

    bool intersects(const AxisExtents& b) const
    {
        return !(m_min.cwiseMax(b.m_min).array() > m_max.cwiseMin(b.m_max).array()).any();
    }

    AxisExtents& scale(Scalar factor)
    {
        m_min *= factor;
        m_max *= factor;
        return *this;
    }

    AxisExtents& inflate(Scalar amount)
    {
        CoordinateType a;
        a.setConstant(amount);
        m_min -= a;
        m_max += a;
        return *this;
    }

    AxisExtents& extend(const CoordinateType& pt)
    {
        m_min = m_min.cwiseMin(pt);
        m_max = m_max.cwiseMax(pt);
        return *this;
    }

    const CoordinateType sizes() const
    {
        return m_max - m_min;
    }

    AxisExtents merge(const AxisExtents& other) const
    {
        return AxisExtents(m_min.cwiseMin(other.m_min), m_max.cwiseMax(other.m_max));
    }

    AxisExtents intersect(const AxisExtents& other) const
    {
        return AxisExtents(m_min.cwiseMax(other.m_min), m_max.cwiseMin(other.m_max));
    }

private:
    CoordinateType m_min;
    CoordinateType m_max;

};

typedef AxisExtents<float, 3> AABoxf;
typedef AxisExtents<float, 2> AARectf;
typedef AxisExtents<int, 3> AABoxi;
typedef AxisExtents<int, 2> AARecti;

template<typename T> typename AxisExtents<T, 2>::CoordinateType getRectCorner(const AxisExtents<T, 2>& rect, int idx)
{
    UMBRA_ASSERT(idx >= 0 && idx <= 3);
    return typename AxisExtents<T, 2>::CoordinateType((idx & 1) ? rect.max().x() : rect.min().x(),
                                                      (idx & 2) ? rect.max().y() : rect.min().y());
}

template<typename T> typename AxisExtents<T, 3>::CoordinateType getBoxCorner(const AxisExtents<T, 3>& box, int idx)
{
    UMBRA_ASSERT(idx >= 0 && idx <= 7);
    return typename AxisExtents<T, 3>::CoordinateType((idx & 1) ? box.max().x() : box.min().x(),
                                                      (idx & 2) ? box.max().y() : box.min().y(),
                                                      (idx & 4) ? box.max().z() : box.min().z());
}

template<typename T> AxisExtents<T, 2> getBoxFaceRect(const AxisExtents<T, 3>& box, int face)
{
    UMBRA_ASSERT(face >= 0 && face <= 5);

    int axis = face >> 1;
    int axisX = (1<<axis)&3;
    int axisY = (1<<axisX)&3;

    return AxisExtents<T, 2>(
        typename AxisExtents<T, 2>::CoordinateType(box.min()[axisX], box.min()[axisY]),
        typename AxisExtents<T, 2>::CoordinateType(box.max()[axisX], box.max()[axisY]));
}

}
