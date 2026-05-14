// Copyright (c) 2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com
#ifndef UMBRACLOUDCOMPUTATION_HPP
#define UMBRACLOUDCOMPUTATION_HPP

/*!
 * \file
 * \brief   Cloud computation interface
 */

#include "umbraScene.hpp"
#include "umbraComputationParams.hpp"
#include "umbraComputation.hpp"

namespace Umbra
{

class ImpCloudComputation;

/*
 * WARNING! This interface is still subject to change.
 * Use at your own risk. You have been warned.
 */

class UMBRADEC CloudComputation : public Computation
{
public:
    // Params specific to running the computation in the cloud
    struct Params : public Computation::Params
    {
        Params (void)
            : endPointUrl("https://api.umbracloud.com/")
            , apiKey(NULL)
        {}

        const char* endPointUrl;
        const char* apiKey;
    };

    virtual Result              waitForResult   (Allocator* tomeAllocator = NULL, unsigned int timeoutMs = (unsigned int)-1);
    virtual void                requestAbort    (void);
    virtual void                visualize       (class DebugRenderer* debugRenderer) const;

    static  CloudComputation*   create          (const Params& cloudParams);
    virtual void                release         (void);

protected:

    CloudComputation            (void);                     // not allowed
    CloudComputation            (const CloudComputation&);  // not allowed
    CloudComputation& operator= (const CloudComputation&);  // not allowed
    virtual ~CloudComputation   ();                         // not allowed
    ImpCloudComputation* m_imp;
};

} // namespace Umbra

#endif
