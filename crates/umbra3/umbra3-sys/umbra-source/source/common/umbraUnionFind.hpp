#pragma once

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
 * \brief   Union find
 *
 */

#include "umbraArray.hpp"
#include "umbraHash.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief           Simple union-find data structure.
 *//*-------------------------------------------------------------------*/

template <class T> class UnionFind: public Base
{
public:
                                UnionFind           (Allocator* a = NULL);
                                UnionFind           (const UnionFind<T>& uf);
                                ~UnionFind          (void);

    UnionFind&                  operator=           (const UnionFind<T>& uf);

    int                         findSet             (const T& obj) const;
    void                        unionSets           (const T& a, const T& b);
    bool                        isAlone             (const T& obj) const;
    void                        clear               (void);

private:
    int                         getIndex            (const T& obj) const;

    mutable Hash<T, int32>      m_hash;
    mutable Array<int32>        m_links;
    mutable Array<int32>        m_height;
};

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class T> UnionFind<T>::UnionFind(Allocator* a): Base(a), m_hash(a), m_links(a), m_height(a)
{
    /* empty */
}

template <class T> UnionFind<T>::UnionFind(const UnionFind<T>& uf)
    : Base(uf.getAllocator()), m_hash(uf.getAllocator()), m_links(uf.getAllocator()), m_height(uf.getAllocator())
{
    operator=(uf);
}

template <class T> UnionFind<T>::~UnionFind(void)
{
    /* empty */
}

template <class T> UnionFind<T>& UnionFind<T>::operator=(const UnionFind<T>& uf)
{
    if (&uf != this)
    {
        m_hash = uf.m_hash;
        m_links = uf.m_links;
        m_height = uf.m_height;
    }
    return *this;
}

template <class T> int UnionFind<T>::getIndex(const T& obj) const
{
    int32 idx = -1;
    if (!m_hash.get(obj, idx))
    {
        idx = m_links.getSize();
        m_hash.insert(obj, idx);
        m_links.pushBack(idx);
        m_height.pushBack(1);
    }
    return idx;
}

template <class T> int UnionFind<T>::findSet(const T& obj) const
{
    int objIdx = getIndex(obj);
    int setIdx = objIdx;
    while (m_links[setIdx] != setIdx && m_links[setIdx] != -1)
        setIdx = m_links[setIdx];
    while (objIdx != setIdx)
    {
        int tmp = m_links[objIdx];
        m_links[objIdx] = setIdx;
        objIdx = tmp;
    }
    return setIdx;
}

template <class T> void UnionFind<T>::unionSets(const T& a, const T& b)
{
    int setA = findSet(a);
    int setB = findSet(b);
    if (m_height[setA] < m_height[setB])
    {
        m_links[setA] = setB;
        m_links[setB] = -1;
    } else if (m_height[setA] > m_height[setB])
    {
        m_links[setA] = -1;
        m_links[setB] = setA;
    } else if (setA != setB)
    {
        m_links[setA] = setB;
        m_links[setB] = -1;
        m_height[setB]++;
    }
}

template <class T> bool UnionFind<T>::isAlone(const T& obj) const
{
    int set = findSet(obj);
    return (m_links[set] == set);
}

template <class T> void UnionFind<T>::clear(void)
{
    m_hash.clear();
    m_links.reset(0);
    m_height.reset(0);
}

} // namespace Umbra

//--------------------------------------------------------------------
