#pragma once

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
 * \brief   Umbra3 computation task implementation
 *
 */

#include "umbraVector.hpp"
#include "umbraAABB.hpp"
#include "umbraSet.hpp"
#include "umbraString.hpp"
#include "umbraThread.hpp"
#include "umbraFileStream.hpp"
#include "umbraLogger.hpp"
#include "umbraRandom.hpp"
#include "umbraProgress.hpp"

#include "optimizer/umbraTask.hpp"
#include "optimizer/umbraComputation.hpp"
#include "optimizer/umbraBuilder.hpp"

#define UMBRA_MAX_WORKERS 48
#if defined(_WIN64) || defined(__LP64__)
#   define MEMORY_USAGE_HARD_LIMIT (6LL*1024LL*1024LL*1024LL)
#else
#   define MEMORY_USAGE_HARD_LIMIT (3LL*1024LL*1024LL*1024LL)
#endif

namespace Umbra
{

/*!
 * \brief   The complete input for running a build task
 */
class TaskParams
{
public:

    class LicenseKey : public Umbra::LicenseKey
    {
    public:
        LicenseKey() : m_params(NULL) {}        
        void setParams (TaskParams* params) { m_params = params; }
        void readKey (char key[128])
        {
            UMBRA_ASSERT(m_params);
            if (!m_params)
            {
                key[0] = '\0';
                return;
            }
            memcpy(key, m_params->m_licenseKey, 128);
        }

        TaskParams* m_params;
    };

    enum Verbosity
    {
        VERBOSITY_FULL,
        VERBOSITY_NORMAL,
        VERBOSITY_SILENT
    };

    TaskParams (void);

    void initCompParams (Allocator* a) { new (&m_computationParams) ComputationParams(a); }
    // Allocator passing is a bit of a mess here, but can't set is as a member or have this class inherit base,
    // since it's shared between processes.
    String getCacheFile      (const String& hash, Allocator* a) const { return String(m_path, a) + String("/", a) + hash + String(".umbracache", a); }
    String getCachePath      (Allocator* a) const { return String(m_path, a) + String("/", a); }
    String getPrefixPath     (Allocator* a) const { return String(m_path, a) + String("/", a) + String(m_prefix, a); }
    String getLogFile        (Allocator* a) const { return getPrefixPath(a)  + String("log.txt", a); }
    String getInputSceneFile (Allocator* a) const { return getPrefixPath(a)  + String("input.scene", a); }
    String getOutputTomeFile (Allocator* a) const { return getPrefixPath(a)  + String("output.tome", a); }
    String getParamFile      (Allocator* a) const { return getPrefixPath(a)  + String("params.json", a); }

    int                 m_memoryUsageLimit;
    int                 m_numThreads;
    Verbosity           m_verbosity;
    int                 m_cacheSize;
    ComputationParams   m_computationParams;
    AABB                m_compAABB;
    char                m_path[512];
    char                m_prefix[512];
    char                m_licenseKey[128];
};

/*!
 * \brief   Logger callbacks for Task
 */
class TaskLogger: public Logger
{
public:
    TaskLogger (void): m_minLevel(LEVEL_INFO), m_userLogger(NULL) {}
    void log (Level level, const char* str);
    void setMinLevel (Level l)              { m_minLevel = l; }
    void setOutputFile (const char* path);
    void setUserLogger (Logger* logger)     { m_userLogger = logger; }
private:
    Level m_minLevel;
    FileOutputStream m_out;
    StreamLogger m_all;
    Logger* m_userLogger;
};

/*!
 * \brief   Allocator callbacks for Task
 */
class TaskAllocator: public Allocator
{
public:
    TaskAllocator(Allocator* userAllocator, size_t budget): m_allocator(userAllocator), m_allocated(0), m_budget(budget) {}

    void*       allocate         (size_t size, const char* info = NULL);
    void        deallocate       (void* ptr);
    Allocator*  getUserAllocator (void) { return m_allocator; }
    void        setBudget        (size_t budget) { m_budget = budget; }

private:
    CriticalSection m_lock;
    Allocator*      m_allocator;
    size_t          m_allocated;
    size_t          m_budget;
};

/*!
 * \brief   The shared tile processing state
 */

class BuildState : public Base
{
public:

