#if !defined(UMBRA_EXCLUDE_COMPUTATION)

/*!
 *
 * Umbra3
 * -----------------------------------------
 *
 * (C) 2010-2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraTileGrid.hpp"
#include "umbraImpTask.hpp"
#include "umbraImpScene.hpp"
#include "umbraDirScan.hpp"
#include "umbraRemoteTask.hpp"
#include "umbraHttp.hpp"
#include "umbraJson.hpp"
#include "umbraChecksum.hpp"
#include "runtime/umbraTome.hpp"
#include "optimizer/umbraLocalComputation.hpp"
#include "umbraPrivateVersion.hpp"

// define this to dump TileInputs into cache in addition to TileResults (for debugging etc)
//#define UMBRA_DUMP_TILE_INPUTS

using namespace Umbra;

#if UMBRA_OS == UMBRA_OSX && MAC_OS_X_VERSION_MIN_REQUIRED >= MAC_OS_X_VERSION_10_6
size_t strnlen(const char *s, size_t maxlen)
{
    if (!maxlen)
        return 0;
    size_t size = 0;
    while(*s)
    {
        s++;
        size++;
        if (size == maxlen)
            break;
    }
    return size;
}
#endif

namespace Umbra
{

/*!
 * \brief   Debug helper for finding diff spot in data
 */
class DiffFinder: public OutputStream
{
public:
    DiffFinder (InputStream* in, bool assert = true):
      m_forward(NULL), m_assert(assert), m_diff(in), m_pos(0) {}

    void setInput (InputStream* in) { m_diff = in; }

