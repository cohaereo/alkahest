#ifndef UMBRATHREAD_HPP
#define UMBRATHREAD_HPP
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
 * \brief   Umbra Thread
 * \todo [wili] Using inheritance rather than 'using' ?
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{

class Thread;
class String;
class Allocator;

/*-------------------------------------------------------------------*//*!
 * \brief           Pool of threads. Should be used to get the Threads
 *                  used in the application.
 *//*-------------------------------------------------------------------*/

class ThreadPool
{
public:

    static Thread *     get             (void);
    static void         release         (Thread * thread);

private:

                        ThreadPool      (void)              {};
                        ~ThreadPool     (void)              {};
                        ThreadPool      (const ThreadPool&);        // not allowed!
    ThreadPool&         operator=       (const ThreadPool&);        // not allowed!

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

};

/*-------------------------------------------------------------------*//*!
 * \brief           Interface for classes implementing the user desired
 *                  functionality for the threads.
 *//*-------------------------------------------------------------------*/

class Runnable
{
public:

                            Runnable        (void);
    virtual                 ~Runnable       (void);

    virtual unsigned long   run             (void * param)      = 0;

private:
                            Runnable        (const Runnable&);      // not allowed!
    Runnable&               operator=       (const Runnable&);      // not allowed!

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

};

/*-------------------------------------------------------------------*//*!
 * \brief           Class representing one thread.
 *
 * \note            For creating and destroying threads, see ThreadPool
 *
 * \note            Thread class is thread safe between the controlling thread
 *                  and the client thread modeled by the class. However, there
 *                  are no safeties for situations where multiple threads try
 *                  to use interface of the Thread class concurrently.
 *//*-------------------------------------------------------------------*/

class Thread
{
public:
                        Thread          (Allocator* a = NULL);
                        ~Thread         (void);


    static void         sleep           (int millis);
    static void         yield           (void);
    void                setFunction     (Runnable * runMe);
    bool                run             (void * param);
    bool                isFinished      (void) const;
    uint32              getExitCode     (void) const;
    bool                waitToFinish    (unsigned int timeoutMs = (unsigned int)-1);
    void                setPriority     (int priority);

    static int          getNumProcessors (void);

    static int          allocTls        (void);
    static void         freeTls         (int idx);
    static void         setTlsValue     (int idx, UINTPTR value);
    static UINTPTR      getTlsValue     (int idx);

private:
    Allocator*          getAllocator    (void) {return m_allocator;}

    Thread          (const Thread&);        // not allowed!
    Thread&             operator=       (const Thread&);        // not allowed!
    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    Allocator*          m_allocator;
    class ImpThread *   m_impl;
};

/*-------------------------------------------------------------------*//*!
 * \brief           Synchronizer for threads within one process.
 *
 * \note            This will use the fastest known implementation for platform.
 *//*-------------------------------------------------------------------*/

class CriticalSection
{
public:
                                CriticalSection         (Allocator* a = NULL); /* \todo [antti 6.10.2011]: remove alloc */
                                ~CriticalSection        (void);

    void                        enter                   (void);
    void                        leave                   (void);

private:
                                CriticalSection         (const CriticalSection&);       // not allowed!
    CriticalSection&            operator=               (const CriticalSection&);       // not allowed!

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    Allocator* m_heap;
    class ImpCriticalSection *  m_impl;
};


/*-------------------------------------------------------------------*//*!
 * \brief           Lock can be used to mark whole method as critical section
 *                  and be assured that critical section is left at the end
 *                  of the method.
 *//*-------------------------------------------------------------------*/

class Lock
{
public:
    inline explicit     Lock            (CriticalSection & critSection):m_critSection(critSection)  {m_critSection.enter();}
    inline              ~Lock           (void)                                                      {m_critSection.leave();}

private:
                        Lock            (const Lock&);      // not allowed!
    Lock&               operator=       (const Lock&);      // not allowed!

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    CriticalSection &   m_critSection;
};

/*-------------------------------------------------------------------*//*!
 * \brief           Mutual Exclusion entity.
 *
 * \note            This can be used to serialize access to resourses etc.
 *//*-------------------------------------------------------------------*/

class Mutex
{
public:
                Mutex           (Allocator* = 0);
                ~Mutex          (void);

    void        lock            (void);
    void        release         (void);
    bool        tryLock         (int millis);
private:
                Mutex           (const Mutex&);     // not allowed!
    Mutex&      operator=       (const Mutex&);     // not allowed!

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    class ImpMutex *            m_impl;
};

/*-------------------------------------------------------------------*//*!
 * \brief           Helper struct used to lock a Mutex in a scope.
 *//*-------------------------------------------------------------------*/

struct ScopedLock
{
    ScopedLock(Mutex& m) : m_mutex(m) { m_mutex.lock(); }
    ~ScopedLock() { m_mutex.release(); }

private:
    ScopedLock& operator=(const ScopedLock&) { return *this; } // not allowed!

    Mutex&  m_mutex;
};

struct ScopedCriticalSectionEnter
{
    ScopedCriticalSectionEnter(CriticalSection& m) : m_mutex(m) { m_mutex.enter(); }
    ~ScopedCriticalSectionEnter() { m_mutex.leave(); }

private:
    ScopedCriticalSectionEnter& operator=(const ScopedCriticalSectionEnter&) { return *this; } // not allowed!

    CriticalSection&  m_mutex;
};

/*-------------------------------------------------------------------*//*!
 * \brief           Semaphore limits the amount of threads accessing e.g.
 *                  limited resource at any given moment.
 *//*-------------------------------------------------------------------*/

class Semaphore
{
public:
                Semaphore       (int initialCount, int maxCount);
                Semaphore       (const String& name, int initialCount, int maxCount);
                ~Semaphore      (void);

    void        up              (void);
    void        down            (void);
    bool        tryDown         (int millis);
    bool        checkDown       (void);       // No wait, returns immediately

private:
                                Semaphore       (const Semaphore&);     // not allowed!
    Semaphore&                  operator=       (const Semaphore&);     // not allowed!
    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    class ImpSemaphore *            m_impl;
};

namespace Atomic
{
INT32  add(volatile INT32*  value, int a);
INT64  add(volatile INT64*  value, INT64 a);
UINT32 add(volatile UINT32* value, int a);
UINT64 add(volatile UINT64* value, INT64 a);
size_t add(volatile size_t* value, size_t a);

INT32  sub(volatile INT32*  value, int a);
INT64  sub(volatile INT64*  value, INT64 a);
UINT32 sub(volatile UINT32* value, int a);
UINT64 sub(volatile UINT64* value, INT64 a);
size_t sub(volatile size_t* value, size_t a);
}

} // namespace Umbra

#endif // UMBRATHREAD_HPP

//--------------------------------------------------------------------
