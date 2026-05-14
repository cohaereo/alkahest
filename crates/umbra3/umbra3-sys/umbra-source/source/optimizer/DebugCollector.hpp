// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Memory.hpp>
#include <standard/Vector.hpp>
#include <standard/AxisExtents.hpp>

namespace Umbra
{

class GraphicsContainer;

class DebugCollector
{
public:
    DebugCollector(MemoryManager& mm) : mm(mm), gc(*(GraphicsContainer*)0), activeFlag(false) { filterSpec[0] = currentFilter[0] = 0; }
    DebugCollector(MemoryManager& mm, GraphicsContainer& gc) : mm(mm), gc(gc), activeFlag(false) { filterSpec[0] = currentFilter[0] = 0; }
    ~DebugCollector() {}

    void setFilterSpec(const char* s);

    bool pushActive(const char* s);
    void popActive ();
    bool isActive  () const { return activeFlag; }

    MemoryManager&     getMemoryManager()     { return mm; }
    GraphicsContainer& getGraphicsContainer() { return gc; }

private:
    bool updateActivity();

    enum
    {
        MAX_FILTER_SPEC_LENGTH = 512,
        MAX_FILTER_LENGTH      = 64
    };

    MemoryManager&     mm;
    GraphicsContainer& gc;
    char filterSpec[MAX_FILTER_SPEC_LENGTH];
    char currentFilter[MAX_FILTER_LENGTH];
    bool activeFlag;

    DebugCollector& operator= (const DebugCollector&);
};

}
