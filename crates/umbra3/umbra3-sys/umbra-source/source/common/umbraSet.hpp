#ifndef UMBRASET_HPP
#define UMBRASET_HPP

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
 * \brief   Umbra Set
 * \todo [wili] Using inheritance rather than 'using' ?
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraBitMath.hpp"
#include "umbraArray.hpp"

#include <cstring> // memcpy

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief           Set class template.
 *
 * \note            The Set class represents a 'set' of data elements.
 *                  Standard set operation include inserting and removing
 *                  elements from a set, determining whether an element
 *                  is part of a set and a variety of operations between
 *                  sets (comparison, copying, subtraction, union).
 *
 * \note            Sets never contain duplicate components. For example
 *                  if the sets A = {1,3,5} and B = {1,4,5} were unioned,
 *                  the resulting set would be {1,3,4,5}.
 *
 * \note            When a set is converted into a linearly accessible
 *                  array (either through the Set::Array class constructor
 *                  or Set::getArray()), the order of elements is not
 *                  constant. Thus the set A = {1,3,5} could return an
 *                  array {1,3,5} or {5,1,3} or any other combination. If
 *                  ordering of arrays is required, the easiest way is to
 *                  apply any of SurRender's sorting functions to the
 *                  resulting set. However, for purposes of comparing
 *                  sets two sets are considered equal if they contain
 *                  the same elements (i.e. {1,3,5} and {5,3,1} are equal).
 *
 * \note            The Set class is implemented with a powerful hash
 *                  table mechanism for optimal query times. On average
 *                  all single element operations are O(1) (i.e. insertion,
 *                  removal and query) and all set-to-set operations
 *                  are O(N). The hash function can be overloaded.
 *
 * \note            Sets can be used for all data types which have an
 *                  equality operator (i.e. operator==) defined.
 *//*-------------------------------------------------------------------*/

template <class S> class Set: public Base
{

public:
                Set                 (Allocator* a = NULL);
                Set                 (const Set<S>& src);
                Set                 (const S* src, int cnt, Allocator* a = NULL);
                ~Set                (void);
    bool        operator==          (const Set<S>& src) const;
    bool        operator!=          (const Set<S>& src) const       { return (!(*this == src));}
    Set<S>&     operator+=          (const S& s)                    { insert(s); return *this; }
    Set<S>&     operator-=          (const S& s)                    { remove(s); return *this; }
    Set<S>&     operator=           (const Set<S>& src);
    Set<S>&     operator&=          (const Set<S>& src);
    Set<S>&     operator|=          (const Set<S>& src);
    Set<S>&     operator-=          (const Set<S>& src);
    void        checkConsistency    (void);
    bool        contains            (const S& s) const;
    bool        contains            (const Set<S>& src) const;
    bool        intersects          (const Set<S>& src) const;
    void        getArray            (Array<S>& arr, bool exactSize=false) const;
    int         getMemoryUsage      (void) const                    { return sizeof(Set<S>) + sizeof(Handle)*size + sizeof(Entry)*size; }
    int         getSize             (void) const                    { return elemCount; }
    bool        insert              (const S& s);
    bool        isEmpty             (void) const                    { return (elemCount == 0); }
    void        remove              (const S& s);
    S           removeAny           (void);
    void        removeAll           (bool minimizeMemoryUsage = true);

    // DEBUG DEBUG usability testing - do not use
    const S*    get                 (const S& s) const;

private:

    typedef int Handle;                     //!< signed int

    enum
    {
        NIL         = -1,                   //!< value used for terminating linked lists
        MIN_SIZE    = 8                     //!< smallest size the hash can shrink
    };

    struct Entry
    {
        Handle      next;                   //!< next pointer in linked list
        S           s;                      //!< src val
    };

public:

    class Iterator
    {
    public:
        bool next (void)
        {
            // move to next in chain
            if (h != NIL)
                h = set->table[h].next;

            // find non-empty chain
            while (h == NIL)
            {
                if (++idx >= (int)set->size)
                    return false;
                h = set->hash[idx];
            }

            return true;
        }

        const S& getValue (void) const
        {
            UMBRA_ASSERT(h != NIL);
            return set->table[h].s;
        }

    private:
        Iterator(const Set* s): set(s), idx(-1), h(NIL) {}

        const Set* set;
        int idx;
        Handle h;

        friend class Set;
    };

    Iterator iterate (void) const
    {
        return Iterator(this);
    }

private:

    inline friend unsigned int getHashValue (const Set<S>& s)
    {
        int hval = 0;
        for (int i = 0; i < (int)s.size; i++)
        for (Handle h = s.hash[i]; h != NIL; h = s.table[h].next)
            hval += getHashValue(s.table[h].s);
        return hval;
    }

    unsigned int    size;                   //!< size of hash table
    Handle*         hash;                   //!< hash pointers
    Entry*          table;                  //!< allocation table
    Handle          first;                  //!< handle of first free entry
    int             elemCount;

    // extension to provide fast stack-based serving for
    // small sets

    Handle          sHash[MIN_SIZE];
    Entry           sTable[MIN_SIZE];

    unsigned int    getHashVal      (const S& s, const unsigned int hashArraySize) const;
    void            rehash          (size_t newSize);
    void            cleanup         (void);
};

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------


