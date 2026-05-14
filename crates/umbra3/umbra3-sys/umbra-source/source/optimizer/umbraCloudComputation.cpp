#include "umbraImpTask.hpp"
#include "umbraChecksum.hpp"
#include "umbraJson.hpp"
#include "umbraHttp.hpp"
#include "umbraImpScene.hpp"
#include "umbraPrivateVersion.hpp"

#include "optimizer/umbraCloudComputation.hpp"

#define UMBRA_CLOUD_API_VERSION "0.1"

namespace Umbra
{

class BuildMasterCloud : public BuildMaster, private Http::ProgressListener
{
public:
    enum Phase
    {
        PHASE_UPLOAD,
        PHASE_COMPUTATION,
        PHASE_DOWNLOAD
    };

    BuildMasterCloud  (const String& m_cloudRoot, const String& cloudKey, Allocator* a);
    virtual ~BuildMasterCloud (void);

    void            init            (Scene* scene, const TaskParams& params, Logger* userLogger);
    unsigned long   run             (void* param);
    void            requestAbort    (void);
    float           getProgress     (void) { return m_progress; }
    Phase           getPhase        (void) { return m_phase; }
    void            cacheCleanUp    (void);

    UINT32          copyResult      (void* buf, UINT32 size);

    void            setError        (Computation::Error err, const String& reason) { setError(err, reason.toCharPtr()); }
    void            setError        (Computation::Error err, const char* reason)   { m_logger.log(Logger::LEVEL_ERROR, reason); m_error = err; strcpy(m_errorReason, reason); }
    Computation::Error     getError        (void) { return m_error; }
    const char*     getErrorReason  (void) { return m_errorReason; }

    void            visualize       (class DebugRenderer*);

private:
    virtual bool transferProgress (size_t dlCur, size_t dlTotal, size_t ulCur, size_t ulTotal);

    bool httpRequest (
        Http::Verb          verb,
        const String&       url,
        Http::Response&     response,
        const String&       key,
        const void*         data,
        int                 dataLen,
        const char*         contentType = NULL);

    Allocator*              getAllocator() { return m_allocator; }
    Allocator*              m_allocator;
    String                  m_cloudRoot;
    String                  m_cloudKey;
    TaskLogger              m_logger;
    TaskLogger*             getLogger() { return &m_logger; }
    float                   m_progress; // \no need to lock
    Phase                   m_phase;

