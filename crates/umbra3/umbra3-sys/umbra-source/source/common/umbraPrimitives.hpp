// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRAPRIMITIVES_HPP
#define UMBRAPRIMITIVES_HPP

#include "umbraVector.hpp"
#include "umbraAABB.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct LineSegment
{
    LineSegment(void) {}

    LineSegment(const Vector3& start, const Vector3& end): a(start), b(end) {}

    Vector3     a;
    Vector3     b;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct Ray
{
    Ray(void) {}

    Ray(const Vector3& origin, const Vector3& dir): origin(origin), dir(dir) {}

    Vector3     origin;
    Vector3     dir;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct Triangle
{
    Triangle(void) {}
    Triangle(const Vector3& a, const Vector3& b, const Vector3& c): a(a), b(b), c(c) {}

    Vector3& operator[] (int i) { UMBRA_ASSERT (i >= 0 && i < 3); return (&a)[i]; }
    const Vector3& operator[] (int i) const { UMBRA_ASSERT (i >= 0 && i < 3); return (&a)[i]; }

    LineSegment getEdge (int edge) const
    {
        switch (edge)
        {
        case 0: return LineSegment(a, b);
        case 1: return LineSegment(b, c);
        case 2: return LineSegment(c, a);
        default: UMBRA_ASSERT(!"invalid triangle edge"); return LineSegment();
        }
    }

    Vector3     a;
    Vector3     b;
    Vector3     c;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct Quad
{
    Vector3     a;
    Vector3     b;
    Vector3     c;
    Vector3     d;

    Vector3& operator[] (int i) { UMBRA_ASSERT (i >= 0 && i < 4); return (&a)[i]; }
    const Vector3& operator[] (int i) const { UMBRA_ASSERT (i >= 0 && i < 4); return (&a)[i]; }

    LineSegment getEdge (int edge) const
    {
        switch (edge)
        {
        case 0: return LineSegment(a, b);
        case 1: return LineSegment(b, c);
        case 2: return LineSegment(c, d);
        case 3: return LineSegment(d, a);
        default: UMBRA_ASSERT(!"invalid quad edge"); return LineSegment();
        }
    }
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct Sphere
{
    Vector3     center;
    float       radius;
};

}


#endif // UMBRAPRIMITIVES_HPP
