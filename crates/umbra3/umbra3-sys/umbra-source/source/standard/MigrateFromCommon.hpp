// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

// Include this header only in translation units that HAVE to use both common and
// standard types. This header is temporary.

#include <standard/AxisExtents.hpp>
#include <standard/GeometricPrimitives.hpp>
#include <common/umbraAABB.hpp>
#include <common/umbraMatrix.hpp>

namespace Umbra
{

static inline AABoxf migrate(AABB a) { return (AABoxf&)a; }
static inline AABB migrate(AABoxf a) { return (AABB&)a; }
static inline Vec2f migrate(Vector2 v) { return (Vec2f&)v; }
static inline Vec3f migrate(Vector3 v) { return (Vec3f&)v; }
static inline Vec3i migrate(Vector3i v) { return (Vec3i&)v; }
static inline Vec4f migrate(Vector4 v) { return (Vec4f&)v; }
static inline Matrix<float, 4, 4, Eigen::RowMajor> migrate(Matrix4x4 m) { return Matrix<float, 4, 4, Eigen::RowMajor>(&m[0][0]); }
static inline Vector3 migrate(Vec3f v) { return (Vector3&)v; }
static inline Vector3i migrate(Vec3i v) { return (Vector3i&)v; }
static inline Vector2 migrate(Vec2f v) { return (Vector2&)v; }

}