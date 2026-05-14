#ifndef UMBRASTATICHEAP_HPP
#define UMBRASTATICHEAP_HPP

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
 * \brief   Umbra heap
 *
 */

#if !defined (UMBRAARRAY_HPP)
#   include "umbraArray.hpp"
#endif

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief           Simple heap data structure. The first key is always
 *                  the lowest.
 *//*-------------------------------------------------------------------*/

template <class K, class V, int SIZE>
class StaticHeap
{
public:
                                StaticHeap          (void);
                                StaticHeap          (const StaticHeap<K, V, SIZE>& h);
                                StaticHeap          (const StaticArray<K, SIZE>& keys, const StaticArray<V, SIZE>& values);
                                ~StaticHeap         (void);

    StaticHeap&                 operator=           (const StaticHeap<K, V, SIZE>& h);

    int                         getSize             (void) const;
    const K&                    getKey              (int i) const;
    const V&                    getValue            (int i) const;
    void                        insert              (const K& key, const V& value);
    void                        removeFirst         (void);
    void                        clear               (void);
    bool                        decreaseKey         (const K& key, const V& value);
    StaticArray<V, SIZE>*       getValueArr         (void) { return &m_values; }

private:
    void                        heapify             (int idx);

    StaticArray<K, SIZE>        m_keys;
    StaticArray<V, SIZE>        m_values;
};

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class K, class V, int SIZE> StaticHeap<K, V, SIZE>::StaticHeap(void)
{
    /* empty */
}

template <class K, class V, int SIZE> StaticHeap<K, V, SIZE>::StaticHeap(const StaticHeap<K, V, SIZE>& h)
:   m_keys      (h.m_keys),
    m_values    (h.m_values)
{
    /* empty */
}

template <class K, class V, int SIZE> StaticHeap<K, V, SIZE>::StaticHeap(const StaticArray<K, SIZE>& keys, const StaticArray<V, SIZE>& values)
:   m_keys      (keys),
    m_values    (values)
{
    UMBRA_ASSERT(m_keys.getSize() == m_values.getSize());
    for (int i = (m_keys.getSize() >> 1) - 1; i >= 0; i--)
        heapify(i);
}

template <class K, class V, int SIZE> StaticHeap<K, V, SIZE>::~StaticHeap(void)
{
    /* empty */
}

template <class K, class V, int SIZE> StaticHeap<K, V, SIZE>& StaticHeap<K, V, SIZE>::operator=(const StaticHeap<K, V, SIZE>& h)
{
    if (&h != this)
    {
        m_keys = h.m_keys;
        m_values = h.m_values;
    }
    return *this;
}

template <class K, class V, int SIZE>
UMBRA_FORCE_INLINE int StaticHeap<K, V, SIZE>::getSize(void) const
{
    return m_keys.getSize();
}

template <class K, class V, int SIZE>
UMBRA_FORCE_INLINE const K& StaticHeap<K, V, SIZE>::getKey(int i) const
{
    return m_keys[i];
}

template <class K, class V, int SIZE>
UMBRA_FORCE_INLINE const V& StaticHeap<K, V, SIZE>::getValue(int i) const
{
    return m_values[i];
}

template <class K, class V, int SIZE> void StaticHeap<K, V, SIZE>::insert(const K& key, const V& value)
{
    int idx = m_keys.getSize();
    UMBRA_ASSERT(idx < SIZE);
    m_keys.grow();
    m_values.grow();
    while (idx > 0)
    {
        int pidx = ((idx + 1) >> 1) - 1;
        if (m_keys[pidx] <= key)
            break;
        m_keys[idx] = m_keys[pidx];
        m_values[idx] = m_values[pidx];
        idx = pidx;
    }
    m_keys[idx] = key;
    m_values[idx] = value;
}

template <class K, class V, int SIZE> void StaticHeap<K, V, SIZE>::removeFirst(void)
{
    UMBRA_ASSERT(getSize());

    if (m_keys.getSize() > 1)
    {
        int size = m_keys.getSize()-1;
        K mKey = m_keys.popBack();
        V mVal = m_values.popBack();

        int idx = 0;
        for (;;)
        {
            int cidx = (idx << 1) + 1;
            if (cidx >= size)
                break;
            if (cidx + 1 < size && m_keys[cidx + 1] <= m_keys[cidx])
                cidx++;
            if (mKey <= m_keys[cidx])
                break;
            m_keys[idx] = m_keys[cidx];
            m_values[idx] = m_values[cidx];
            idx = cidx;
        }
        m_keys[idx] = mKey;
        m_values[idx] = mVal;
    }
    else
    {
        m_keys.popBack();
        m_values.popBack();
    }
}

template <class K, class V, int SIZE> void StaticHeap<K, V, SIZE>::heapify(int idx)
{
    for (;;)
    {
        int cidx = (idx << 1) + 1;
        if (cidx >= m_keys.getSize())
            break;
        if (cidx + 1 < m_keys.getSize() && m_keys[cidx + 1] <= m_keys[cidx])
            cidx++;
        if (m_keys[idx] <= m_keys[cidx])
            break;
        swap(m_keys[idx], m_keys[cidx]);
        swap(m_values[idx], m_values[cidx]);
        idx = cidx;
    }
}

template <class K, class V, int SIZE> void StaticHeap<K, V, SIZE>::clear(void)
{
    m_keys.clear();
    m_values.clear();
}

template <class K, class V, int SIZE> bool StaticHeap<K, V, SIZE>::decreaseKey(const K& key, const V& value)
{
    // Slow, must iterate all nodes to find value.
    // Better way to find/keep track of value's position?
    for(int idx = 0; idx < m_values.getSize(); idx++)
    {
        if( m_values[idx] == value )
        {
            if (key >= m_keys[idx])
                return false;
            m_keys[idx] = key;

            while (idx > 0)
            {
                int pidx = ((idx + 1) >> 1) - 1;
                if (m_keys[pidx] <= key)
                    break;
                m_keys[idx] = m_keys[pidx];
                m_values[idx] = m_values[pidx];
                idx = pidx;
            }
            m_keys[idx] = key;
            m_values[idx] = value;

            return true;
        }
    }
    return false;
}

} // namespace Umbra

#endif

//--------------------------------------------------------------------
