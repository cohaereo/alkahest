/*=========================================================================
	Copyright (C) 2013 Umbra Software. All rights reserved.
=========================================================================*/

#pragma once
#include "umbraPrivateDefs.hpp"
#include "umbraArray.hpp"
#include "umbraSet.hpp"
#include "umbraHash.hpp"
#include "umbraProgress.hpp"

namespace Umbra
{

/*---------------------------------------------------------------*//*!
 * \brief   Combines index lists, finding overlapping segments
 *
 * \param   Id  Index list identifier
 *//*---------------------------------------------------------------*/

template<typename Id>
class IndexListCombiner : public BuilderBase
{
public:

    IndexListCombiner(BuildContext* ctx)
        : BuilderBase(ctx), 
        m_lists(ctx->getPlatform().allocator),
        m_result(ctx->getPlatform().allocator),
        m_resultOutputIndices(ctx->getPlatform().allocator),
        m_idStart(ctx->getPlatform().allocator),
        m_output(ctx->getPlatform().allocator),
        m_b(ctx->getPlatform().allocator),
        m_resultValid(false),
        m_progress(ctx->getPlatform().allocator)
    {}

    ~IndexListCombiner() { clear(); }

    const Array<int>& getOutput(void) { return m_output; }

    inline void getRange        (const Id& id, int& idx, int& count);
    inline void insert          (const int* indices, int N, const Id& id);
    inline void combineSimple   (void);
    inline void combineRanged   (Progress* parent);
    inline void clear           (void);

private:

    struct IndexList;

    /*---------------------------------------------------------------*//*!
     * \brief   A set of indices. Indices inside a SubList don't have
     *          assigned order.
     *
     *          The indices are stored in ascending order in order to
     *          make merge operations easier.
     *
     *//*---------------------------------------------------------------*/

    struct SubList
    {
        SubList(Allocator* a = NULL)
            : indices(a),
            ids(a)
        {}

        int                 getSize         (void) const { return indices.getSize(); }
        int                 getMin          (void) const { UMBRA_ASSERT(getSize()); return indices[0]; }
        int                 getMax          (void) const { UMBRA_ASSERT(getSize()); return indices[indices.getSize() - 1]; }

        const Array<int>&   getElements     (void) const { return indices; }
        const Set<Id>&      getIds          (void) const { return ids; }

        // Appends an index, expects ascending order
        void                insertAscending (int idx)    { UMBRA_ASSERT(!getSize() || indices[getSize() - 1] < idx); indices.pushBack(idx); }
        void                resize          (int size)   { indices.resize(size); }

        void                swapElements    (int idxA, int idxB)
        {
            swap2(indices[idxA], indices[idxB]);
        }

        void                addIds          (const Set<Id>& newIds)
        {
            ids |= newIds;
        }

        void clear()
        {
            indices.clear();
            ids.removeAll(false);
        }

        SubList& operator=(const SubList& other)
        {
            if (&other != this)
            {
                indices.resize(other.indices.getSize());
                memcpy(indices.getPtr(), other.indices.getPtr(), sizeof(int) * indices.getSize());
                ids = other.ids;
            }
            return *this;
        }

    private:
        friend struct IndexList;

        Array<int>          indices;
        Set<Id>             ids;
    };

    /*---------------------------------------------------------------*//*!
     * \brief   A set of SubLists. Sublists inside an IndexList have
     *          a set order.
     *//*---------------------------------------------------------------*/

    struct IndexList
    {
        IndexList(Allocator* a = NULL) : sublists(a), minValue(INT_MAX), maxValue(INT_MIN), elemCount(0) {}

