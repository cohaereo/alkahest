#ifndef UMBRASORT_HPP
#define UMBRASORT_HPP

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
 * \brief   Umbra sort routines
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{
//------------------------------------------------------------------------
// Prototypes for sort functions. Note that class T must have
// operators =, > and < in order for the functions to compile.
//------------------------------------------------------------------------

template <class T> UMBRA_FORCE_INLINE void insertionSort(T* a, int N);
template <class T> inline void quickSort(T* a, int N);

//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class T> UMBRA_FORCE_INLINE bool isSorted(const T* a, int num)
{
    UMBRA_ASSERT(a && num >= 0);
    for (int i = 1; i < num; i++)
    if (a[i-1] > a[i])
        return false;
    return true;
}

template <class T> void insertionSort (T* a, int N)
{
    UMBRA_ASSERT(a && N >= 0);
    for (int i = 1; i < N; i++)
    if (a[i-1] > a[i])
    {
        T       v(a[i]);
        int     j   = i;
        while (a[j-1]> v)
        {
            a[j] = a[j-1];                      // copy data
            j--;
            if (!j)
                break;
        };
        a[j]    = v;
    }

    UMBRA_ASSERT(isSorted(a,N));                // make sure that the data is sorted
}

template <class T> UMBRA_FORCE_INLINE int median3 (T* elements, int low, int high)
{
    UMBRA_ASSERT(low >= 0 && high >= 2);
    int l = low;
    int c = (unsigned int)(high + low)>>1;
    int h = high-2;                                         // DEBUG DEBUG requires switchpoint bigger than 2 !!

    UMBRA_ASSERT (h >= 0);

    if(elements[l] > elements[h]) swap(l, h);
    if(elements[l] > elements[c]) swap(l, c);
    if(elements[c] > elements[h]) swap(c, h);

    UMBRA_ASSERT(!(elements[l] > elements[c]));
    UMBRA_ASSERT(!(elements[c] > elements[h]));

    return c;
}

// Note: we made it non-inline because some idiot compilers
// barf when using recursive inline functions!!
// The quickSort() here used Median3 partitioning.

template <class T> void quickSort (T* elements, int low, int high)
{
    //--------------------------------------------------------------------
    // Reached cut-off point --> switch to insertionSort..
    //--------------------------------------------------------------------

    const int SWITCHPOINT = 15;                             // optimal value, see paper :)

    if((high - low) <= SWITCHPOINT)
    {
        insertionSort(&elements[low], high - low);
        return;
    }


    //--------------------------------------------------------------------
    // Select pivot using median-3
    //--------------------------------------------------------------------

    int pivotIndex  = median3(elements, low, high);

    UMBRA_ASSERT(high >= 1);
    UMBRA_ASSERT(pivotIndex >= 0);

    swap(elements[high-1], elements[pivotIndex]);           // hide pivot to highest entry

    T pivot(elements[high-1]);

    //--------------------------------------------------------------------
    // Partition data
    //--------------------------------------------------------------------

    int i = low  - 1;
    int j = high - 1;

    while (i < j)
    {
        do { i++; } while(elements[i] < pivot);
        do { j--; } while(elements[j] > pivot);

        UMBRA_ASSERT(i>=low && j>=low && i < high && j < high);

        swap(elements[i], elements[j]);
    }

    //--------------------------------------------------------------------
    // Restore pivot
    //--------------------------------------------------------------------

    T tmp(elements[j]);
    elements[j] = elements[i];
    elements[i] = elements[high-1];
    elements[high-1] = tmp;

    //--------------------------------------------------------------------
    // sort sub-partitions
    //--------------------------------------------------------------------

    if((i - low) > 1)       quickSort(elements, low, i);
    if((high - (i+1)) > 1)  quickSort(elements, i+1, high);
}

// wrapper function (also validates itself in debug build)
template <class T> inline void quickSort (T* a, int N)
{
    quickSort(a, 0, N);
    UMBRA_ASSERT(isSorted(a, N));       // make sure that the data is sorted
}

template<typename T>
inline int binarySearch(const T& value, const T* array, int n)
{
    int s = 0, e = n;

    while (s < e)
    {
        int i = (s+e)/2;
        if (array[i] == value)
            return i;
        else if (array[i] < value)
            s = i+1;
        else
        {
            UMBRA_ASSERT(array[i] > value);
            e = i;
        }
    }

    return -1;
}

} // namespace Umbra

#endif // UMBRASORT_HPP

//--------------------------------------------------------------------
