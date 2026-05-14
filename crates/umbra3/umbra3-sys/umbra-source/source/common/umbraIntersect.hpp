// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRAINTERSECT_HPP
#define UMBRAINTERSECT_HPP

#include "umbraPrimitives.hpp"
#include "umbraAABB.hpp"

namespace Umbra
{

bool        intersect           (const AABB& a, const AABB& b);
bool        intersect           (const AABB& box, const Sphere& sphere);
bool        intersect           (const AABB& box, const LineSegment& line);
bool        intersect           (const AABB& box, const Triangle& triangle);
bool        intersect           (const AABB& box, const Quad& quad);
bool        intersect           (const Sphere& sphere, const Triangle& triangle);
bool        intersect           (const LineSegment& line, const Quad& quad);
bool        intersect           (const Vector4& plane, const AABB& box);
bool        intersect           (const Vector4& plane, const Triangle& tri);

} // namespace Umbra

#endif
