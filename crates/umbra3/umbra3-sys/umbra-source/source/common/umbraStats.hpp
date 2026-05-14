#pragma once

#include "umbraHash.hpp"
#include "umbraString.hpp"
#include "umbraTimer.hpp"

namespace Umbra
{

class Stats
{
public:
    Stats(Allocator* a) : m_stats(a), m_timers(a) {}

    void add(const char* s, double v)
    {
        set(s, get(s) + v);
    }

    double get(const char* ss)
    {
        String s(ss, m_stats.getAllocator());
        double* val = m_stats.get(s);
        return val ? *val : 0.0;
    }

    void set(const char* ss, double d)
    {
        String s(ss, m_stats.getAllocator());
        double* val = m_stats.get(s);
        if (val)
            *val = d;
        else
            m_stats.insert(s, d);
    }

    void startTimer(const char* ss)
    {
        String s(ss, m_stats.getAllocator());
        UMBRA_ASSERT(!m_timers.contains(s));
        m_timers.insert(s, Timer::getTime());
    }

    void endTimer(const char* ss)
    {
        String s(ss, m_stats.getAllocator());
        UMBRA_ASSERT(m_timers.contains(s));
        add(ss, Timer::getTime() - *m_timers.get(s));
        m_timers.remove(s);
    }

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, m_stats);
    }

    Stats& operator+=(const Stats& stats)
    {
        Array<String> keys(m_stats.getAllocator());
        Array<double> values(m_stats.getAllocator());

        stats.m_stats.getKeyArray(keys);
        stats.m_stats.getValueArray(values);

        for (int i = 0; i < keys.getSize(); i++)
            add(keys[i].toCharPtr(), values[i]);

        return *this;
    }

    void printAll() const
    {
        Array<String> keys(m_stats.getAllocator());
        Array<double> values(m_stats.getAllocator());

        m_stats.getKeyArray(keys);
        m_stats.getValueArray(values);

        for (int i = 0; i < keys.getSize(); i++)
            printf("%15s %f\n", keys[i].toCharPtr(), values[i]);
    }

private:
    Hash<String, double> m_stats;
    Hash<String, double> m_timers;
};

}
