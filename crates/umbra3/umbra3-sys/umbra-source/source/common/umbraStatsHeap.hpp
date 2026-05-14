#pragma once
#ifndef UMBRASTATSHEAP_HPP
#define UMBRASTATSHEAP_HPP

#include "umbraMemory.hpp"

namespace Umbra
{
// A multi-purpose statistics heap, use as alloc filter

class StatsHeapState;

class StatsHeap: public Allocator
{
public:
    StatsHeap (void): m_state(NULL), m_forward(NULL) {}
    ~StatsHeap (void);

    virtual void*   allocate     (size_t size, const char* info = NULL);
    virtual void    deallocate   (void* ptr);
    void            setAllocator (Allocator* a) { m_forward = a; }
    Allocator*      getAllocator (void) { return m_forward; }
    void            dump         (Logger* logger = NULL);
    void            reset        (void);
    void            dumpAccumulated (Logger* logger = NULL, Logger::Level level = Logger::LEVEL_INFO);

    int             getNumAllocations (void) const;
    size_t          getAllocatedBytes (void) const;
    int             getPeakNumAllocations (void) const;
    size_t          getPeakAllocatedBytes (void) const;

private:

    StatsHeapState* m_state;
    Allocator*      m_forward;
};

} // namespace Umbra

#endif // UMBRASTATSHEAP_HPP
