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
 * \brief   DPVS runtime implementation
 *
 */

#include "umbraDPVSShared.hpp"
#include "runtime/umbraDPVS.hpp"
#include "runtime/umbraQuery.hpp"
#if UMBRA_COMPILER == UMBRA_MSC
#include <new.h>
#else
#include <new>
#endif

#undef THIS
#define THIS(x) ((x*)this)

namespace Umbra
{

//------------------------------------------------------------------------

DPVS::DPVS(void)
{
    for (int i = 0; i < UMBRA_DPVS_SIZE; i++)
        m_mem[i] = 0;
}

//------------------------------------------------------------------------

DPVS::ErrorCode DPVS::init(const UINT8* inBuffer, int inBufferSize)
{
    if (!inBuffer || !inBufferSize)
        return DPVS::ERRORCODE_GENERIC_ERROR;

    if (((UINTPTR)inBuffer)&0x3)
        return DPVS::ERRORCODE_BAD_ALIGN;

    new (this) DPVSRuntime(inBuffer);

    return DPVS::ERRORCODE_OK;
}

//------------------------------------------------------------------------

int DPVS::getListCapacity(void) const
{
    return THIS(DPVSRuntime)->getObjectCount();
}

//------------------------------------------------------------------------

DPVS::ErrorCode DPVS::lookup(IndexList* outIndexList, float inTime) const
{
    if (!outIndexList || outIndexList->getCapacity() < getListCapacity())
        return DPVS::ERRORCODE_LOOKUP_FAILED;

    outIndexList->setSize(0);

    PVSVector* pvs = THIS(DPVSRuntime)->lookup(inTime);

    if (!pvs)
        return DPVS::ERRORCODE_LOOKUP_FAILED;

    int* ptr = outIndexList->getPtr();
    for (int i = 0; i < pvs->getSize(); i++)
    {
        if (pvs->get(i))
            *ptr++ = THIS(DPVSRuntime)->remap(i);
    }
    outIndexList->setSize((int)(ptr-outIndexList->getPtr()));

    return DPVS::ERRORCODE_OK;
}

//------------------------------------------------------------------------

} // namespace Umbra

#undef THIS
