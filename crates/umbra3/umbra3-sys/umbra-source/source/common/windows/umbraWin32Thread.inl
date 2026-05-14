#ifndef UMBRAWIN32THREAD_HPP
#define UMBRAWIN32THREAD_HPP
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
 * \brief   Threading implementation for win32.
 *
 */

#include "umbraThread.hpp"
#include "umbraString.hpp"

#if UMBRA_OS == UMBRA_XBOX360
#   include <xtl.h>
#elif UMBRA_OS == UMBRA_XBOXONE
#   include <xdk.h>
#   include <windows.h>
#   include <intrin.h>
#else
#   ifndef NOMINMAX
#       define NOMINMAX
#   endif
#   include <windows.h>
#   include <process.h>
#endif

/*****************************************************************************
 *
 * Class:           ImpThread
 *
 * Description:     Implementation for the Thread class in _WIN32.
 *
 * Notes:
 *
 *****************************************************************************/



//------------------------------------------------------------------------
// Some required waiting times.
//------------------------------------------------------------------------

static const int SMALL_WAIT_MS = 1;
static const int YIELD_WAIT_MS = 0;

namespace Umbra
{

class ImpThread
{
public:

    enum State
    {
        NOT_RUN     = -1,   // State before first run
        STARTING    = 0,    // Thread is being started.
        RUNNING     = 1,    // Thread is running
        FINISHED    = 2     // Thread has finished its run and return value is available.
    };

    inline                      ImpThread       (Allocator* a = NULL);
    inline                      ~ImpThread      (void);

