#include "umbraStatsHeap.hpp"
#include "umbraString.hpp"
#include "umbraThread.hpp"
#include "umbraHash.hpp"
#include "umbraLogger.hpp"

namespace Umbra
{
    int g_assertOnMemoryLeaks = 1;
}

#if (UMBRA_OS == UMBRA_WINDOWS) && UMBRA_ENABLE_CALLSTACK
#include "windows.h"
#include "dbghelp.h"

namespace Umbra
{
/*
Warning: this needs dbghelp.dll on windows, which might not be present on
non-development systems or some windows versions.
*/
void printCallStack(Logger* logger)
{
    static bool initialized = false;

    if (!initialized)
    {
        SymInitialize(GetCurrentProcess(), "", TRUE);
        initialized = true;
    }

    DWORD options = SymGetOptions();
    options |= SYMOPT_LOAD_LINES;
    options |= SYMOPT_FAIL_CRITICAL_ERRORS;
    SymSetOptions(options);

    CONTEXT context;
    RtlCaptureContext(&context);

    STACKFRAME64 stackFrame;
    DWORD        type;

#if _M_IX86
    type = IMAGE_FILE_MACHINE_I386;
    stackFrame.AddrPC.Offset     = context.Eip;
    stackFrame.AddrPC.Mode       = AddrModeFlat;
    stackFrame.AddrFrame.Offset  = context.Ebp;
    stackFrame.AddrFrame.Mode    = AddrModeFlat;
    stackFrame.AddrStack.Offset  = context.Esp;
    stackFrame.AddrStack.Mode    = AddrModeFlat;
#elif _M_IA64
    type = IMAGE_FILE_MACHINE_IA64;
    stackFrame.AddrPC.Offset        = context.StIIP;
    stackFrame.AddrPC.Mode          = AddrModeFlat;
    stackFrame.AddrFrame.Offset     = context.IntSp;
    stackFrame.AddrFrame.Mode       = AddrModeFlat;
    stackFrame.AddrBStore.Offset    = context.RsBSP;
    stackFrame.AddrBStore.Mode      = AddrModeFlat;
    stackFrame.AddrStack.Offset     = context.IntSp;
    stackFrame.AddrStack.Mode       = AddrModeFlat;
#elif _M_X64
    type = IMAGE_FILE_MACHINE_AMD64;
    stackFrame.AddrPC.Offset    = context.Rip;
    stackFrame.AddrPC.Mode      = AddrModeFlat;
    stackFrame.AddrFrame.Offset = context.Rsp;
    stackFrame.AddrFrame.Mode   = AddrModeFlat;
    stackFrame.AddrStack.Offset = context.Rsp;
    stackFrame.AddrStack.Mode   = AddrModeFlat;
#else
#error "unimplemented"
#endif

    char  filenameCopy[MAX_PATH] = "";
    UINT8 mem[sizeof(SYMBOL_INFO) + 256];

    while(StackWalk64(type, GetCurrentProcess(), GetCurrentThread(), &stackFrame, (void*)&context, NULL, SymFunctionTableAccess64, SymGetModuleBase64, NULL))
    {
        if (stackFrame.AddrPC.Offset != 0)
        {
            SYMBOL_INFO* symbol = (SYMBOL_INFO*)mem;
            memset(symbol, 0, sizeof(SYMBOL_INFO)+256);
            symbol->SizeOfStruct = sizeof(SYMBOL_INFO);
            symbol->MaxNameLen = 256;

            IMAGEHLP_LINE64 line;
            memset(&line, 0, sizeof(IMAGEHLP_LINE64));
            line.SizeOfStruct = sizeof(IMAGEHLP_LINE64);

            DWORD  dwDisplacement;
            bool symOK  = !!SymFromAddr(GetCurrentProcess(), stackFrame.AddrPC.Offset, 0, symbol);
            bool lineOK = !!SymGetLineFromAddr64(GetCurrentProcess(), stackFrame.AddrPC.Offset, &dwDisplacement, &line);

            char linestr[16];
            char* filename = NULL;

            if (lineOK)
            {
                filenameCopy[0] = '\0';
                strcpy_s(filenameCopy, MAX_PATH, line.FileName);
                filename = strrchr(filenameCopy, '\\');
                sprintf_s(linestr, 16, "%d", line.LineNumber);
                if (filename)
                    filename++;
                else
                    filename = filenameCopy;

            } else
                strcpy_s(linestr, 16, "??");


            if (logger)
                umbraLog(logger, Logger::LEVEL_INFO, "%s (%s:%s)", symOK ? symbol->Name : "??", lineOK ? filename : "??", linestr);
            else
                printf("%s (%s:%s)\n", symOK ? symbol->Name : "??", lineOK ? filename : "??", linestr);
        }
    }
}
}
#else
namespace Umbra
{
void printCallStack(Logger*) { /* disabled or not implemented on this platform */ }
}
#endif

