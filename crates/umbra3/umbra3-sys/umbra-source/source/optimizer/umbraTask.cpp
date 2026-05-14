#include "umbraPrivateDefs.hpp"
#include "optimizer/umbraTask.hpp"
#include "optimizer/umbraLocalComputation.hpp"
#include "umbraMemory.hpp"
#include "umbraAABB.hpp"
#include "runtime/umbraQuery.hpp"
#include "umbraImpScene.hpp"
#include "umbraTileGrid.hpp"
#include "umbraFileStream.hpp"

namespace Umbra
{

class ImpTask
{
public:
    ImpTask (Scene* scene, const Vector3* mn, const Vector3* mx, Allocator* allocator)
        : m_computationParams(allocator)
        , m_executablePath(allocator)
        , m_licenseKey(allocator)
    {
        ImpScene::ref(scene);
        m_localParams.scene = scene;
        m_localParams.computationParams = &m_computationParams;
        if (mn && mx)
        {
            m_compAABB = AABB(*mn, *mx);
            m_localParams.boundsMin = &m_compAABB.getMin();
            m_localParams.boundsMax = &m_compAABB.getMax();
        }
        else
        {
            m_localParams.boundsMin = NULL;
            m_localParams.boundsMax = NULL;
        }

        m_localParams.allocator = allocator;
        memset(&m_computationResult, 0, sizeof(m_computationResult));
    }

    ~ImpTask ()
    {
        if (m_computation)
            m_computation->release();
        if (m_localParams.scene)
            m_localParams.scene->release();
    }

    Allocator* getAllocator() { return m_localParams.allocator; }

    void setRunAsProcess(const char* executablePath)
    {
        m_executablePath = executablePath;
    }

    void setComputationParams (const ComputationParams& params)
    {
        m_computationParams = params;
    }

    void setMemoryUsageLimit (int megabytes)
    {
        m_localParams.memUsageLimitMegs = megabytes;
    }

    void setNumThreads (int numThreads)
    {
        m_localParams.numThreads  = numThreads;
    }

    void setSilent (bool b)
    {
        m_localParams.silent = b;
    }

    void setCacheSize (int s)
    {
        m_localParams.cacheSizeMegs = s;
    }

    void setLogger (Logger* logger)
    {
        m_localParams.logger = logger;
    }

    void start (const char* tempPath, const char* prefix)
    {
        m_localParams.tempPath			= tempPath;
        m_localParams.tempFilePrefix	= prefix;
        m_localParams.licenseKey        = m_licenseKey.toCharPtr();
        m_localParams.runAsProcessPath  = m_executablePath.toCharPtr();
        m_computation = LocalComputation::create(m_localParams);
    }

    void abort(void)
    {
        m_computation->requestAbort();
        // Task API abort is blocking
        waitForFinish();
    }

    void waitForFinish(void)
    {
        m_computationResult = m_computation->waitForResult(getAllocator());
    }

    bool isFinished(void)
    {
        m_computationResult = m_computation->waitForResult(getAllocator(), 0);
        return m_computationResult.error != Computation::ERROR_WAIT_TIMEOUT;
    }

    float getProgress(void)
    {
        m_computationResult = m_computation->waitForResult(getAllocator(), 0);
        return m_computationResult.progress;
    }

    void setLicenseKey (const char* key)
    {
        m_licenseKey = key;
    }

    Task::Error getError(void)
    {
        waitForFinish();
        return (Task::Error)m_computationResult.error;
    }

    const char* getErrorString()
    {
        waitForFinish();
        return m_computationResult.errorStr;
    }

    UINT32 getTomeSize(void) const
    {
        return m_computationResult.tomeSize;
    }

    const Tome* getTome (void* buf, UINT32 size) const
    {
        if (size < m_computationResult.tomeSize)
            return NULL;
        memcpy(buf, m_computationResult.tome, m_computationResult.tomeSize);
        return (const Tome*)buf;
    }

    bool intersectsViewVolumes(const AABB& bound)
    {
        if (!m_localParams.scene)
            return true;

        if (m_localParams.scene->getViewVolumeCount() == 0)
            return true; // uses scene bounds by default

        for (int i = 0; i < m_localParams.scene->getViewVolumeCount(); i++)
        {
            const SceneVolume* v = m_localParams.scene->getViewVolume(i);
            AABB aabb((Vector3&)v->getMin(), (Vector3&)v->getMax());

            if (bound.intersects(aabb))
                return true;
        }

        return false;
    }

    /*-------------------------------------------------------------------*//*!
     * \brief   visualizeViewCells
     *//*-------------------------------------------------------------------*/
    void visualizeCellSize(DebugRenderer* debug, const AABB& bound, int depth, int limit)
    {
         if (!intersectsViewVolumes(bound))
         {
            debug->addAABB(bound.getMin(), bound.getMax(), Vector4(1,1,1,1));
            return;
         }

        if (depth < limit)
        {
            Vector3 p = bound.getCenter();
            for (int octant = 0; octant < 8; octant++)
            {
                AABB aabbChild = bound;
                for (int axis = 0; axis < 3; axis++)
                    if (octant & (1 << axis))
                        aabbChild.setMin(axis, p[axis]);
                    else
                        aabbChild.setMax(axis, p[axis]);

                visualizeCellSize(debug, aabbChild, depth + 1, limit);
            }
        }
        else
        {
            debug->addAABB(bound.getMin(), bound.getMax(), Vector4(1,1,1,1));
        }
    }

    /*-------------------------------------------------------------------*//*!
     * \brief   Visualization
     *//*-------------------------------------------------------------------*/