template <class S> inline void  Set<S>::getArray (Array<S>& array,bool exactSize) const
{
    UMBRA_UNREF(exactSize);
    array.reset(elemCount);
    int cnt=0;

    for (int i=0; i<(int)size; i++)
    for (Handle h = hash[i]; h != NIL; h = table[h].next)
        array[cnt++] = table[h].s;

    UMBRA_ASSERT(cnt == elemCount);
}


template <class S> inline void Set<S>::checkConsistency (void)
{
#if defined (UMBRA_DEBUG)
    unsigned int cnt = 0;
    if (hash)
    {
        if (hash == sHash)
        {
            UMBRA_ASSERT (size == MIN_SIZE);
            UMBRA_ASSERT (table == sTable);
        }

        for (unsigned int i = 0; i < size; i++)
        {
            UMBRA_ASSERT (hash[i] == NIL || (hash[i] >= 0 && hash[i] < (int)size));
            for (Handle h = hash[i]; h != NIL; h = table[h].next)
            {
                UMBRA_ASSERT (table[h].next == NIL || (table[h].next >= 0 && table[h].next < (int)size));
                cnt++;
            }
        }
    }

    UMBRA_ASSERT ((int)cnt == elemCount);

//  if (size)
//      UMBRA_ASSERT (size == getNextPowerOfTwo(size));
#endif
}

template <class S> inline unsigned int Set<S>::getHashVal   (const S& s, const unsigned int hashArraySize) const
{
    return getHashValue (s) & (hashArraySize-1);
}

template <class S> inline void Set<S>::rehash   (size_t newSize)
{
#ifdef UMBRA_DEBUG
    checkConsistency();
#endif
    if (newSize < MIN_SIZE)
        newSize = MIN_SIZE;

    UMBRA_ASSERT ((int)newSize >= elemCount);

    Entry*  newTable;
    Handle*     newHash;

    // If performing minimum size alloc then use
    // the arrays allocated originally along with
    // the set itself.

    if (newSize == MIN_SIZE)
    {
        UMBRA_ASSERT (hash != sHash);
        UMBRA_ASSERT (table != sTable);
        newTable = sTable;
        newHash  = sHash;
    } else
    {
        newTable = UMBRA_NEW_ARRAY(Entry, newSize);
        newHash  = UMBRA_NEW_ARRAY(Handle, newSize);
    }

    unsigned int i, cnt = 0;

    for (i = 0; i < newSize; i++)
        newHash[i] = NIL;

    for (i = 0; i < size; i++)                      // step through each old hash set
    for (Handle h = hash[i]; h != NIL; h = table[h].next)
    {
        unsigned int hVal   = getHashVal(table[h].s, (unsigned int)newSize);
        newTable[cnt].s     = table[h].s;
        newTable[cnt].next  = newHash[hVal];
        newHash[hVal]       = (int)cnt;
        cnt++;
    }
    UMBRA_ASSERT ((int)cnt == elemCount);

    for (i = cnt; i < newSize; i++)
        newTable[i].next = (int)(i+1);
    newTable[newSize-1].next = NIL;

    if (hash != sHash)
        UMBRA_DELETE_ARRAY(hash);
    if (table != sTable)
        UMBRA_DELETE_ARRAY(table);

    first   = (int)cnt;
    hash    = newHash;
    table   = newTable;
    size    = (unsigned int) newSize;
}

