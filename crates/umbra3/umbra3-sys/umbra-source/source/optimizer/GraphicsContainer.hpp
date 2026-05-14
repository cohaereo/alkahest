// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Memory.hpp>
#include <standard/Vector.hpp>

namespace Umbra
{
class AABB;

class GraphicsContainer
{
    struct GraphicsChunk;
public:
    GraphicsContainer(MemoryManager& mm) : mm(mm), first(0), last(0) { for (int i = 0; i < MAX_NAME_LEN; i++) currentName[i] = 0; }
    ~GraphicsContainer() { clear(); }

    // TODO: primitive naming (maybe per chunk)

    void appendRemove(GraphicsContainer& other);
    void appendCopy(const GraphicsContainer& other);

    void setName(const char* name);

    void point   (const Vec3f& a, int id, const Vec4f& color);
    void line    (const Vec3f& a, const Vec3f& b, int id, const Vec4f& color);
    void triangle(const Vec3f& a, const Vec3f& b, const Vec3f& c, int id, const Vec4f& color);
    void clear   ();

    bool isEmpty() const { return first == 0; }

    enum PrimitiveType
    {
        POINT,
        LINE,
        TRIANGLE
    };

    struct Primitive
    {
        int   type;
        int   id;
        Vec3f v[3];
        Vec4f color;

        template<typename OP> void streamOp (OP& op)
        {
            stream(op, type);
            for (int i = 0; i < 3; i++)
                for (int j = 0; j < 3; j++)
                    stream(op, v[i][j]);
            stream(op, id);
            stream(op, color.x());
            stream(op, color.y());
            stream(op, color.z());
            stream(op, color.w());
        }
    };

    template <typename OP> void streamOp (OP& op)
    {
        GraphicsChunk** ch = &first;

        for (;;)
        {
            bool hasNext = (*ch != 0);
            stream(op, hasNext);
            if (hasNext)
            {
                if (*ch == 0)
                    *ch = M_NEW(mm, GraphicsChunk);

                stream(op, **ch);

                ch = &(*ch)->next;
            }
            else
                break;
        }

        last = first;
        if (last)
            while (last->next)
                last = last->next;
    }

    struct Iterator
    {
        typedef Primitive ElementType;

        Iterator() : ch(0), idx(0) {}
        Iterator(GraphicsChunk* ch, int idx) : ch(ch), idx(idx) {}

        Primitive& operator*() { UMBRA_ASSERT(ch && idx >= 0 && idx < ch->num); return ch->primitives[idx]; }
        const Primitive& operator*() const { UMBRA_ASSERT(ch && idx >= 0 && idx < ch->num); return ch->primitives[idx]; }
        void operator++() { idx++; if (idx >= ch->num) { ch = ch->next; idx = 0; }}
        void operator++(int) { ++(*this); }
        operator bool() const { return ch && (idx < ch->num || ch->next); } // TODO: next ch may not contain zero primitives

    private:
        GraphicsChunk* ch;
        int            idx;
    };

    Iterator iterate() const
    {
        Iterator iter(first, 0);
        return iter;
    }

private:
    enum
    {
        MAX_PRIMITIVES_IN_CHUNK = 1024,
        MAX_NAME_LEN            = 32
    };

    struct GraphicsChunk
    {
        GraphicsChunk() : num(0), next(0) { for (int i = 0; i < MAX_NAME_LEN; i++) name[i] = 0; }

        char           name[MAX_NAME_LEN];
        int            num;
        Primitive      primitives[MAX_PRIMITIVES_IN_CHUNK];
        GraphicsChunk* next;

        template<typename OP> void streamOp (OP& op)
        {
            for (int i = 0; i < 32; i++)
                stream(op, name[i]);
            stream(op, num);
            for (int i = 0; i < num; i++)
                stream(op, primitives[i]);
        }
    };

    Primitive& newPrimitive();

    MemoryManager& mm;
    char           currentName[MAX_NAME_LEN];
    GraphicsChunk* first;
    GraphicsChunk* last;

    GraphicsContainer& operator= (const GraphicsContainer&);
};

}