    void visualizeParam(DebugRenderer* debug)
    {
        float tileSize, smallestOccluder;
        m_computationParams.getParam(ComputationParams::TILE_SIZE, tileSize);
        m_computationParams.getParam(ComputationParams::SMALLEST_OCCLUDER, smallestOccluder);

        if (!m_localParams.scene || tileSize <= 0.f || smallestOccluder <= 0.f)
            return;

        ImpScene* scene = ImpScene::getImplementation(m_localParams.scene);
        AABB sceneBounds = scene->getAABB();

        int cellSplits = max2(Math::intChop(::log(tileSize / smallestOccluder) / ::log(2.f) + 0.5f), 0);

        AABBi grid;
        Vector3i mn, mx;
        TileGrid::calcGrid(mn, mx, sceneBounds, tileSize);
        grid.set(mn, mx);

        for (int x = grid.getMin().i; x < grid.getMax().i; x++)
        for (int y = grid.getMin().j; y < grid.getMax().j; y++)
        for (int z = grid.getMin().k; z < grid.getMax().k; z++)
        {
            AABB aabb(Vector3(x * tileSize, y * tileSize, z * tileSize),
                      Vector3((x+1) * tileSize, (y+1) * tileSize, (z+1) * tileSize));
            debug->addAABB(aabb.getMin(), aabb.getMax(), Vector4(0,1,0,1));
        }

        for (int x = grid.getMin().i; x < grid.getMax().i; x++)
        for (int y = grid.getMin().j; y < grid.getMax().j; y++)
        for (int z = grid.getMin().k; z < grid.getMax().k; z++)
        {
            AABB aabb(Vector3(x * tileSize, y * tileSize, z * tileSize),
                      Vector3((x+1) * tileSize, (y+1) * tileSize, (z+1) * tileSize));
            visualizeCellSize(debug, aabb, 0, cellSplits);
        }
    }

    /*-------------------------------------------------------------------*//*!
     * \brief   Visualization
     *//*-------------------------------------------------------------------*/

    void visualize (Task::VisualizationFlags flags, DebugRenderer* debugRenderer)
    {
        if (flags & Task::VISUALIZATION_PARAM)
            visualizeParam(debugRenderer);
        if (flags & Task::VISUALIZATION_PROGRESS)
            m_computation->visualize(debugRenderer);
    }


private:
    LocalComputation::Params	m_localParams;
    ComputationParams           m_computationParams;
    Computation*				m_computation;
    Computation::Result         m_computationResult;
    AABB                        m_compAABB;

    String                      m_executablePath;
    String                      m_licenseKey;
};

Task::Task(ImpTask* imp) : m_imp(imp) {}

Task::~Task(void) 
{ 
    Allocator* allocator = m_imp->getAllocator();
    UMBRA_HEAP_DELETE(allocator, m_imp); 
    m_imp = 0; 
}

Task* Task::create(Scene* scene, const Vector3* mn, const Vector3* mx, Allocator* allocator)
{
    if (!scene)
        return NULL;

    if (!allocator)
        allocator = Umbra::getAllocator();
    ImpTask* imp = UMBRA_HEAP_NEW(allocator, ImpTask, scene, mn, mx, allocator);
    if (!imp)
        return NULL;
    return UMBRA_HEAP_NEW(allocator, Task, imp);
}

void Task::release(void)
{
    if (this && m_imp)
    {
        Allocator* allocator = m_imp->getAllocator();
        UMBRA_HEAP_DELETE2(allocator, Task, this);
    }
}

void Task::setRunAsProcess(const char* executablePath)
{
    m_imp->setRunAsProcess(executablePath);
}

void Task::setComputationParams (const ComputationParams& params)
{
    m_imp->setComputationParams(params);
}

void Task::setMemoryUsageLimit(int megabytes)
{
    m_imp->setMemoryUsageLimit(megabytes);
}

void Task::setNumThreads(int numThreads)
{
    m_imp->setNumThreads(numThreads);
}

void Task::setSilent(bool b)
{
    m_imp->setSilent(b);
}

void Task::setCacheSize(int s)
{
    m_imp->setCacheSize(s);
}

void Task::start(const char* tempPath, const char* prefix)
{
    m_imp->start(tempPath, prefix);
}

void Task::abort(void)
{
    m_imp->abort();
}

void Task::waitForFinish(void)
{
    m_imp->waitForFinish();
}

bool Task::isFinished(void)
{
    return m_imp->isFinished();
}

float Task::getProgress(void)
{
    return m_imp->getProgress();
}

void Task::setLicenseKey (const char* key)
{
    m_imp->setLicenseKey(key);
}

void Task::setLogger (Logger* logger)
{
    m_imp->setLogger(logger);
}

Task::Error Task::getError(void)
{
    return m_imp->getError();
}

const char* Task::getErrorString()
{
    return m_imp->getErrorString();
}

Umbra::UINT32 Task::getTomeSize(void) const
{
    return m_imp->getTomeSize();
}

void Task::visualize(Task::VisualizationFlags flags, class DebugRenderer* debugRenderer) const
{
    m_imp->visualize(flags, debugRenderer);
}

void Task::writeTomeToFile (const char* fileName) const
{
    UINT32 rtSize = getTomeSize();
    if (!rtSize)
        return;

    UINT8* tmp = UMBRA_NEW_ARRAY(UINT8, rtSize);
    getTome(tmp, rtSize);

    FileOutputStream s(fileName);

    if (s.isOpen())
    {
        UINT32 dwords = rtSize / 4;
        UINT32* ptr = (UINT32*)tmp;
        StreamWriter writer(&s);
        while (dwords--)
            writer.put(*ptr++);
    }

    UMBRA_DELETE_ARRAY(tmp);
}

const Tome* Task::getTome (void* buf, Umbra::UINT32 size) const
{
    return m_imp->getTome(buf, size);
}

} // namespace Umbra