template <class S> inline void Set<S>::remove (const S& s)
{
    Handle  prev = NIL;
    unsigned int hval = getHashVal(s,size);
    Handle  h    = hash[hval];

    while (h != NIL)
    {
        if (table[h].s == s)
        {
            if (prev!=NIL)
                table[prev].next = table[h].next;
            else
                hash[hval]  = table[h].next;
            table[h].next   = first;
            first           = h;
            elemCount--;

            if (size > MIN_SIZE && (elemCount*3)<(int)size)
                rehash(size/2);
            return;
        }
        prev = h;
        h    = table[h].next;
    };
}

template <class S> inline S Set<S>::removeAny (void)
{
    int i = 0;
    while (hash[i] == NIL)
        i++;
    S ret = table[hash[i]].s;
    remove(ret);
    return ret;
}

template <class S> inline Set<S>::Set (Allocator* a): Base(a),size(MIN_SIZE),hash(sHash),table(sTable),first(0),elemCount(0)
{
    for (unsigned int i = 0; i < size; i++)
    {
        hash[i] = NIL;
        table[i].next = (int)(i+1);
    }
    table[size-1].next = NIL;
}

template <class S> inline Set<S>::Set (const S* src, int cnt, Allocator* a): Base(a),size(0),hash(NULL),table(NULL),first(NIL),elemCount(0)
{
    rehash ((size_t)getNextPowerOfTwo((unsigned int)cnt));

    if (src)
    for (int i = 0; i < cnt; i++)
        insert(src[i]);
}

template <class S> inline Set<S>::Set (const Set<S>& src) : Base(src.getAllocator()),size(0),hash(NULL),table(NULL),first(NIL),elemCount(0)
{
    *this = src;
}

template <class S> inline Set<S>::~Set (void)
{
    // check consistency (in debug build only)
#ifdef UMBRA_DEBUG
        checkConsistency();
#endif

    // Delete hash/table only if they're not pointed to sHash and sTable.
    // Because the sHash/sTable selection is synched we only need to test
    // for one case.

    if (hash != sHash && hash != 0)
    {
        UMBRA_ASSERT (table != sTable && table != 0);
        UMBRA_DELETE_ARRAY(hash);
        UMBRA_DELETE_ARRAY(table);
    }
}

template <class S> inline Set<S>& Set<S>::operator= (const Set<S>& src)
{
    UMBRA_ASSERT (&src);

    if (&src != this)
    {
        // Performs a direct copy since we use indices
        // rather than pointers in the linked lists...
        // Quite a bit faster than inserting stuff
        // to the hash table.

        cleanup();

        size        = src.size;
        elemCount   = src.elemCount;
        first       = src.first;

        if (size > MIN_SIZE)
        {
            Handle* newHash  = UMBRA_NEW_ARRAY(Handle, size);
            Entry*  newTable = UMBRA_NEW_ARRAY(Entry, size);
            hash  = newHash;
            table = newTable;
        }
        else
        {
            UMBRA_ASSERT (size == MIN_SIZE);
            hash    = sHash;
            table   = sTable;
        }

        std::memcpy (hash,   src.hash,   size*sizeof(Handle));

        for (unsigned i = 0; i < size; i++)
            table[i] = src.table[i];
    }

    return *this;
}

template <class S> inline bool Set<S>::operator== (const Set<S>& src) const
{
    if (elemCount != src.elemCount)
        return false;

    for (unsigned int i = 0; i < src.size; i++)
    for (Handle  h = src.hash[i]; h != NIL; h = src.table[h].next)
        if (!contains(src.table[h].s))
            return false;
    return true;
}