        Allocator*                  getAllocator    (void) const    { return sublists.getAllocator(); }
        const Array<SubList*>&      getSubLists     (void) const    { return sublists; }
        SubList&                    getSubList      (int idx)       { UMBRA_ASSERT(sublists[idx]); return *sublists[idx]; }
        const SubList&              getSubList      (int idx) const { UMBRA_ASSERT(sublists[idx]); return *sublists[idx]; }
        int                         getSize         (void) const    { return sublists.getSize(); }
        int                         getTotalSize    (void) const    { return elemCount; }
        int                         getMin          (void) const    { UMBRA_ASSERT(getSize()); return minValue; }
        int                         getMax          (void) const    { UMBRA_ASSERT(getSize()); return maxValue; }

        IndexList& operator=(const IndexList& other)
        {
            if (&other != this)
            {
                for (int i = 0; i < sublists.getSize(); i++)
                    UMBRA_DELETE(sublists[i]);
                sublists.clear();
                for (int i = 0; i < other.getSize(); i++)
                    sublists.pushBack(UMBRA_NEW(SubList, other.getSubList(i)));
                minValue   = other.minValue;
                maxValue   = other.maxValue;
                elemCount  = other.elemCount;
            }
            return *this;
        }

        ~IndexList()
        {
            for (int i = 0; i < sublists.getSize(); i++)
                UMBRA_DELETE(sublists[i]);
            sublists.clear();
        }
        
        void swapSubLists (int idxA, int idxB)
        {
            swap2(sublists[idxA], sublists[idxB]);
        }

        void clear()
        {
            minValue = INT_MAX;
            maxValue = INT_MIN;
            elemCount = 0;
            for (int i = 0; i < sublists.getSize(); i++)
                UMBRA_DELETE(sublists[i]);
            sublists.clear();
        }

        void print() const
        {
            for (int i = 0; i < sublists.getSize(); i++)
            {
                for (int j = 0; j < sublists[i]->indices.getSize(); j++)
                    printf("%02d ", sublists[i]->indices[j]);
                if (i != sublists.getSize() - 1)
                    printf("| ");
            }
            printf("\n");
        }

        void insertMiddle(int idx, SubList* set)
        {
            int oldSize = sublists.getSize();
            sublists.resize(oldSize + 1);
            memmove(sublists.getPtr() + idx + 1, sublists.getPtr() + idx, (oldSize - idx) * sizeof(SubList*));
            sublists[idx] = set;
        }

        void append(const IndexList& set)
        {
            for (int i = 0; i < set.getSize(); i++)
                sublists.pushBack(UMBRA_NEW(SubList, set.getSubList(i)));
            elemCount += set.elemCount;
            minValue = min2(minValue, set.getMin());
            maxValue = max2(maxValue, set.getMax());
        }

        void append(const SubList& set)
        {
            if (!set.getSize())
                return;
            sublists.pushBack(UMBRA_NEW(SubList, set));
            elemCount += set.getSize();
            minValue = min2(minValue, set.getMin());
            maxValue = max2(maxValue, set.getMax());
        }

        void append(const int* indices, int N, const Id& id)
        {
            UMBRA_ASSERT(N > 0);
            SubList* p = UMBRA_NEW(SubList, getAllocator());
            p->indices.append(indices, N);
            p->ids.insert(id);

            quickSort(p->indices.getPtr(), N);
            sublists.pushBack(p);

            elemCount += N;
            minValue = min2(minValue, p->getMin());
            maxValue = max2(maxValue, p->getMax());
        }

    private:
        IndexList(const IndexList&);

        Array<SubList*>       sublists;
        int                   minValue;
        int                   maxValue;
        int                   elemCount;
    };

    /*---------------------------------------------------------------*//*!
     * \brief   Sortable list of IndexLists by element count.
     *//*---------------------------------------------------------------*/

    struct SortableList
    {
        SortableList() : list(NULL) {}
        IndexList* list;

        // Comparison operators for sorting
        bool operator< (const SortableList& other) const
        {
            UMBRA_ASSERT(list);
            return list->getTotalSize() > other.list->getTotalSize();
        }
        bool operator> (const SortableList& other) const
        {
            UMBRA_ASSERT(list);
            return list->getTotalSize() < other.list->getTotalSize();
        }
    };

