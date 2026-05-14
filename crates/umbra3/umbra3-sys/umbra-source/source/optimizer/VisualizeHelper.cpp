// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "VisualizeHelper.hpp"
#include "GraphicsContainer.hpp"

using namespace Umbra;

VisualizeHelper::VisualizeHelper(DebugCollector& sc)
:   mm(sc.getMemoryManager()),
    gc(sc.getGraphicsContainer()),
    activeFlag(sc.isActive()),
    rectSet0(mm),
    rectSet1(mm),
    rectSet2(mm),
    rectSet3(mm),
    rectSet4(mm),
    rectSet5(mm)
{
    rectSets[0] = &rectSet0;
    rectSets[1] = &rectSet1;
    rectSets[2] = &rectSet2;
    rectSets[3] = &rectSet3;
    rectSets[4] = &rectSet4;
    rectSets[5] = &rectSet5;
}

VisualizeHelper::~VisualizeHelper()
{
    flushVoxelFaces();

    for (int face = 0; face < 6; face++)
        for (UnorderedMap<Pair<float, int>, RectSet*>::Iterator iter = rectSets[face]->iterate(); iter; iter++)
            M_DELETE(mm, RectSet, (*iter).b);
}

void VisualizeHelper::point(const Vec3f& a, int id, const Color& color)
{
    if (!isActive())
        return;
    gc.point(a, id, color.v);
}

void VisualizeHelper::line(const Vec3f& a, const Vec3f& b, int id, const Color& color)
{
    if (!isActive())
        return;
    gc.line(a, b, id, color.v);
}

void VisualizeHelper::triangle(const Vec3f& a, const Vec3f& b, const Vec3f& c, int id, const Color& color)
{
    if (!isActive())
        return;
    gc.triangle(a, b, c, id, color.v);
}

void VisualizeHelper::aabbEdges(const AABoxf& aabb, int id, const Color& color)
{
    if (!isActive())
        return;

    for (int i = 0; i < 8; i++)
        for (int j = 0; j < 3; j++)
            if (!(i & (1 << j)))
            {
                Vec3f a = getBoxCorner(aabb, i);
                Vec3f b = getBoxCorner(aabb, i | (1 << j));
                gc.line(a, b, id, color.v);
            }
}

void VisualizeHelper::axialQuad(const AABoxf& aabb, int id, const Color& color)
{
    if (!isActive())
        return;

    int axis = -1;
    for (int i = 0; i < 3; i++)
        if (aabb.min()[i] == aabb.max()[i])
        {
            UMBRA_ASSERT(axis == -1);
            axis = i;
        }
    UMBRA_ASSERT(axis >= 0);

    axialQuad(axis<<1, getBoxFaceRect(aabb, axis<<1), aabb.min()[axis], id, color);
}

void VisualizeHelper::axialQuad(int face, const AARectf& rect, float z, int id, const Color& color)
{
    if (!isActive())
        return;

    int axis = face >> 1;
    int axisX = (axis+1)%3;
    int axisY = (axis+2)%3;

    Vec3f p[4];

    for (int i = 0; i < 4; i++)
    {
        p[i][axis] = z;
        p[i][axisX] = getRectCorner(rect, i).x();
        p[i][axisY] = getRectCorner(rect, i).y();
    }

    // TODO: flip according to face

    triangle(p[0], p[1], p[2], id, color);
    triangle(p[1], p[3], p[2], id, color);
#if 0
    line(p[0], p[1], id, color);
    line(p[1], p[3], id, color);
    line(p[3], p[2], id, color);
    line(p[2], p[0], id, color);
#endif
}


void VisualizeHelper::voxelFace(int face, float z, AARectf r, int color)
{
    UMBRA_ASSERT(face >= 0 && face <= 5);

    RectSet* const* rs2 = rectSets[face]->get(makePair(z, color));
    RectSet* rs;

    if (!rs2)
    {
        // TODO: if max RectSets is reached, flush one and reuse
        rs = M_NEW(mm, RectSet);
        rs->color = color;
        rs->face = face;
        rs->z = z;
        rs->numRects = 0;
        rectSets[face]->insert(makePair(z, color), rs);
    }
    else
        rs = *rs2;

    if (rs->numRects >= MAX_RECTS)
        flushRectSet(rs);

    UMBRA_ASSERT(rs->numRects < MAX_RECTS);
    rs->rects[rs->numRects++] = r;

    while (rs->numRects >= 2)
    {
        if (canCombine(rs->rects[rs->numRects-2], rs->rects[rs->numRects-1]))
        {
            rs->rects[rs->numRects-2] = rs->rects[rs->numRects-2].merge(rs->rects[rs->numRects-1]);
            rs->numRects--;
            continue;
        }
        if (canCombine(rs->rects[0], rs->rects[rs->numRects-1]))
        {
            rs->rects[0] = rs->rects[0].merge(rs->rects[rs->numRects-1]);
            rs->numRects--;
            continue;
        }
        break;
    }
}

bool VisualizeHelper::canCombine(const AARectf& a, const AARectf& b)
{
    if (!a.intersects(b))
        return false;

    // TODO: this isn't actually only case (b may be fully inside a)
    return (a.min().x() == b.min().x() && a.max().x() == b.max().x()) ||
           (a.min().y() == b.min().y() && a.max().y() == b.max().y());
}

void VisualizeHelper::flushRectSet(RectSet* rs)
{
    // TODO: Before flushing, do more expensive grouping by scanning and
    // subdividing rect set. Also do it before attemping to flush a full
    // buffer.

    Vec4f c = VisualizeHelper::Color::getColor(rs->color);
    float intensity = 0.6f + rs->face / 5.f * 0.4f;
    c.x() *= intensity;
    c.y() *= intensity;
    c.z() *= intensity;

    for (int i = 0; i < rs->numRects; i++)
        axialQuad(rs->face, rs->rects[i], rs->z, 0, c);

    rs->numRects = 0;
}

void VisualizeHelper::flushVoxelFaces()
{
    for (int face = 0; face < 6; face++)
        for (UnorderedMap<Pair<float, int>, RectSet*>::Iterator iter = rectSets[face]->iterate(); iter; iter++)
            flushRectSet((*iter).b);
}
