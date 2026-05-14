#ifndef UMBRAARRAY_HPP
#define UMBRAARRAY_HPP

/*!
 *
 * Umbra3
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra Array
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraMemory.hpp"

namespace Umbra
{

#define SHRINK_THRESHOLD(current, use) ((use) < (current))
#define TARGET_CAPACITY(x) (max2(16, (x) * 2))

/*-------------------------------------------------------------------*//*!
 * \brief   A basic resizable array, in the spirit of std::vector.
 *
 * An array has one well defined notion of size, which is the number of
 * elements visible to the user and accessible through the [] operator.
 * This size can be 0 as well. The underlying storage allocated for the
 * array may be larger than the size (see getCapacity()), and the user has
 * only limited control over the size of the storage. In particular, no
 * requests for modifying the storage size need to be honored by the
 * implementation and therefore no assumptions should be made about it.
 *//*-------------------------------------------------------------------*/

template<class T> class Array : public Base
{
private:
    T*              m_data;                                         //!< m_data m_allocated (or 0 if zero-length array)
    int             m_allocated;                                    //!< number of elements alloced (capacity of array)
    int             m_size;                                         //!< size of array

public:
                    Array           (Allocator* a = NULL)               : Base(a), m_data(0), m_allocated(0), m_size(0) { /* nothing to do */ }
    explicit        Array           (int size, Allocator* a = NULL)     : Base(a), m_data(0), m_allocated(0), m_size(0) { reset(size); }
                    Array           (const Array<T>& src)               : Base(src.getAllocator()), m_data(0), m_allocated(0), m_size(0) { *this = src; }
                    Array           (const T* src, int size, Allocator* a = NULL) : Base(a), m_data(0), m_allocated(0), m_size(0) { set(src, size); }
                    ~Array          (void)                              { release(); }

    Array<T>& operator= (const Array<T> &src)
    {
        if (&src != this)
        {
            release();
            if (src.getSize())
            {
                reset(src.getSize());
                copyElements(m_data, src.getPtr(), src.getSize());
            }
        }
        return *this;
    }

    bool operator== (const Array<T> &src) const
    {
        if (m_size != src.m_size)
            return false;
        for (int i = 0; i < m_size; i++)
            if (m_data[i] != src.m_data[i])
                return false;
        return true;
    }

    bool operator!= (const Array<T>& src) const
    {
        return !operator==(src);
    }

    const T&        operator[]      (int i) const;
    T&              operator[]      (int i);

    T*              getPtr          (void) const                        { return m_data; }
    void            set             (const T* src, int size)            { reset(size); copyElements (m_data, src, size); }
    void            append          (const T* src, int size)
    {
        int idx = getSize();
        resize(getSize() + size);
        for (int i = 0; i < size; i++)
            m_data[idx + i] = src[i];
    }
    void            append          (const Array<T>& a)
    {
        append(a.getPtr(), a.getSize());
    }

    bool            contains        (const T& t)
    {
        for (int i = 0; i < getSize(); i++)
            if ((*this)[i] == t)
                return true;
        return false;
    }

    int             getSize         (void) const                        { return m_size; }
    int             getByteSize     (void) const                        { return m_size * sizeof(T); }
    int             getCapacity     (void) const                        { return m_allocated; }

    bool            pushBack        (const T& element);                 // std::vector compatibility
    T               popBack         (void);                             // std::vector compatibility

    void            reserve         (int size);                         // std::vector "reserve"
    bool            resize          (int newSize);                      // copies elements, sets size
    bool            reset           (int newSize);                      // doesn't copy elements, sets size
    void            clear           (void)                              { reset(0); }
    void            shrinkToFit     (bool preserveElements = true);

    void            removeMove      (int idx);                          // copies elements to fill empty slot
    void            removeSwap      (int idx);                          // swaps last element in place

private:
    bool            update          (int newSize, bool preserve, bool mayShrink = false, bool exact = false);
    void            release         (void)                              { UMBRA_DELETE_ARRAY(m_data); m_data = 0; m_allocated = 0; m_size = 0; }
    static void     copyElements    (T* dst, const T* src, int cnt)     { for (int i = 0; i < cnt; i++) dst[i] = src[i]; }

    inline friend unsigned int getHashValue (const Array<T>& a)
    {
        int hval = 0;
        for (int i = 0; i < a.getSize(); i++)
            hval += getHashValue(a[i]);
        return hval;
    }
};

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

template <class T, int SIZE>
class StaticArray
{
public:

    StaticArray() :
    m_used(0) {}