    inline static void          yield           (void);
    inline static void          sleep           (int millis);
    inline void                 setFunction     (Runnable * runMe);
    inline bool                 run             (void * param);
    inline bool                 isFinished      (void) const;
    inline uint32               getExitCode     (void) const;
    inline void                 threadMain      (void * param);
    inline bool                 waitToFinish    (unsigned int timeoutMs);
    inline void                 setPriority     (int priority);

private:
                                ImpThread       (const ImpThread&);     // not allowed!
    ImpThread&                  operator=       (const ImpThread&);     // not allowed!
    inline State                getState        (void) const    { m_waitCSect.enter(); State s = m_state; m_waitCSect.leave(); return s; }
    inline void                 setState        (State s)       { m_waitCSect.enter(); m_state = s; m_waitCSect.leave(); }

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    HANDLE             m_hThread;                              // Thread handle
    UINT32             m_returnValue;                          // Return value of the users function.
    Runnable*          m_client;                               // Runnable that wishes itself to be run
    void*              m_realParam;                            // Parameters given by the user for the function.
    State              m_state;                                // state of the Thread. See enumeration.
    int                m_priority;                             // Thread priority
    mutable CriticalSection m_waitCSect;                       // Mutex for waiting the thread to finish.
};

static unsigned int WINAPI threadFunc(void * pThis);

/*****************************************************************************
 *
 * Function:        ImpThread::ImpThread()
 *
 * Description:     ImpThread constructor
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpThread::ImpThread(Allocator* a):
    m_waitCSect(a)
{
    m_returnValue = 0;
    m_realParam = NULL;
    m_state = NOT_RUN;
    m_client = NULL;
    m_hThread = NULL;
    m_priority = 0;
}

/*****************************************************************************
 *
 * Function:        ImpThread::~ImpThread()
 *
 * Description:     ImpThread destructor
 *
 * Returns:
 *
 * Notes:           Exits the thread finally.
 *
 *****************************************************************************/

inline ImpThread::~ImpThread(void)
{
    waitToFinish(0xffffffff);
    UMBRA_ASSERT(getState() == NOT_RUN || getState() == FINISHED);
}

/*****************************************************************************
 *
 * Function:        ImpThread::setFunction()
 *
 * Description:     Sets the task to be accomplished by this thread on its next
 *                  run() invocation.
 *
 * Parameters:      runMe = Runnable class that implements the desired functionality.
 *
 * Notes:           Use run() to actually run the thread and give the possible
 *                  parameters.
 *                  This is NOT thread safe with the run method. Calling these
 *                  two from two different threads may result in undefined
 *                  behaviour.
 *
 *****************************************************************************/

inline void ImpThread::setFunction(Runnable* runMe)
{
    UMBRA_ASSERT(getState() == NOT_RUN);
    UMBRA_ASSERT(runMe != NULL && "NULL Runnable is invalid");
    m_client = runMe;
}

/*****************************************************************************
 *
 * Function:        ImpThread::threadFunc()
 *
 * Description:     Internal function used as the inner loop of the thread.
 *
 * Parameters:      param = Empty. Required by the specification.
 *
 * Notes:
 *
 *****************************************************************************/

inline void ImpThread::threadMain(void *)
{
    UMBRA_ASSERT((getState() == STARTING) && "State wrong for running the thread");
    UMBRA_ASSERT((m_client != NULL) && "Runnable class not provided");
    setState(RUNNING);
    m_returnValue = m_client->run(m_realParam);
    setState(FINISHED);
}

/*****************************************************************************
 *
 * Function:        ImpThread::run()
 *
 * Description:     Starts the thread
 *
 * Parameters:      param = parameter to be passed to the runnable function.
 *
 * Returns:         True is succeeds, false on failure.
 *
 *
 *****************************************************************************/

#if UMBRA_OS == UMBRA_XBOX360 || UMBRA_OS == UMBRA_XBOXONE
inline bool ImpThread::run (void*)
{
    UMBRA_ASSERT(!"not implemented");
    return false;
}
#else
inline bool ImpThread::run (void* param)
{
    UMBRA_ASSERT((m_client != NULL) && "Runnable for the Thread not set!");
    UMBRA_ASSERT((getState() == NOT_RUN) && "Still starting the previous run.");

    m_realParam = param;
    m_state = STARTING;
    m_hThread = (HANDLE)_beginthreadex(NULL, 0, threadFunc, reinterpret_cast<void *>(this), 0, 0);
    if (!m_hThread)
    {
        m_state = FINISHED;
        return false;
    }

    if (m_priority)
        SetThreadPriority(m_hThread, THREAD_PRIORITY_NORMAL + m_priority);

    return true;
}
#endif

/*****************************************************************************
 *
 * Function:        ImpThread::isFinished() const
 *
 * Description:     Checks if the thread has finished it latest run.
 *
 * Returns:         True if thread has finished latest run, false otherwise
 *
 * Notes:           Thread that has never started returns also false. Thread
 *                  that has been terminated returns false.
 *
 *****************************************************************************/

inline bool ImpThread::isFinished(void) const
{
    return getState() == FINISHED;
}

/*****************************************************************************
 *
 * Function:        ImpThread::getExitCode() const
 *
 * Description:     Gets the exit code of the function.
 *
 * Returns:         Exit code.
 *
 * Notes:           Check that run is finished before querying this. Otherwise
 *                  you will get an old value.
 *
 *****************************************************************************/

inline uint32 ImpThread::getExitCode(void) const
{
    UMBRA_ASSERT(getState() == FINISHED);
    return m_returnValue;
}

/*****************************************************************************
 *
 * Function:        ImpThread::suspend()
 *
 * Description:     Puts the calling thread to sleep for millis milliseconds
 *
 * Parameters:      millis = millisecons to sleep.
 *
 * Notes:
 *
 *****************************************************************************/

inline void ImpThread::sleep(int millis)
{
    Sleep(millis);
}

/*****************************************************************************
 *
 * Function:        ImpThread::yield()
 *
 * Description:     Yields the current time slice of the calling thread.
 *
 * Notes:
 *
 *****************************************************************************/

inline void ImpThread::yield(void)
{
    Sleep(YIELD_WAIT_MS);
}

/*****************************************************************************
 *
 * Function:        ImpThread::waitToFinish()
 *
 * Description:
 *
 * Notes:
 *
 *****************************************************************************/

inline bool ImpThread::waitToFinish(unsigned int timeoutMs)
{
    if (getState() == NOT_RUN)
        return true;
    UMBRA_ASSERT(INFINITE == 0xffffffff);
    if (WaitForSingleObject(m_hThread, timeoutMs) == WAIT_TIMEOUT)
        return false;
    UMBRA_ASSERT(isFinished());
    CloseHandle(m_hThread);
    m_hThread = NULL;
    return true;
}

/*****************************************************************************
 *
 * Function:        ImpThread::setPriority(int)
 *
 * Description:
 *
 * Notes:
 *
 *****************************************************************************/

inline void ImpThread::setPriority (int priority)
{
    m_priority = priority;
    if (m_hThread)
        SetThreadPriority(m_hThread, THREAD_PRIORITY_NORMAL + m_priority);
}

/*****************************************************************************
 *
 * Class:           ImpCriticalSection
 *
 * Description:
 *
 * Notes:
 *
 *****************************************************************************/

class ImpCriticalSection
{
public:
    inline                  ImpCriticalSection      (void);
    inline                  ~ImpCriticalSection     (void);

