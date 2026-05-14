// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Array.hpp>
#include <standard/Pair.hpp>
#include <standard/Float.hpp>

namespace Umbra
{

template<typename T>
struct Hasher;

template<typename T, typename Hash = Hasher<T> >
struct DefaultHashFuncs
{
    uint32_t hash  (const T& t) const { return Hash().hash(t); }
    bool     equals(const T& a, const T& b) const { return a == b; }
};

template<typename T, typename HashFuncs = DefaultHashFuncs<T> >
class HashBase
{
public:
    typedef T ContainedType;
    typedef typename ResizableArray<T>::Iterator Iterator;

    HashBase(MemoryManager& mm) : m_hash(mm), m_containedEntries(mm), m_entries(&m_containedEntries), m_used(0) {}

    void setEntries(ResizableArray<T>* entries)
    {
        UMBRA_ASSERT(m_containedEntries.size() == 0);
        UMBRA_ASSERT(m_entries == &m_containedEntries);
        m_entries = entries;
    }

    int insert(const T& t)
    {
        int idx = find(t, true);
        if (idx < 0)
        {
            m_entries->add(t);
            rehash();
            return m_entries->size()-1;
        }

        if (isUsed(m_hash[idx]))
        {
            // TODO: do not update here
            (*m_entries)[m_hash[idx] >> 2] = t;
            return m_hash[idx] >> 2;
        }

        if (isEmpty(m_hash[idx]))
        {
            if (m_used >= m_hash.size()*3/4)
            {
                m_entries->add(t);
                rehash();
                return m_entries->size()-1;
            }

            m_used++;
        }

        UMBRA_ASSERT(isEmpty(m_hash[idx]) || isRemoved(m_hash[idx]));

        m_entries->add(t);
        m_hash[idx] = ((m_entries->size()-1) << 2) | 2;

        return m_entries->size()-1;
    }

    bool remove(const T& t)
    {
        int idx = find(t, false);
        if (idx < 0 || isEmpty(m_hash[idx]))
            return false;

        UMBRA_ASSERT(isUsed(m_hash[idx]));

        // TODO: Shrinking policy.

        if (m_entries->size() == 1)
        {
            m_entries->clear();
            rehash();
            return true;
        }

        int lastIdx = find((*m_entries)[m_entries->size()-1], false);
        UMBRA_ASSERT(isUsed(m_hash[lastIdx]));

        (*m_entries)[m_hash[idx]>>2] = (*m_entries)[m_entries->size()-1];
        m_entries->removeLast();

        m_hash[lastIdx] = m_hash[idx];
        m_hash[idx] = 1;

        return true;
    }

    bool contains(const T& t) const
    {
        int idx = find(t, false);
        return idx >= 0 && isUsed(m_hash[idx]);
    }

    const T* get(const T& t) const
    {
        int idx = find(t, false);
        return (idx >= 0 && isUsed(m_hash[idx])) ? &(*m_entries)[m_hash[idx]>>2] : 0;
    }

    Iterator iterate() const
    {
        return m_entries->iterate();
    }

protected:
    static bool isEmpty(uint32_t code)
    {
        return code == 0;
    }

    static bool isRemoved(uint32_t code)
    {
        return code == 1;
    }

    static bool isUsed(uint32_t code)
    {
        return (code & 3) == 2;
    }

    int find(const T& t, bool allowRemoved) const
    {
        if (m_hash.size() == 0)
            return -1;

        uint32_t limit = m_hash.size()-1;
        uint32_t idx = m_hasher.hash(t) & limit;

        for (int i = 0; i <= (int)limit; i++)
        {
            if (isEmpty(m_hash[idx]))
                return idx;

            if (isRemoved(m_hash[idx]))
            {
                if (allowRemoved)
                    return idx;
            }
            else
            {
                UMBRA_ASSERT(isUsed(m_hash[idx]));

                // TODO: store some of the hash value to extra bits to avoid equality tests
                if (m_hasher.equals(t, (*m_entries)[m_hash[idx] >> 2]))
                    return idx;
            }

            idx = (idx + i + 1) & limit;
        }

        UMBRA_ASSERT(!"internal hash fail");
        return -1;
    }

