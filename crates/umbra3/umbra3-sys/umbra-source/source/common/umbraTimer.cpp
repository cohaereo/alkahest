// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraTimer.hpp"
#include "umbraOs.hpp"

using namespace Umbra;

Timer& Timer::instance()
{
    static Timer g_TimerInstance;
    return g_TimerInstance;
}

Timer::InternalTimer* Timer::getTimer (const char *name)
{
    String strName(name, getAllocator());
    if (!m_timers.contains(strName))
    {
        return m_timers.insert(strName, InternalTimer());
    }
    else
    {
        return m_timers.get(strName);
    }
}

void Timer::startTimer(const char *name)
{
    getTimer(name)->start = OS::getCurrentTime();
}

void Timer::stopTimer(const char *name)
{
    InternalTimer* timer = getTimer(name);
    timer->acc += (OS::getCurrentTime() - timer->start);
    timer->start = 0.0;
}

void Timer::resetTimer(const char *name)
{
    getTimer(name)->acc = 0.0;
}

double Timer::getTimerValue (const char *name)
{
    return (double)getTimer(name)->acc;
}

double Timer::getTime (void)
{
    return (double)OS::getCurrentTime();
}
