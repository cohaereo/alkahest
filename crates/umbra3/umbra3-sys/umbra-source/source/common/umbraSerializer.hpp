#ifndef UMBRASERIALIZER_HPP
#define UMBRASERIALIZER_HPP

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
 * \brief   Umbra Serializer
 *
 */

#include "umbraBinStream.hpp"
#include "umbraArray.hpp"
#include "umbraAABB.hpp"
#include "umbraString.hpp"
#include "umbraBitMath.hpp"
#include "umbraMatrix.hpp"
#include "umbraSet.hpp"

#ifdef require
#   undef require
#endif

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

class Serializer
{
public:
    Serializer (OutputStream* out): m_out(out) {}

    enum { IsWrite = 0 };
    enum { IsRead = 1 };

    bool minVersion (int i) { UMBRA_UNREF(i); return true; }

    void require (bool value) { UMBRA_UNREF(value); }

    bool isOk (void) const { return m_out.isOk(); }

    template<typename T> void prepare (T&) {}

    template<typename T> void primitive (const T& i)
    {
        m_out.put(i);
    }

    template<typename T> void primitiveArray (T* arr, int count)
    {
        for (int i = 0; i < count; i++)
            primitive(arr[i]);
    }

    void skip (UINT32 size)
    {
        // nada
        UMBRA_UNREF(size);
    }

    void setOpaque (void* ptr) { m_opaque = ptr; }
    void* getOpaque (void) const { return m_opaque; }

private:
    StreamWriter m_out;
    void* m_opaque;
};


/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

class Deserializer
{
public:
    Deserializer (InputStream* in, Allocator* a = NULL):
      m_in(in), m_alloc(a), m_version(-1), m_opaque(NULL), m_failed(false) {}

    enum { IsWrite = 1 };
    enum { IsRead = 0 };

    template<typename T> void prepare (T&) { /* nada */ }

    bool minVersion (int i)
    {
        UMBRA_ASSERT(m_version != -1);
        return m_version >= i;
    }

    void require (bool value)
    {
        if (!value)
            m_failed = true;
    }

    void setVersion (int version)
    {
        m_version = version;
    }

    void skip (UINT32 size)
    {
        if (m_failed)
            return;
        m_in.skip(size);
    }

    void prepare (Base& t)
    {
        t.setAllocator(m_alloc);
    }

    template<typename T> void primitive (T& i)
    {
        if (m_failed)
            return;
        m_in.get(i);
    }

    template<typename T> void primitiveArray (T* arr, int count)
    {
        if (m_failed)
            return;
        m_in.rawRead(arr, count*sizeof(T));
        if (StreamUtil::needSwap())
        {
            for (int i = 0; i < count; i++)
                StreamUtil::swap<T>((UINT8*)&arr[i]);
        }
    }

    bool isOk (void) const { return m_in.isOk() && !m_failed; }

    void setOpaque (void* ptr) { m_opaque = ptr; }
    void* getOpaque (void) const { return m_opaque; }

private:
    StreamReader m_in;
    Allocator* m_alloc;
    int m_version;
    void* m_opaque;
    bool m_failed;
};


// default implementation for non-primitives

template<typename OP, typename T> static UMBRA_INLINE void stream (OP& op, T& t)
{
    op.prepare(t);
    t.streamOp(op);
}

// pass primitive types to op

template<typename OP> static UMBRA_INLINE void stream (OP& op, bool& t) { op.primitive(t); }
template<typename OP> static UMBRA_INLINE void stream (OP& op, char& t) { op.primitive(t); }
template<typename OP> static UMBRA_INLINE void stream (OP& op, INT32& t) { op.primitive(t); }
template<typename OP> static UMBRA_INLINE void stream (OP& op, UINT32& t) { op.primitive(t); }
template<typename OP> static UMBRA_INLINE void stream (OP& op, const UINT32& t) { op.primitive(t); }
template<typename OP> static UMBRA_INLINE void stream (OP& op, float& t) { op.primitive(t); }
template<typename OP> static UMBRA_INLINE void stream (OP& op, double& t) { op.primitive(t); }
template<typename OP> static UMBRA_INLINE void streamArray (OP& op, INT32* buf, int size) { op.primitiveArray(buf, size); }
template<typename OP> static UMBRA_INLINE void streamArray (OP& op, UINT32* buf, int size) { op.primitiveArray(buf, size); }
template<typename OP> static UMBRA_INLINE void streamArray (OP& op, float* buf, int size) { op.primitiveArray(buf, size); }