    void rehash()
    {
        int n = m_entries->size()*2 + 7;
        n |= n >> 1; n |= n >> 2; n |= n >> 4; n |= n >> 8; n |= n >> 16;
        n++;

        m_hash.resize(n);
        for (int i = 0; i < m_hash.size(); i++)
            m_hash[i] = 0;

        for (int i = 0; i < m_entries->size(); i++)
        {
            int idx = find((*m_entries)[i], true);
            UMBRA_ASSERT(idx >= 0 && isEmpty(m_hash[idx]));
            m_hash[idx] = (i << 2) | 2;
        }

        m_used = m_entries->size();
    }

private:
    ResizableArray<uint32_t>      m_hash;
    ResizableArray<T>             m_containedEntries;
    ResizableArray<T>*            m_entries;
    int                           m_used; // also counts for removed entries in m_hash
    HashFuncs                     m_hasher;
};

template<typename K, typename V, typename Hash = DefaultHashFuncs<K> >
struct PairFirstHasher
{
    uint32_t hash  (const Pair<K, V>& t) const { return m_hasher.hash(t.a); }
    bool     equals(const Pair<K, V>& a, const Pair<K, V>& b) const { return m_hasher.equals(a.a, b.a); }

private:
    Hash m_hasher;
};

template<typename K, typename V, typename HashFuncs = DefaultHashFuncs<K> >
class UnorderedMap : public HashBase<Pair<K, V>, PairFirstHasher<K, V, HashFuncs> >
{
public:
    typedef K KeyType;
    typedef V ValueType;
    typedef HashBase<Pair<K, V>, PairFirstHasher<K, V, HashFuncs> > HashType;
    typedef typename HashBase<Pair<K, V>, PairFirstHasher<K, V, HashFuncs> >::Iterator Iterator;

    UnorderedMap(MemoryManager& mm) : HashType(mm) {}

    void insert(const K& k, const V& v)
    {
        ((HashType*)this)->insert(Pair<K, V>(k, v));
    }

    const V* get(const K& k) const
    {
        const Pair<K, V>* ret = ((const HashType*)this)->get(Pair<K, V>(k, V()));
        return ret ? &ret->b : 0;
    }

    bool remove(const K& k)
    {
        return ((HashType*)this)->remove(Pair<K, V>(k, V()));
    }

    struct KeyIterator
    {
        KeyIterator() {}
        KeyIterator(const Iterator& iter) : iter(iter) {}

        K& operator*() { return (*iter).a; }
        const K& operator*() const { return (*iter).a; }
        void operator++() { ++iter; }
        void operator++(int) { ++iter; }
        operator bool() const { return (bool)iter; }

    private:
        Iterator iter;
    };

    KeyIterator iterateKeys()
    {
        return KeyIterator(this->iterate());
    }

    struct ValueIterator
    {
        ValueIterator() {}
        ValueIterator(const Iterator& iter) : iter(iter) {}

        K& operator*() { return (*iter).b; }
        const K& operator*() const { return (*iter).b; }
        void operator++() { ++iter; }
        void operator++(int) { ++iter; }
        operator bool() const { return (bool)iter; }

    private:
        Iterator iter;
    };

    ValueIterator iterateValues()
    {
        return ValueIterator(this->iterate());
    }
};

template<typename T, typename HashFuncs = DefaultHashFuncs<T> >
class UnorderedSet : public HashBase<T, HashFuncs>
{
public:
    UnorderedSet(MemoryManager& mm) : HashBase<T, HashFuncs>(mm) {}
};

// A few hash functions.

template<>
struct Hasher<float>
{
    uint32_t hash(const float& s) const
    {
        UMBRA_ASSERT(isFloatFinite(s));
        if (s == -0.f || s == 0.f)
            return 0;
        uint32_t ui = floatAsInt(s);
        return (ui>>22) + (ui>>12) + ui;
    }
};

template<>
struct Hasher<int>
{
    uint32_t hash(const int& s) const
    {
        uint32_t hval = s;
        hval = hval + (hval>>5) + (hval>>10) + (hval>>20);
        return hval;
    }
};

template<typename A, typename B> struct Hasher<Pair<A, B> >
{
    uint32_t hash(const Pair<A, B>& t) const
    {
        return Hasher<A>().hash(t.a) + Hasher<B>().hash(t.b);
    }
};

}
