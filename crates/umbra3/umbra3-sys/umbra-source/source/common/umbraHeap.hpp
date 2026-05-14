#ifndef UMBRAHEAP_HPP
#define UMBRAHEAP_HPP

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

template <class K, class V> class Heap
{
public:
                                Heap                (Allocator* a = NULL);
                                Heap                (const Heap<K, V>& h);
                                Heap                (const Array<K>& keys, const Array<V>& values);
                                ~Heap               (void);

    Heap&                       operator=           (const Heap<K, V>& h);

    int                         getSize             (void) const;
    const K&                    getKey              (int i) const;
    const V&                    getValue            (int i) const;
    void                        insert              (const K& key, const V& value);
    void                        removeFirst         (void);
    void                        clear               (void);
    bool                        decreaseKey         (const K& key, const V& value);

private:
    void                        heapify             (int idx);

    Array<K>                    m_keys;
    Array<V>                    m_values;
};

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class K, class V> Heap<K, V>::Heap(Allocator* a)
: m_keys(a),
  m_values(a)
{
    /* empty */
}

template <class K, class V> Heap<K, V>::Heap(const Heap<K, V>& h)
:   m_keys      (h.m_keys),
    m_values    (h.m_values)
{
    /* empty */
}

template <class K, class V> Heap<K, V>::Heap(const Array<K>& keys, const Array<V>& values)
:   m_keys      (keys),
    m_values    (values)
{
    UMBRA_ASSERT(m_keys.getSize() == m_values.getSize());
    for (int i = (m_keys.getSize() >> 1) - 1; i >= 0; i--)
        heapify(i);
}

template <class K, class V> Heap<K, V>::~Heap(void)
{
    /* empty */
}

template <class K, class V> Heap<K, V>& Heap<K, V>::operator=(const Heap<K, V>& h)
{
    if (&h != this)
    {
        m_keys = h.m_keys;
        m_values = h.m_values;
    }
    return *this;
}

template <class K, class V> int Heap<K, V>::getSize(void) const
{
    return m_keys.getSize();
}

template <class K, class V> const K& Heap<K, V>::getKey(int i) const
{
    return m_keys[i];
}

template <class K, class V> const V& Heap<K, V>::getValue(int i) const
{
    return m_values[i];
}

template <class K, class V> void Heap<K, V>::insert(const K& key, const V& value)
{
    int idx = m_keys.getSize();
    m_keys.pushBack(key);
    m_values.pushBack(value);
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

template <class K, class V> void Heap<K, V>::removeFirst(void)
{
    UMBRA_ASSERT(getSize());
    if (m_keys.getSize() > 1)
    {
        m_keys[0] = m_keys[m_keys.getSize() - 1];
        m_values[0] = m_values[m_values.getSize() - 1];
        m_keys.popBack();
        m_values.popBack();
        heapify(0);
    } else
    {
        m_keys.popBack();
        m_values.popBack();
    }
}

template <class K, class V> void Heap<K, V>::heapify(int idx)
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

template <class K, class V> void Heap<K, V>::clear(void)
{
    m_keys.reset(0);
    m_values.reset(0);
}

template <class K, class V> bool Heap<K, V>::decreaseKey(const K& key, const V& value)
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

#endif // UMBRAHEAP_HPP

//--------------------------------------------------------------------
