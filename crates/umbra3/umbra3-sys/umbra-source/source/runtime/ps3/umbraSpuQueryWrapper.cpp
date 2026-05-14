/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   PPU -> SPU query wrapper
 *
 */

#include "umbraPrivateDefs.hpp"

#if UMBRA_OS == UMBRA_PS3 && UMBRA_ARCH == UMBRA_PPC

#include "umbraQueryWrapper.hpp"
#include "umbraQueryContext.hpp"
#include "umbraQuery.hpp" // for error codes
#include "umbraQueryJob.hpp"
#include <spu_printf.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define USE_SPU_PRINTF
#define JOB_QUEUE_DEPTH 1024
#define NUM_THREADS 2

using namespace Umbra;
using namespace ::cell::Spurs;

typedef JobQueue::JobQueue<JOB_QUEUE_DEPTH> JobQueueType;

// global spurs context pointer, defined as weak symbol so that
// users can provide their own
cell::Spurs::Spurs2* g_spurs __attribute__((weak)) = NULL;

static bool g_umbraOwnsSpurs = false;

namespace Umbra
{

enum JobType
{
    JOBTYPE_PORTAL = 0,
    JOBTYPE_CONNECTIVITY,
    JOBTYPE_FRUSTUM,
    JOBTYPE_MAX
};

struct WrapperThreadCtx
{
    JobQueue::Port2*        port;
    QuerySpursJob*          jobDesc;
};

struct WrapperCtx
{
    bool                    initialized;
    JobQueueType*           jobQueue;
    size_t                  checked[JOBTYPE_MAX];
    WrapperThreadCtx        threadCtx[NUM_THREADS];
};

static WrapperCtx g_wrapper = { 0, };

#define UMBRA_SPURS_ERROR(error) {error, #error}
struct SpursJobError
{
    int         error;
    const char* description;
} errors[] =
{
    UMBRA_SPURS_ERROR(CELL_SPURS_JOB_ERROR_INVAL),
    UMBRA_SPURS_ERROR(CELL_SPURS_JOB_ERROR_ALIGN),
    UMBRA_SPURS_ERROR(CELL_SPURS_JOB_ERROR_NULL_POINTER),
    UMBRA_SPURS_ERROR(CELL_SPURS_JOB_ERROR_MEMORY_SIZE),
    UMBRA_SPURS_ERROR(CELL_OK)
};

static const char* jobTypeStr(JobType type)
{
    switch (type)
    {
    case JOBTYPE_PORTAL: return "portal";
    case JOBTYPE_CONNECTIVITY: return "connectivity";
    case JOBTYPE_FRUSTUM: return "frustum";
    default: return "unknown";
    }
}

static const CellSpursJobHeader& jobTypeHeader(JobType type)
{
    switch (type)
    {
    case JOBTYPE_PORTAL: return UMBRA_PORTAL_JOBHEADER;
    case JOBTYPE_FRUSTUM: return UMBRA_FRUSTUM_JOBHEADER;
    default: return UMBRA_CONNECTIVITY_JOBHEADER;
    }
}

static JobType jobTypeForId(QueryId id)
{
    switch (id)
    {
    case QID_QUERY_PORTALVISIBILITY_CAMERA:
        return JOBTYPE_PORTAL;
    case QID_QUERY_FRUSTUMVISIBILITY:
        return JOBTYPE_FRUSTUM;
    case QID_QUERY_SHORTESTPATH:
    case QID_QUERY_CONNECTEDREGION:
    case QID_QUERY_LINESEGMENT:
        return JOBTYPE_CONNECTIVITY;
    default:
        return JOBTYPE_MAX;
    }
}

static SpursJobError* findError(int code)
{
    SpursJobError* err = errors;
    while (err->error != CELL_OK && err->error != code)
        err++;

    if (err->error == CELL_OK)
        return NULL;

    return err;
}

static void reportError(const char* phase, JobType type, int ret)
{
    SpursJobError* err = findError(ret);
    printf("%s failed! Type: %s, Error: %s (0x%x)\n",
        phase,
        jobTypeStr(type),
        err ? err->description : "UNKNOWN",
        ret);
}

static bool wrapperInit (WrapperCtx* ctx)
{
    if (ctx->initialized)
        return true;

	int ret;

    if (!g_spurs)
	{
#if defined(USE_SPU_PRINTF)
#define SPU_PRINTF_PRIORITY 999
        spu_printf_initialize(SPU_PRINTF_PRIORITY, NULL);
#endif
        SpursAttribute attr;

#define SPURS_NUM_SPU       5
#define SPURS_PPU_PRIORITY  2
#define SPURS_SPU_PRIORITY  100
#define NAME                "umbraRuntime"

		ret = SpursAttribute::initialize(&attr, SPURS_NUM_SPU, SPURS_SPU_PRIORITY, SPURS_PPU_PRIORITY, false);
        if (ret != CELL_OK)
            return false;

        ret = attr.setNamePrefix(NAME, strlen(NAME));
        if (ret != CELL_OK)
            return false;

		ret = attr.setSpuThreadGroupType(SYS_SPU_THREAD_GROUP_TYPE_EXCLUSIVE_NON_CONTEXT);
		if (ret != CELL_OK)
			return false;

        ret = attr.enableSpuPrintfIfAvailable();
        UMBRA_ASSERT(ret == CELL_OK);

        // Create and initialize Spurs
        g_spurs = (Spurs2*)memalign(Spurs2::kAlign, Spurs2::kSize);
        if (!g_spurs)
            return false;

        ret = Spurs2::initialize(g_spurs, &attr);
        if (ret != CELL_OK)
            return false;

        g_umbraOwnsSpurs = true;
	}

    // Create and initialize job queue
    using namespace ::cell::Spurs::JobQueue;

    ctx->jobQueue = (JobQueueType*)memalign(JobQueueType::kAlign, JobQueueType::kSize);
    if (!ctx->jobQueue)
        return false;

    // \todo [jasin 2011-02-18] Verify these
    uint8_t priority[8] = { 8, 0, 0, 0, 0, 0, 0, 0 };

    ret = JobQueueType::create(ctx->jobQueue, g_spurs, NAME, SPURS_NUM_SPU, priority);
    if (ret != CELL_OK)
        return false;

	for (int i = 0; i < NUM_THREADS; ++i)
	{
        WrapperThreadCtx* thread = &ctx->threadCtx[i];

        // Create and initialize port
		thread->port = (Port2*)memalign(CELL_SPURS_JOBQUEUE_PORT2_ALIGN, CELL_SPURS_JOBQUEUE_PORT2_SIZE);
		UMBRA_ASSERT(thread->port);
		if (!thread->port)
            return false;

		ret = Port2::create(thread->port, ctx->jobQueue);
        UMBRA_ASSERT(ret == CELL_OK);
        if (ret != CELL_OK)
            return false;

		thread->jobDesc = (QuerySpursJob*)memalign(128, sizeof(QuerySpursJob));
		UMBRA_ASSERT(thread->jobDesc);
		if (!thread->jobDesc)
            return false;
	}

    ctx->initialized = true;
    return true;
}

static void wrapperDeinit(WrapperCtx* ctx)
{
    for (int i = 0; i < NUM_THREADS; ++i)
    {
        WrapperThreadCtx* thread = &ctx->threadCtx[i];
        thread->port->destroy();
    }
    ctx->jobQueue->shutdown();
    int exitCode;
    ctx->jobQueue->join(&exitCode);
    free(ctx->jobQueue);

    if (g_umbraOwnsSpurs)
    {
        g_spurs->finalize();
        free(g_spurs);
        g_spurs = NULL;
        g_umbraOwnsSpurs = false;
    }

    ctx->initialized = false;
}

} // namespace Umbra

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