    int         getSize     (void) const    { return m_used; }
    const T&    operator[]  (int i) const   { UMBRA_ASSERT(i >= 0 && i < SIZE); return m_data[i]; }
    T&          operator[]  (int i)         { UMBRA_ASSERT(i >= 0 && i < SIZE); return m_data[i]; }
    void        pushBack    (const T& t)    { UMBRA_ASSERT(m_used < SIZE); m_data[m_used++] = t; }
    T           popBack     (void)          { UMBRA_ASSERT(m_used>0); return m_data[--m_used]; }
    void        clear       (void)          { m_used = 0; }
    inline void grow        (void)          { UMBRA_ASSERT(m_used < SIZE); m_used++; }
    inline void grow        (int used)      { UMBRA_ASSERT(used <= SIZE); m_used = used; }

private:

    T m_data[SIZE];
    int m_used;
};

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class T> inline  const T&  Array<T>::operator[]    (int i) const   { UMBRA_ASSERT(i >= 0 && i < m_size); return m_data[i]; }
template <class T> inline T&        Array<T>::operator[]    (int i)         { UMBRA_ASSERT(i >= 0 && i < m_size); return m_data[i]; }


template <class T> bool Array<T>::update (int size, bool preserve, bool mayShrink, bool exact)
{
    if (!size && mayShrink)
    {
        release();
        return true;
    }

    T* newArr = NULL;
    if ((size > m_allocated) || (mayShrink && SHRINK_THRESHOLD(m_allocated, size)))
    {
        int target = exact ? size : TARGET_CAPACITY(size);
        newArr = UMBRA_HEAP_NEW_ARRAY(getAllocator(), T, target);
#if defined(UMBRA_COMP_NO_EXCEPTIONS)
        if (!newArr)
            return false;
#endif
        m_allocated = target;
    }
    UMBRA_ASSERT(m_allocated >= size);

    if (newArr)
    {
        if (m_data)
        {
            if (preserve && m_size)
                copyElements(newArr, m_data, min2(m_size, size));
            UMBRA_HEAP_DELETE_ARRAY(getAllocator(), m_data);
        }
        m_data = newArr;
    }
    m_size = size;
    return true;
}

template <class T> void Array<T>::reserve (int size)
{
    // this currently does nothing!
    UMBRA_UNREF(size);
}

template <class T> bool Array<T>::resize (int newSize)
{
    return update(newSize, true, false, false);
}

template <class T> bool Array<T>::reset(int newSize)
{
    return update(newSize, false, false, true);
}

template <class T> inline bool Array<T>::pushBack(const T& element)
{
    int idx = m_size;
#if defined(UMBRA_COMP_NO_EXCEPTIONS)
    if (!update(m_size + 1, true))
        return false;
#else
    UMBRA_DEBUG_CODE(bool status =) update(m_size + 1, true);
    UMBRA_ASSERT(status);
#endif
    m_data[idx] = element;
    return true;
}

template <class T> inline T Array<T>::popBack(void)
{
    UMBRA_ASSERT(m_size);
    T elem = m_data[m_size - 1];
    update(m_size - 1, true);
    return elem;
}

template <class T> inline void Array<T>::shrinkToFit(bool preserveElements)
{
    update(m_size, preserveElements, true, true);
}

template <class T> inline void Array<T>::removeMove(int idx)
{
    UMBRA_ASSERT(idx >= 0 && idx < getSize());
    memmove(m_data + idx, m_data + idx + 1, (m_size - idx - 1) * sizeof(T));
    m_size--;
}

template <class T> inline void Array<T>::removeSwap(int idx)
{
    UMBRA_ASSERT(idx >= 0 && idx < getSize());
    m_data[idx] = m_data[m_size - 1];
    m_size--;
}

template <class T>
class FIFOQueue : public Base
{
public:

    FIFOQueue(Allocator* a, int size) :
    Base(a), m_head(0), m_tail(0), m_used(0), m_size(size)
    {
        m_queue = UMBRA_NEW_ARRAY(T, size);
    }
    ~FIFOQueue()
    {
        UMBRA_DELETE_ARRAY(m_queue);
    }

    bool        isOk        (void) const    { return m_queue != NULL; }
    int         getCapacity (void) const    { return m_size; }
    int         getSize     (void) const    { return m_used; }
    void        pushBack    (const T& t)    { UMBRA_ASSERT(hasSpace()); m_queue[m_tail++] = t; m_used++; if (m_tail == m_size) m_tail = 0;}
    T           popFront    (void)          { UMBRA_ASSERT(m_used > 0); T& ret = m_queue[m_head++]; m_used--; if (m_head == m_size) m_head = 0; return ret;}
    void        clear       (void)          { m_head = 0; m_tail = 0; m_used = 0; }
    bool        hasSpace    (void) const    { return m_used < m_size; }

private:
    T*      m_queue;
    int     m_head;
    int     m_tail;
    int     m_used;
    int     m_size;
};

} // namespace Umbra

#endif // UMBRAARRAY_HPP

//--------------------------------------------------------------------