    inline void             enter                   (void);
    inline void             leave                   (void);

private:
                            ImpCriticalSection      (const ImpCriticalSection&);        // not allowed!
    ImpCriticalSection&     operator=               (const ImpCriticalSection&);        // not allowed!

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------


    CRITICAL_SECTION        m_critSection;
    int                     m_entered;
};


/*****************************************************************************
 *
 * Function:        ImpCriticalSection::ImpCriticalSection()
 *
 * Description:     ImpCriticalSection constructor
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpCriticalSection::ImpCriticalSection(void)
{
    InitializeCriticalSection(&m_critSection);
    m_entered = 0;
}

/*****************************************************************************
 *
 * Function:        ImpCriticalSection::~ImpCriticalSection()
 *
 * Description:     ImpCriticalSection destructor
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpCriticalSection::~ImpCriticalSection(void)
{
    UMBRA_ASSERT(!m_entered);
    DeleteCriticalSection(&m_critSection);
}

/*****************************************************************************
 *
 * Function:        ImpCriticalSection::enter()
 *
 * Description:     Enters the critical section i.e. blocks other threads from
 *                  entering THIS critical section at the same time.
 *
 * Notes:           Blocks until critical section may be entered.
 *                  Once the block is entered, remember to leave it.
 *
 *****************************************************************************/

inline void ImpCriticalSection::enter(void)
{
    EnterCriticalSection(&m_critSection);
    UMBRA_ASSERT(!m_entered);
    UMBRA_DEBUG_CODE(m_entered = 1);
}

/*****************************************************************************
 *
 * Function:        ImpCriticalSection::leave()
 *
 * Description:     Leaves the critical section.
 *
 * Notes:
 *
 *****************************************************************************/

inline void ImpCriticalSection::leave(void)
{
    UMBRA_ASSERT(m_entered);
    UMBRA_DEBUG_CODE(m_entered = 0);
    LeaveCriticalSection(&m_critSection);
}


/*****************************************************************************
 *
 * Class:           ImpMutex
 *
 * Description:     Implementation of the Mutual Exclusion entity.
 *
 * Notes:           If performance is an issue, prefer CriticalSection.
 *
 *****************************************************************************/

class ImpMutex
{
public:
    inline          ImpMutex        (void);
    inline          ~ImpMutex       (void);

