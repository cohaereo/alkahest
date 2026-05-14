#pragma once
#ifndef __UMBRASTATICHEAP_HPP
#define __UMBRASTATICHEAP_HPP

/*!
 *
 * Umbra
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
 * \brief   Heap implementation
 *
 */

#include "umbraPrivateDefs.hpp"

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
                                ~StaticHeap         (void);

    UMBRA_FORCE_INLINE int      getSize             (void) const { return m_used; }
    UMBRA_FORCE_INLINE int      getMaxSize          (void) const { return SIZE; }
    UMBRA_FORCE_INLINE const K& getKey              (int i) const { return m_keys[i]; }
    UMBRA_FORCE_INLINE const V& getValue            (int i) const { return m_values[i]; }
    void                        insert              (const K& key, const V& value);
    void                        removeFirst         (void);
    void                        remove              (const V& value);
    void                        clear               (void);
    bool                        decreaseKey         (const K& key, const V& value);
    V*                          getValueArr         (void) { return m_values; }

private:
    void                        heapify             (int idx);

    int                         m_used;
    K                           m_keys[SIZE];
    V                           m_values[SIZE];
};

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class K, class V, int SIZE> StaticHeap<K, V, SIZE>::StaticHeap(void): m_used(0)
{
    /* empty */
}

template <class K, class V, int SIZE> StaticHeap<K, V, SIZE>::~StaticHeap(void)
{
    /* empty */
}

template <class K, class V, int SIZE> UMBRA_FORCE_INLINE void StaticHeap<K, V, SIZE>::insert(const K& key, const V& value)
{
    int idx = m_used++;
    UMBRA_ASSERT(idx < SIZE);
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

template <class K, class V, int SIZE> UMBRA_FORCE_INLINE void StaticHeap<K, V, SIZE>::removeFirst(void)
{
    UMBRA_ASSERT(getSize());

    if (--m_used)
    {
        int size = m_used;
        K mKey = m_keys[m_used];
        V mVal = m_values[m_used];

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
}

template <class K, class V, int SIZE> void StaticHeap<K, V, SIZE>::clear(void)
{
    m_used = 0;
}

template <class K, class V, int SIZE> bool StaticHeap<K, V, SIZE>::decreaseKey(const K& key, const V& value)
{
    // Slow, must iterate all nodes to find value.
    // Better way to find/keep track of value's position?
    for(int idx = 0; idx < m_used; idx++)
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

template <class K, class V, int SIZE> void StaticHeap<K, V, SIZE>::remove(const V& value)
{
    for (int idx = 0; idx < m_used; idx++)
    {
        if (m_values[idx] == value)
        {
            if (--m_used == 0)
                return;

            K mKey = m_keys[m_used];
            V mValue = m_values[m_used];
            int size = m_used;
            for (;;)
            {
                int cidx = (idx << 1) + 1;
                if (cidx >= size)
                    break;
                if (cidx + 1 < size && m_keys[cidx + 1] <= m_keys[cidx])
                    cidx++;
                if (m_keys[idx] <= m_keys[cidx])
                    break;
                m_keys[idx] = m_keys[cidx];
                m_values[idx] = m_values[cidx];
                idx = cidx;
            }
            m_keys[idx] = mKey;
            m_values[idx] = mValue;
            break;
        }
    }
}

} // namespace Umbra

#endif

//--------------------------------------------------------------------