    enum Phase
    {
        TILES = 0,
        FAILED_TILES,
        TOMEGENERATION
    };
    
    BuildState(Allocator* allocator);
    ~BuildState();

    void            init            (const TaskParams& params);
    bool            shutdown        (void);

    Computation::Error     getError        (void) { return m_error; }
    const char*     getErrorReason  (void) { return m_errorReason; }
    void            setError        (Computation::Error err, const char* reason);

    TaskParams&     getParams       (void) { return m_params; }
    Phase           getPhase        (void) const { return m_phase; }
    void            setPhase        (Phase p) { m_phase = p; }

    bool            failedRun       (void);

    int             getWork         (TileInput& input);

    void            addFailed       (int idx);
    void            addSkipped      (int idx);
    bool            addResult       (TileResult& result, bool cached);
    void            incRevision     (void) { lock(); m_revision++; unlock(); }
    float           getProgress     (void);
    Progress&       getProgressHelper (void) { return m_progress; }
    void            updateTome      (Allocator* a);
    Tome*           copyTome        (Allocator* a);
    UINT32          copyTome        (UINT8* buf, UINT32 size);

    void            visualize       (DebugRenderer* debug) { m_generator.visualizeState(debug); }

private:

    void lock (void) { m_lock.lock(); }
    void unlock (void) { m_lock.release(); }

    Phase               m_phase;
    TaskParams          m_params;
    Mutex               m_lock;
    Computation::Error  m_error;
    char                m_errorReason[128];
    TileInputSet        m_inputs;
    Set<int>            m_failed;
    int                 m_current;
    TomeGenerator       m_generator;
    int                 m_numTilesDone;
    int                 m_numTilesFromCache;
    int                 m_revision;
    int                 m_generatedRevision;
    Allocator*          m_tomeAllocator;
    Tome*               m_tome;
    Progress            m_progress;

    friend class BuildMasterLocal;
};

/*!
 * \brief   The tile processing worker thread
 */

class TileWorker : public Runnable
{
public:
    TileWorker() {}

    void init (BuildState* state, Allocator* a)
    {
        // Silence logger completely, distributed tile workers should not produce log.
        // Alternatively should buffer tileworker log output and let build master
        // coordinate output.
        m_key.setParams(&state->getParams());
        m_logger.setMinLevel((Logger::Level)(Logger::LEVEL_ERROR + 1));
        m_builder.init(PlatformServices(a, &m_logger, &m_key));
        m_state = state;
        m_heap = a;
    }

    bool doWorkItem (void)
    {
        return processOneTile();
    }

    unsigned long run (void*)
    {
        while (!m_state->shutdown())
        {
            if (!doWorkItem())
                break;
        }
        return 0;
    }

private:

    bool processOneTile (void);

    bool cacheLoad (TileResult& result, const String& hash);
    void cacheSave (const TileResult& result, const TileInput& input, const String& hash);

    Allocator*              getAllocator() { return m_heap; }
    TaskParams::LicenseKey  m_key;
    TaskLogger              m_logger;
    Builder                 m_builder;
    BuildState*             m_state;
    Allocator*              m_heap;
};

class BuildMaster : public Runnable
{
public:
    virtual void            init            (Scene* scene, const TaskParams& params, Logger* userLogger) = 0;
    virtual unsigned long   run             (void* param) = 0;
    virtual void            requestAbort    (void) = 0;
    virtual float           getProgress     (void) = 0;

    virtual UINT32          copyResult      (void* buf, UINT32 size) = 0;

    virtual void            setError        (Computation::Error err, const char* reason) = 0;
    virtual Computation::Error     getError        (void) = 0;
    virtual const char*     getErrorReason  (void) = 0;

    virtual void            visualize       (class DebugRenderer*) = 0;
};

/*!
 * \brief   The build task master
 */
class BuildMasterLocal : public BuildMaster
{
public:

