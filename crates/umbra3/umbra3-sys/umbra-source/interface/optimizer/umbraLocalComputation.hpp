// Copyright (c) 2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com
#ifndef UMBRALOCALCOMPUTATION_HPP
#define UMBRALOCALCOMPUTATION_HPP

/*!
 * \file
 * \brief   Local computation interface
 */

#include "umbraScene.hpp"
#include "umbraComputation.hpp"

namespace Umbra
{

class ImpLocalComputation;

/*
 * WARNING! This interface is still subject to change.
 * Use at your own risk. You have been warned.
 */

class UMBRADEC LocalComputation : public Computation
{
public:
    // Params specific to running the computation locally
    struct Params : public Computation::Params
    {
        Params (void)
            : runAsProcessPath(NULL)
            , memUsageLimitMegs(-1)
            , silent(false)
            , cacheSizeMegs(-1)
            , numThreads(-1)
            , licenseKey(NULL)
            , tempPath(NULL)
            , tempFilePrefix(NULL)
        {}


        const char* runAsProcessPath;
        int			memUsageLimitMegs;
        bool		silent;
        int			cacheSizeMegs;
        int			numThreads;
        const char*	licenseKey;
        const char* tempPath;
        const char* tempFilePrefix;
    };

    // \todo timeout
    virtual Result              waitForResult   (Allocator* tomeAllocator = NULL, unsigned int timeoutMs = (unsigned int)-1);
    virtual void                requestAbort    (void);
    virtual void                visualize       (class DebugRenderer* debugRenderer) const;

    static  LocalComputation*   create          (const Params& localParams);
    virtual void                release         (void);

protected:

    LocalComputation            (void);                     // not allowed
    LocalComputation            (const LocalComputation&);  // not allowed
    LocalComputation& operator= (const LocalComputation&);  // not allowed
    virtual ~LocalComputation   ();                         // not allowed
    ImpLocalComputation* m_imp;
};

} // namespace Umbra


#endif