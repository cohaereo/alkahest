/*!
 *
 * Umbra POSIX Threading Implementation
 * -----------------------------------------
 *
 * (C) 2007-2013 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement.
 *
 * \file
 * \brief   POSIX threading implementation
 *
 */

#include <errno.h>
#include <pthread.h>

namespace Umbra {

/*-------------------------------------------------------------------*//*!
 * \brief   POSIX thread implementation
 *//*-------------------------------------------------------------------*/

Thread::Thread(void)
    : m_handle(0), m_started(false), m_exited(false), m_exitCode(0) {}

Thread::~Thread(void) {
  if (m_started && !m_exited) {
    pthread_join((pthread_t)m_handle, NULL);
  }
}

bool Thread::start(ThreadFunction func, void *param) {
  if (m_started)
    return false;

  m_func = func;
  m_param = param;
  m_started = true;
  m_exited = false;

  pthread_t thread_id;
  int result = pthread_create(&thread_id, NULL, ThreadWrapper, this);

  if (result != 0) {
    m_started = false;
    return false;
  }

  m_handle = (uint64_t)thread_id;
  return true;
}

bool Thread::isRunning(void) const { return m_started && !m_exited; }

int Thread::getExitCode(void) const { return m_exitCode; }

bool Thread::wait(int timeout) {
  if (!m_started)
    return true;

  if (timeout < 0) {
    pthread_join((pthread_t)m_handle, NULL);
    m_exited = true;
    return true;
  }

  // POSIX doesn't have timed join, so we use a polling approach
  uint32_t elapsed = 0;
  const uint32_t POLL_INTERVAL = 10; // milliseconds

  while (elapsed < (uint32_t)timeout) {
    if (m_exited)
      return true;

    Thread::sleep(POLL_INTERVAL);
    elapsed += POLL_INTERVAL;
  }

  return m_exited;
}

void Thread::sleep(uint32_t ms) {
#ifdef HAVE_NANOSLEEP
  struct timespec ts;
  ts.tv_sec = ms / 1000;
  ts.tv_nsec = (ms % 1000) * 1000000;
  nanosleep(&ts, NULL);
#else
  usleep(ms * 1000);
#endif
}

void *Thread::ThreadWrapper(void *arg) {
  Thread *thread = (Thread *)arg;

  if (thread->m_func) {
    thread->m_exitCode = thread->m_func(thread->m_param);
  }

  thread->m_exited = true;
  return NULL;
}

} // namespace Umbra
