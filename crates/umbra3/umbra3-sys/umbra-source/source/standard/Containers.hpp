// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Memory.hpp>
#include <standard/Util.hpp>

namespace Umbra
{

/**!
 * ContainerStorage is the backend memory of all container types.
 */

struct BasicContainerStorage
{
    template<typename T>
    struct ForType
    {
    public:
        ForType(MemoryManager& mgr): m_memoryMgr(mgr), m_block(NULL), m_capacity(0)
        {
        }

        ~ForType()
        {
            m_memoryMgr.dealloc(m_block);
        }

        MemoryManager& memoryManager() const
        {
            return m_memoryMgr;
        }

        T* block() const
        {
            return m_block;
        }

        int capacity() const
        {
            return m_capacity;
        }

        void resize (int count)
        {
            if (count == m_capacity)
                return;
            T* newBlock = NULL;
            if (count)
                newBlock = (T*)m_memoryMgr.alloc(count * sizeof(T), UMBRA_ALIGNOF(T));
            if (m_block && newBlock)
                Mem::copy(newBlock, m_block, min2(count, m_capacity) * sizeof(T));
            m_memoryMgr.dealloc(m_block);
            m_block = newBlock;
            m_capacity = count;
        }

    private:
        ForType(const ForType&);
        ForType& operator=(const ForType&);

        MemoryManager& m_memoryMgr;
        T* m_block;
        int m_capacity;
    };
};

template<int N, typename FallbackContainerStorage = BasicContainerStorage>
struct SmallContainerStorage
{
    template<typename T>
    struct ForType
    {
    public:
        ForType(MemoryManager& mgr): m_fallback(mgr)
        {
        }

        MemoryManager& memoryManager() const
        {
            return m_fallback.memoryManager();
        }

        T* block() const
        {
            if (m_fallback.block())
                return m_fallback.block();
            return (T*)m_block;
        }

        int capacity() const
        {
            return max2(N, m_fallback.capacity());
        }

        void resize (int count)
        {
            if (count <= N)
            {
                // use inline memory, free fallback memory if it exists
                if (m_fallback.capacity())
                {
                    UMBRA_ASSERT(m_fallback.block());
                    Mem::copy(m_block, m_fallback.block(), count*sizeof(T));
                    m_fallback.resize(0);
                }
            }
            else
            {
                // use fallback memory, copy elements from inline if it didn't exist
                bool needsCopy = (m_fallback.capacity() == 0);
                m_fallback.resize(count);
                if (needsCopy)
                    Mem::copy(m_fallback.block(), m_block, N*sizeof(T));
            }
        }
    private:
        typedef typename FallbackContainerStorage::template ForType<T> FallbackMemory;

        ForType(const ForType&);
        ForType& operator=(const ForType&);

        AlignedElementMem<T> m_block[N];
        FallbackMemory m_fallback;
    };
};

// Iterator usage:
//
// for (Container<T>::(TypeOfIterator)Iterator i = container.iterate(); i, ++i)
// {
//     T current = *i;
// }

template <typename T>
class DirectPointerIterator
{
public:
    typedef T ElementType;

    DirectPointerIterator(): m_cur(NULL), m_end(NULL) {}
    DirectPointerIterator(T* first, T* end): m_cur(first), m_end(end) {}

    T& operator*() { return *m_cur; }
    const T& operator*() const { return *m_cur; }
    void operator++() { ++m_cur; }
    void operator++(int) { ++m_cur; }
    operator bool() const { return m_cur < m_end; }

private:
    T* m_cur;
    T* m_end;
};

template <typename T>
class DirectPointerReverseIterator
{
public:
    typedef T ElementType;

    DirectPointerReverseIterator(): m_cur(NULL), m_end(NULL) {}
    DirectPointerReverseIterator(T* first, T* end): m_cur(end - 1), m_end(first - 1) {}

    T& operator*() { return *m_cur; }
    const T& operator*() const { return *m_cur; }
    void operator++() { --m_cur; }
    void operator++(int) { --m_cur; }
    operator bool() const { return m_cur > m_end; }

private:
    T* m_cur;
    T* m_end;
};

} // namespace Umbra