    UINT32 write (const void* vptr, UINT32 numBytes)
    {
        const UINT8* ptr = (const UINT8*)vptr;

        if (m_diff)
        {
            for (UINT32 i = 0; i < numBytes; i++)
            {
                UINT8 c;
                if ((m_diff->read(&c, 1) != 1) || (c != ptr[i]))
                {
                    if (m_assert)
                        UMBRA_ASSERT(!"Difference found");
                    m_diff = NULL;
                    break;
                }
                m_pos++;
            }
        }

        if (m_forward)
            return m_forward->write(ptr, numBytes);
        return numBytes;
    }

private:
    OutputStream*   m_forward;
    bool            m_assert;
    InputStream*    m_diff;
    UINT32          m_pos;
};

} // namespace Umbra

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TaskParams::TaskParams (void):
    m_memoryUsageLimit(0),
    m_numThreads(Thread::getNumProcessors()),
    m_verbosity(VERBOSITY_NORMAL),
    m_cacheSize(-1),
    m_computationParams(NULL)   // \todo this is risky!
{
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TaskLogger::log (Level level, const char* msg)
{
    if (level >= m_minLevel)
    {
        getDefaultLogger()->log(level, msg);
        if (m_userLogger)
            m_userLogger->log(level, msg);
    }
    m_all.log(level, msg);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TaskLogger::setOutputFile (const char* path)
{
    m_out.open(path, true);
    if (m_out.isOpen())
        m_all.setOutput(&m_out);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void* TaskAllocator::allocate (size_t size, const char* info)
{
    UMBRA_UNREF(info);
    size += sizeof(size_t); // room for header

    if (Atomic::add(&m_allocated, size) > m_budget)
    {
        Atomic::sub(&m_allocated, size);
        return NULL;
    }

    void* ptr = m_allocator->allocate(size);
    if (!ptr)
    {
        Atomic::sub(&m_allocated, size);
        return NULL;
    }

    *(size_t*)ptr = size;
    return (char*)ptr + sizeof(size_t);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TaskAllocator::deallocate (void* ptr)
{
    if (ptr)
    {
        char* ptr2 = (char*)ptr - sizeof(size_t);
        size_t s = *(size_t*)ptr2;
        m_allocator->deallocate(ptr2);
        Atomic::sub(&m_allocated, s);
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BuildState::BuildState(Allocator* a)
    : m_lock(a),
    m_failed(a),
    m_tome(0),
    m_progress(a)
{
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BuildState::~BuildState()
{
    if (m_tome)
        UMBRA_HEAP_FREE(m_tomeAllocator, m_tome);
    m_tome = 0;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildState::init (const TaskParams& params)
{
    m_params.initCompParams(getAllocator());
    m_params = params;
    m_error = Computation::ERROR_OK;
    m_errorReason[0] = 0;
    m_numTilesDone = 0;
    m_numTilesFromCache = 0;
    setPhase(TILES);
    m_current = 0;
    m_tome = NULL;
    m_revision = 0;
    m_generatedRevision = 0;
    m_progress.addPhase(95.f, "tiles");
    if (params.m_cacheSize > 0)
        m_progress.addPhase(5.f,  "cache");
    m_progress.start();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildState::addFailed (int idx)
{
    bool ok = true;
    lock();
    if (m_phase == TILES)
        m_failed.insert(idx);
    else
        ok = false;
    unlock();
    if (!ok)
    {
        setError(Computation::ERROR_OUT_OF_MEMORY,
        "Not enough memory to complete tile computation");
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool BuildState::failedRun (void)
{
    if (m_failed.getSize())
    {
        // if all threads exited, all inputs have not been processed
        while (m_current < m_inputs.size())
            m_failed.insert(m_current++);

        UMBRA_ASSERT(m_failed.getSize() == (m_inputs.size() - m_numTilesDone));
        setPhase(FAILED_TILES);
        m_current = 0;
        return true;
    }
    return false;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

float BuildState::getProgress (void)
{
    float progress = m_progress.getValue();
    if (m_phase == TOMEGENERATION)
        return progress * 0.5f + m_generator.getProgress() * 0.5f;
    else
        return progress * 0.5f;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool BuildState::shutdown (void)
{
    bool ret;
    lock();
    ret = (m_error != Computation::ERROR_OK);
    unlock();
    return ret;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

int BuildState::getWork (TileInput& input)
{
    int idx = -1;
    lock();
    UMBRA_ASSERT(m_phase == TILES || m_phase == FAILED_TILES);
    while (m_current < m_inputs.size())
    {
        int candidate = m_current++;

        // skip already successfully computed
        if (m_phase == FAILED_TILES && !m_failed.contains(candidate))
            continue;

        idx = candidate;
        break;
    }
    unlock();
    if (idx != -1)
    {
        Builder::Error err = m_inputs.get(input, idx);
        if (err != Builder::SUCCESS)
        {
            if (err == Builder::ERROR_OUT_OF_MEMORY)
                addFailed(idx);
            else
                setError(Computation::ERROR_UNKNOWN, "Failed in getting tile input");
            idx = -1;
        }
    }
    return idx;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool BuildState::addResult (TileResult& result, bool cached)
{
    bool ret = false;
    lock();
    Builder::Error err = m_generator.addTileResult(result);
    if (err == Builder::SUCCESS)
    {
        ret = true;
        m_numTilesDone++;
        m_progress.setPhaseProgress((float)m_numTilesDone / m_inputs.size());
        if (cached)
            m_numTilesFromCache++;
        m_revision++;

    }
    unlock();
    return ret;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildState::updateTome (Allocator* a)
{
    lock();

    if (m_generatedRevision == m_revision)
    {
        unlock();
        return;
    }
    
    UINT32 size = 0;
    Builder::Error err = m_generator.getTomeSize(size);

    if (err == Builder::ERROR_OUT_OF_MEMORY)
    {
        unlock();
        setError(Computation::ERROR_OUT_OF_MEMORY, "Out of memory in TomeGenerator");
        return;
    }
    else if (err != Builder::SUCCESS)
    {
        unlock();
        setError(Computation::ERROR_UNKNOWN, "Error generating output, see log for details");
        return;
    }

    if (size)
    {
        Tome* tome = (Tome*)UMBRA_HEAP_ALLOC(a, size);
        if (!m_generator.getTome((UINT8*)tome, size))
        {
            unlock();
            UMBRA_HEAP_FREE(a, tome);
            /* \todo [antti 25.11.2011]: error code */
            setError(Computation::ERROR_OUT_OF_MEMORY, "Out of memory in TomeGenerator::getTome()");
            return;
        }
        if (m_tome)
            UMBRA_HEAP_FREE(m_tomeAllocator, m_tome);
        m_tomeAllocator = a;
        m_tome = tome;
        m_generatedRevision = m_revision;
    }
    unlock();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Tome* BuildState::copyTome (Allocator* a)
{
    Tome* ret = NULL;
    lock();
    if (m_tome)
    {
        UINT32 size = m_tome->getSize();
        ret = (Tome*)a->allocate(size);
        if (ret && size)
            memcpy(ret, m_tome, size);
    }
    unlock();
    return ret;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Umbra::UINT32 BuildState::copyTome (Umbra::UINT8* buf, Umbra::UINT32 size)
{
    UINT32 ret = 0;
    lock();
    if (m_tome)
    {
        ret = m_tome->getSize();
        if (ret && buf && (size >= ret))
            memcpy(buf, m_tome, ret);
    }
    unlock();
    return ret;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildState::setError (Computation::Error err, const char* reason)
{
    UMBRA_ASSERT(err != Computation::ERROR_OK);
    lock();
    if (m_error == Computation::ERROR_OK || m_error == Computation::ERROR_ABORTED)
    {
        m_error = err;
        if (reason)
            strcpy(m_errorReason, reason);
    }
    unlock();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BuildMasterLocal::BuildMasterLocal (Allocator* a):
    m_logger(),
    m_allocator(a),
    m_scene(NULL),
    m_state(a)
{
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BuildMasterLocal::~BuildMasterLocal (void)
{
    if (m_scene)
        m_scene->release();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildMasterLocal::init (Scene* scene, const TaskParams& params, Logger* userLogger)
{
    m_logger.setUserLogger(userLogger);

    m_scene = scene;
    if (m_scene)
        ImpScene::ref(m_scene);

    m_state.init(params);
    m_licenseKey.setParams(&m_state.m_params);
    m_builder.init(PlatformServices(m_allocator, &m_logger, &m_licenseKey));
    for (int i = 0; i < UMBRA_MAX_WORKERS; i++)
        m_workers[i].init(&m_state, m_allocator);

    switch (params.m_verbosity)
    {
    case TaskParams::VERBOSITY_FULL:
        m_logger.setMinLevel(Logger::LEVEL_DEBUG); break;
    case TaskParams::VERBOSITY_NORMAL:
        m_logger.setMinLevel(Logger::LEVEL_INFO); break;
    case TaskParams::VERBOSITY_SILENT:
        m_logger.setMinLevel(Logger::LEVEL_ERROR); break;
    default:
        break;
    }

    String logFile = m_state.getParams().getLogFile(getAllocator());
    m_logger.setOutputFile(logFile.toCharPtr());

    UMBRA_LOG_I(&m_logger, "Umbra 3 Optimizer version %s", getOptimizerInfoString(INFOSTRING_VERSION));
}


/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TileWorker::processOneTile (void)
{
    TileInput input;
    TileResult result;

    int idx = m_state->getWork(input);
    if (idx == -1)
        return false;

    const char* hash = input.getHashValue();
    if (m_state->getParams().m_cacheSize > 0 && !hash)
    {
        m_state->setError(Computation::ERROR_UNKNOWN, "TileInput serialization failed");
        return false;
    }

    // try cache
    bool cached = (m_state->getParams().m_cacheSize > 0) ? cacheLoad(result, String(hash, getAllocator())) : false;

    // compute
    if (!cached)
    {
        Builder::Error err = m_builder.computeTile(result, input);

        if (err != Builder::SUCCESS)
        {
            if (err == Builder::ERROR_OUT_OF_MEMORY)
                m_state->addFailed(idx);
            else
                m_state->setError(Computation::ERROR_UNKNOWN, "ComputeTile failed");
            return false;
        }

        if (m_state->getParams().m_cacheSize > 0)
            cacheSave(result, input, String(hash, getAllocator()));
    }

    return m_state->addResult(result, cached);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TileWorker::cacheLoad (TileResult& result, const String& hash)
{
    String fileName = m_state->getParams().getCacheFile(hash, getAllocator());
    FileInputStream in(fileName.toCharPtr());
    if (!in.isOpen())
        return false;
    return (m_builder.loadTileResult(result, in) == Builder::SUCCESS);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TileWorker::cacheSave (const TileResult& result, const TileInput& input, const String& hash)
{
    String fileName = m_state->getParams().getCacheFile(hash, getAllocator());
    FileOutputStream out(fileName.toCharPtr());
    UMBRA_ASSERT(out.isOpen());
    if (out.isOpen())
        result.serialize(out);
#if defined(UMBRA_DUMP_TILE_INPUTS)
    String inputfileName = fileName + ".tileinput";
    BinOFStream out2(inputfileName.toCharPtr());
    input.serialize(out2);
#else
    UMBRA_UNREF(input);
#endif
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

#ifdef CHECK_EXIT
#   undef CHECK_EXIT
#endif
#define CHECK_EXIT() { if (m_state.shutdown()) return (unsigned)m_state.getError(); }

unsigned long BuildMasterLocal::run (void*)
{
    const TaskParams& taskParams = m_state.getParams();

    // Input validation

    if (!m_scene)
        m_state.setError(Computation::ERROR_INVALID_SCENE, "Invalid scene file in BuildMasterLocal::run()");

    CHECK_EXIT();

    // Output scene.
    m_scene->writeToFile(taskParams.getInputSceneFile(getAllocator()).toCharPtr());

    // Output computation params
    taskParams.m_computationParams.writeToFile(String(taskParams.getInputSceneFile(getAllocator()) + ".json").toCharPtr());
    // todo: license key setup?

    // Enforce that the generated tomes encapsulate all of computation aabb, for now
    AABB outputAABB;
    if (taskParams.m_compAABB.isOK())
        outputAABB = taskParams.m_compAABB;
    else
        outputAABB = ImpScene::getImplementation(m_scene)->getAABB();

    Builder::Error builderError = m_builder.join(m_state.m_generator, taskParams.m_computationParams, outputAABB.getMin(), outputAABB.getMax());
    if (builderError == Builder::ERROR_LICENSE_KEY)
        m_state.setError(Computation::ERROR_LICENSE_EXPIRED, "Expired license key, invalid key or license file not found");
    else if (builderError != Builder::SUCCESS)
        m_state.setError(Computation::ERROR_UNKNOWN, "Failure in Builder::join");

    CHECK_EXIT();

    m_state.m_generator.setNumThreadsExt(taskParams.m_numThreads);
    m_state.m_generator.setCachePathExt(m_state.getParams().getCachePath(getAllocator()).toCharPtr());
    m_state.incRevision();

    CHECK_EXIT();

    // computation stage 1

    Builder::Error err;
    if (taskParams.m_compAABB.isOK())
        err = m_builder.split(m_state.m_inputs,
                              m_scene,
                              taskParams.m_computationParams,
                              taskParams.m_compAABB.getMin(),
                              taskParams.m_compAABB.getMax());
    else
        err = m_builder.split(m_state.m_inputs, m_scene, taskParams.m_computationParams);

    if (err != Builder::SUCCESS)
    {
        switch (err)
        {
        case Builder::ERROR_PARAM:
            m_state.setError(Computation::ERROR_PARAM, "Invalid computation parameters");
            break;
        case Builder::ERROR_INVALID_SCENE:
            m_state.setError(Computation::ERROR_INVALID_SCENE, "Invalid scene (possibly too large bounds)");
            break;
        default:
            m_state.setError(Computation::ERROR_UNKNOWN, "Failure in split phase");
        }
    }

    // Scene not needed after split phase.

    if (m_scene)
    {
        m_scene->release();
        m_scene = 0;
    }

    // completely empty scene, no tome to generate but otherwise valid situation

    if (m_state.m_inputs.size() == 0)
    {
        return Computation::ERROR_OK;
    }

    CHECK_EXIT();

    // computation stage 2, distributed

    m_state.getProgressHelper().nextPhase();
    int numThreads = min2(max2(1, taskParams.m_numThreads), UMBRA_MAX_WORKERS);
    UMBRA_LOG_I(&m_logger, "Using %d threads to compute %d tiles, cache %d MB",
        numThreads, m_state.m_inputs.size(), m_state.getParams().m_cacheSize);
    numThreads--; // exclude me!

    // launch workers

    for (int i = 0; i < numThreads; i++)
    {
        m_threads[i].setFunction(&m_workers[i + 1]);
        m_threads[i].setPriority(-1);
        m_threads[i].run(NULL);
    }
    m_workers[0].run(NULL);

    // join workers

    for (int i = 0; i < numThreads; i++)
        m_threads[i].waitToFinish();

    CHECK_EXIT();

    // do OOM failed tiles one at a time

    if (m_state.failedRun())
    {
        if (numThreads > 0)
        {
            UMBRA_LOG_W(&m_logger, "Ran out of memory with %d tiles, trying again in single-threaded mode",
                m_state.m_failed.getSize());
            m_workers[0].run(NULL);
            if (m_state.getError() == Computation::ERROR_OUT_OF_MEMORY)
                UMBRA_LOG_E(&m_logger, "Not enough memory to compute tiles even in single-threaded mode");
        }
        else
            m_state.setError(Computation::ERROR_OUT_OF_MEMORY, "Not enough memory to complete tile computation");
    }

    CHECK_EXIT();

    UMBRA_LOG_I(&m_logger, "Tile processing complete, computed %d tiles",
        m_state.m_numTilesDone - m_state.m_numTilesFromCache);

    // Free inputs.

    m_state.m_inputs.~TileInputSet();

    // Write output tome

    m_state.setPhase(BuildState::TOMEGENERATION);
    m_state.incRevision();
    m_state.updateTome(m_allocator);

    CHECK_EXIT();

    Tome* t = m_state.copyTome(m_allocator);
    if (t)
    {
        FileOutputStream s(m_state.getParams().getOutputTomeFile(getAllocator()).toCharPtr());
        if (s.isOpen())
        {
            UINT32 dwords = t->getSize() / 4;
            UINT32* ptr = (UINT32*)t;
            StreamWriter writer(&s);
            while (dwords--)
                writer.put(*ptr++);
        }
        m_allocator->deallocate(t);
    }

    cacheCleanUp();        

    return Computation::ERROR_OK;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildMasterLocal::visualize(DebugRenderer* debug)
{
    m_state.visualize(debug);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildMasterLocal::cacheCleanUp()
{
    if (m_state.getParams().m_cacheSize <= 0)
        return;

    m_state.getProgressHelper().nextPhase();

    UMBRA_LOG_I(&m_logger, "Checking cache...");

    Array<String> cacheFiles(getAllocator());
    Array<UINT64> cacheSizes(getAllocator());
    Array<UINT64> cacheTimes(getAllocator());

    DirScan ds(m_state.getParams().getCachePath(getAllocator()).toCharPtr(), "*.umbracache", getAllocator());
    for (int i = 0; i < ds.getNumFiles(); i++)
    {
        String fn = m_state.getParams().getCachePath(getAllocator()) + String(ds.getFile(i), getAllocator());
        cacheFiles.pushBack(fn);
    }

    int n = cacheFiles.getSize();

    for (int i = 0; i < n; i++)
    {
        m_state.getProgressHelper().setPhaseProgress((float)i / (float)n);

        UINT64 time, size;
        DirScan::getFileAttrib(cacheFiles[i].toCharPtr(), time, size);
        cacheSizes.pushBack(size);
        cacheTimes.pushBack(time);
    }

    // \todo [Hannu] sort according to atime

    UMBRA_ASSERT(cacheSizes.getSize() == n&& cacheTimes.getSize() == n);

    UINT64 sum = 0;
    for (int i = 0; i < n; i++)
        sum += cacheSizes[i];
    UINT64 origSum = sum;

    UINT64 cacheSize = m_state.getParams().m_cacheSize * 1024LL * 1024LL;

    while (sum > cacheSize)
    {
        int j = -1;
        UINT64 t = 0;

        for (int i = 0; i < n; i++)
            if (cacheTimes[i] > 0 && (t == 0 || cacheTimes[i] < t))
            {
                j = i;
                t = cacheTimes[i];
            }

        UMBRA_ASSERT(j >= 0);
        if (j < 0)
            break;

        DirScan::removeFile(cacheFiles[j].toCharPtr());

        sum -= cacheSizes[j];
        cacheTimes[j] = 0;
    }

    if (origSum != sum)
        UMBRA_LOG_I(&m_logger, "Cache cleaned from %d kB to %d kB", int(origSum/1024), int(sum/1024));
    else
        UMBRA_LOG_I(&m_logger, "Cache check finished");

    m_state.getProgressHelper().setPhaseProgress(1.f);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildMasterLocal::requestAbort (void)
{
    m_state.setError(Computation::ERROR_ABORTED, "Computation aborted");
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Umbra::UINT32 BuildMasterLocal::copyResult (void* buf, Umbra::UINT32 size)
{
    m_state.updateTome(m_allocator);
    return m_state.copyTome((UINT8*)buf, size);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

ImpLocalComputation::ImpLocalComputation (Scene* scene, const Vector3* mn, const Vector3* mx, TaskAllocator* allocator):
    Base(allocator),
    m_taskAllocator(allocator),
    m_userLogger(NULL),
    m_exePath(),
    m_separateProcess(false),
    m_scene(scene),
    m_params(),
    m_state(STATE_INIT),
    m_thread(NULL),
    m_build(NULL)
{
    if (scene)
        ImpScene::ref(scene);
    if (mn && mx)
        m_params.m_compAABB = AABB(*mn, *mx);

    memset(m_params.m_licenseKey, 0, sizeof(m_params.m_licenseKey));
    FILE* file = fopen("umbra_license.txt", "rb");
    if (file)
    {
        fseek(file, 0, SEEK_END);
        size_t len = ftell(file);
        fseek(file, 0, SEEK_SET);

        if (len > 127)
            len = 127;
        size_t read = 0;
        while (read < len)
            read += fread(&m_params.m_licenseKey[read], 1, len - read, file);
        fclose(file);
    }
}


/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

ImpLocalComputation::~ImpLocalComputation (void)
{
    requestAbort();
    waitForFinish(0);
    if (m_scene)
        m_scene->release();
    UMBRA_DELETE(m_thread);
    UMBRA_DELETE(m_build);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpLocalComputation::setLicenseKey (const char* key)
{
    if (key)
    {
        strncpy(m_params.m_licenseKey, key, 127);
        m_params.m_licenseKey[127] = '\0';
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpLocalComputation::setLogger (Logger* logger)
{
    m_userLogger = logger;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpLocalComputation::setError (Computation::Error err, const char* reason)
{
    UMBRA_ASSERT(m_state != STATE_RUNNING);
    m_state = STATE_FINISHED;
    m_build->setError(err, reason);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool ImpLocalComputation::validateString (const char* in, char* out, int maxlen)
{
    if (!in)
    {
        *out = '\0';
        return true;
    }
    int len = (int)strnlen(in, maxlen);
    if (len >= (maxlen - 1))
    {
        setError(Computation::ERROR_INVALID_PATH, "string too long");
        return false;
    }

    strncpy(out, in, len + 1);
    return true;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpLocalComputation::start (const char* tempPath, const char* prefix)
{
    // A task is once-off
    if (m_state != STATE_INIT)
    {
        requestAbort();
        waitForFinish(0);
        setError(Computation::ERROR_ALREADY_RUNNING);
        return;
    }

    if (m_separateProcess)
    {
        BuildMasterRemote* build = UMBRA_NEW(BuildMasterRemote, getAllocator());
        build->setExePath(m_exePath);
        m_build = (BuildMaster*)build;
    }
    else
    {
        m_build = UMBRA_NEW(BuildMasterLocal, getAllocator());
    }

    if (!tempPath)
        tempPath = ".";
    // todo: separate process

    if (!validateString(tempPath, m_params.m_path, sizeof(m_params.m_path)))
        return;
    if (!validateString(prefix, m_params.m_prefix, sizeof(m_params.m_prefix)))
        return;

    m_thread = UMBRA_NEW(Thread, getAllocator());
    m_thread->setFunction(m_build);

    m_build->init(m_scene, m_params, m_userLogger);

    m_thread->run(NULL);
    m_state = STATE_RUNNING;

    m_scene->release();
    m_scene = NULL;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool ImpLocalComputation::waitForFinish (unsigned int timeoutMs)
{
    if (m_state == STATE_RUNNING)
    {
        // todo: the implementation of this on win32 sucks
        if (!m_thread->waitToFinish(timeoutMs))
            return false;
        m_state = STATE_FINISHED;
    }
    return true;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool ImpLocalComputation::isFinished (void)
{
    if (m_state == STATE_RUNNING)
    {
        if (m_thread->isFinished())
            m_state = STATE_FINISHED;
    }

    return (m_state == STATE_FINISHED);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpLocalComputation::requestAbort (void)
{
    if (m_state == STATE_RUNNING)
    {
        while (!isFinished())
        {
            m_build->requestAbort();
            // non-blocking!!!
            //m_thread->waitToFinish();
        }
    }
    m_state = STATE_FINISHED;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

float ImpLocalComputation::getProgress (void)
{
    if (!m_build)
        return 0.f;
    return m_build->getProgress();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Computation::Error ImpLocalComputation::getError (void)
{
    if (!isFinished())
        return Computation::ERROR_OK;
    return m_build->getError();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

const char* ImpLocalComputation::getErrorString()
{
    if (!isFinished())
        return "";
    return m_build->getErrorReason();
}

/*-------------------------------------------------------------------*//*!
 * \brief   Get result
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpLocalComputation::serializeTome (void* buf, Umbra::UINT32 size)
{
    if (!m_build)
        return 0;
    return m_build->copyResult(buf, size);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Visualization
 *//*-------------------------------------------------------------------*/

void ImpLocalComputation::visualize (DebugRenderer* debugRenderer)
{
    if (!m_build)
        return;

    m_build->visualize(debugRenderer);
}

void ImpLocalComputation::setMemoryUsageLimit (int megabytes)
{
    m_params.m_memoryUsageLimit = megabytes;
    if (!megabytes)
        m_taskAllocator->setBudget(MEMORY_USAGE_HARD_LIMIT);
    else
        m_taskAllocator->setBudget(megabytes * 1024LL * 1024LL);
}

/*---------------------------------------------------------------*//*!
 * Public API
 *//*---------------------------------------------------------------*/

/*!
 * \brief   Launch local computation
 */

LocalComputation::LocalComputation (void) : m_imp(NULL) {}
LocalComputation::~LocalComputation () { }

LocalComputation* LocalComputation::create (const LocalComputation::Params& params)
{
    Scene* scene = params.scene;
    if (!scene)
        return NULL;
    Allocator* allocator = params.allocator;
    if (!allocator)
        allocator = Umbra::getAllocator();
    TaskAllocator* taskAllocator = UMBRA_HEAP_NEW(allocator, TaskAllocator, allocator, MEMORY_USAGE_HARD_LIMIT);
    LocalComputation* c = UMBRA_HEAP_NEW(taskAllocator, LocalComputation);
    c->m_imp = UMBRA_HEAP_NEW(taskAllocator, ImpLocalComputation, scene, params.boundsMin, params.boundsMax, taskAllocator);
    if (!c->m_imp)
    {
        UMBRA_HEAP_DELETE2(taskAllocator, LocalComputation, c);
        UMBRA_HEAP_DELETE(allocator, taskAllocator);
        return NULL;
    }

    if (params.logger)
        c->m_imp->setLogger(params.logger);
    c->m_imp->setComputationParams(*params.computationParams);
    if (params.cacheSizeMegs > 0)
        c->m_imp->setCacheSize(params.cacheSizeMegs);
    if (params.licenseKey)
        c->m_imp->setLicenseKey(params.licenseKey);
    if (params.memUsageLimitMegs > 0)
        c->m_imp->setMemoryUsageLimit(params.memUsageLimitMegs);
    if (params.numThreads > 0)
        c->m_imp->setNumThreads(params.numThreads);
    if (params.runAsProcessPath)
        c->m_imp->setRunAsProcess(params.runAsProcessPath);
    c->m_imp->setSilent(params.silent);
    c->m_imp->start(params.tempPath, params.tempFilePrefix);
    return c;
}

void LocalComputation::release (void)
{
    if (!m_imp)
        return;
    m_imp->requestAbort();
    m_imp->waitForFinish(0);
    TaskAllocator* taskAllocator = (TaskAllocator*)m_imp->getAllocator();
    UMBRA_HEAP_DELETE(taskAllocator, m_imp);
    Allocator* userAllocator = taskAllocator->getUserAllocator();
    UMBRA_HEAP_DELETE(userAllocator, taskAllocator);
}

void LocalComputation::requestAbort (void)
{
    if (m_imp)
        m_imp->requestAbort();
}

#define SET_AND_RET(r, code, str) do { r.error = code; strcpy(r.errorStr, str); return r; } while (false)
Computation::Result LocalComputation::waitForResult (Allocator* tomeAllocator, unsigned int timeoutMs)
{
    Result result;
    result.tome     = NULL;
    result.tomeSize = 0;
    result.progress = m_imp->getProgress();
    strcpy(result.statusStr, "Computation");

    if (!m_imp)
    {
        SET_AND_RET(result, Computation::ERROR_OUT_OF_MEMORY, "Computation initialization failed.");
    }
    if (!m_imp->waitForFinish(timeoutMs))
    {
        SET_AND_RET(result, Computation::ERROR_WAIT_TIMEOUT, "waitForResult() timed out");
    }
    result.error = m_imp->getError();
    if (result.error != Computation::ERROR_OK)
    {
        SET_AND_RET(result, m_imp->getError(), m_imp->getErrorString());
    }

    UINT32 size = m_imp->serializeTome(NULL, 0);
    result.tome = (Tome*)UMBRA_HEAP_ALLOC(tomeAllocator ? tomeAllocator : getAllocator(), size);
    if (!result.tome)
    {
        SET_AND_RET(result, Computation::ERROR_OUT_OF_MEMORY, "Tome allocation failed.");
    }
    result.tomeSize = m_imp->serializeTome(result.tome, size);
    strcpy(result.statusStr, "Finished");
    result.progress = 1.f;
    SET_AND_RET(result, Computation::ERROR_OK, "");
}

void LocalComputation::visualize (DebugRenderer* renderer) const
{
    if (m_imp)
        m_imp->visualize(renderer);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void Umbra::startFromProcess (const char* UID, void* parameters)
{
    try
    {
        BackgroundProcess bgProcess;
        bgProcess.init(UID, parameters);
        bgProcess.run();
    }
    catch(OOMException)
    {
        printf("out of memory\n");
    }
}


#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)
