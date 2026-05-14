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
#include "umbraRemoteTask.hpp"
#include "umbraDirScan.hpp"
#include "umbraSerializer.hpp"
#include "umbraComputationArgs.hpp"

using namespace Umbra;

namespace Umbra
{

    // UMBRA_CONFIG_STR
#if defined(UMBRA_DEBUG)
#   define UMBRA_CONFIG_STR "_d"
#else
#   define UMBRA_CONFIG_STR ""
#endif

    // EXE_SUFFIX, PATH_SEP
#if UMBRA_OS == UMBRA_WINDOWS
#   define UMBRA_EXE_SUFFIX ".exe"
#   define UMBRA_PATH_SEPARATOR '\\'
#else
#   define UMBRA_EXE_SUFFIX ""
#   define UMBRA_PATH_SEPARATOR '/'
#endif

    /*---------------------------------------------------------------*//*!
     * \brief
     *//*---------------------------------------------------------------*/

    static bool getBackgroundExecutable(const String& exePath, String& executable)
    {
        String path = exePath;

        CheckPathResult res = checkPath(path.toCharPtr());
        if (res == CHECKPATH_NOT_FOUND)
            return false;

        if (res == CHECKPATH_DIRECTORY)
        {
            // append executable name unless explicitly specified
            if (path.length() > 0 && path[path.length() - 1] != UMBRA_PATH_SEPARATOR)
                path += String(UMBRA_PATH_SEPARATOR);

            if (Process::is64BitCapable() && !Process::is64BitProcess())
            {
                executable = path + String("umbraprocess64") + UMBRA_CONFIG_STR UMBRA_EXE_SUFFIX;
                if (fileExists(executable))
                    return true;

                executable = path + String("umbraprocess32") + UMBRA_CONFIG_STR UMBRA_EXE_SUFFIX;
                return true;
            }

            const char* bitness = "";
            bitness = (Process::is64BitProcess() ? "64" : "32");

            executable = path + String("umbraprocess") + bitness + UMBRA_CONFIG_STR UMBRA_EXE_SUFFIX;
        } else
            executable = path;

        return true;
    }

} // namespace Umbra

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildStateRemote::init(const String& UID, const TaskParams& params)
{
    m_localError = Computation::ERROR_OK;
    m_localErrorReason[0] = 0;

    m_shared = processAlloc<Shared>(UID);
    if (!m_shared)
    {
        setError(Computation::ERROR_PROCESS, "Background process: unable to allocate shared memory");
        return;
    }

    Shared& shared = *m_shared.p;

    new (m_shared.p) Shared();

    shared.m_structSize  = (UINT32)sizeof(Shared);
    strncpy(shared.m_buildNumber, UMBRA_STRINGIFY(UMBRA_BUILD_ID), 63);
    shared.m_buildNumber[63] = '\0';

    String sharingUID   = generateProcessUID();
    strncpy(shared.m_sharingUID, sharingUID.toCharPtr(), 63);
    shared.m_sharingUID[63] = '\0';

    String semaphoreUID = generateProcessUID();
    strncpy(shared.m_semaphoreUID, semaphoreUID.toCharPtr(), 63);
    shared.m_semaphoreUID[63] = '\0';

    shared.m_progress       = 0.f;
    shared.m_params         = params;
    shared.m_stopChild      = false;
    shared.m_childFinished  = false;
    shared.m_error          = Computation::ERROR_OK;
    shared.m_errorReason[0] = 0;

    // todo serialized in file
    memset(&shared.m_params.m_computationParams, 0, sizeof(ComputationParams));
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildStateRemote::init(const String& UID)
{
    m_localError = Computation::ERROR_OK;
    m_localErrorReason[0] = 0;

    m_shared = processAlloc<Shared>(UID);
    if (!m_shared)
    {
        setError(Computation::ERROR_PROCESS, "Background process: unable to get shared memory");
        return;
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildStateRemote::setError (Computation::Error err, const char* reason)
{
    UMBRA_ASSERT(err != Computation::ERROR_OK);
    if (!m_shared)
    {
        if (m_localError == Computation::ERROR_OK || m_shared.p->m_error == Computation::ERROR_ABORTED)
        {
            m_localError = err;
            if (reason)
                strcpy(m_localErrorReason, reason);
        }
    } else
    {
        if (m_shared.p->m_error == Computation::ERROR_OK || m_shared.p->m_error == Computation::ERROR_ABORTED)
        {
            if (reason)
                strcpy(m_shared.p->m_errorReason, reason);
            m_shared.p->m_error = err;
        }
    }
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Computation::Error BuildStateRemote::getError (void)
{
    Computation::Error error = (!m_shared) ? Computation::ERROR_OK : m_shared.p->m_error;
    if (error != Computation::ERROR_OK)
        return error;
    return m_localError;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

const char* BuildStateRemote::getErrorReason (void)
{
    Computation::Error error = (!m_shared) ? Computation::ERROR_OK : m_shared.p->m_error;
    if (error != Computation::ERROR_OK)
        return m_shared.p->m_errorReason;
    return m_localErrorReason;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool BuildStateRemote::checkVersion (void)
{
    UINT32 structSize = (UINT32)sizeof(Shared);
    String buildNumber = UMBRA_STRINGIFY(UMBRA_BUILD_ID);

    if (m_shared.p->m_structSize  != structSize ||
        String(m_shared.p->m_buildNumber) != buildNumber)
    {
        String error =
            "Background process: executable version mismatch: host version: " +
            String(m_shared.p->m_buildNumber) +
            ", client version: " +
            buildNumber +
            " (" +
            String(m_shared.p->m_structSize) +
            ", " +
            String(structSize) +
            ")";

        setError(Computation::ERROR_PROCESS, error.toCharPtr());

        return false;
    }

    return true;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BuildMasterRemote::BuildMasterRemote (Allocator* a)
: m_allocator(a),
  m_process(NULL),
  m_reader(NULL),
  m_semaphore(NULL)
{
    m_visTimer = clock();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BuildMasterRemote::~BuildMasterRemote(void)
{
    UMBRA_DELETE(m_semaphore);

    requestAbort();

    if (m_process)
        m_process->waitToFinish();

    UMBRA_DELETE(m_reader);
    UMBRA_DELETE(m_process);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildMasterRemote::init (Scene* scene, const TaskParams& params, Logger* userLogger)
{
    UMBRA_UNREF(userLogger);

    // Write scene to file
    UMBRA_ASSERT(scene);
    scene->writeToFile(params.getInputSceneFile(getAllocator()).toCharPtr());
    FileOutputStream fos(params.getParamFile(getAllocator()).toCharPtr());
    params.m_computationParams.writeToFile(params.getParamFile(getAllocator()).toCharPtr());
    
    m_shareUID    = generateProcessUID();
    m_state.init(m_shareUID, params);

    if (getError() != Computation::ERROR_OK)
        return;

    m_semaphore = UMBRA_NEW(Semaphore, m_state.getSemaphoreUID(), 0, 100);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

unsigned long BuildMasterRemote::run (void*)
{
    try
    {
        if (getError() != Computation::ERROR_OK)
            return 0;

        String executable;
        if (!getBackgroundExecutable(m_exePath, executable))
        {
            String notFound = String("Background process: executable path \"") + m_exePath + "\" was not found";
            setError(Computation::ERROR_EXECUTABLE_NOT_FOUND, notFound.toCharPtr());
            return 0;
        }

        m_process = UMBRA_NEW(Process, executable);
        m_reader = UMBRA_NEW(ProcessDataReader, m_state.getSharingUID(), true, m_process);

        Array<String> commandline;
        commandline.pushBack(String("\"") + String(m_shareUID) + String("\""));
        m_process->setCommandLine(commandline);

        Process::Error processError = m_process->run();

        if (processError == Process::ERROR_EXECUTABLE_NOT_FOUND)
        {
            String notFound = String("Background process: executable \"") + executable + "\" was not found";
            setError(Computation::ERROR_EXECUTABLE_NOT_FOUND, notFound.toCharPtr());

        }
        else if (processError != Process::ERROR_OK)
        {
            String str = "Error creating process";
            String err = getProcessError();
            if (err.length() > 0)
                str += ". " + err;
            setError(Computation::ERROR_PROCESS, str.toCharPtr());
        }

        while (!m_semaphore->tryDown(500))
        {
            if(m_process->isFinished())
            {
                UMBRA_DELETE(m_semaphore);
                m_semaphore = NULL;
                break;
            }
        }

    } catch (OOMException)
    {
        setError(Computation::ERROR_OUT_OF_MEMORY);
    }

    return 0;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BackgroundProcess::BackgroundProcess(Allocator* allocator) :
    m_taskAllocator(allocator, MEMORY_USAGE_HARD_LIMIT),
    m_parentProcess(NULL),
    m_scene(NULL),
    m_writer(NULL),
    m_semaphore(NULL)
{
    setAllocator(&m_taskAllocator);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

BackgroundProcess::~BackgroundProcess()
{
    UMBRA_DELETE(m_semaphore);
    UMBRA_DELETE(m_writer);
    UMBRA_DELETE(m_parentProcess);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BackgroundProcess::computationFinished (void)
{
    if (!m_state.isFinished())
    {
        m_state.setFinished();
        m_semaphore->up();
    }
}


/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool BackgroundProcess::shouldExit (void)
{
    if (m_parentProcess && m_parentProcess->isFinished())
    {
        // Parent process has exited.
        // Exit our process cleanly.
        if (m_state.getError() == Computation::ERROR_OK)
            m_state.setError(Computation::ERROR_ABORTED);
        return true;
    }

    if (!!m_state.m_shared && m_state.m_shared.p->m_stopChild)
    {
        if (m_state.getError() == Computation::ERROR_OK)
            m_state.setError(Computation::ERROR_ABORTED);
        return true;
    }

    return false;
}

/*---------------------------------------------------------------*//*!
* \brief
*//*---------------------------------------------------------------*/

ProcessBase* BackgroundProcess::getParentProcess(void* parameters)
{
    HandleProcess* parentProcess = 0;

#if UMBRA_OS == UMBRA_WINDOWS
    // Under windows we need to explicitly communicate what process is the parent process,
    // as this information is not available from the OS.
    if (parameters)
    {
        HandleProcess::OSProcessHandle handle = *(HandleProcess::OSProcessHandle*)parameters;
        parentProcess = UMBRA_NEW(HandleProcess, handle);
    }
#else
    UMBRA_UNREF(parameters);
    // Under posix special handling is implemented for the parent, as status information
    // of a process is only available for child processes.
    parentProcess = UMBRA_NEW(HandleProcess, HandleProcess::HANDLE_PARENT);
#endif

    return parentProcess;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BackgroundProcess::init (const String& memUID, void* parameters)
{
    m_state.init(memUID);

    if (m_state.getError() != Computation::ERROR_OK)
    {
        TaskLogger logger;
        logger.setMinLevel(Logger::LEVEL_DEBUG);
        UMBRA_LOG_E(&logger, "unexpected error (%d): \"%s\"", m_state.getError(), m_state.getErrorReason());
        return;
    }

    TaskParams& params = m_state.getTaskParams();
    m_scene = Umbra::Scene::create(params.getInputSceneFile(getAllocator()).toCharPtr());

    if (!m_scene)
    {
        // Report ERROR_INVALID_SCENE if scene couldn't be loaded.
        String error = String("scene \"") + params.getInputSceneFile(getAllocator()) + "\" could not be loaded";
        m_state.setError(Computation::ERROR_INVALID_SCENE, error.toCharPtr());
        return;
    }

    // Deserialize ComputationParams
    new (&params.m_computationParams) ComputationParams(getAllocator());
    bool paramsLoaded = ((ImpComputationParams&)params.m_computationParams).deserialize(params.getParamFile(getAllocator()).toCharPtr());

    if (!paramsLoaded)
    {
        String error = String("scene \"", getAllocator()) + params.getParamFile(getAllocator()) + "\" could not be loaded";
        m_state.setError(Computation::ERROR_UNKNOWN, error.toCharPtr());
        return;
    }

    m_parentProcess = getParentProcess(parameters);
    m_writer        = UMBRA_NEW(ProcessDataWriter, m_state.getSharingUID(), false, m_parentProcess);
    m_semaphore     = UMBRA_NEW(Semaphore, m_state.getSemaphoreUID(), 0, 100);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BackgroundProcess::run (void)
{
    if (m_state.getError() != Computation::ERROR_OK)
        return;

    if (!m_state.checkVersion())
        return;

    TaskParams& params = m_state.getTaskParams();

    if (params.m_memoryUsageLimit > 0)
        m_taskAllocator.setBudget(params.m_memoryUsageLimit*1024LL*1024LL);

    BuildMasterLocal build(getAllocator());
    build.init(m_scene, params, NULL);

    Thread thread;
    thread.setFunction(&build);
    thread.run(NULL);

    ProcessDataCopy::RequestType request = ProcessDataCopy::RT_NONE;

    // Loop until finished.
    // Process must continue to run after computation
    // in order to serve tome requests.

    do
    {
        m_state.setProgress(build.getProgress());

        if (thread.isFinished())
        {
            if (build.getError() != Computation::ERROR_OK)
                m_state.setError(build.getError(), build.getErrorReason());

            computationFinished();
        }

        // Monitor requests from writer.
        UMBRA_ASSERT(m_writer);

        // Wait for new request if none active
        if (request == ProcessDataCopy::RT_NONE)
            request = m_writer->waitRequest(500);

        ProcessOutputStream output(m_writer);

        if (request == ProcessDataCopy::RT_RUNTIME)
        {
            UINT32        size = 0;
            Umbra::UINT8* data = NULL;

            size = build.copyResult(NULL, 0);

            if (!size)
            {
                m_writer->write(0, 0);
                request = ProcessDataCopy::RT_NONE;
                continue;
            }

            data    = (Umbra::UINT8*)UMBRA_MALLOC(size);
            build.copyResult(data, size);

            output.write(data, size);

            UMBRA_FREE(data);
        }
        else if (request == ProcessDataCopy::RT_VISUALIZATIONS)
        {
            m_debug.reset();
            build.visualize(&m_debug);

            Serializer serializer(&output);
            stream(serializer, m_debug);
        }

        if (request != ProcessDataCopy::RT_NONE)
        {
            while(!output.communicate())
            {
                if (thread.isFinished())
                    computationFinished();

                // Check occasionally whether we should exit
                // This prevents background process from hanging
                if (shouldExit())
                {
                    build.requestAbort();
                    thread.waitToFinish();

                    return;
                }
            }

            request = ProcessDataCopy::RT_NONE;
        }

    } while(!shouldExit());

    build.requestAbort();
    computationFinished();
    thread.waitToFinish();

    m_scene->release();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Umbra::UINT32 BuildMasterRemote::copyResult (void* buf, Umbra::UINT32 size)
{
    if (!m_reader->active())
    {
        m_reader->request(ProcessDataCopy::RT_RUNTIME);
        m_reader->wait();
    }

    if (m_reader->getActive() != ProcessDataCopy::RT_RUNTIME)
        return 0;

    UINT32 runtimeSize = m_reader->getSize();

    if (!runtimeSize)
    {
        UINT32 bytesLeft = 0;
        UINT32 bytesWritten = 0;
        // Perform empty read
        m_reader->read( 0, bytesLeft, bytesWritten );
        return 0;
    }

    if (!buf || size < runtimeSize)
        return runtimeSize;

    ProcessInputStream stream(m_reader, getAllocator());
    stream.communicate();

    return stream.read(buf, size);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void BuildMasterRemote::visualize(DebugRenderer* debug)
{
    UMBRA_ASSERT(m_reader);

    if (m_reader && !m_reader->active() && clock() - m_visTimer > CLOCKS_PER_SEC / 5)
    {
        ProcessInputStream input(ProcessDataCopy::RT_VISUALIZATIONS, m_reader, getAllocator());
        input.communicate();

        Deserializer deserializer(&input);
        m_state.getDebugRenderer().reset();
        stream(deserializer, m_state.getDebugRenderer());

        m_visTimer = clock();
    }

    m_state.getDebugRenderer().forward(debug);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void CachingDebugRenderer::reset(void)
{
    m_lines.clear();
    m_points.clear();
    m_aabbs.clear();
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void CachingDebugRenderer::addLine(const Vector3& start, const Vector3& end, const Vector4& color)
{
    Line line;
    line.p1 = start;
    line.p2 = end;
    line.color = color;
    m_lines.pushBack(line);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void CachingDebugRenderer::addPoint(const Vector3& pt, const Vector4& color)
{
    Point p;
    p.p = pt;
    p.color = color;
    m_points.pushBack(p);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void CachingDebugRenderer::addAABB(const Vector3& mn, const Vector3& mx, const Vector4& color)
{
    Bound bound;
    bound.aabb.set(mn, mx);
    bound.color = color;
    m_aabbs.pushBack(bound);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void CachingDebugRenderer::forward(DebugRenderer* debug)
{
    for (int i = 0; i < m_lines.getSize(); i++)
        debug->addLine(m_lines[i].p1, m_lines[i].p2, m_lines[i].color);

    for (int i = 0; i < m_points.getSize(); i++)
        debug->addPoint(m_points[i].p, m_points[i].color);

    for (int i = 0; i < m_aabbs.getSize(); i++)
        debug->addAABB(m_aabbs[i].aabb.getMin(), m_aabbs[i].aabb.getMax(), m_aabbs[i].color);
}

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)