    BuildMasterLocal  (Allocator* a);
    ~BuildMasterLocal (void);

    void            init            (Scene* scene, const TaskParams& params, Logger* userLogger);
    unsigned long   run             (void* param);
    void            requestAbort    (void);
    float           getProgress     (void) { return m_state.getProgress(); }
    void            cacheCleanUp    (void);

    UINT32          copyResult      (void* buf, UINT32 size);

    void            setError        (Computation::Error err, const char* reason) { m_state.setError(err, reason); }
    Computation::Error     getError        (void) { return m_state.getError(); }
    const char*     getErrorReason  (void) { return m_state.getErrorReason(); }

    void            visualize       (class DebugRenderer*);

private:

    Allocator*              getAllocator() { return m_allocator; }
    TaskLogger              m_logger;
    Allocator*              m_allocator;
    Builder                 m_builder;
    Scene*                  m_scene;
    BuildState              m_state;
    TaskParams::LicenseKey  m_licenseKey;
    Thread                  m_threads[UMBRA_MAX_WORKERS - 1];
    TileWorker              m_workers[UMBRA_MAX_WORKERS];
};


/*!
 * \brief   Task implementation
 *
 * Note that a task is once-off, a single instance can only be used to launch
 * a single computation that can fail at various stages.
 *
 * A second call to start() results in aborting the computation and setting ERROR_ALREADY_RUNNING.
 *
 */
class ImpLocalComputation : public Base
{
public:

                                ImpLocalComputation         (Scene* scene, const Vector3* mn, const Vector3* mx, TaskAllocator* a);
                                ~ImpLocalComputation        (void);

    void                        setComputationParams        (const ComputationParams& params)    { m_params.m_computationParams = params; }
    void                        setRunAsProcess             (const char* executablePath)         { m_exePath = executablePath; m_separateProcess = (executablePath != NULL); }
    void                        setMemoryUsageLimit         (int megabytes);
    void                        setNumThreads               (int numThreads)                     { m_params.m_numThreads = numThreads; }
    void                        setSilent                   (bool silent)                        { m_params.m_verbosity = silent ? TaskParams::VERBOSITY_SILENT : TaskParams::VERBOSITY_NORMAL; }
    void                        setCacheSize                (int s)                              { m_params.m_cacheSize = s; }
    void                        setLicenseKey               (const char* key);
    void                        setLogger                   (Logger* logger);

    /*!
     * \brief   Start building
     */
    void                        start                       (const char* tempPath, const char* prefix);

    /*!
     * \brief   Requests aborting the build but doesn't block
     */
    void                        requestAbort                (void);

    /*!
     * \brief   Waits for build completion
     * \return  returns false if timeout
     */
    bool                        waitForFinish               (unsigned int timeoutMs);

    /*!
     * \brief   Polls for build completion
     */
    bool                        isFinished                  (void);
    float                       getProgress                 (void);

    /*!
     * \brief   Get the error status of the build
     */
    Computation::Error          getError                    (void);
    const char*                 getErrorString              (void);
    UINT32                      serializeTome               (void* buf, UINT32 size);

    /*!
    * \brief    Compute-time visualizations
    */
    void                        visualize                   (DebugRenderer*);

#if 0
    // Process-related
    void                        startFromProcess            (void* parameters);
    bool                        getShouldProcessExit        (HandleProcess* parentProcess);
#endif

private:

    enum State
    {
        STATE_INIT,
        STATE_RUNNING,
        STATE_FINISHED
    };

    bool  validateString    (const char* in, char* out, int len);
    void  setError          (Computation::Error e, const char* reason = NULL);

    // user parameters for next build

    TaskAllocator*  m_taskAllocator;
    Logger*         m_userLogger;
    String          m_exePath;
    bool            m_separateProcess;
    Scene*          m_scene;
    TaskParams      m_params;

    // current build state

    State           m_state;
    Thread*         m_thread;
    BuildMaster*    m_build;
    int             m_lastRevision;
};

} // namespace Umbra
