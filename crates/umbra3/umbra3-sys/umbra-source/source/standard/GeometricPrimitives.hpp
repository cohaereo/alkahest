// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Vector.hpp>

namespace Umbra
{

template<typename _Scalar, int Dim, int NumVertices>
class Polygon
{
public:
    typedef _Scalar Scalar;
    typedef NumTraits<Scalar> ScalarTraits;
    typedef Matrix<Scalar, Dim, 1> CoordinateType;

    Polygon() {}

    const CoordinateType& operator[] (int n) const { return m_vert[n]; }
    CoordinateType& operator[] (int n) { return m_vert[n]; }

    bool operator== (const Polygon& other)
    {
        for (int i = 0; i < NumVertices; i++)
        {
            if (m_vert[i] != other.m_vert[i])
                return false;
        }
        return true;
    }

private:
    CoordinateType m_vert[NumVertices];
};

typedef Polygon<float, 3, 3> Tri3f;

using Eigen::Hyperplane;
using Eigen::ParametrizedLine;

typedef Hyperplane<float, 3> Plane3f;
typedef ParametrizedLine<float, 3> Line3f;


}
