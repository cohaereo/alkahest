/*!
 *
 * Umbra PVS Base
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
 * \brief   Threading library.
 *
 */

#include "umbraThread.hpp"
#include "umbraPrivateDefs.hpp"

#if (UMBRA_OS == UMBRA_WINDOWS || UMBRA_OS == UMBRA_XBOXONE || UMBRA_IS_POSIX)

#if UMBRA_IS_POSIX
#   include "posix/umbraPosixThread.inl"
#   include <unistd.h>
#elif UMBRA_OS == UMBRA_WINDOWS || UMBRA_OS == UMBRA_XBOXONE
#   include "windows/umbraWin32Thread.inl"
#endif

using namespace Umbra;

/*-------------------------------------------------------------------*//*!
 * \brief           Gets thread from the pool
 * \return          Pointer to a free thread
 * \note            Hoax implementation, makes a new thread every time.
 *//*-------------------------------------------------------------------*/

Thread * ThreadPool::get(void)
{
    return UMBRA_NEW(Thread);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Releases thread back to the pool
 * \param           thread  pointer to the thread to be released
 * \note            Hoax implementation, just deletes the thread.
 *//*-------------------------------------------------------------------*/

void ThreadPool::release(Thread * thread)
{
    UMBRA_DELETE(thread);
}

Runnable::Runnable(void)
{
}

Runnable::~Runnable(void)
{
}

/*-------------------------------------------------------------------*//*!
 * \brief           Thread constructor
 *//*-------------------------------------------------------------------*/

Thread::Thread(Allocator* a)
: m_allocator(a)
{
    if (m_allocator == NULL)
        m_allocator = Umbra::getAllocator();
    m_impl = UMBRA_NEW(ImpThread, m_allocator);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Thread destructor
 *//*-------------------------------------------------------------------*/

Thread::~Thread(void)
{
    UMBRA_DELETE(m_impl);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Puts the calling thread to sleep for millis milliseconds
 * \param           millis  millisecons to sleep.
 *//*-------------------------------------------------------------------*/

void Thread::sleep(int millis)
{
    ImpThread::sleep(millis);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Yields the current time slice of the calling thread.
 *//*-------------------------------------------------------------------*/

void Thread::yield(void)
{
    ImpThread::yield();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Sets the task to be accomplished by this thread on its next
 *                  run() invocation.
 *
 * \param           runMe   Runnable class that implements the desired functionality.
 *
 * \note            Use run() to actually run the thread and give the possible
 *                  parameters.
 *                  This is NOT thread safe with the run method. Calling these
 *                  two from two different threads may result in undefined
 *                  behaviour.
 *//*-------------------------------------------------------------------*/

void Thread::setFunction(Runnable * runMe)
{
    m_impl->setFunction(runMe);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Starts the thread
 *
 * \param           param   parameter to be passed to the runnable function.
 *
 * \return          True is succeeds, false on failure.
 *
 * \note            This yields the calling thread to give time to the Thread
 *                  begin its execution.
 *                  This will wait for max 1 second in 10 ms intervals for
 *                  the Thread to prepare itself for the next run, in case it is
 *                  in some kind of middle state.
 *//*-------------------------------------------------------------------*/

bool Thread::run (void * param)
{
    return m_impl->run(param);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Checks if the thread has finished it latest run.
 *
 * \return          True if thread has finished latest run, false otherwise
 *
 * \note            Thread that has never started returns also false.
 *//*-------------------------------------------------------------------*/

bool Thread::isFinished (void) const
{
    return m_impl->isFinished();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Gets the exit code of the function.
 *
 * \return          Exit code.
 *
 * \note            Check that run is finished before querying this. Otherwise
 *                  you will get an old value.
 *//*-------------------------------------------------------------------*/

uint32 Thread::getExitCode (void) const
{
    return m_impl->getExitCode();
}


/*-------------------------------------------------------------------*//*!
 * \brief           Blocks the calling thread until the Thread has finished
 *                  its current run.
 *
 * \note            Asserts in debug build if the thread is not running.
 *                  If there are several threads blocking with this, they
 *                  will be released serially.
 *//*-------------------------------------------------------------------*/

bool Thread::waitToFinish(unsigned int timeoutMs)
{
    return m_impl->waitToFinish(timeoutMs);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Sets the priority of the thread.
 *
 * \param           priority    Priority level relative to the normal priority.
 *//*-------------------------------------------------------------------*/

void Thread::setPriority(int priority)
{
    m_impl->setPriority(priority);
}


int Thread::getNumProcessors (void)
{
    // TODO: implement per platform!
#if UMBRA_OS == UMBRA_WINDOWS
    _SYSTEM_INFO si;
    GetSystemInfo(&si);
    return (int)si.dwNumberOfProcessors;
#elif UMBRA_OS == UMBRA_LINUX
    return sysconf(_SC_NPROCESSORS_CONF);
#else
    return 4;
#endif
}

/*-------------------------------------------------------------------*//*!
 * \brief           CriticalSection constructor
 *//*-------------------------------------------------------------------*/

CriticalSection::CriticalSection(Allocator* a)
{
    m_heap = a;
    if (!m_heap)
        m_heap = getAllocator();
    m_impl = UMBRA_HEAP_NEW(m_heap, ImpCriticalSection);
}

/*-------------------------------------------------------------------*//*!
 * \brief           CriticalSection destructor
 *//*-------------------------------------------------------------------*/

CriticalSection::~CriticalSection(void)
{
    UMBRA_HEAP_DELETE(m_heap, m_impl);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Enters the critical section i.e. blocks other threads from
 *                  entering THIS critical section at the same time.
 *
 * \note            Blocks until critical section may be entered.
 *                  Once the block is entered, remember to leave it.
 *//*-------------------------------------------------------------------*/

void CriticalSection::enter(void)
{
    m_impl->enter();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Leaves the critical section.
 *//*-------------------------------------------------------------------*/

void CriticalSection::leave(void)
{
    m_impl->leave();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Mutex constructor
 *//*-------------------------------------------------------------------*/

Mutex::Mutex(Allocator*)
{
    m_impl = new ImpMutex;
}

/*-------------------------------------------------------------------*//*!
 * \brief           Mutex destructor
 *//*-------------------------------------------------------------------*/

Mutex::~Mutex(void)
{
    delete m_impl;
}

/*-------------------------------------------------------------------*//*!
 * \brief           Blocks until the Mutex is acquired.
 *
 * \note            Will assert itself in the debug build for any anomalies.
 *                  Remember to release the Mutex!
 *//*-------------------------------------------------------------------*/

void Mutex::lock(void)
{
    m_impl->lock();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Releases the mutex.
 *
 * \note            Asserts on failure in debug build.
 *//*-------------------------------------------------------------------*/

void Mutex::release(void)
{
    m_impl->release();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Trys to lock the Mutex
 *
 * \param           millis  how many milliseconds to try for the lock, 0
 *                          makes one quick check.
 *
 * \return          True if locked, false otherwise.
 *
 * \note            Remember to release the Mutex!
 *//*-------------------------------------------------------------------*/

bool Mutex::tryLock(int millis)
{
    return m_impl->tryLock(millis);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Semaphore constructor
 *
 * \param           initialCount    What is the initial counter value. [0, maxCount].
 * \param           maxCount        Maximum counter value. This many downs can be made
 *                                  without blocking.
 *//*-------------------------------------------------------------------*/

Semaphore::Semaphore(int initialCount, int maxCount)
{
    m_impl = new ImpSemaphore(initialCount, maxCount);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Named Semaphore constructor.
 *
 * \note            Named semaphores are shared between processes.
 *
 * \param           name            Semaphore name, same for all processes.
 * \param           initialCount    What is the initial counter value. [0, maxCount].
 * \param           maxCount        Maximum counter value. This many downs can be made
 *                                  without blocking.
 *//*-------------------------------------------------------------------*/

Semaphore::Semaphore(const String& name, int initialCount, int maxCount)
{
    m_impl = new ImpSemaphore(name, initialCount, maxCount);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Semaphore destructor
 *//*-------------------------------------------------------------------*/

Semaphore::~Semaphore(void)
{
    delete m_impl;
    m_impl = NULL;
}

/*-------------------------------------------------------------------*//*!
 * \brief           Downs the semaphore i.e. decreases the counter if counter
 *                  is over zero, otherwise blocks until the counter becomes
 *                  greater than zero.
 *
 * \note            Remember to up the semaphore after the use!
 *//*-------------------------------------------------------------------*/

void Semaphore::down(void)
{
    m_impl->down();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Downs the semaphore i.e. decreases the counter if counter
 *                  is over zero, otherwise blocks until the counter becomes
 *                  greater than zero or the specified time passes.
 *
 * \param           millis  How many milliseconds to wait for availabitlity.
 *                          0 makes a quick check.
 *
 * \return          True if downing was successful, false otherwise
 *
 * \note            Remember to up the semaphore after the use!
 *//*-------------------------------------------------------------------*/

bool Semaphore::tryDown(int millis)
{
    return m_impl->tryDown(millis);
}

/*-------------------------------------------------------------------*//*!
 * \brief           Downs the semaphore i.e. decreases the counter if counter
 *                  is over zero. Never blocks.
 *
 * \return          True if downing was successful, false otherwise
 *
 * \note            Remember to up the semaphore after the use!
 *//*-------------------------------------------------------------------*/

bool Semaphore::checkDown(void)
{
    return m_impl->checkDown();
}

/*-------------------------------------------------------------------*//*!
 * \brief           Ups semaphore i.e. increases the counter.
 *//*-------------------------------------------------------------------*/

void Semaphore::up(void)
{
    m_impl->up();
}

#else

using namespace Umbra;

CriticalSection::CriticalSection(Allocator* a) { UMBRA_UNREF(a); }
CriticalSection::~CriticalSection(void) {}
void CriticalSection::enter (void) {}
void CriticalSection::leave (void) {}

int Thread::allocTls (void)
{
    return -1;
}

void Thread::freeTls (int idx)
{
    UMBRA_UNREF(idx);
}

void Thread::setTlsValue(int idx, UINTPTR value)
{
    UMBRA_UNREF(idx);
    UMBRA_UNREF(value);
}

UINTPTR Thread::getTlsValue(int idx)
{
    UMBRA_UNREF(idx);
    return 0;
}


#endif
