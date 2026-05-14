// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRACONVEXHULL_HPP
#define UMBRACONVEXHULL_HPP

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "umbraArray.hpp"

namespace Umbra
{

#define EDGE_EPSILON 0.001f

class ConvexHull2D
{
public:

    ConvexHull2D (Allocator* a): polygon(a)
    {
    }

    const Array<Vector2>& getHull(void) const { return polygon; }

    void addPoint (const Vector2& pt)
    {
        // detect duplicates
        for (int i = 0; i < polygon.getSize(); i++)
        {
            if (samePoints(polygon[i], pt))
                return;
        }

        // initialization
        if (polygon.getSize() < 2)
        {
            polygon.pushBack(pt);
            return;
        }
        // special case for colinear initial triangle
        if ((polygon.getSize() == 2) &&
            (edgeTest(polygon[0], pt, polygon[1]) == ON_EDGE ||
             edgeTest(polygon[1], pt, polygon[0]) == ON_EDGE ||
             edgeTest(polygon[0], polygon[1], pt) == ON_EDGE))
        {
            Vector2 edge = polygon[1] - polygon[0];
            Vector2 a = pt - polygon[0];
            if (dot(a, edge) < 0.f)
                polygon[0] = pt;
            else if (a.lengthSqr() > edge.lengthSqr())
                polygon[1] = pt;
            return;
        }

        // find first inside edge
        int first = -1;
        for (int i = 0; i < polygon.getSize(); i++)
        {
            int j = nextEdge(i);
            int k = nextEdge(j);
            if ((edgeTest(polygon[i], pt, polygon[j]) == OUTSIDE) &&
                (edgeTest(polygon[j], pt, polygon[k]) != OUTSIDE))
            {
                first = j;
                break;
            }
        }

        // completely inside
        if (first == -1)
            return;
        // handle on edge case
        if (edgeTest(polygon[first], pt, polygon[nextEdge(first)]) == ON_EDGE)
        {
            if ((pt - polygon[first]).lengthSqr() <= edgeLenSqr(first))
                return;
        }

        // find last edge
        int last;
        for (last = nextEdge(first); last != first; last = nextEdge(last))
        {
            if (edgeTest(pt, polygon[nextEdge(last)], polygon[last]) == OUTSIDE)
                break;
        }

        int len;
        if (first == last)
        {
            polygon[0] = polygon[first];
            len = 1;
        }
        else
        {
            // see if point is worth adding
            if (edgeTest(polygon[first], polygon[last], pt) != OUTSIDE)
                return;
            len = moveRange(last, first);
        }
        polygon.resize(len);
        polygon.pushBack(pt);
        UMBRA_ASSERT(testHullness());
    }

    bool testHullness (void) const
    {
        if (polygon.getSize() < 3)
            return true;

        for (int i = 0; i < polygon.getSize(); i++)
        {
            int j = nextEdge(i);
            int k = nextEdge(j);
            if (edgeTest(polygon[i], polygon[k], polygon[j]) != OUTSIDE)
                return false;
        }
        return true;
    }

private:

    enum EdgeSide
    {
        INSIDE,
        OUTSIDE,
        ON_EDGE
    };

    EdgeSide edgeTest (const Vector2& a, const Vector2& b, const Vector2& pt) const
    {
        Vector2 edge = b - a;
        Vector2 norm = Vector2(-edge.y, edge.x);
        float d = dot(pt - a, norm);
        float dd = d * d;
        // (|d| / |norm|) < EDGE_EPSILON
        if (dd < norm.lengthSqr() * (EDGE_EPSILON * EDGE_EPSILON))
            return ON_EDGE;
        return (d > 0.f) ? INSIDE : OUTSIDE;
    }

    float edgeLenSqr (int i) const
    {
        int start = i;
        int end = nextEdge(i);
        return (polygon[end] - polygon[start]).lengthSqr();
    }

    bool samePoints (const Vector2& a, const Vector2& b)
    {
        return ((a - b).lengthSqr() < (EDGE_EPSILON * EDGE_EPSILON));
    }

    int nextEdge (int i) const
    {
        return (i + 1) % polygon.getSize();
    }

    int rangeLen (int start, int end)
    {
        if (end < start)
            return (end + 1) + (polygon.getSize() - start);
        return end - start + 1;
    }

    int moveRange (int start, int end)
    {
        UMBRA_ASSERT(start != end);
        int l = rangeLen(start, end);

        while (end < start)
        {
            // rotate left
            Vector2 endVal = polygon[end];
            for (int i = 0; i < (l - 1); i++)
                polygon[(end + i) % polygon.getSize()] = polygon[(start + i) % polygon.getSize()];
            start = end;
            end = (start + l - 1) % polygon.getSize();
            polygon[end] = endVal;
        }
        if (start != 0)
            for (int i = 0; i < l; i++)
                polygon[i] = polygon[start + i];

        return l;
    }

    Array<Vector2> polygon;
};

}

#endif