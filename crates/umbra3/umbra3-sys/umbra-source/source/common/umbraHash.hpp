#ifndef UMBRAHASH_HPP
#define UMBRAHASH_HPP

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
 * \brief   Umbra Hash
 *
 */

#include "umbraArray.hpp"
#include "umbraMemory.hpp"
#include <string.h>

namespace Umbra
{

template <class Key, class Value>
class Hash : public Base
{
public:

    enum
    {
        NIL = -1                                //!< internal enumeration for representing a null link
    };

                    Hash                (Allocator* a = NULL);
                    Hash                (const Hash&);
                    ~Hash               (void);
    Value*          insert              (const Key& s, const Value& d);
    void            update              (const Hash& o);
    void            remove              (const Key& s);
    Value*          get                 (const Key& s);
    const Value*    get                 (const Key& s) const { return const_cast<Hash<Key, Value>&>(*this).get(s); }
    bool            get                 (const Key& s, Value& d) const;
    Value&          getDefault          (const Key& s, const Value& defaultValue);
    bool            contains            (const Key& s) const;
    void            getKeyArray         (Array<Key>& arr) const;
    void            getValueArray       (Array<Value>& arr) const;
    void            getKeyArray         (Key* arr) const;
    void            getValueArray       (Value* arr) const;
    void            clear               (void);
    int             getNumKeys          (void) const;

    /*
    // Iterator example:
    Hash<A,B>::Iterator it = hash.iterate();
    while(hash.isValid(it))
    {
        const A& key = hash.getKey(it);
        const B& val = hash.getValue(it);
        hash.next(it);
    }
    */
    struct Iterator { int table; int entry; };
    Iterator        iterate             (void) const;
    void            next                (Iterator& it) const;
    bool            isValid             (Iterator& it) const    { return it.table < (int)size && it.entry != NIL; }
    const Key&      getKey              (Iterator it) const     { return table[it.entry].key; }
    const Value&    getValue            (Iterator it) const     { return table[it.entry].value; }
    Value&          getValue            (Iterator it)           { return table[it.entry].value; }

    Hash& operator= (const Hash& o)
    {
        clear();
        first = o.first;
        size = o.size;
        numKeys = o.numKeys;
        if (size)
        {
            table = UMBRA_NEW_ARRAY(Entry, size);
            hash = UMBRA_NEW_ARRAY(int, size);
            memset(table, 0, sizeof(Entry) * size);
            memset(hash, 0, sizeof(int) * size);
        }
        for (int i = 0; i < size; i++)
        {
            copyHeap(&table[i].key, getAllocator());
            copyHeap(&table[i].value, getAllocator());
            table[i] = o.table[i];
            hash[i] = o.hash[i];
        }
        return *this;
    }

public:
    template<typename OP> void streamOp (OP& op)
    {
        stream(op, first);
        stream(op, size);
        stream(op, numKeys);
        if (OP::IsWrite && size)
        {
            table = UMBRA_NEW_ARRAY(Entry, size);
            hash = UMBRA_NEW_ARRAY(int, size);
            for (int i = 0; i < size; i++)
            {
                copyHeap(&table[i].key, getAllocator());
                copyHeap(&table[i].value, getAllocator());
            }
        }
        for (int i = 0; i < size; i++)
        {
            stream(op, table[i]);
            stream(op, hash[i]);
        }
    }

private:
    int* getListHead (const Key& s) const { if (!size) return NULL; return &hash[getHashVal(s, size)]; }
    static inline unsigned int  getHashVal  (const Key& s, const unsigned int hashArraySize);

    bool        rehash              (size_t newsize = 0);

    struct Entry
    {
        int         next;                   //!< next pointer in linked list
        Key         key;                    //!< key
        Value       value;                  //!< value

        template<typename OP> void streamOp (OP & op)
        {
            stream(op, next);
            stream(op, key);
            stream(op, value);
        }
    };

