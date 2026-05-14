// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRADPVSBUILDER_HPP
#define UMBRADPVSBUILDER_HPP

/*!
 * \file
 * \brief   Directional PVS interface
 */

#include "umbraDefs.hpp"
#include "umbraPlatform.hpp"
#include "umbraScene.hpp"

#define UMBRA_DPVS_MAX_HARDWARE_THREADS 64

namespace Umbra
{

//------------------------------------------------------------------------

class DPVSInputPath
{
public:

    UMBRADEC            DPVSInputPath   (void);
    UMBRADEC            ~DPVSInputPath  (void);

    UMBRADEC void       init            (const PlatformServices& inPlatformServices, const UINT8* inBuffer = NULL, int inBufferSize = 0);
    UMBRADEC void       addNode         (float inTime, const Vector3& inDirection);
    UMBRADEC void       setNodeArray    (const float* inTimeArray, const Vector3* inDirectionArray, int inArraySize);
    UMBRADEC int        getNodeCount    (void) const;
    UMBRADEC void       getNode         (float& outTime, Vector3& outDirection, int inIndex) const;
    UMBRADEC int        getBuffer       (UINT8* outBuffer, int outBufferSize);
    UMBRADEC void       setBuffer       (const UINT8* inBuffer, int inBufferSize);

private:

    friend class ImpDPVSBuilder;
    class ImpDPVSInputPath* m_imp;
};

//------------------------------------------------------------------------

struct DPVSParams
{
    UMBRADEC DPVSParams(int inMaxThreads = UMBRA_DPVS_MAX_HARDWARE_THREADS)
    :   maxThreads  (inMaxThreads)
    {
        if (maxThreads <= 0) maxThreads = 1;
        if (maxThreads > UMBRA_DPVS_MAX_HARDWARE_THREADS) maxThreads = UMBRA_DPVS_MAX_HARDWARE_THREADS;
    }

    int               maxThreads;
};

//------------------------------------------------------------------------

class DPVSResult
{
public:

    UMBRADEC            DPVSResult	(void);
    UMBRADEC            ~DPVSResult	(void);

    UMBRADEC bool       serialize	(OutputStream& out) const;

private:

    friend class ImpDPVSBuilder;
    class ImpDPVSResult* m_imp;

};

//------------------------------------------------------------------------

class DPVSOutputWriter
{
public:

    UMBRADEC            DPVSOutputWriter (void);
    UMBRADEC            ~DPVSOutputWriter (void);


    UMBRADEC int        getBuffer   (UINT8* outBuffer, int outBufferSize);

private:

    friend class ImpDPVSBuilder;
    class ImpDPVSOutputWriter* m_imp;
};

//------------------------------------------------------------------------

class DPVSBuilder
{
public:

    UMBRADEC            DPVSBuilder     (void);
    UMBRADEC            ~DPVSBuilder    (void);

    UMBRADEC void       init            (const PlatformServices& inPlaformServices);

    UMBRADEC bool       build           (DPVSResult& result,
                                         const Scene& inScene,
                                         const DPVSInputPath* inInputPathArray,
                                         int inInputPathCount,
                                         const DPVSParams& inParams);

    UMBRADEC bool       loadResult      (DPVSResult& result, InputStream& in);

    UMBRADEC bool       generateOutput  (DPVSOutputWriter& out,
                                         const DPVSResult& result,
                                         const class Tome* tome,
                                         int maxCells = -1);

private:

    class ImpDPVSBuilder*  m_imp;
};

//------------------------------------------------------------------------

} // namespace Umbra

#endif // UMBRADPVSBUILDER_HPP