    MemOutputStream         m_sceneStream;
    MemOutputStream         m_compArgsStream;
    Array<char>             m_tomeBuf;
    Computation::Error      m_error;
    char                    m_errorReason[128];
};


BuildMasterCloud::BuildMasterCloud (const String& cloudRoot, const String& cloudKey, Allocator* allocator):
    m_allocator(allocator),
    m_cloudRoot(cloudRoot),
    m_cloudKey(cloudKey),
    m_logger(),
    m_progress(.0f),
    m_phase(PHASE_UPLOAD),
    m_sceneStream(allocator),
    m_compArgsStream(allocator),
    m_error(Computation::ERROR_OK)
{
    m_errorReason[0] = '\0';
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BuildMasterCloud::~BuildMasterCloud (void)
{
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildMasterCloud::init (Scene* scene, const TaskParams& params, Logger* userLogger)
{
    m_logger.setUserLogger(userLogger);

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

    String logFile = params.getLogFile(getAllocator());
    m_logger.setOutputFile(logFile.toCharPtr());

    // serialize into mem for upload
    scene->serialize(m_sceneStream);
    params.m_computationParams.writeToStream(m_compArgsStream);

    UMBRA_LOG_I(&m_logger, "Umbra 3 Optimizer version %s (cloud)", getOptimizerInfoString(INFOSTRING_VERSION));
}

#ifdef CHECK_EXIT
#   undef CHECK_EXIT
#endif
#define CHECK_EXIT() { if (m_error != Computation::ERROR_OK) { m_progress = 1.f; return m_error; } }

const char* responseCodeStr (int code)
{
    switch (code)
    {
    case Http::Response::STATUS_OK:                       return "OK";
    case Http::Response::STATUS_NOT_FOUND:                return "404/Not found";
    case Http::Response::STATUS_UNAUTHORIZED:             return "401/Unauthorized (Check your credentials)" ;
    case Http::Response::STATUS_INTERNAL_SERVER_ERROR:    return "500/Internal server error (please report this error)";
    }
    UMBRA_ASSERT(!"unhandled response code");
    return "unknown";
}

const char* httpErrStr (Http::ErrorCode err)
{
    switch (err)
    {
    case Http::ERR_OK:                    return "success";
    case Http::ERR_INIT:                  return "intialization failed";
    case Http::ERR_INVALID_URL:           return "invalid url";
    case Http::ERR_CANNOT_CONNECT:        return "cannot connect";
    case Http::ERR_CONNECTION_TERMINATED: return "connection terminated";
    case Http::ERR_TIMEOUT:               return "timeout";
    case Http::ERR_NO_DATA:               return "no data";
    case Http::ERR_NOT_IMPLEMENTED:       return "not implemented";
    case Http::ERR_UNKNOWN:               return "unknown";
    }
    UMBRA_ASSERT(!"unhandled error code");
    return "unknown";
}

const char* httpVerbStr (Http::Verb verb)
{
    switch (verb)
    {
    case Http::GET:     return "GET";
    case Http::POST:    return "PUT";
    case Http::PUT:     return "POST";
    }
    UMBRA_ASSERT(!"unknown http verb");
    return "unknown";
}

bool BuildMasterCloud::httpRequest (
    Http::Verb   verb,
    const String&       url,
    Http::Response&     response,
    const String&       key,
    const void*         data,
    int                 dataLen,
    const char*         contentType)
{
    PlatformServices platform(getAllocator(), &m_logger);
    Http::RequestParams reqParams;

    bool needParams = false;
    if (key.length())
    {
        reqParams.username = key;
        needParams = true;
    }
    if (data && dataLen)
    {
        reqParams.data.set((const char*)data, dataLen);
        needParams = true;

        if (contentType)
            reqParams.headers.set("Content-Type", contentType);
    }

    // send (start) request
    Http http(platform);
    Http::ErrorCode err = http.request(verb, url, response, needParams ? &reqParams : NULL, this);
    if (err != Http::ERR_OK)
    {
        UMBRA_LOG_D(&m_logger, "Sending request %s %s failed: %s", httpVerbStr(verb), url.toCharPtr(), httpErrStr(err));
        setError(Computation::ERROR_CLOUD_CONNECTIVITY, String("Error sending cloud API request: ") + httpErrStr(err));
        return false;
    }

    return true;
}

bool BuildMasterCloud::transferProgress (size_t dlCur, size_t dlTotal, size_t ulCur, size_t ulTotal)
{
    if (ulTotal)
        m_progress = ((float)ulCur / ulTotal);
    else if (dlTotal)
        m_progress = ((float)dlCur / dlTotal);
    return true;
}

unsigned long BuildMasterCloud::run (void*)
{
    // Construct scene root URL
    String cloudUrl = m_cloudRoot + "/scenes";
    m_phase = PHASE_UPLOAD;

    // Compute hash for scene data to check if it needs to be uploaded
    char sceneHash[41];
    sha1Hash((const UINT8*)m_sceneStream.getPtr(), m_sceneStream.getSize()).str(sceneHash);
    Http::Response response;
    if (!httpRequest(Http::GET, cloudUrl + "?sha1=" + sceneHash, response, m_cloudKey, NULL, 0))
        return m_error;

    String msg = String("Scene with hash ") + sceneHash + String(" found in destination: ") +
        String(response.code == Http::Response::STATUS_OK ? "yes" : "no");

    m_logger.log(Logger::LEVEL_INFO, msg.toCharPtr());

    const JsonValue* jsonRoot = NULL;
    INT64 sceneId = 0;

    JsonObject o(getAllocator());

    if (response.code == Http::Response::STATUS_NOT_FOUND)
    {
        m_logger.log(Logger::LEVEL_INFO, "Uploading scene.");
        // POST scene, create new object (metadata only)
        if (!httpRequest(Http::POST, cloudUrl, response, m_cloudKey, NULL, 0))
            return m_error;

        if (response.code != Http::Response::STATUS_OK)
            setError(Computation::ERROR_CLOUD_RESPONSE, String("Scene upload returned: ") + responseCodeStr(response.code));

        CHECK_EXIT()
        if (response.headers["Content-Type"] != "application/json")
            setError(Computation::ERROR_CLOUD_RESPONSE, "Invalid (non-json) response content type header in scene upload response");

        String uploadUrl;
        jsonRoot = JsonParser(getAllocator(), NULL).parse(response.data.getPtr(), response.data.getSize());
        if (!jsonRoot || !jsonRoot->get(o) || !o.getMember("id", sceneId) || !o.getMember("upload_endpoint", uploadUrl))
            setError(Computation::ERROR_CLOUD_RESPONSE, "Malformed scene meta data");
        CHECK_EXIT()

        if (!httpRequest(Http::PUT, uploadUrl, response, "", m_sceneStream.getPtr(), m_sceneStream.getSize()))
            return m_error;

        if (response.code != Http::Response::STATUS_OK)
            setError(Computation::ERROR_CLOUD_RESPONSE, String("Scene upload returned: ") + responseCodeStr(response.code));
    }
    else if (response.code == Http::Response::STATUS_OK)
    {
        jsonRoot = JsonParser(getAllocator(), NULL).parse(response.data.getPtr(), response.data.getSize());
        if (!jsonRoot || !jsonRoot->get(o) || !o.getMember("id", sceneId))
            setError(Computation::ERROR_CLOUD_RESPONSE, "Malformed scene meta data");
    }
    else
        setError(Computation::ERROR_CLOUD_RESPONSE, String("Scene check returned: ") + responseCodeStr(response.code));

    CHECK_EXIT()

    // Create Tome object
    cloudUrl += "/" + String((int)sceneId) + "/tomes";
    if (!httpRequest(Http::POST, cloudUrl, response, m_cloudKey, m_compArgsStream.getPtr(), m_compArgsStream.getSize(), "application/json"))
        return m_error;

    // Enter progress-polling loop
    m_progress = 0.f;
    m_phase = PHASE_COMPUTATION;

    m_logger.log(Logger::LEVEL_INFO, "Starting computation");

    while (m_error == Computation::ERROR_OK)
    {
        // Not binary but not json either?
        if (response.headers["Content-Type"] != "application/json")
            setError(Computation::ERROR_CLOUD_CONNECTIVITY, "Invalid Content-Type header encountered!");

        CHECK_EXIT()

        if (response.code != Http::Response::STATUS_OK)
            setError(Computation::ERROR_CLOUD_RESPONSE, String("Tome polling returned: ") + responseCodeStr(response.code));

        CHECK_EXIT()

        jsonRoot = JsonParser(getAllocator(), NULL).parse(response.data.getPtr(), response.data.getSize());
        double progress = -1.0;
        if (!jsonRoot || !jsonRoot->get(o) || !o.getMember("progress", progress))
            setError(Computation::ERROR_CLOUD_RESPONSE, "Malformed tome meta data!");
        CHECK_EXIT()

        m_progress = (float)progress;

        // Failure in the cloud is indicated with progress set to -1
        if (m_progress < 0.f)
        {
            String errReason;
            if (!o.getMember("error_reason", errReason))
                errReason = "unknown";

            setError(Computation::ERROR_CLOUD_RESPONSE, String("Remote computation failed: ") + errReason);
        }

        CHECK_EXIT()

        if (m_progress == 1.f)
        {
            m_logger.log(Logger::LEVEL_INFO, "Computation complete, downloading Tome");
            m_phase = PHASE_DOWNLOAD;

            String downloadUrl;
            if (o.getMember("download_endpoint", downloadUrl))
            {
                if (!httpRequest(Http::GET, downloadUrl, response, "", NULL, 0))
                    return m_error;
                if (response.code == Http::Response::STATUS_OK)
                {
                    m_tomeBuf.resize(response.data.getSize());
                    memcpy(m_tomeBuf.getPtr(), response.data.getPtr(), response.data.getSize());
                    m_progress = 1.f;
                    break;
                }
                else
                    setError(Computation::ERROR_CLOUD_RESPONSE, String("Tome download returned: ") + responseCodeStr(response.code));
            }
            setError(Computation::ERROR_CLOUD_RESPONSE, "Got complete Tome but no download url!");
        }

        CHECK_EXIT()

        INT64 tomeId = -1;
        if (!o.getMember("id", tomeId))
            setError(Computation::ERROR_CLOUD_RESPONSE, "Malformed tome meta data.");
        CHECK_EXIT()

        Thread::sleep(1000);

        if (!httpRequest(Http::GET, cloudUrl + "/" + String((int)tomeId), response, m_cloudKey, NULL, 0))
            return m_error;
    }

    m_logger.log(Logger::LEVEL_INFO, (String("Successfully fetched Tome, size: ") + String(m_tomeBuf.getSize()) + " bytes").toCharPtr());
    return Computation::ERROR_OK;
}

void BuildMasterCloud::requestAbort (void)
{
    // \todo should computation be aborted in the cloud here as well?
    m_error = Computation::ERROR_ABORTED;
    strcpy(m_errorReason, "Computation aborted");
}

Umbra::UINT32 BuildMasterCloud::copyResult (void* buf, Umbra::UINT32 size)
{
    if (buf && size >= (UINT32)m_tomeBuf.getSize())
        memcpy(buf, m_tomeBuf.getPtr(), m_tomeBuf.getSize());

    return m_tomeBuf.getSize();
}

void BuildMasterCloud::visualize(DebugRenderer*)
{
}

class ImpCloudComputation : public Base
{
public:

                                ImpCloudComputation         (Scene* scene, const Vector3* mn, const Vector3* mx, TaskAllocator* a);
                                ~ImpCloudComputation        (void);

    void                        setComputationParams        (const ComputationParams& params)    { m_computationParams = params; }
    void                        setLogger                   (Logger* logger);

    /*!
     * \brief   Start building
     */
    void                        start                       (const String& cloudRoot, const String& cloudKey);

    /*!
     * \brief   Send abort signal to build, doesn't block
     */
    void                        requestAbort                 (void);

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
    BuildMasterCloud::Phase     getPhase                    (void);

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

private:

    enum State
    {
        STATE_INIT,
        STATE_RUNNING,
        STATE_FINISHED
    };

    void  setError          (Computation::Error e, const char* reason = NULL);

    // user parameters for next build

    TaskAllocator*  m_taskAllocator;
    Logger*         m_userLogger;
    Scene*          m_scene;
    ComputationParams m_computationParams;
    // current build state
    AABB            m_compAABB;

    State           m_state;
    Thread*         m_thread;
    BuildMasterCloud*    m_build;
    int             m_lastRevision;
};



ImpCloudComputation::ImpCloudComputation (Scene* scene, const Vector3* mn, const Vector3* mx, TaskAllocator* allocator):
    Base(allocator),
    m_taskAllocator(allocator),
    m_userLogger(NULL),
    m_scene(scene),
    m_computationParams(allocator),
    m_state(STATE_INIT),
    m_thread(NULL),
    m_build(NULL)
{
    if (scene)
        ImpScene::ref(scene);
    if (mn && mx)
        m_compAABB = AABB(*mn, *mx);

}


/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

ImpCloudComputation::~ImpCloudComputation (void)
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

void ImpCloudComputation::setLogger (Logger* logger)
{
    m_userLogger = logger;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpCloudComputation::setError (Computation::Error err, const char* reason)
{
    UMBRA_ASSERT(m_state != STATE_RUNNING);
    m_state = STATE_FINISHED;
    m_build->setError(err, reason);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void ImpCloudComputation::start (const String& cloudRoot, const String& cloudKey)
{
    // A task is once-off
    if (m_state != STATE_INIT)
    {
        requestAbort();
        waitForFinish(0);
        setError(Computation::ERROR_ALREADY_RUNNING);
        return;
    }

    m_build = UMBRA_NEW(BuildMasterCloud, cloudRoot, cloudKey, getAllocator());
    
    m_thread = UMBRA_NEW(Thread, getAllocator());
    m_thread->setFunction(m_build);

    TaskParams taskParams; // dummy obj
    taskParams.initCompParams(getAllocator());
    taskParams.m_computationParams = m_computationParams;

    m_build->init(m_scene, taskParams, m_userLogger);

    m_thread->run(NULL);
    m_state = STATE_RUNNING;

    m_scene->release();
    m_scene = NULL;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool ImpCloudComputation::waitForFinish (unsigned int timeoutMs)
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

bool ImpCloudComputation::isFinished (void)
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

void ImpCloudComputation::requestAbort (void)
{
    if (m_state == STATE_RUNNING)
    {
        while (!isFinished())
        {
            m_build->requestAbort();
            //m_thread->waitToFinish();
        }
    }
    m_state = STATE_FINISHED;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

float ImpCloudComputation::getProgress (void)
{
    if (!m_build)
        return 0.f;
    return m_build->getProgress();
}


BuildMasterCloud::Phase ImpCloudComputation::getPhase (void)
{
    if (!m_build)
        return (BuildMasterCloud::Phase)0;
    return m_build->getPhase();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Computation::Error ImpCloudComputation::getError (void)
{
    if (!isFinished())
        return Computation::ERROR_OK;
    return m_build->getError();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

const char* ImpCloudComputation::getErrorString()
{
    if (!isFinished())
        return "";
    return m_build->getErrorReason();
}

/*-------------------------------------------------------------------*//*!
 * \brief   Get result
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ImpCloudComputation::serializeTome (void* buf, Umbra::UINT32 size)
{
    if (!m_build)
        return 0;
    return m_build->copyResult(buf, size);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Visualization
 *//*-------------------------------------------------------------------*/

void ImpCloudComputation::visualize (DebugRenderer* debugRenderer)
{
    if (!m_build)
        return;

    m_build->visualize(debugRenderer);
}

/*---------------------------------------------------------------*//*!
 * Public API
 *//*---------------------------------------------------------------*/

CloudComputation::CloudComputation (void) : m_imp(NULL) {}
CloudComputation::~CloudComputation () {}
/*!
 * \brief   Launch local computation
 */

CloudComputation* CloudComputation::create (const CloudComputation::Params& params)
{
    Scene* scene = params.scene;
    if (!scene)
        return NULL;
    Allocator* allocator = params.allocator;
    if (!allocator)
        allocator = Umbra::getAllocator();
    TaskAllocator* taskAllocator = UMBRA_HEAP_NEW(allocator, TaskAllocator, allocator, MEMORY_USAGE_HARD_LIMIT);
    CloudComputation* c = UMBRA_HEAP_NEW(taskAllocator, CloudComputation);
    c->m_imp = UMBRA_HEAP_NEW(taskAllocator, ImpCloudComputation, scene, params.boundsMin, params.boundsMax, taskAllocator);
    if (!c->m_imp)
    {
        UMBRA_HEAP_DELETE2(taskAllocator, CloudComputation, c);
        UMBRA_HEAP_DELETE(allocator, taskAllocator);
        return NULL;
    }

    if (params.logger)
        c->m_imp->setLogger(params.logger);
    c->m_imp->setComputationParams(*params.computationParams);
    c->m_imp->start("https://api.umbracloud.com/" UMBRA_CLOUD_API_VERSION, params.apiKey);
    return c;
}

void CloudComputation::release (void)
{

    if (!m_imp)
        return;
    m_imp->requestAbort();
    m_imp->waitForFinish(0);
    TaskAllocator* taskAllocator = (TaskAllocator*)m_imp->getAllocator();
    UMBRA_HEAP_DELETE(taskAllocator, m_imp);
    UMBRA_HEAP_DELETE2(taskAllocator, CloudComputation, this);
    Allocator* userAllocator = taskAllocator->getUserAllocator();
    UMBRA_HEAP_DELETE(userAllocator, taskAllocator);
}

void CloudComputation::requestAbort (void)
{
    if (m_imp)
        m_imp->requestAbort();
}

#define SET_AND_RET(r, code, str) do { r.error = code; strncpy(r.errorStr, str, strlen(str)); return r; } while (false)
Computation::Result CloudComputation::waitForResult (Allocator* tomeAllocator, unsigned int timeoutMs)
{
    Computation::Result result;
    result.tome     = NULL;
    result.tomeSize = 0;
    result.progress = m_imp->getProgress();
    switch (m_imp->getPhase())
    {
    case BuildMasterCloud::PHASE_COMPUTATION:   strcpy(result.statusStr, "Computation");   break;
    case BuildMasterCloud::PHASE_UPLOAD:        strcpy(result.statusStr, "Upload");        break;
    default:                                    strcpy(result.statusStr, "Download");      break;
    }

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
    result.progress = 1.f;
    strcpy(result.statusStr, "Finished");
    SET_AND_RET(result, Computation::ERROR_OK, "");
}

void CloudComputation::visualize (DebugRenderer* debugRenderer) const
{
    if (m_imp)
        m_imp->visualize(debugRenderer);
}


} // namespace Umbra
