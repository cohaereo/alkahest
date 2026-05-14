// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "DebugCollector.hpp"

using namespace Umbra;

void DebugCollector::setFilterSpec(const char* s)
{
    strncpy(filterSpec, s, MAX_FILTER_SPEC_LENGTH);
}

bool DebugCollector::pushActive(const char* s)
{
    UMBRA_ASSERT(s && *s && strlen(currentFilter)+1+strlen(s) < MAX_FILTER_LENGTH);

    const char* end = currentFilter + MAX_FILTER_LENGTH;
    char* p = currentFilter;
    while (*p && p < end)
        p++;

    if (p+1 >= end)
    {
        activeFlag = false;
        return false;
    }

    if (p > currentFilter)
        *p++ = '.';

    while (*s && p < end)
        *p++ = *s++;
    *p = 0;

    if (!updateActivity())
    {
        popActive();
        return false;
    }
    return true;
}

void DebugCollector::popActive()
{
    UMBRA_ASSERT(strlen(currentFilter) > 0);

    const char* start = currentFilter;

    char* p = currentFilter + strlen(currentFilter) - 1;

    while (p > start && *p != '.')
        p--;
    if (p >= start)
        *p = 0;

    updateActivity();
}

bool DebugCollector::updateActivity()
{
    const char* p = filterSpec;

    bool continueFlag = false;
    activeFlag = false;

    while (*p)
    {
        while (*p == ' ')
            p++;

        const char* q = currentFilter;

        while (*p && *q && *p == *q)
        {
            p++;
            q++;
        }

        if (!*p || *p == ' ')
        {
            continueFlag = true;
            activeFlag = true;
            break;
        }

        if (*p == '.' && *q == 0)
            continueFlag = true;

        while (*p && *p != ' ')
            p++;
    }

    return continueFlag;
}
