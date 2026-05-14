#pragma once

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
 * \brief   Umbra intersection routines
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraAABB.hpp"
#include "umbraSIMD.hpp"

namespace Umbra
{

bool  intersectAABBTriangle         (const AABB& aabb,const Vector3& t0, const Vector3& t1, const Vector3& t2);
// \todo SIMDRegisters for vertices too
bool  intersectAABBTriangleSIMD     (const SIMDRegister& aabbMin, const SIMDRegister& aabbMax, const Vector3& t0, const Vector3& t1, const Vector3& t2);
bool  intersectAABBTriangle_Fast    (const AABB& aabb,const Vector3& t0, const Vector3& t1, const Vector3& t2);
bool  intersectLineSegmentTriangle  (const Vector3& lineStart, const Vector3& lineEnd, const Vector3& triangleVertex0, const Vector3& triangleVertex1, const Vector3& triangleVertex2);
bool  intersectLineSegmentTriangle  (float& distance, const Vector3&  lineStart, const Vector3& lineEnd, const Vector3& triangleVertex0, const Vector3& triangleVertex1, const Vector3& triangleVertex2);
bool  intersectAABBLineSegment      (const AABB& box, const Vector3& p1, const Vector3& p2);
bool  intersectAABBLineSegment      (const AABB& box, const Vector3& p1, const Vector3& p2, Vector3& intersectionPoint);
bool  intersectAABBLineSegment_Fast (const AABB& box, const Vector3& p1, const Vector3& p2);
UINT8 triangleOctants               (const AABB& aabb, const Vector3& a, const Vector3& b, const Vector3& c);

} // namespace Umbra