    inline void     lock            (void);
    inline void     release         (void);
    inline bool     tryLock         (int millis);
private:
                    ImpMutex        (const ImpMutex&);      // not allowed!
    ImpMutex&       operator=       (const ImpMutex&);      // not allowed!

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    HANDLE          m_hMutex;
};



/*****************************************************************************
 *
 * Function:        ImpMutex::ImpMutex()
 *
 * Description:     ImpMutex constructor
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpMutex::ImpMutex(void)
{
    m_hMutex = CreateMutex(NULL, FALSE, NULL);
    UMBRA_ASSERT(m_hMutex != NULL && "Mutex initialization failed.");
}


/*****************************************************************************
 *
 * Function:        ImpMutex::~ImpMutex()
 *
 * Description:     ImpMutex destructor
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpMutex::~ImpMutex(void)
{
    UMBRA_ASSERT(m_hMutex != NULL);
    CloseHandle(m_hMutex);
}

/*****************************************************************************
 *
 * Function:        ImpMutex::lock()
 *
 * Description:     Blocks until the Mutex is acquired.
 *
 * Notes:           Will assert itself in the debug build for any anomalies.
 *                  Remember to release the Mutex!
 *
 *****************************************************************************/

inline void ImpMutex::lock(void)
{
    UMBRA_ASSERT(m_hMutex != NULL);
    DWORD result = WaitForSingleObject(m_hMutex, INFINITE);
    UMBRA_UNREF(result);
    UMBRA_ASSERT(result != WAIT_ABANDONED_0 && "Other thread exited or terminated without releasing this mutex!");
    UMBRA_ASSERT(result != WAIT_FAILED && "Indefinite waiting failed for some reason.");
    UMBRA_ASSERT(result == WAIT_OBJECT_0 && "Mutex not acquired.");
}


/*****************************************************************************
 *
 * Function:        ImpMutex::release()
 *
 * Description:     Releases the mutex.
 *
 * Notes:           Asserts on failure in debug build.
 *
 *****************************************************************************/

inline void ImpMutex::release(void)
{
    UMBRA_ASSERT(m_hMutex != NULL);
    BOOL result = ReleaseMutex(m_hMutex);
    UMBRA_UNREF(result);
    UMBRA_ASSERT(result && "Release failed. Maybe mutex was not acquired in the first place");
}


/*****************************************************************************
 *
 * Function:        ImpMutex::tryLock()
 *
 * Description:     Trys to lock the Mutex
 *
 * Parameters:      millis = how many milliseconds to try for the lock. Result
 *                  on using 0 is unknown.
 *
 * Returns:         True if locked, false otherwise.
 *
 * Notes:
 *                  Remember to release the Mutex!
 *
 *****************************************************************************/

inline bool ImpMutex::tryLock(int millis)
{
    UMBRA_ASSERT(millis >= 0);
    UMBRA_ASSERT(m_hMutex != NULL);
    DWORD result = WaitForSingleObject(m_hMutex, millis);
    UMBRA_ASSERT(result != WAIT_ABANDONED_0 && "Other thread exited or terminated without releasing this mutex!");
    return result == WAIT_OBJECT_0;
}

/*****************************************************************************
 *
 * Class:           ImpSemaphore
 *
 * Description:     Semaphore limits the amount of threads accessing e.g.
 *                  limited resource at any given moment.
 *
 * Notes:
 *
 *****************************************************************************/

class ImpSemaphore
{
public:
    inline              ImpSemaphore        (int initialCount, int maxCount);
        inline              ImpSemaphore        (const String& name, int initialCount, int maxCount);
    inline              ~ImpSemaphore       (void);

