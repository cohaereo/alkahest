#pragma once
#ifndef UMBRAQUERYJOB_H
#define UMBRAQUERYJOB_H

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
 * \brief   Umbra spurs job(s)
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraQueryWrapper.hpp"
#include "umbraVector.hpp"
#include "umbraQuery.hpp"

#include <cell/spurs.h>

#ifdef UMBRA_CONNECTIVITY_JOB
#include "umbraQueryContext.hpp"
#include "umbraConnectivity.hpp"
#endif

#if defined(UMBRA_DEBUG) && !defined(UMBRA_FORCE_RELEASE_SPURS_JOB)
#   define JOBHEADER(x) _binary_job_umbraspursruntime_ ## x ## _d_jobbin2_jobheader
#else
#   define JOBHEADER(x) _binary_job_umbraspursruntime_ ## x ## _jobbin2_jobheader
#endif

#define UMBRA_PORTAL_JOBHEADER JOBHEADER(portal)
#define UMBRA_CONNECTIVITY_JOBHEADER JOBHEADER(connectivity)
#define UMBRA_FRUSTUM_JOBHEADER JOBHEADER(frustum)

/* addresses of spurs jobs */
extern "C" {
    extern const CellSpursJobHeader UMBRA_PORTAL_JOBHEADER;
    extern const CellSpursJobHeader UMBRA_CONNECTIVITY_JOBHEADER;
    extern const CellSpursJobHeader UMBRA_FRUSTUM_JOBHEADER;
};

namespace Umbra
{

class QueryArgReader
{
public:
    QueryArgReader (QueryDataShared* args, UINT8* outMem): args(args), outMem(outMem)
    {
        largeOfs = args->inSize + args->inOutSize;
    }

    template <class T> T* get (int idx)
    {
        UMBRA_ASSERT(idx < args->paramCount);
        QueryParam& p = args->params[idx];
        if (p.m_offset == (UINT32)-1)
            return NULL;
        if (p.m_offset >= largeOfs)
            return (T*)(outMem + p.m_offset - largeOfs);
        return (T*)(((UINT8*)args) + p.m_offset);
    }

