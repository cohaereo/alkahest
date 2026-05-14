// Copyright (c) 2010-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include "umbraVector.hpp"

namespace Umbra
{
    int     clipPolygonPlane          (Vector3* clipped, const Vector3* polygon, const Vector4& plEq, int N);
    Vector4 getNormalizedPlaneEquation(const Vector3& a, const Vector3& b, const Vector3& c);
}
