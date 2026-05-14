// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Vector.hpp>

namespace Umbra
{

float orient2dExact   (const Vec2f& a, const Vec2f& b, const Vec2f& c);
float orient3dExact   (const Vec3f& a, const Vec3f& b, const Vec3f& c, const Vec3f& d);
float orient2dInexact (const Vec2f& a, const Vec2f& b, const Vec2f& c);
float orient3dInexact (const Vec3f& a, const Vec3f& b, const Vec3f& c, const Vec3f& d);

}