template <class S> inline Set<S>& Set<S>::operator&= (const Set<S>& src)
{
    // this could probably be optimized

//  typename Set<S>::Array a(*this);

    Array<S> a;

    getArray(a, true);

    int i,cnt = a.getSize();
    for (i = 0; i < cnt; i++)
        if (!src.contains(a[i]))
            remove(a[i]);
    return *this;
}

template <class S> inline Set<S>& Set<S>::operator|= (const Set<S>& src)
{
    for (unsigned int i = 0; i < src.size; i++)
    for (Handle h = src.hash[i]; h != NIL; h = src.table[h].next)
        insert(src.table[h].s);
    return *this;
}

template <class S> inline Set<S>& Set<S>::operator-= (const Set<S>& src)
{
    if (isEmpty() || src.isEmpty())
        return *this;

    for (unsigned int i = 0; i < src.size; i++)
    for (Handle h = src.hash[i]; h != NIL; h = src.table[h].next)
        remove(src.table[h].s);
    return *this;
}

template <class S> inline bool Set<S>::contains(const Set<S>& src) const
{
    if (src.elemCount > elemCount)          // source is larger than this
        return false;

    for (unsigned int i = 0; i < src.size; i++)
    for (Handle h = src.hash[i]; h != NIL; h = src.table[h].next)
        if (!contains(src.table[h].s))
            return false;
    return true;
}

template <class S> inline bool Set<S>::contains (const S& s) const
{
    Handle  h = hash[getHashVal(s,size)];
    while (h!=NIL)
    {
        if (table[h].s == s)
            return true;
        h = table[h].next;
    };
    return false;
}

template <class S> inline bool Set<S>::intersects (const Set<S>& s) const
{
    for (unsigned int i = 0; i < size; i++)
    for (Handle h = hash[i]; h != NIL; h = table[h].next)
    {
        if (s.contains(table[h].s))
            return true;
    }
    return false;
}

template <class S> inline const S* Set<S>::get(const S& s) const
{
    Handle  h = hash[getHashVal(s,size)];
    while (h!=NIL)
    {
        if (table[h].s == s)
            return &table[h].s;
        h = table[h].next;
    };
    return NULL;
}

template <class S> inline bool Set<S>::insert (const S& s)
{
    if (first == NIL)                   // need to re-alloc and re-hash tables
        rehash((size_t)(size*2));

    Handle  h;
    unsigned int    hval    = getHashVal(s,size);

    UMBRA_ASSERT (hash);
    for (h = hash[hval]; h!=NIL; h = table[h].next)
    if (table[h].s == s)                            // already there
        return true;
    h               = first;
    first           = table[first].next;
    table[h].s      = s;
    table[h].next   = hash[hval];
    hash[hval]      = h;
    elemCount++;
    return false;
}

template <class S> inline void Set<S>::removeAll (bool minimizeMemoryUsage)
{
    if (minimizeMemoryUsage)                // free all allocs
    {
        cleanup ();
        rehash  (MIN_SIZE);
    } else
    {
        unsigned int i;

        for (i = 0; i < size; i++)          // clear hash table
        {
            hash[i]         = NIL;
            table[i].next   = i+1;
        }

        if (size)
            table[size-1].next = NIL;

        first       = 0;
        elemCount   = 0;
    }

#ifdef UMBRA_DEBUG
    checkConsistency();
#endif
}

template <class S> inline void Set<S>::cleanup (void)
{
    // check consistency (in debug build only)
#ifdef UMBRA_DEBUG
    checkConsistency();
#endif

    // Delete hash/table only if they're not pointed to sHash and sTable.
    // Deleting NULL pointers is OK in C++.

    if (hash != sHash)
        UMBRA_DELETE_ARRAY(hash);
    if (table != sTable)
        UMBRA_DELETE_ARRAY(table);
    hash        = 0;
    table       = 0;
    size        = 0;
    elemCount   = 0;
    first       = NIL;
}

} // namespace Umbra

#endif // UMBRASET_HPP

//--------------------------------------------------------------------