namespace Umbra
{
/* \todo [antti 5.10.2011]: should get full backtrace as alloc identifier! */

class StatsHeapState
{
public:
    StatsHeapState (Allocator *a): m_mutex(a), m_allocs(a), m_locHash(a), m_perloc(a) {}

    void    addAlloc        (void* ptr, size_t size, const char* info);
    void    removeAlloc     (void* ptr);
    void    dumpCurrent     (Logger* logger, Logger::Level level);
    void    dumpAccumulated (Logger* logger, Logger::Level level);
    int     numAllocs       (void) const { return m_global.m_numAllocs; }
    int     numAllocsPeak   (void) const { return m_global.m_numAllocsPeak; }
    size_t  allocatedBytes  (void) const { return m_global.m_allocSize; }
    size_t  allocatedBytesPeak (void) const { return m_global.m_allocSizePeak; }

private:

    class Stats
    {
    public:
        Stats(void): m_numAllocs(0), m_allocSize(0),
            m_numAllocsPeak(0), m_allocSizePeak(0), m_numAllocsTotal(0) {}

        int     m_numAllocs;
        size_t  m_allocSize;
        int     m_numAllocsPeak;
        size_t  m_allocSizePeak;
        int     m_numAllocsTotal;

        void add (size_t size)
        {
            m_numAllocs++;
            m_numAllocsTotal++;
            m_allocSize += size;
            m_numAllocsPeak = max2(m_numAllocsPeak, m_numAllocs);
            m_allocSizePeak = max2(m_allocSizePeak, m_allocSize);
        }

        void remove (size_t size)
        {
            UMBRA_ASSERT(m_numAllocs);
            UMBRA_ASSERT(m_allocSize >= size);
            m_numAllocs--;
            m_allocSize -= size;
        }
    };

    struct Entry
    {
        Entry(void): loc(-1), size(0) {}
        Entry(int loc, size_t size): loc(loc), size(size) {}

        int     loc;
        size_t  size;
    };

    int getLocIdx (const char* info)
    {
        if (!info)
            return -1;
        String s(info, m_allocs.getAllocator());
        int* loc = m_locHash.get(s);
        if (!loc)
        {
            m_perloc.pushBack(Stats());

#if defined(UMBRA_COMP_NO_EXCEPTIONS)
            loc = m_locHash.insert(s, m_perloc.getSize() - 1);
#else
            try
            {
                loc = m_locHash.insert(s, m_perloc.getSize() - 1);
            } catch(OOMException)
            {
                m_perloc.resize(m_perloc.getSize() - 1);
                throw;
            }
#endif
        }
        return *loc;
    }

    void buildReverseLookupTable (Array<String>& lut)
    {
        lut.reset(m_perloc.getSize());

        Hash<String, int>::Iterator it = m_locHash.iterate();
        while (m_locHash.isValid(it))
        {
            int idx = m_locHash.getValue(it);
            if (idx >= 0 && idx < m_perloc.getSize())
                lut[idx] = m_locHash.getKey(it);
            m_locHash.next(it);
        }
    }

    CriticalSection     m_mutex;