    inline void intersectSplit  (SubList&       subListB,
                                 IndexList&     listA,
                                 int&           idxA,
                                 int&           complementASize,
                                 int&           intersectionSize) const;

    inline void intersect    (const SubList&   subListA,
                              SubList&         subListB,
                              SubList&         complementA,
                              SubList&         intersection) const;

    inline void intersect    (const SubList&   subListA,
                              SubList&         subListB,
                              int&             complementA,
                              int&             intersection) const;

    inline void mergeMeasure (const IndexList& listA,
                              const IndexList& listB,
                              int              aStart,
                              int&             overlap,
                              int&             minSet);

    inline bool mergeFrom    (IndexList&       listA,
                              const IndexList& listB,
                              int              startA) const;

    inline void mergeMeasure (const IndexList& listA,
                              const IndexList& listB,
                              int&             bestOverlap,
                              int&             bestPosition);

    inline void merge        (IndexList&       listA,
                              const IndexList& listB);

    inline void mergeInputs  (void);

    Array<IndexList*>         m_lists;
    IndexList                 m_result;
    Array<int>                m_resultOutputIndices;
    Hash<Id, int>             m_idStart;
    Array<int>                m_output;
    SubList                   m_b;
    bool                      m_resultValid;
    Progress                  m_progress;
};

/*---------------------------------------------------------------*//*!
 * \brief Reset combiner
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::clear (void)
{
    for (int i = 0; i < m_lists.getSize(); i++)
        UMBRA_DELETE(m_lists[i]);
    m_lists.clear();
    m_result.clear();
    m_resultOutputIndices.clear();
    m_idStart.clear();
    m_output.clear();
}

/*---------------------------------------------------------------*//*!
 * \brief Get final range for an input list
 *
 * \param id        Input list identifier
 * \param idx       Index in output array
 * \param count     Number of elements
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::getRange(const Id& id, int& idx, int& count)
{
    UMBRA_ASSERT(m_resultValid);

    idx   = 0;
    count = 0;

    if (!m_result.getSize())
        return;

    int* start = m_idStart.get(id);
    if (!start)
        return;
    int curList = *start;
    idx = m_resultOutputIndices[curList];

    while(curList < m_result.getSize() && m_result.getSubList(curList).getIds().contains(id))
    {
        count += m_result.getSubList(curList).getSize();
        curList++;
    }
}

/*---------------------------------------------------------------*//*!
 * \brief Insert an index list
 *
 * \param indices   Array of indices
 * \param N         Number of indices in array
 * \param id        Identifier for these indices
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::insert(const int* indices, int N, const Id& id)
{
    if (N)
    {
        UMBRA_ASSERT(indices);
        IndexList* list = UMBRA_NEW(IndexList, getAllocator());
        list->append(indices, N, id);
        m_lists.pushBack(list);
        m_resultValid = false;
    }
}

/*---------------------------------------------------------------*//*!
 * \brief Outputs result as simple unpacked array.
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::combineSimple(void)
{
    m_progress.reset();
    m_progress.addPhase(100.f, "merge");
    m_progress.start(NULL, 0, 1.f);

    m_resultValid = true;
    mergeInputs();

    for (int i = 0; i < m_lists.getSize(); i++)
        UMBRA_DELETE(m_lists[i]);
    m_lists.clear();

    m_resultOutputIndices.clear();
    m_output.clear();

    for (int j = 0; j < m_result.getSize(); j++)
    {
        m_resultOutputIndices.pushBack(m_output.getSize());
        m_output.append(m_result.getSubList(j).getElements());
    }
}

/*---------------------------------------------------------------*//*!
 * \brief Outputs result as start element - count pairs
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::combineRanged(Progress* parent)
{
    m_progress.reset();
    m_progress.addPhase(100.f, "merge");
    m_progress.start(NULL, 0, 1.f, parent);

    m_resultValid = true;
    mergeInputs();

    m_output.clear();
    for (int j = 0; j < m_result.getSize(); j++)
    {
        m_resultOutputIndices.pushBack(m_output.getSize() / 2);

        int k = 0;
        while (k < m_result.getSubList(j).getSize())
        {
            m_output.pushBack(m_result.getSubList(j).getElements()[k]);
            int prev = m_result.getSubList(j).getElements()[k];
            int c = 1;
            k++;
            while (k < m_result.getSubList(j).getSize() && m_result.getSubList(j).getElements()[k] == prev + 1)
            {
                prev = m_result.getSubList(j).getElements()[k];
                k++;
                c++;
                if (c == (1 << 5) - 1)
                    break;
            }
            m_output.pushBack(c);
        }            
    }
}

/*---------------------------------------------------------------*//*!
 * \brief   Merge current set of inputs.
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::mergeInputs()
{
    m_result.clear();
    m_idStart.clear();

#if 1
    m_progress.nextPhase();

    if (m_lists.getSize())
    {
        Array<SortableList> sortables(m_lists.getSize(), getAllocator());
        for (int i = 0; i < m_lists.getSize(); i++)
            sortables[i].list = m_lists[i];

        // Sort by size
        quickSort(sortables.getPtr(), sortables.getSize());

        // Biggest first
        UMBRA_ASSERT(sortables[0].list);
        m_result = *sortables[0].list;
        for (int i = 1; i < sortables.getSize(); i++)
        {
            m_progress.setPhaseProgress((float)i / (float)sortables.getSize());
            UMBRA_ASSERT(sortables[i].list);
            merge(m_result, *sortables[i].list);
        }
    }

    m_progress.setPhaseProgress(1.f);
#else
    Set<int> lists(getAllocator());
    for (int i = 0; i < m_lists.getSize() - 1; i++)
        lists.insert(i);

    quickSort(m_lists.getPtr(), m_lists.getSize());
    m_result = m_lists[m_lists.getSize() - 1];

    while (lists.getSize())
    {
        //LOGD("%d", lists.getSize());
        int bestOverlapAll = INT_MIN;
        int bestIdx = -1;

        Set<int>::Iterator it = lists.iterate();
        while(it.next())
        {
            int i = it.getValue();

            int bestOverlap, bestPosition;
            mergeMeasure(m_result, m_lists[i], bestOverlap, bestPosition);

            if (bestOverlap > bestOverlapAll)
            {
                bestOverlapAll = bestOverlap;
                bestIdx = i;
            }            
        }

        if (bestIdx < 0)
        {
            for (int i = 0; i < m_lists.getSize(); i++)
            {
                if (lists.contains(i))
                    continue;
                IndexList oldResult = m_result;
                merge(m_result, oldResult, m_lists[i]);
            }

            break;
        }

        IndexList oldResult = m_result;
        merge(m_result, oldResult, m_lists[bestIdx]);
        lists.remove(bestIdx);
    }
#endif

    for (int i = 0; i < m_result.getSize(); i++)
    {
        typename Set<Id>::Iterator it = m_result.getSubList(i).getIds().iterate();
        while(it.next())
        {
            const Id& id = it.getValue();
            if (!m_idStart.contains(id))
                m_idStart.insert(id, i);
        }
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::intersectSplit  (SubList&       subListB,
                                                    IndexList&     listA,
                                                    int&           idxA,
                                                    int&           complementASize,
                                                    int&           intersectionSize) const
{
    SubList  complementA(getAllocator());
    SubList* intersection = UMBRA_NEW(SubList, getAllocator());

    intersect(listA.getSubList(idxA), subListB, complementA, *intersection);

    complementASize  = complementA.getSize();
    intersectionSize = intersection->getSize();

    if (!intersectionSize)
    {
        UMBRA_DELETE(intersection);
        return;
    }

    if (complementA.getSize())
    {
        listA.getSubList(idxA++) = complementA;
        listA.insertMiddle(idxA, intersection);
    } else
    {
        listA.getSubList(idxA) = *intersection;
        UMBRA_DELETE(intersection);
    }
}

/*---------------------------------------------------------------*//*!
 * \brief   Intersects two sublists, A and B
 *
 *          Produces:
 *          - intersection (the common part)
 *          - B's complement in A (subListB)
 *          - A's complement in B (complementB)
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::intersect  (const SubList& subListA,
                                               SubList&       subListB,
                                               SubList&       complementA,
                                               SubList&       intersection) const
{
    complementA.clear();
    intersection.clear();
    //complementB.clear();

    complementA.addIds(subListA.getIds());
    //complementB.addIds(subListB.getIds());
    intersection.addIds(subListA.getIds());
    intersection.addIds(subListB.getIds());

    int i = 0, j = 0;
    int bNext = 0;
    while(i < subListA.getSize() && j < subListB.getSize())
    {
        int aValue = subListA.getElements()[i];
        int bValue = subListB.getElements()[j];
        if (aValue < bValue)
        {
            complementA.insertAscending(aValue);
            i++;
        } else
        if (bValue < aValue)
        {
            subListB.swapElements(j, bNext);
            bNext++;
            j++;
        } else
        if (aValue == bValue)
        {
            intersection.insertAscending(bValue);
            i++;
            j++;
        }
    };

    for (; i < subListA.getSize(); i++)
        complementA.insertAscending(subListA.getElements()[i]);
    for (; j < subListB.getSize(); j++)
    {
        subListB.swapElements(j, bNext);
        bNext++;
    }
    subListB.resize(bNext);
}

/*---------------------------------------------------------------*//*!
 * \brief   Intersects two sublists, A and B
 *
 *          Produces:
 *          - size of intersection (the common part)
 *          - size of B's complement in A (complementA)
 *          - A's complement in B (subListB)
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::intersect  (const SubList& subListA,
                                               SubList&       subListB,
                                               int&           complementA,
                                               int&           intersection) const
{
    complementA = 0;
    intersection = 0;

    int i = 0, j = 0;
    int bNext = 0;

    while(i < subListA.getSize() && j < subListB.getSize())
    {
        int aValue = subListA.getElements()[i];
        int bValue = subListB.getElements()[j];
        if (aValue < bValue)
        {
            complementA++;
            i++;
        } else
        if (bValue < aValue)
        {
            subListB.swapElements(j, bNext);
            bNext++;
            j++;
        } else
        if (aValue == bValue)
        {
            intersection++;
            i++;
            j++;
        }
    };

    complementA += subListA.getSize() - i;
    for (; j < subListB.getSize(); j++)
    {
        subListB.swapElements(j, bNext);
        bNext++;
    }
    subListB.resize(bNext);
}

/*---------------------------------------------------------------*//*!
 * \brief  Merge B into A, measuring the result this operation
 *         would produce
 *
 * \param  listA    List to merge into
 * \param  listB    List to merge
 * \param  startA   A's first position
 * \param  overlap  Number of overlapping elements, -1 on failure
 * \param  minSet   Smallest set size created by this operation
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::mergeMeasure (const IndexList& listA,
                                                 const IndexList& listB,
                                                 int              aStart,
                                                 int&             overlap,
                                                 int&             minSet)
{
    overlap  = 0;
    minSet   = INT_MAX;

    int aIdx = aStart;
    int bIdx = 0;

    if (listB.getSubList(0).getMax() < listA.getSubList(aStart).getMin() || listB.getSubList(0).getMin() > listA.getSubList(aStart).getMax())
    {
        overlap = -1;
        return;
    }

    // Start iterating B from zero
    m_b = listB.getSubList(bIdx);
    int bMin = m_b.getMin();
    int bMax = m_b.getMax();

    // Iterate A's sublists starting from startA
    bool first = true;
    while (aIdx < listA.getSize())
    {
        const SubList& a = listA.getSubList(aIdx);

        // fast rejection
        if (bMin > a.getMax() || bMax < a.getMin())  // ranges don't overlap
        {
            overlap = -1;
            return;
        }

        // Intersect A and B
        int complementA = 0, intersection = 0;
        intersect(a, m_b, complementA, intersection);

        if (m_b.getSize())
        {
            bMin = m_b.getMin();
            bMax = m_b.getMax();
        }

        // Can't merge, no intersection
        if (!intersection)
        {
            if (aStart == 0 && aIdx == 0)
            {
                // Append lists without overlap
                minSet  = min2(listA.getTotalSize(), listB.getTotalSize());
                overlap = 0;
            } else
                overlap = -1;
            return;
        }

        bool compAEmpty = complementA == 0;
        bool lastB      = bIdx == listB.getSize() - 1;

        // A can leave a remainder (complement) only before or after merge
        if (!compAEmpty && !first && (m_b.getSize() || !lastB))
        {
            overlap = -1;
            return;
        }

        // Measure
        overlap += intersection;
        if (complementA)
            minSet = min2(minSet, complementA);
        minSet = min2(minSet, intersection);

        // Get next B, if B empty
        if (!m_b.getSize())
        {
            bIdx++;
            if (bIdx >= listB.getSize())
            {
                aIdx++;
                break;
            }
            m_b = listB.getSubList(bIdx);
            bMin = m_b.getMin();
            bMax = m_b.getMax();
        }

        first = false;
        aIdx++;
    }

    if (m_b.getSize())
        minSet = min2(minSet, m_b.getSize());
}

/*---------------------------------------------------------------*//*!
 * \brief  Merge B into A, starting from a's position startA.
 *         Note: we already know the merge will succeed.
 *
 * \param  listA    List to merge into (receives result)
 * \param  listB    List to merge
 * \param  startA   A's first position
 * \return          True on success
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline bool IndexListCombiner<Id>::mergeFrom (IndexList&        listA,
                                              const IndexList&  listB,
                                              int               startA) const
{
    SubList b(getAllocator());
    SubList complementA(getAllocator());
    SubList intersection(getAllocator());
    SubList complementB(getAllocator());

    int aIdx = startA;

    bool first = true;

    int bIdx = 0;
    while(aIdx < 0)
    {
        SubList* newList = UMBRA_NEW(SubList, getAllocator());
        *newList = listB.getSubList(bIdx);
        listA.insertMiddle(0, newList);
        bIdx++;
        aIdx++;
        first = false;
    }

    if (startA < 0)
        aIdx -= startA;

    // Start B from zero
    b = listB.getSubList(bIdx);

    // Iterate A's sublists starting from startA
    while (aIdx < listA.getSize())
    {
        const SubList& a = listA.getSubList(aIdx);

        // fast rejection
        if ((b.getMax() < a.getMin() || b.getMin() > a.getMax()))  // ranges don't overlap
        {
            UMBRA_ASSERT(false);
            return false;
        }

        int complementASize = 0;
        int intersectionSize = 0;

        // Intersect A and B
        // Places b's remainder into b.
        intersectSplit(b, listA, aIdx, complementASize, intersectionSize);

        // Can't merge, no intersection
        if (!intersectionSize)
        {
            // Append lists without overlap
            if (startA == 0 && aIdx == 0)
            {
                for (bIdx = 0; bIdx < listB.getSize(); bIdx++)
                    listA.append(listB.getSubList(bIdx));
                return true;
            }

            UMBRA_ASSERT(false);
            return false;
        }

        bool lastB = bIdx == listB.getSize() - 1;

        // A can leave a remainder (complement) only before or after merge
        if (complementASize != 0 && !first && (complementB.getSize() || !lastB))
        {
            UMBRA_ASSERT(false);
            return false;
        }

        // Get next B, if B empty
        if (!b.getSize())
        {
            bIdx++;
            if (bIdx >= listB.getSize())
            {
                // Merge finished, insert "after merge" A remainder
                if (complementASize)
                    listA.swapSubLists(aIdx-1, aIdx);
                aIdx++;
                break;
            }
            b = listB.getSubList(bIdx);
        }

        first = false;
        aIdx++;
    }

    // We must be out of A, B or both at this point
    UMBRA_ASSERT((!b.getSize() && !(listB.getSize() - bIdx)) || !(listA.getSize() - aIdx));

    // Insert remainder from B (if exists)
    if (b.getSize())
        listA.append(b);
    bIdx++;
    for (; bIdx < listB.getSize(); bIdx++)
        listA.append(listB.getSubList(bIdx));

    return true;
}

/*---------------------------------------------------------------*//*!
 * \brief  Find best starting position for B's merge into A
 *
 * \param  listA        List to merge into
 * \param  listB        List to merge
 * \param  bestOverlap  overlap
 * \param  bestPosition Best starting position
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::mergeMeasure (const IndexList& listA,
                                                 const IndexList& listB,
                                                 int&             bestOverlap,
                                                 int&             bestPosition)
{
    bestPosition = INT_MIN;
    bestOverlap  = INT_MIN;

    // fast rejection
    if (listB.getMax() <  listA.getMin()  || listB.getMin() > listA.getMax())   // ranges don't overlap
        return;

    // Best minimal set size
    int setSize = INT_MIN;

    // B's position relative to A
    for (int aStart = -(listB.getSize() - 1); aStart < listA.getSize(); aStart++)
    {
        int overlap = 0;
        int minSetSize = 0;
        if (aStart < 0)
            mergeMeasure(listB, listA, -aStart, overlap, minSetSize);
        else
            mergeMeasure(listA, listB,  aStart, overlap, minSetSize);

        // negative overlap indicates failure
        if (overlap < 0)
            continue;

        // Select best overlap, or biggest minimal set size
        if (overlap > bestOverlap || (overlap == bestOverlap && minSetSize > setSize))
        {
            bestPosition = aStart;
            bestOverlap  = overlap;
            setSize      = minSetSize;

            //if (overlap == listB.getTotalSize())
            if (overlap > 0)
                return;
        }
    }
}
/*---------------------------------------------------------------*//*!
 * \brief  Merge IndexList B into A
 *
 * \param  result       Merge result
 * \param  listA        List to merge into
 * \param  listB        List to merge
 *
 *//*---------------------------------------------------------------*/

template<typename Id>
inline void IndexListCombiner<Id>::merge (IndexList&       listA,
                                          const IndexList& listB)
{
    // Find best merge position
    int bestOverlap, bestPosition;
    mergeMeasure(listA, listB, bestOverlap, bestPosition);

    if (bestOverlap < 0)
    {
        // Simple append on failure
        for (int i = 0; i < listB.getSize(); i++)
            listA.append(listB.getSubList(i));
    } else
    {
        // Perform the merge
        if (bestPosition < 0)
        {
            IndexList temp(getAllocator());
            temp = listB;
            mergeFrom(temp, listA, -bestPosition);
            listA = temp;
        } else
            mergeFrom(listA, listB, bestPosition);
    }

#if 0
    Set<int> setR(getAllocator());

    for (int i = 0; i < result.getSize(); i++)
    for (int j = 0; j < result.getSubList(i).getSize(); j++)
        setR.insert(result.getSubList(i).getElements()[j]);

    for (int x = 0; x < 2; x++)
    {
        const IndexList& list = (x == 0) ? listA : listB;
        for (int i = 0; i < list.getSize(); i++)
        for (int j = 0; j < list.getSubList(i).getSize(); j++)
        {
            UMBRA_ASSERT(setR.contains(list.getSubList(i).getElements()[j]));
        }
    }
#endif
}

} // namespace Umbra

//------------------------------------------------------------------------