// some stream operation implementations for common classes

template<typename OP, typename T> static UMBRA_INLINE void streamArray (OP& op, T* buf, int size)
{
    for (int i = 0; i < size; i++)
        stream(op, buf[i]);
}

template<typename OP> static UMBRA_INLINE void streamArray (OP& op, Vector3* buf, int size)
{
    streamArray(op, (float*)buf, size * 3);
}

template<typename OP> static UMBRA_INLINE void streamArray (OP& op, Vector3i* buf, int size)
{
    streamArray(op, (INT32*)buf, size * 3);
}

template<typename OP, typename T> static UMBRA_INLINE void stream (OP& op, Array<T>& t, int size = -1)
{
    op.prepare(t);
    if (size == -1)
    {
        size = t.getSize();
        stream(op, size);
    }
    if (OP::IsWrite)
        t.reset(size);
    streamArray(op, t.getPtr(), size);
}

template<typename OP, typename T> static UMBRA_INLINE void stream (OP& op, Set<T>& t, int size = -1)
{
    op.prepare(t);
    if (size == -1)
    {
        size = t.getSize();
        stream(op, size);
    }
    if (OP::IsWrite)
    {
        t.removeAll();
        for (int i = 0; i < size; i++)
        {
            T value(0);
            stream(op, value);
            t.insert(value);
        }
    }
    else
    {
        typename Set<T>::Iterator iter = t.iterate();
        while (iter.next())
        {
            T value = iter.getValue();
            stream(op, value);
        }
    }
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, Vector2& v)
{
    stream(op, v.x);
    stream(op, v.y);
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, Vector3& v)
{
    stream(op, v.x);
    stream(op, v.y);
    stream(op, v.z);
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, Vector3i& v)
{
    stream(op, v.i);
    stream(op, v.j);
    stream(op, v.k);
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, Vector4& v)
{
    stream(op, v.x);
    stream(op, v.y);
    stream(op, v.z);
    stream(op, v.w);
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, Matrix4x3& v)
{
    stream(op, v[0]);
    stream(op, v[1]);
    stream(op, v[2]);
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, Matrix4x4& v)
{
    stream(op, v[0]);
    stream(op, v[1]);
    stream(op, v[2]);
    stream(op, v[3]);
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, AABB& aabb)
{
    Vector3 mn, mx;

    if (OP::IsRead)
    {
        mn = aabb.getMin();
        mx = aabb.getMax();
    }
    stream(op, mn);
    stream(op, mx);
    if (OP::IsWrite)
    {
        aabb.setMin(mn);
        aabb.setMax(mx);
    }
}

template<typename OP> static UMBRA_INLINE void stream (OP& op, AABBi& aabb)
{
    Vector3i mn, mx;

    if (OP::IsRead)
    {
        mn = aabb.getMin();
        mx = aabb.getMax();
    }
    stream(op, mn);
    stream(op, mx);
    if (OP::IsWrite)
    {
        aabb.setMin(mn);
        aabb.setMax(mx);
    }
}

template<typename OP> static void stream (OP& op, BitVector& bv)
{
    op.prepare(bv);
    int numBlocks = (int)bv.numBlocks();
    stream(op, numBlocks);
    if (OP::IsWrite)
        bv.reset(numBlocks << 5);
    streamArray(op, bv.getArray(), numBlocks);
}

template<typename OP> static void stream (OP& op, String& s)
{
    // \todo [Hannu] make this faster
    Array<char> ary(s.getAllocator());
    for (int i = 0; i < s.length(); i++)
        ary.pushBack(s[i]);
    ary.pushBack(0);
    stream(op, ary);
    s = String(ary.getPtr(), s.getAllocator());
}


} // namespace Umbra

#endif // UMBRASERIALIZER_HPP

//--------------------------------------------------------------------
