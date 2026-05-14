// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Memory.hpp>
#include <standard/Vector.hpp>
#include <standard/Hash.hpp>
#include <standard/AxisExtents.hpp>
#include <optimizer/DebugCollector.hpp>

namespace Umbra
{
class VisualizeHelper
{
public:
    struct Color
    {
        Color(Vec3f c) : v(Vec4f(c.x(), c.y(), c.z(), 1.f)) {}
        Color(const Vec4f& c) : v(c) {}
        Color(int n) : v(getColor(n)) {}

        static Vec4f getColor(int n)
        {
            return Vec4f((((n+1)%2) + .5f) / 2.f, (((n+1)%3) + .5f) / 3.f, (((n+1)%5) + .5f) / 5.f, 1.f);
        }

        Vec4f v;
    };

    VisualizeHelper(DebugCollector& sc);
    ~VisualizeHelper();

    bool isActive() const { return activeFlag; }

    void point    (const Vec3f& a, int id = 0, const Color& color = 0);
    void line     (const Vec3f& a, const Vec3f& b, int id = 0, const Color& color = 0);
    void triangle (const Vec3f& a, const Vec3f& b, const Vec3f& c, int id = 0, const Color& color = 0);
    void aabbEdges(const AABoxf& aabb, int id = 0, const Color& color = 0);
    void axialQuad(const AABoxf& aabb, int id = 0, const Color& color = 0);
    void axialQuad(int face, const AARectf& rect, float z, int id = 0, const Color& color = 0);

    void voxelFace(int face, float z, AARectf r, int color);
    void flushVoxelFaces();

private:
    enum
    {
        MAX_RECTS = 256
    };

    struct RectSet
    {
        RectSet() : color(0), face(-1), z(0.f), numRects(0) {}

        int     color;
        int     face;
        float   z;
        int     numRects;
        AARectf rects[MAX_RECTS];
    };

    void flushRectSet(RectSet* rs);
    bool tryOptimize(RectSet* rs);
    bool canCombine(const AARectf& a, const AARectf& b);

    MemoryManager&                            mm;
    GraphicsContainer&                        gc;
    bool                                      activeFlag;
    UnorderedMap<Pair<float, int>, RectSet*>* rectSets[6];
    UnorderedMap<Pair<float, int>, RectSet*>  rectSet0, rectSet1, rectSet2, rectSet3, rectSet4, rectSet5;

    VisualizeHelper& operator= (const VisualizeHelper&);
};

}