    int*            hash;
    Entry*          table;
    int             first;                  //!< handle of first free entry
    int             size;                   //!< size of hash table
    int             numKeys;                //!< number of contained keys
};

//------------------------------------------------------------------------
// Helpers for hash function implementation
//------------------------------------------------------------------------

UMBRA_FORCE_INLINE void shuffle(uint32& a)
{
    a += (a << 12);
    a ^= (a >> 22);
    a += (a << 4);
    a ^= (a >> 9);
    a += (a << 10);
    a ^= (a >> 2);
    a += (a << 7);
    a ^= (a >> 12);
}

UMBRA_FORCE_INLINE void shuffle(uint32& a, uint32& b, uint32& c)
{
    a = a - b;  a = a - c;  a = a ^ (c >> 13);
    b = b - c;  b = b - a;  b = b ^ (a << 8);
    c = c - a;  c = c - b;  c = c ^ (b >> 13);
    a = a - b;  a = a - c;  a = a ^ (c >> 12);
    b = b - c;  b = b - a;  b = b ^ (a << 16);
    c = c - a;  c = c - b;  c = c ^ (b >> 5);
    a = a - b;  a = a - c;  a = a ^ (c >> 3);
    b = b - c;  b = b - a;  b = b ^ (a << 10);
    c = c - a;  c = c - b;  c = c ^ (b >> 15);
}

UMBRA_FORCE_INLINE void shuffleInts(uint32& a, uint32& b, uint32& c, const uint32* data, int n)
{
    UMBRA_ASSERT(n >= 0);
    int i = 0;
    while (i < n-2)
    {
        a += data[i++];
        b += data[i++];
        c += data[i++];
        shuffle(a, b, c);
    }
    if (i < n)
    {
        a += data[i++];
        if (i < n)
            b += data[i++];
        shuffle(a, b, c);
    }
    UMBRA_ASSERT(i == n);
}

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class Key, class Value> inline unsigned int Hash<Key,Value>::getHashVal   (const Key& s, const unsigned int hashArraySize)
{
    return getHashValue (s) & (hashArraySize-1);
}

template <class Key, class Value> inline Value* Hash<Key,Value>::insert   (const Key& s, const Value& d)
{
    UMBRA_ASSERT(!contains(s));
    if (first == NIL && !rehash())                   // need to re-alloc and re-hash tables
        return NULL;
    int h = first;
    first = table[first].next;

    unsigned int hval   = getHashVal(s, size);
    table[h].key    = s;
    table[h].value  = d;
    table[h].next   = hash[hval];
    hash[hval]      = h;
    numKeys++;

    return &table[h].value;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Gets the number of inserted keys.
 * \return  Number of entries in hash.
 *//*-------------------------------------------------------------------*/

template <class Key, class Value> inline int Hash<Key,Value>::getNumKeys (void) const
{
    return numKeys;
}

template <class Key, class Value> inline void Hash<Key,Value>::getKeyArray (Key* array) const
{
    int cnt = 0;
    for (int i=0; i<(int)size; i++)
        for (int h = hash[i]; h != NIL; h = table[h].next)
            array[cnt++] = table[h].key;
}

template <class Key, class Value> inline void Hash<Key,Value>::getValueArray (Value* array) const
{
    int cnt = 0;
    for (int i=0; i<(int)size; i++)
        for (int h = hash[i]; h != NIL; h = table[h].next)
            array[cnt++] = table[h].value;
}

template <class Key, class Value> inline void Hash<Key,Value>::getKeyArray (Array<Key>& array) const
{
    array.reset(numKeys);
    getKeyArray(array.getPtr());
}

template <class Key, class Value> inline void Hash<Key,Value>::getValueArray (Array<Value>& array) const
{
    array.reset(numKeys);
    getValueArray(array.getPtr());
}

template <class Key, class Value> inline void Hash<Key,Value>::clear (void)
{
    UMBRA_DELETE_ARRAY(hash);
    UMBRA_DELETE_ARRAY(table);
    hash = NULL;
    table = NULL;
    first = NIL;
    size = 0;
    numKeys = 0;
}

template <class Key, class Value> inline void Hash<Key,Value>::remove   (const Key& s)
{
    int* slot = getListHead(s);
    if (!slot)
        return;

    while (*slot != NIL)
    {
        int h = *slot;
        if (table[h].key == s)
        {
            *slot = table[h].next;
            table[h].next = first;
            first = h;
            numKeys--;
            return;
        }
        slot = &table[h].next;
    }
}

template <class Key, class Value> inline void Hash<Key,Value>::update (const Hash<Key,Value>& o)
{
    Iterator i = o.iterate();
    while (o.isValid(i))
    {
        Value* v = get(o.getKey(i));
        if (v)
            *v = o.getValue(i);
        else
            insert(o.getKey(i), o.getValue(i));
        o.next(i);
    }
}

template <class Key, class Value> inline Value* Hash<Key,Value>::get (const Key& s)
{
    int* slot = getListHead(s);
    int h = slot ? *slot : NIL;
    while (h != NIL)
    {
        if (table[h].key == s)
            return &table[h].value;
        h = table[h].next;
    }
    return NULL;
}

template <class Key, class Value> inline Value& Hash<Key,Value>::getDefault (const Key& s, const Value& d)
{
    Value* v = get(s);
    if (!v)
        v = insert(s, d);
    return *v;
}


template <class Key, class Value> inline bool Hash<Key,Value>::get (const Key& s, Value& d) const
{
    const Value* v = get(s);
    if (!v)
        return false;
    d = *v;
    return true;
}

template <class Key, class Value> inline bool Hash<Key,Value>::contains (const Key& s) const
{
    int* slot = getListHead(s);
    int h = slot ? *slot : NIL;
    while (h != NIL)
    {
        if (table[h].key == s)
            return true;
        h = table[h].next;
    }
    return false;
}

template <class Key, class Value> inline bool Hash<Key,Value>::rehash   (size_t newSize)
{
    if (newSize == 0)
    {
        newSize = size*2;
        if (newSize < 4)
            newSize = 4;
    }
    UMBRA_ASSERT((int)newSize > size);

    Entry* newTable = NULL;
    int*    newHash = NULL;

#if !defined(UMBRA_COMP_NO_EXCEPTIONS)
    try
    {
#endif

    newTable = UMBRA_NEW_ARRAY(Entry, newSize);
    newHash  = UMBRA_NEW_ARRAY(int, newSize);

#if defined(UMBRA_COMP_NO_EXCEPTIONS)
    if (!newTable || !newHash)
    {
        UMBRA_DELETE_ARRAY(newHash);
        UMBRA_DELETE_ARRAY(newTable);
        return false;
    }
#else
    } catch(OOMException)
    {
        UMBRA_DELETE_ARRAY(newHash);
        UMBRA_DELETE_ARRAY(newTable);
        throw;
    }
#endif

    memset(newTable, 0, sizeof(Entry) * newSize);
    memset(newHash, 0, sizeof(int) * newSize);

    int cnt = 0;
    int i;

    for (i = 0; i < (int)newSize; i++)
    {
        copyHeap(&newTable[i].key, getAllocator());
        copyHeap(&newTable[i].value, getAllocator());
        newTable[i].next    = NIL;
        newHash[i]          = NIL;
    }

    if (size)                                           // if we have existing data, it needs to be rehashed
    {
        for (i = 0; i < (int)size; i++)                     // step through each old hash set
        {
            int h = hash[i];
            while (h != NIL)
            {
                unsigned int hVal   = getHashVal(table[h].key, (unsigned int)newSize);
                newTable[cnt].key   = table[h].key;
                newTable[cnt].value = table[h].value;
                newTable[cnt].next  = newHash[hVal];
                newHash[hVal]       = cnt;
                cnt++;
                h = table[h].next;
            }
        }
        UMBRA_DELETE_ARRAY(hash);
        UMBRA_DELETE_ARRAY(table);
    }

    for (i = cnt; i < (int)newSize; i++)
        newTable[i].next = i+1;
    newTable[newSize-1].next = NIL;

    first   = cnt;
    hash    = newHash;
    table   = newTable;
    size    = (int)newSize;

    return true;
}

template <class Key, class Value> inline Hash<Key,Value>::Hash (Allocator* a)
:   Base    (a),
    hash    (0),
    table   (0),
    first   (NIL),
    size    (0),
    numKeys (0)
{
}

template <class Key, class Value> inline Hash<Key,Value>::Hash (const Hash& h)
:   Base    (h.getAllocator()),
    hash    (0),
    table   (0),
    first   (NIL),
    size    (0),
    numKeys (0)
{
    *this = h;
}

template <class Key, class Value> inline Hash<Key,Value>::~Hash ()
{
    clear();
}

template <class Key, class Value> inline typename Hash<Key,Value>::Iterator Hash<Key,Value>::iterate(void) const
{
    Iterator it;
    it.table = 0;
    it.entry = NIL;
    if (size == 0)
        return it;
    it.entry = hash[0];
    if (it.entry == NIL)
        next(it);
    return it;
}

template <class Key, class Value> inline void Hash<Key,Value>::next (Iterator& it) const
{
    if (it.entry != NIL)
        it.entry = table[it.entry].next;
    while (it.entry == NIL && it.table < (int)size)
    {
        it.table++;
        it.entry = it.table < (int)size ? hash[it.table] : NIL;
    }
}

} // namespace Umbra

#endif // UMBRAHASH_HPP

//--------------------------------------------------------------------
