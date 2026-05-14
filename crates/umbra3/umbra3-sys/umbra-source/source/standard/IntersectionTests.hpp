// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/GeometricPrimitives.hpp>
#include <standard/AxisExtents.hpp>

namespace Umbra
{

template<typename Scalar, int NumPolygonVerts>
bool polygonIntersectsAABox(Polygon<Scalar, 3, NumPolygonVerts> polygon, AxisExtents<Scalar, 3> box);

template<>
bool polygonIntersectsAABox(Tri3f polygon, AABoxf box);

bool lineIntersectsTriangle(Line3f ray, Tri3f tri);

}
