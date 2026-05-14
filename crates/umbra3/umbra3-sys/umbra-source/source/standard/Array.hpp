// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Containers.hpp>

namespace Umbra
{

template <typename T, typename Storage = SmallContainerStorage<4> >
class ResizableArray
{
public:
    typedef T ContainedType;
    typedef typename Storage::template ForType<T> ElementMemory;
    typedef DirectPointerIterator<T> Iterator;
    typedef DirectPointerReverseIterator<T> ReverseIterator;

    ResizableArray(MemoryManager& mgr, int size = 0): m_memory(mgr), m_size(0)
    {
        resize(size);
    }

    ResizableArray(const ResizableArray& other): m_memory(other.memoryManager()), m_size(0)
    {
        append(other);
    }

    ~ResizableArray()
    {
        clear();
    }

    MemoryManager& memoryManager() const
    {
        return m_memory.memoryManager();
    }

    T* ptr() const
    {
        return m_memory.block();
    }

    T& at(int index) const
    {
        UMBRA_ASSERT(index >= 0 && index < m_size);
        return ptr()[index];
    }

    T& operator[] (int index)
    {
        return at(index);
    }

    const T& operator[] (int index) const
    {
        return at(index);
    }

    int size() const
    {
        return m_size;
    }

    int capacity() const
    {
        return m_memory.capacity();
    }

    T* first() const
    {
        return ptr();
    }

    T* end() const
    {
        return ptr() + m_size;
    }

    void reserve(int size)
    {
        if (size > m_memory.capacity())
            m_memory.resize(max2(size, nextCapacity()));
    }

    void resize(int size)
    {
        reserve(size);
        m_size = size;
    }

    void shrinkToFit()
    {
        m_memory.resize(m_size);
    }

    void clear()
    {
        m_size = 0;
        shrinkToFit();
    }

    T& add()
    {
        int idx = m_size;
        resize(m_size + 1);
        return at(idx);
    }

    void add(const T& item)
    {
        add() = item;
    }

    T& insert(int index)
    {
        UMBRA_ASSERT(index >= 0 && index < m_size);
        int elemsAfter = m_size - index;
        resize(m_size + 1);
        Mem::move(&at(index + 1), &at(index), elemsAfter*sizeof(T));
        return at(index);
    }

    void insert(int index, const T& item)
    {
        insert(index) = item;
    }

    void removeLast()
    {
        resize(m_size - 1);
    }

    void remove(int index)
    {
        UMBRA_ASSERT(index >= 0 && index < m_size);
        int elemsAfter = m_size - index - 1;
        if (elemsAfter)
            Mem::move(&at(index), &at(index+1), elemsAfter*sizeof(T));
        removeLast();
    }
    void removeSwap(int index)
    {
        UMBRA_ASSERT(index >= 0 && index < m_size);
        // assigning to self is possible here
        at(index) = at(m_size - 1);
        removeLast();
    }

    template<typename OtherContainer>
    void append(const OtherContainer& other)
    {
        int pos = m_size;
        resize(m_size + other.size());
        if (other.size())
            Mem::copy(&at(pos), other.first(), other.size()*sizeof(T));
    }

    Iterator iterate() const
    {
        return Iterator(first(), end());
    }

    ReverseIterator iterateReverse() const
    {
        return ReverseIterator(first(), end());
    }

    int find (const T& item)
    {
        for (int i = 0; i < m_size; i++)
        {
            if (at(i) == item)
                return i;
        }
        return -1;
    }

private:

    int nextCapacity() const
    {
        return m_memory.capacity()*2 + 8;
    }

    ElementMemory m_memory;
    int m_size;
};

} // namespace Umbra