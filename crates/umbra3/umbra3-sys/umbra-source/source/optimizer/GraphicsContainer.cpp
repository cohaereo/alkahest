// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include <optimizer/GraphicsContainer.hpp>

using namespace Umbra;

void GraphicsContainer::appendRemove(GraphicsContainer& other)
{
    UMBRA_ASSERT(&mm == &other.mm);

    GraphicsChunk* ch = other.first;
    other.first = other.last = 0;

    if (!first)
    {
        first = ch;
        last = first;
    }
    else
        last->next = ch;

    while (last->next)
        last = last->next;
}

void GraphicsContainer::appendCopy(const GraphicsContainer& other)
{
    const GraphicsChunk* ch = other.first;

    while (ch)
    {
        GraphicsChunk* ch2 = M_NEW(mm, GraphicsChunk);
        memcpy(ch2, ch, sizeof(GraphicsChunk));
        ch2->next = 0;

        if (!first)
            first = ch2;
        else
            last->next = ch2;
        last = ch2;

        ch = ch->next;
    }
}

void GraphicsContainer::clear()
{
    GraphicsChunk* ch = first;
    while (ch)
    {
        GraphicsChunk* next = ch->next;
        M_DELETE(mm, GraphicsChunk, ch);
        ch = next;
    }
    first = 0;
}

GraphicsContainer::Primitive& GraphicsContainer::newPrimitive()
{
    UMBRA_ASSERT((first == 0) == (last == 0));

    if (!last)
    {
        first = last = M_NEW(mm, GraphicsChunk);
    }

    if (last->num >= MAX_PRIMITIVES_IN_CHUNK)
    {
        GraphicsChunk* ch = M_NEW(mm, GraphicsChunk);
        last->next = ch;
        last = ch;
    }

    UMBRA_ASSERT(last->num < MAX_PRIMITIVES_IN_CHUNK);
    return last->primitives[last->num++];
}

void GraphicsContainer::point(const Vec3f& a, int id, const Vec4f& color)
{
    Primitive& prim = newPrimitive();
    prim.type = POINT;
    prim.id = id;
    prim.v[0] = a;
    prim.v[1] = Vec3f();
    prim.v[2] = Vec3f();
    prim.color = color;
}

void GraphicsContainer::line(const Vec3f& a, const Vec3f& b, int id, const Vec4f& color)
{
    Primitive& prim = newPrimitive();
    prim.type = LINE;
    prim.id = id;
    prim.v[0] = a;
    prim.v[1] = b;
    prim.v[2] = Vec3f();
    prim.color = color;
}

void GraphicsContainer::triangle(const Vec3f& a, const Vec3f& b, const Vec3f& c, int id, const Vec4f& color)
{
    Primitive& prim = newPrimitive();
    prim.type = TRIANGLE;
    prim.id = id;
    prim.v[0] = a;
    prim.v[1] = b;
    prim.v[2] = c;
    prim.color = color;
}