    QueryDataShared* args;
    UINT8* outMem;
    UINT32 largeOfs;
};
    
class QuerySpursJob : public CellSpursJobHeader
{
private:
    // this must be 1st
    CellSpursJobInputList m_inputList;
public:
    static const size_t kSize = 64;

#if UMBRA_ARCH != UMBRA_SPU
    int initialize (void* inout, size_t inoutSize, size_t ctxSize, const CellSpursJobHeader& jobHeader)
    {
        memset(this, 0, kSize);

        // Fill header
        *(CellSpursJobHeader*)this = jobHeader;
        // Request scratch
        sizeScratch = ctxSize / 16;
        // Request buffer checks
        jobType |= CELL_SPURS_JOB_TYPE_MEMORY_CHECK;
        useInOutBuffer = 1;
        sizeInOrInOut = inoutSize;
        sizeDmaList = sizeof(m_inputList);

        return cellSpursJobGetInputList(&m_inputList.asUint, inoutSize, (uintptr_t)inout);
    }
#endif
    
#if UMBRA_ARCH == UMBRA_SPU
    void execute (CellSpursJobContext2* ctx)
    {
        QueryExt* query = (QueryExt*)ctx->sBuffer;
        QueryDataShared* data = (QueryDataShared*)ctx->ioBuffer;
        QueryArgReader args(data, (UINT8*)(query + 1));

        // assemble query object and set state
        if (data->collection)
            query->init((const TomeCollection*)data->collection);
        else
            query->init((const Tome*)data->tome);

        if (data->gateVectorSize)
        {
            GateStateVector gates(((UINT8*)data) + data->gateVectorOfs, data->gateVectorSize, false);
            query->setGateStates(&gates);
        }

        UINT32 retvalue = (UINT32)Query::ERROR_UNSUPPORTED_OPERATION;

        // run appropriate query
        // \todo transfer QueryOutput struct as a whole instead of individual members!!
#ifdef UMBRA_PORTAL_JOB
        if (data->id == QID_QUERY_PORTALVISIBILITY_CAMERA)
        {
            UINT32* flags = args.get<UINT32>(0);
            Visibility params;
            params.setOutputObjects(args.get<IndexList>(1));
            params.setInputObjects(args.get<IndexList>(2));
            params.setOutputClusters(args.get<IndexList>(3));
            params.setOutputBuffer(*args.get<OcclusionBuffer*>(4));
            params.setInputBuffer(*args.get<const OcclusionBuffer*>(5));
            params.setOutputObjectDistances(*args.get<float*>(6));
            CameraTransform* camera = args.get<CameraTransform>(7);
            float* distance = args.get<float>(8);
            float* clusterThreshold = args.get<float>(9);
            const ObjectDistanceParams* objDist = args.get<const ObjectDistanceParams>(10);
            int* threadId = args.get<int>(11);
            int* numThreads = args.get<int>(12);
            int* xSplits = args.get<int>(13);
            retvalue = query->queryPortalVisibility(*flags, params, *camera, *distance, *clusterThreshold, objDist, *threadId, *numThreads, *xSplits);
        }
#endif

#ifdef UMBRA_FRUSTUM_JOB
        if (data->id == QID_QUERY_FRUSTUMVISIBILITY)
        {
            UINT32* flags = args.get<UINT32>(0);
            Visibility params;
            params.setOutputObjects(args.get<IndexList>(1));
            params.setInputObjects(args.get<IndexList>(2));
            params.setOutputClusters(args.get<IndexList>(3));
            params.setOutputBuffer(*args.get<OcclusionBuffer*>(4));
            params.setInputBuffer(*args.get<const OcclusionBuffer*>(5));
            params.setOutputObjectDistances(*args.get<float*>(6));
            CameraTransform* camera = args.get<CameraTransform>(7);
            float* distance = args.get<float>(8);
            const ObjectDistanceParams* objDist = args.get<const ObjectDistanceParams>(9);
            int* threadId = args.get<int>(10);
            int* numThreads = args.get<int>(11);
            retvalue = query->queryFrustumVisibility(*flags, params, *camera, *distance, objDist, *threadId, *numThreads);
        }
#endif

#ifdef UMBRA_CONNECTIVITY_JOB

        if (data->id == QID_QUERY_SHORTESTPATH)
        {
            UINT32* flags = args.get<UINT32>(0);
            Path* p = args.get<Path>(1);
            Vector3* start = args.get<Vector3>(2);
            Vector3* end = args.get<Vector3>(3);
            retvalue = query->queryShortestPath(*flags, *p, *start, *end);

        }
        else if (data->id == QID_QUERY_CONNECTEDREGION)
        {
            UINT32* flags = args.get<UINT32>(0);
            IndexList* clusters = args.get<IndexList>(1);
            int* cluster = args.get<int>(2);
            Vector3* pt = args.get<Vector3>(3);
            float* distance = args.get<float>(4);
            float* bound = args.get<float>(5);
            FloatList* pathDist = args.get<FloatList>(6);
            FloatList* pathModifiers = args.get<FloatList>(7);
            IndexList* entryPortals = args.get<IndexList>(8);
            retvalue = query->queryConnectedRegion(*flags, *clusters, *cluster, *pt, *distance, bound, pathDist, pathModifiers, entryPortals);
        } 
        else if (data->id == QID_QUERY_LINESEGMENT)
        {
            LineSegmentQuery* queries = args.get<LineSegmentQuery>(0);
            int* count = args.get<int>(1);
            retvalue = query->queryLineSegment(queries, *count);
        }
#endif

        UINT32* errPtr = args.get<UINT32>(data->paramCount - 1);
        *errPtr = retvalue;
        if (data->largeSize)
            cellDmaLargePut(args.outMem, getOutAddr(data->inSize + data->inOutSize), data->largeSize, ctx->dmaTag, 0, 0);
        cellDmaPut((UINT8*)data + data->inSize, getOutAddr(data->inSize), data->inOutSize, ctx->dmaTag, 0, 0);        
    }
#endif

    uintptr_t getOutAddr (size_t offset)
    {
        return m_inputList.asInputList.eal + offset;
    }

private:
    uint8_t padding[kSize - sizeof(CellSpursJobHeader)
                          - sizeof(CellSpursJobInputList)];
};

        
} // namespace Umbra

#endif // UMBRAQUERYJOB_H
