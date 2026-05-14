#pragma once
#ifndef UMBRAQUERYWRAPPER_HPP
#define UMBRAQUERYWRAPPER_HPP

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
 * \brief   Umbra PS3 PPU query wrapper
 *
 */

#include "umbraQueryContext.hpp"
#include "runtime/umbraTome.hpp"

#define UMBRA_MAX_ARGS 16
#define UMBRA_INPUT_BUFFER_LEN 2048

namespace Umbra
{

enum QueryId
{
    QID_QUERY_PORTALVISIBILITY_CAMERA = 0,
    QID_QUERY_PVSVISIBILITY_CAMERA,
    QID_QUERY_PVSVISIBILITY_POINT,
    QID_QUERY_FRUSTUMVISIBILITY,
    QID_QUERY_SHORTESTPATH,
    QID_QUERY_CONNECTEDREGION,
    QID_QUERY_LINESEGMENT,
    QID_MAX
};

enum QueryParamFlags
{
    QueryParam_Input     = 1 << 0,
    QueryParam_Output    = 1 << 1
};

typedef void (*copyParamObject) (void* dst, const void* src);

struct QueryParam
{
    UINT32              m_flags;
    size_t              m_size;
    void*               m_origAddr;
    UINT32              m_offset;
    copyParamObject     m_copy;
};

struct QueryDataShared
{
    QueryId         id;
    UINTPTR         tome;
    UINTPTR         collection;
    UINT32          gateVectorOfs;
    UINT32          gateVectorSize;
    int             paramCount;
    QueryParam      params[UMBRA_MAX_ARGS];
    UINT32          inSize;
    UINT32          inOutSize;
    UINT32          largeSize;
};

template<class Type, bool IsApiType> struct ParamObjectCopy;

template<class Type>
struct ParamObjectCopy<Type, true>
{
    static void copy(Type* dst_, const Type* src_)
    {
        typedef typename ApiClassTraits<Type>::ImpClass ImpType;
        ImpType* dst = GetApiObjectImplementation(dst_);
        const ImpType* src = GetApiObjectImplementation(src_);
        *dst = *src;
    }
};

template <class Type>
struct ParamObjectCopy<Type, false>
{
    static void copy (Type* dst, const Type* src)
    {
        *dst = *src;
    }
};

class QueryWrapper
{
public:

    QueryWrapper (QueryContext& ctx): m_query(&ctx)
    {
        // alloc a tomecollection for aligned copy
        m_collection = (TomeCollection*)UMBRA_HEAP_ALLOC_16(ctx.getAllocator(), UMBRA_ALIGN(sizeof(TomeCollection), 16));
        // can grab rest of workmem
        m_size = (UINT32)ctx.getAllocator()->available();
        m_data = (QueryDataShared*)UMBRA_HEAP_ALLOC(ctx.getAllocator(), m_size);
        m_data->paramCount = 0;
    }

    ~QueryWrapper (void)
    {
        UMBRA_HEAP_DELETE(m_query->getAllocator(), m_data);
        UMBRA_HEAP_FREE_16(m_query->getAllocator(), m_collection);
    }

    void put (const Visibility* p)
    {
        put(p->getOutputObjects(), QueryParam_Input | QueryParam_Output);
        put(p->getInputObjects(), QueryParam_Input);
        put(p->getOutputClusters(), QueryParam_Input | QueryParam_Output);
        // occlusion buffers passed by pointer
        m_ptrParam = p->getOutputBuffer();
        put(&m_ptrParam, QueryParam_Input);
        m_ptrParam2 = (void*)p->getInputBuffer();
        put(&m_ptrParam2, QueryParam_Input);
        m_ptrParam3 = (void*)p->getOutputObjectDistances();
        put(&m_ptrParam3, QueryParam_Input);
    }

    void put (const CameraTransform* cam, UINT32 flags)
    {
        // early convert legacy camera transforms
        const ImpCameraTransform* icam = IMPL(cam);
        ((ImpCameraTransform*)icam)->update();
        put((void*)cam, sizeof(CameraTransform), flags, getCopyFunc(cam));
    }

    template <class T> void put (T* arg, UINT32 flags)
    {
        put((void*)arg, sizeof(T), flags, getCopyFunc(arg));
    }


#if UMBRA_OS == UMBRA_PS3 && UMBRA_ARCH == UMBRA_PPC
    void dispatch (QueryId id);
    static void deinit ();
#else
    void dispatch (QueryId)
    {
        UMBRA_ASSERT(!"invalid configuration");
    }
    static void deinit()
    {
        UMBRA_ASSERT(!"invalid configuration");
    }
#endif

private:

    template <class T> static copyParamObject getCopyFunc (T*)
    {
        return (copyParamObject)ParamObjectCopy<T, ApiClassTraits<T>::Declared != 0>::copy;
    }

    template <class T> static copyParamObject getCopyFunc (const T* arg)
    {
        return getCopyFunc((T*)arg);
    }

    void put (void* addr, size_t size, UINT32 flags, copyParamObject copy)
    {
        if (m_data->paramCount < UMBRA_MAX_ARGS)
        {
            QueryParam& p   = m_data->params[m_data->paramCount];
            p.m_flags       = flags;
            p.m_size        = size;
            p.m_origAddr    = addr;
            p.m_offset      = (UINT32)-1;
            p.m_copy        = copy;
        }

        m_data->paramCount++;
    }

    bool applyParams (void);
    void postprocessParams (void);
    void* getCur (void) { return ((UINT8*)m_data) + m_ofs; }

    QueryContext*       m_query;
    TomeCollection*     m_collection;
    void*               m_ptrParam;
    void*               m_ptrParam2;
    void*               m_ptrParam3;
    QueryDataShared*    m_data;
    UINT32              m_size;
    UINT32              m_ofs;
};

} // namespace Umbra

#endif // UMBRAQUERYWRAPPER_HPP
