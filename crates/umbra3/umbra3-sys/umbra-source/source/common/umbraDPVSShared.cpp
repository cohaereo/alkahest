/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   DPVS shared implementation
 *
 */

#include "umbraDPVSShared.hpp"

namespace Umbra
{

//------------------------------------------------------------------------

struct DPVSContext
{
    DPVSContext(void)
    :   allocator   (NULL)
    ,   flags       (0)
    {
    }

    Allocator* allocator;
    UINT32     flags;

    static DPVSContext& get(void)
    {
        static DPVSContext context;
        return context;
    }
};

//------------------------------------------------------------------------

void DPVSSetAllocator(Allocator* allocator)
{
    DPVSContext::get().allocator = allocator;
}

//------------------------------------------------------------------------

Allocator* DPVSGetAllocator(void)
{
    UMBRA_ASSERT(DPVSContext::get().allocator);
    return DPVSContext::get().allocator;
}

//------------------------------------------------------------------------

void DPVSSetAllocatorFlags(UINT32 flags)
{
    DPVSContext::get().flags = flags;
}

//------------------------------------------------------------------------

UINT32 DPVSGetAllocatorFlags(void)
{
    return DPVSContext::get().flags;
}

//------------------------------------------------------------------------

void DPVSRuntime::init(const UINT8* inBase)
{
    base       = inBase;
    cacheIndex = -1;

    DPVSRuntimeData* data = (DPVSRuntimeData*)inBase;
    cachePVS.set(data->getObjectCount(base), mem);
}

//------------------------------------------------------------------------

PVSVector* DPVSRuntime::lookup(float time)
{
    const DPVSRuntimeData* data = (DPVSRuntimeData*)base;
    return data->lookup(this, time);
}

//------------------------------------------------------------------------

int DPVSRuntime::getObjectCount(void) const
{
    const DPVSRuntimeData* data = (DPVSRuntimeData*)base;
    return data->getObjectCount(base);
}

//------------------------------------------------------------------------

int DPVSRuntime::remap(int index) const
{
    const DPVSRuntimeData* data = (DPVSRuntimeData*)base;
    return data->remap(base, index);
}

//------------------------------------------------------------------------

} // namespace Umbra