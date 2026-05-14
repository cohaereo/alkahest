// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRATIMER_HPP
#define UMBRATIMER_HPP

/*!
 * \file    umbraTimer.hpp
 * \brief   Umbra timing utilities
 */


#include "umbraHash.hpp"
#include "umbraString.hpp"

namespace Umbra
{

class Timer: public Base
{
public:
    static Timer &instance(void);

    Timer(Allocator* a): Base(a), m_timers(a) {}

    void   startTimer       (const char *name);
    void   stopTimer        (const char *name);
    void   resetTimer       (const char *name);

    // values are in seconds
    double getTimerValue    (const char *name);
    static double getTime();

private:

    struct InternalTimer
    {
        InternalTimer(void): start(0LL), acc(0LL) {}

        double start;
        double acc;
    };

              Timer     (void) {}
              Timer     (const Timer&);
    Timer&    operator= (const Timer&);

    Hash<String, InternalTimer> m_timers;
    InternalTimer* getTimer(const char *name);
};

class ScopedTimer
{
public:
    ScopedTimer(const char *name, bool reset = false)
        : m_name(name)
    {
        if (reset)
            Timer::instance().resetTimer(m_name.toCharPtr());
        Timer::instance().startTimer(m_name.toCharPtr());
    }

    ~ScopedTimer()
    {
        Timer::instance().stopTimer(m_name.toCharPtr());
    }

private:
    String m_name;
};

} // namespace Umbra

#endif