    inline void         up                  (void);
    inline void         down                (void);
    inline bool         tryDown             (int millis);
        inline bool         checkDown           (void);

private:
                        ImpSemaphore        (const ImpSemaphore&);      // not allowed!
    ImpSemaphore&       operator=           (const ImpSemaphore&);      // not allowed!
    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------
    HANDLE              m_hSemaphore;
};

/*****************************************************************************
 *
 * Function:        ImpSemaphore::ImpSemaphore()
 *
 * Description:     ImpSemaphore constructor
 *
 * Parameters:      initialCount    = What is the initial counter value. [0, maxCount].
 *                  maxCount        = Maximum counter value. This many downs can be made
 *                                  without blocking.
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpSemaphore::ImpSemaphore(int initialCount, int maxCount)
{
    UMBRA_ASSERT(initialCount >= 0);
    UMBRA_ASSERT(maxCount > 0);
    UMBRA_ASSERT(initialCount <= maxCount && "Invalid setup");
#if UMBRA_OS == UMBRA_XBOXONE
    m_hSemaphore = CreateSemaphoreExW(NULL, initialCount, maxCount, NULL, 0, SYNCHRONIZE | SEMAPHORE_MODIFY_STATE);
#else
    m_hSemaphore = CreateSemaphore(NULL, initialCount, maxCount, NULL);
#endif
    UMBRA_ASSERT(m_hSemaphore != NULL && "Semaphore construction failed");
}

/*****************************************************************************
 *
 * Function:        ImpSemaphore::ImpSemaphore()
 *
 * Description:     Named ImpSemaphore constructor
 *
 * Parameters:      name            = Semaphore name, same name in different processes.
 *                                  shares the object.
 *                  initialCount    = What is the initial counter value. [0, maxCount].
 *                  maxCount        = Maximum counter value. This many downs can be made
 *                                  without blocking.
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpSemaphore::ImpSemaphore(const String& name, int initialCount, int maxCount)
{
    UMBRA_ASSERT(initialCount >= 0);
    UMBRA_ASSERT(maxCount > 0);
    UMBRA_ASSERT(initialCount <= maxCount && "Invalid setup");
#if UMBRA_OS == UMBRA_XBOXONE
    WCHAR tmp[32] = L"";
    MultiByteToWideChar(CP_ACP, 0, name.toCharPtr(), -1, tmp, 32);
    m_hSemaphore = CreateSemaphoreExW(NULL, initialCount, maxCount, tmp, 0, SYNCHRONIZE | SEMAPHORE_MODIFY_STATE);
#else
    m_hSemaphore = CreateSemaphore(NULL,initialCount,maxCount,name.toCharPtr());
#endif
    UMBRA_ASSERT(m_hSemaphore != NULL && "Semaphore construction failed");
}


/*****************************************************************************
 *
 * Function:        ImpSemaphore::~ImpSemaphore()
 *
 * Description:     ImpSemaphore destructor
 *
 * Notes:
 *
 *****************************************************************************/

inline ImpSemaphore::~ImpSemaphore(void)
{
    CloseHandle(m_hSemaphore);
    m_hSemaphore = NULL;
}

/*****************************************************************************
 *
 * Function:        ImpSemaphore::down()
 *
 * Description:     Downs the semaphore i.e. decreases the counter if counter
 *                  is over zero, otherwise blocks until the counter becomes
 *                  greater than zero.
 *
 * Notes:           Remember to up the semaphore after the use!
 *
 *****************************************************************************/

inline void ImpSemaphore::down(void)
{
    UMBRA_ASSERT(m_hSemaphore != NULL && "Invalid handle");
    DWORD result = WaitForSingleObject(m_hSemaphore, INFINITE);
    UMBRA_UNREF(result);
    UMBRA_ASSERT(result != WAIT_ABANDONED_0 && "Thread exited or terminated without upping the semaphore");
    UMBRA_ASSERT(result == WAIT_OBJECT_0 && "Semaphore reservation failed");
}

/*****************************************************************************
 *
 * Function:        ImpSemaphore::tryDown()
 *
 * Description:     Downs the semaphore i.e. decreases the counter if counter
 *                  is over zero, otherwise blocks until the counter becomes
 *                  greater than zero or the specified time passes.
 *
 * Parameters:      millis = How many milliseconds to wait for availability.
 *
 * Returns:         True if downing was successful, false otherwise
 *
 * Notes:           Remember to up the semaphore after the use!
 *
 *****************************************************************************/

inline bool ImpSemaphore::tryDown(int millis)
{
    UMBRA_ASSERT(millis >= 0);
    DWORD result = WaitForSingleObject(m_hSemaphore, millis);
    UMBRA_ASSERT(result != WAIT_ABANDONED_0 && "Thread exited or terminated without upping the semaphore");
    return result == WAIT_OBJECT_0;
}

/*****************************************************************************
 *
 * Function:        ImpSemaphore::checkDown()
 *
 * Description:     Downs the semaphore i.e. decreases the counter if counter
 *          is over zero. Never blocks.
 *
 * Returns:     True if downing was successful, false otherwise
 *
 * Notes:       Remember to up the semaphore after the use!
 *
 *****************************************************************************/

inline bool ImpSemaphore::checkDown()
{
    DWORD result = WaitForSingleObject(m_hSemaphore, 0);
    UMBRA_ASSERT(result != WAIT_ABANDONED_0 && "Thread exited or terminated without upping the semaphore");
    return result == WAIT_OBJECT_0;
}

/*****************************************************************************
 *
 * Function:        ImpSemaphore::up()
 *
 * Description:     Ups semaphore i.e. increases the counter.
 *
 * Notes:
 *
 *****************************************************************************/

inline void ImpSemaphore::up(void)
{
    BOOL result = ReleaseSemaphore(m_hSemaphore, 1, NULL);
    UMBRA_UNREF(result);
    UMBRA_ASSERT(result && "Release failed.");
}

#if UMBRA_OS != UMBRA_XBOX360 && UMBRA_OS != UMBRA_XBOXONE
static unsigned int WINAPI threadFunc(void * pThis)
{
    UMBRA_ASSERT(pThis != NULL && "Invalid pointer, cannot start thread loop");
    reinterpret_cast<ImpThread *>(pThis)->threadMain(NULL);
    return 0;
}
#endif

int Thread::allocTls (void)
{
    return TlsAlloc();
}

void Thread::freeTls (int idx)
{
    if (idx != -1)
        TlsFree(idx);
}

void Thread::setTlsValue(int idx, UINTPTR value)
{
    TlsSetValue(idx, (LPVOID)value);
}

UINTPTR Thread::getTlsValue(int idx)
{
    return (UINTPTR)TlsGetValue(idx);
}


#if UMBRA_OS == UMBRA_WINDOWS

#if (_WIN32_WINNT > 0x0501) || defined(_WIN64)
#define HAS_ATOMIC_64 1
#else
#define HAS_ATOMIC_64 0
#endif

LONGLONG ImpInterlockedExchangeAdd64(
  LONGLONG volatile *Addend,
  LONGLONG Value
)
#if defined(_WIN32_WINNT) && _WIN32_WINNT <= 0x0501 && defined(_WIN64)
{
    return _InterlockedExchangeAdd64(Addend, Value);
}
#elif defined(_WIN32_WINNT) && _WIN32_WINNT <= 0x0501
;
#else
{
    return ::InterlockedExchangeAdd64(Addend, Value);
}
#endif

namespace Atomic
{
Umbra::INT32 add(volatile Umbra::INT32* value, int a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONG result = InterlockedExchangeAdd((volatile LONG*)value, a) + a;
    return (Umbra::INT32&)result;
}
Umbra::UINT32 add(volatile Umbra::UINT32*   value, int a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONG result = InterlockedExchangeAdd((volatile LONG*)value, a) + a;
    return (Umbra::UINT32&)result;
}
#if HAS_ATOMIC_64
Umbra::INT64 add(volatile Umbra::INT64*  value, Umbra::INT64 a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONGLONG result = ImpInterlockedExchangeAdd64((volatile LONGLONG*)value, a) + a;
    return (Umbra::INT64&)result;
}
Umbra::UINT64 add(volatile Umbra::UINT64* value, Umbra::INT64 a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONGLONG result = ImpInterlockedExchangeAdd64((volatile LONGLONG*)value, a) + a;
    return (Umbra::UINT64&)result;
}
#endif
size_t add(volatile size_t* value, size_t a)
{
#if defined(_WIN64)
    return add((volatile Umbra::UINT64*)value, (Umbra::INT64&)a);
#else
    return add((volatile Umbra::UINT32*)value, (int&)a);
#endif
}

Umbra::INT32 sub(volatile Umbra::INT32* value, int a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONG result = InterlockedExchangeAdd((volatile LONG*)value, -a) - a;
    return (Umbra::INT32&)result;
}
Umbra::UINT32 sub(volatile Umbra::UINT32*   value, int a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONG result = InterlockedExchangeAdd((volatile LONG*)value, -a) - a;
    return (Umbra::UINT32&)result;
}
#if HAS_ATOMIC_64
Umbra::INT64 sub(volatile Umbra::INT64*  value, Umbra::INT64 a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONGLONG result = ImpInterlockedExchangeAdd64((volatile LONGLONG*)value, -a) - a;
    return (Umbra::INT64&)result;
}
Umbra::UINT64 sub(volatile Umbra::UINT64* value, Umbra::INT64 a)
{
    UMBRA_ASSERT(!(((UINTPTR)value) & 3));
    LONGLONG result = ImpInterlockedExchangeAdd64((volatile LONGLONG*)value, -a) - a;
    return (Umbra::UINT64&)result;
}
#endif
size_t sub(volatile size_t* value, size_t a)
{
#if defined(_WIN64)
    return sub((volatile Umbra::UINT64*)value, (Umbra::INT64&)a);
#else
    return sub((volatile Umbra::UINT32*)value, (int&)a);
#endif
}

};
#endif

} // namespace Umbra

#endif //UMBRAWIN32THREAD_HPP