bool QueryWrapper::applyParams (void)
{
    if (m_data->paramCount >= UMBRA_MAX_ARGS)
        return false;

    // input only
    for (int i = 0; i < m_data->paramCount; i++)
    {
        QueryParam& p = m_data->params[i];
        if (!p.m_origAddr || (p.m_flags != QueryParam_Input))
            continue;
        p.m_offset = m_ofs;
        p.m_copy(getCur(), p.m_origAddr);
        m_ofs += UMBRA_ALIGN(p.m_size, 16);
    }
    m_data->inSize = m_ofs;

    // in&out
    for (int i = 0; i < m_data->paramCount; i++)
    {
        QueryParam& p = m_data->params[i];
        if (!p.m_origAddr || (p.m_flags == QueryParam_Input))
            continue;
        if (p.m_size >= (16 * 1024))
            continue;
        if (p.m_flags & QueryParam_Input)
            p.m_copy(getCur(), p.m_origAddr);
        p.m_offset = m_ofs;
        m_ofs += UMBRA_ALIGN(p.m_size, 16);
    }
    m_data->inOutSize = m_ofs - m_data->inSize;

    // large
    for (int i = 0; i < m_data->paramCount; i++)
    {
        QueryParam& p = m_data->params[i];
        if (!p.m_origAddr || (p.m_flags == QueryParam_Input))
            continue;
        if (p.m_size < (16 * 1024))
            continue;
        if (p.m_flags & QueryParam_Input)
        {
            UMBRA_ASSERT(!"not supported");
            p.m_copy(getCur(), p.m_origAddr);
        }
        p.m_offset = m_ofs;
        m_ofs += UMBRA_ALIGN(p.m_size, 16);
    }
    m_data->largeSize = m_ofs - (m_data->inSize + m_data->inOutSize);

    /* \todo [antti 24.2.2012]: already wrote over memory  */
    if (m_ofs >= m_size)
        return false;

    return true;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryWrapper::postprocessParams (void)
{
    for (int i = 0; i < m_data->paramCount; i++)
    {
        QueryParam& p = m_data->params[i];
        if (!p.m_origAddr || ((p.m_flags & QueryParam_Output) == 0))
            continue;
        p.m_copy(p.m_origAddr, ((UINT8*)m_data) + p.m_offset);
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void QueryWrapper::dispatch (QueryId id)
{
    UINT32 err = Query::ERROR_GENERIC_ERROR;

    // Memory can not come from stack
    if (sys_process_is_stack(m_data))
    {
        m_query->setError(Query::ERROR_INVALID_ARGUMENT);
        return;
    }

    //-----------------------------------------------------
    // Get job type
    //-----------------------------------------------------

    JobType type = jobTypeForId(id);
    if (type == JOBTYPE_MAX)
    {
        m_query->setError(Query::ERROR_UNSUPPORTED_OPERATION);
        return;
    }

    //-----------------------------------------------------
    // Spurs init
    //-----------------------------------------------------

    WrapperCtx* wrapper = &g_wrapper;
    if (!wrapperInit(wrapper))
    {
        m_query->setError(Query::ERROR_GENERIC_ERROR);
        return;
    }

    //-----------------------------------------------------
    // Fill in QuerySharedData
    //-----------------------------------------------------

    m_ofs = UMBRA_ALIGN(sizeof(QueryDataShared), 16);
    m_data->id = id;
    m_data->tome = (UINTPTR)m_query->getState()->getRootTome();
    m_data->collection = 0;

    if (m_query->getState()->getCollection())
    {
        memcpy(m_collection, m_query->getState()->getCollection(), sizeof(TomeCollection));
        m_data->collection = (UINTPTR)m_collection;
    }

    m_data->gateVectorSize = m_query->getState()->getGateStates() ? UMBRA_BITVECTOR_SIZE(m_query->getState()->getRootTome()->getNumGates()) : 0;
    if (m_data->gateVectorSize)
    {
        memcpy(getCur(), m_query->getState()->getGateStates(), m_data->gateVectorSize);
        m_data->gateVectorOfs = m_ofs;
        m_ofs += UMBRA_ALIGN(m_data->gateVectorSize, 16);
    }

    //-----------------------------------------------------
    // Process params
    //-----------------------------------------------------

    put(&err, QueryParam_Output);
    if (!applyParams())
    {
        m_query->setError(Query::ERROR_GENERIC_ERROR);
        return;
    }

    //-----------------------------------------------------
    // Run job
    //-----------------------------------------------------

    Query::SpuUsage usage = m_query->getState()->getSpuUsage();
    UMBRA_ASSERT((usage >= Query::SPU_USAGE_SPURS_THREAD0) && (usage < (Query::SPU_USAGE_SPURS_THREAD0 + NUM_THREADS)));
    UINT32 threadId = min2((UINT32)usage - 1, (UINT32)(NUM_THREADS - 1));
    WrapperThreadCtx* thread = &wrapper->threadCtx[threadId];

    int ret = thread->jobDesc->initialize(
        m_data,
        m_data->inSize + m_data->inOutSize,
        UMBRA_QUERY_SIZE + m_data->largeSize,
        jobTypeHeader(type));

    if (ret != CELL_OK)
    {
        reportError("Job initialization", type, ret);
        m_query->setError(Query::ERROR_GENERIC_ERROR);
        return;
    }

    size_t size = m_data->inSize + m_data->inOutSize + m_data->largeSize + UMBRA_QUERY_SIZE;
    if (wrapper->checked[type] < size)
    {
        ret = ((Job<64>*)thread->jobDesc)->checkForJobQueue();
        if (ret != CELL_OK)
        {
            reportError("Job check", type, ret);
            if (ret == CELL_SPURS_JOB_ERROR_MEMORY_SIZE)
            {
                printf("\tbinary size: %d bytes\n\tdata size: %d bytes\n\tlimit: %d\n",
                    thread->jobDesc->sizeBinary * 16, size, CELL_SPURS_JOBQUEUE_MAX_SIZE_JOB_MEMORY);
            }
            m_query->setError(Query::ERROR_GENERIC_ERROR);
            return;
        }
        wrapper->checked[type] = size;
    }

    ret = thread->port->pushJob(thread->jobDesc, sizeof(QuerySpursJob), 0, JobQueue::Port2::kFlagSyncJob);
    if (ret != CELL_OK)
    {
        m_query->setError(Query::ERROR_GENERIC_ERROR);
        return;
    }

    // Block until finished
    ret = thread->port->sync(0);
    UMBRA_ASSERT(ret == CELL_OK);

    //-----------------------------------------------------
    // Process output
    //-----------------------------------------------------

    postprocessParams();

    m_query->setError(err);
}

void QueryWrapper::deinit()
{
    WrapperCtx* wrapper = &g_wrapper;
    wrapperDeinit(wrapper);
}
#endif