    Stats               m_global;
    Hash<void*, Entry>  m_allocs;
    Hash<String, int>   m_locHash;
    Array<Stats>        m_perloc;
};


StatsHeap::~StatsHeap (void)
{
    reset();
}

void* StatsHeap::allocate(size_t size, const char* info)
{
    if (!m_state)
    {
        UMBRA_ASSERT(m_forward);
        m_state = UMBRA_HEAP_NEW(m_forward, StatsHeapState, m_forward);
    }

    void* alloc = m_forward->allocate(size, info);

#if defined(UMBRA_COMP_NO_EXCEPTIONS)
    m_state->addAlloc(alloc, size, info);
#else
    try
    {
        m_state->addAlloc(alloc, size, info);
    } catch(OOMException)
    {
        m_forward->deallocate(alloc);
        return NULL;
    }
#endif

    return alloc;
}

void StatsHeap::deallocate(void* ptr)
{
    m_state->removeAlloc(ptr);
    m_forward->deallocate(ptr);
}

void StatsHeap::dump (Logger* logger)
{
    if (m_state)
    {
        m_state->dumpCurrent(logger, Logger::LEVEL_WARNING);

        if (g_assertOnMemoryLeaks)
        {
            UMBRA_ASSERT(m_state->numAllocs() == 0);
        }

        m_state->dumpAccumulated(logger, Logger::LEVEL_DEBUG);
    }
}

void StatsHeap::reset (void)
{
    if (m_state)
    {
        UMBRA_HEAP_DELETE(m_forward, m_state);
        m_state = NULL;
    }
}

void StatsHeap::dumpAccumulated(Logger* logger, Logger::Level level)
{
    if (!m_state)
        return;
    m_state->dumpAccumulated(logger, level);
}

int StatsHeap::getNumAllocations (void) const
{
    if (!m_state)
        return 0;
    return m_state->numAllocs();
}

int StatsHeap::getPeakNumAllocations (void) const
{
    if (!m_state)
        return 0;
    return m_state->numAllocsPeak();
}

size_t StatsHeap::getAllocatedBytes (void) const
{
    if (!m_state)
        return 0;
    return m_state->allocatedBytes();
}

size_t StatsHeap::getPeakAllocatedBytes (void) const
{
    if (!m_state)
        return 0;
    return m_state->allocatedBytesPeak();
}

void StatsHeapState::addAlloc (void* ptr, size_t size, const char* info)
{
    if (!ptr)
        return;

    ScopedCriticalSectionEnter lock(m_mutex);

    int loc = getLocIdx(info);

    UMBRA_ASSERT(!m_allocs.contains(ptr));
    m_allocs.insert(ptr, Entry(loc, size));

    // Increase stats last to leave the stats heap
    // in consistent state in case there's OOM during addAlloc
    m_global.add(size);
    if (loc != -1)
        m_perloc[loc].add(size);
}

void StatsHeapState::removeAlloc (void* ptr)
{
    if (!ptr)
        return;

    ScopedCriticalSectionEnter lock(m_mutex);

    UMBRA_ASSERT(m_allocs.contains(ptr));
    Entry e = *m_allocs.get(ptr);
    m_allocs.remove(ptr);
    if (e.loc != -1)
        m_perloc[e.loc].remove(e.size);
    m_global.remove(e.size);
}

void StatsHeapState::dumpCurrent (Logger* logger, Logger::Level level)
{
    if (!m_global.m_numAllocs)
        return;

    String allocSizeFormatted = String::formatSize(m_global.m_allocSize);
    umbraLog(logger, level, "Number of allocations: %d (total size %s)",
        m_global.m_numAllocs, allocSizeFormatted.toCharPtr());

    Array<String> lut(m_allocs.getAllocator());

#if defined(UMBRA_COMP_NO_EXCEPTIONS)
    buildReverseLookupTable(lut);
#else
    try
    {
        buildReverseLookupTable(lut);
    } catch(OOMException)
    {
        lut.reset(0);
    }
#endif

    for (int i = 0; i < m_perloc.getSize(); i++)
    {
        if (!m_perloc[i].m_numAllocs)
            continue;
        String allocSizeFormatted = String::formatSize(m_perloc[i].m_allocSize);
        umbraLog(logger, level, "LEAKED %s (num %d size %s)",
            lut.getSize() ? lut[i].toCharPtr() : "(N/A)",
            m_perloc[i].m_numAllocs,
            allocSizeFormatted.toCharPtr());
    }
}

void StatsHeapState::dumpAccumulated (Logger* logger, Logger::Level level)
{
    String peakSizeFormatted = String::formatSize(m_global.m_allocSizePeak);
    umbraLog(logger, level, "Total allocations %d, peak %d (size %s)",
        m_global.m_numAllocsTotal, m_global.m_numAllocsPeak, peakSizeFormatted.toCharPtr());

    Array<String> lut(m_allocs.getAllocator());
    buildReverseLookupTable(lut);

    for (int i = 0; i < m_perloc.getSize(); i++)
    {
        umbraLog(logger, level, "Alloc %s (total %d peak %d size %d)",
            lut[i].toCharPtr(), m_perloc[i].m_numAllocsTotal,
            m_perloc[i].m_numAllocsPeak, m_perloc[i].m_allocSizePeak);
    }
}

} // namespace Umbra
