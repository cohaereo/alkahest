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
 * \brief   Process Library
 *
 */

namespace Umbra
{
    template<class T>
    ProcessSharedMemory<T>::ProcessSharedMemory() :
        p(0)
    {}

    template<class T>
    ProcessSharedMemory<T> processAlloc(String identifier)
    {
        bool first = false;
        ProcessSharedMemory<T> sm;

        T* t = (T*)processAlloc(identifier, sizeof(T), first, &sm.m_impl);
        if(first && t)
            memset(t, 0, sizeof(T));

        sm.p = t;

        return sm;
    }

    template<class T>
    void processFree(ProcessSharedMemory<T> &sharedMemory)
    {
        processFree(sharedMemory.p, sharedMemory.m_impl);
    }

} // namespace Umbra
