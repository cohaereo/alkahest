#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraTileGrid.hpp"
#include "umbraImpScene.hpp"
#include "umbraBitMath.hpp"
#include "umbraIntersectExact.hpp"
#include "umbraLogger.hpp"
#include "umbraSIMD.hpp"
#include "umbraFPUControl.hpp"
#include "umbraSort.hpp"
#include "optimizer/umbraComputationParams.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraComputationArgs.hpp"
#include <cstdio>

#define LOGE(...) UMBRA_LOG_E(m_platform.logger, __VA_ARGS__)
#define LOGI(...) UMBRA_LOG_I(m_platform.logger, __VA_ARGS__)
#define LOGW(...) UMBRA_LOG_W(m_platform.logger, __VA_ARGS__)
#define LOGD(...) UMBRA_LOG_D(m_platform.logger, __VA_ARGS__)

using namespace Umbra;

namespace Umbra
{

bool TileGrid::calcGrid (Vector3i& iMin, Vector3i& iMax, const AABB& bounds, float tileSize)
{
    UMBRA_ASSERT(tileSize > 0.f);

    for (int i = 0; i < 3; i++)
    {
        if (bounds.getMin()[i] / tileSize < -float(1<<30) ||
            bounds.getMax()[i] / tileSize >  float(1<<30))
            return false;

        int p = int(bounds.getMin()[i] / tileSize)+2;
        while (p*tileSize > bounds.getMin()[i])
            p--;

        int q = 1;
        while ((p+q)*tileSize < bounds.getMax()[i])
            q++;

        iMin[i] = p;
        iMax[i] = p+q;
    }

    UMBRA_ASSERT(AABB(Vector3((float)iMin.i, (float)iMin.j, (float)iMin.k)*tileSize,
                      Vector3((float)iMax.i, (float)iMax.j, (float)iMax.k)*tileSize).contains(bounds));
    return true;
}

}

TileGrid::TileGrid (const PlatformServices& platform)
:   m_platform(platform),
    m_timer(platform.allocator),
    m_scene(NULL),
    m_nodes(platform.allocator),
    m_viewVolumes(platform.allocator),
    m_compVisualizations(false),
    m_strictViewVolumes(false),
    m_compAccurateDilation(false),
    m_hasFilterAABB(false),
    m_computationString(platform.allocator)
{}

TileGrid::~TileGrid ()
{
    /* \todo [antti 10.10.2011]: fix this */
    bool old = allowDefaultAllocator(true);
    if (m_scene)
    {
        // \todo [Hannu] implement support to refcount const Scene*
        ((Scene*)m_scene)->release();
    }
    allowDefaultAllocator(old);
}

void TileGrid::reset (void)
{
    m_nodes.clear();
}




Builder::Error TileGrid::create (const Scene* scene_, const ComputationParams& params, const AABB& filterAABB)
{
    m_timer.startTimer("TileGrid::create");

    UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY

    /* \todo [antti 10.10.2011]: fix this */
    bool old = allowDefaultAllocator(true);
    if (m_scene)
        ((Scene*)m_scene)->release();
    allowDefaultAllocator(old);

    reset();

    ImpScene::ref((Scene*)scene_);
    m_scene = scene_;

    ImpScene* scene = ImpScene::getImplementation((Scene*)m_scene);
    AABB sceneBounds = filterAABB.isOK() ? filterAABB : scene->getAABB();
    if (sceneBounds.getVolume() <= 0.f)
        return Builder::ERROR_INVALID_SCENE;

    m_hasFilterAABB = filterAABB.isOK();
    m_filterAABB    = filterAABB;

    // \todo validate/fix params
    float smallestHole;
    float clusterSize;
    float objectGroupCost;

    params.getParam(ComputationParams::BACKFACE_LIMIT,    m_bfLimit);
    params.getParam(ComputationParams::SMALLEST_OCCLUDER, m_smallestOccluder);
    params.getParam(ComputationParams::SMALLEST_HOLE,     smallestHole);
    params.getParam(ComputationParams::CLUSTER_SIZE,      clusterSize);
    params.getParam(ComputationParams::OBJECT_GROUP_COST, objectGroupCost);

    if (m_smallestOccluder <= 0.f)
    {
        LOGE("smallest occluder value not given");
        return Builder::ERROR_PARAM; // TODO: ERROR_PARAM
    }

    if (clusterSize > 0.f)
    {
        m_tileSize = clusterSize;

        while (m_tileSize / m_smallestOccluder > 8.f)
            m_tileSize /= 2.f;

        m_unitSize = m_tileSize / 4.f;
    }
    else
    {
        m_unitSize = m_smallestOccluder;
        m_tileSize = 4.f * m_unitSize;
    }

    m_unitsPerTile = int(m_tileSize / m_unitSize);

    UINT32 flags = 0;
    params.getParam(ComputationParams::OUTPUT_FLAGS, flags);
    m_compVisualizations = !!(flags & ComputationParams::DATA_VISUALIZATIONS);
    m_compAccurateDilation = !!(flags & ComputationParams::DATA_ACCURATE_DILATION);
    m_strictViewVolumes = !!(flags & ComputationParams::DATA_STRICT_VIEW_VOLUMES);

    if (smallestHole <= 0.f)
    {
        smallestHole = m_smallestOccluder / 8.f;
        LOGW("Invalid smallest hole value, using %f", smallestHole);
    }

    // \todo should we accept bf-limit of zero?
    if (m_bfLimit <= 0.f)
    {
        m_bfLimit = 20.f;
        LOGW("Invalid backface limit value, using %f", m_bfLimit);
    }

    m_smallestOccluder  = min2(m_smallestOccluder, m_tileSize);
    smallestHole        = min2(smallestHole, m_smallestOccluder);

    LOGI("T: %g O: %g H: %g B: %g", m_tileSize, m_smallestOccluder, smallestHole, m_bfLimit);

    // Tile inflation and bf distance stored in SIMD format
    float tileInflation = 2.f * smallestHole;
    float bfDistance    = max2(1.6f * m_smallestOccluder, tileInflation);
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, vTileInfl)  = Vector4(tileInflation, tileInflation, tileInflation, 0.f);
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, vBfDist)    = Vector4(bfDistance, bfDistance, bfDistance, 0.f);

    m_tileInflation = SIMDLoadAligned(&vTileInfl.x); // RETHINK: should this vary per tile as well?
    m_bfDistance    = SIMDLoadAligned(&vBfDist.x); // RETHINK: should this vary per tile as well?

    // \todo FIX THIS!!!
    UINT32 argFlags;
    params.getParam(ComputationParams::OUTPUT_FLAGS, argFlags);
    m_featureSize = m_smallestOccluder * 1.f; // RETHINK: this should vary per tile as well

    m_cellSplits = max2(Math::intChop(::log(m_tileSize / m_smallestOccluder) / ::log(2.f) + 0.5f), 0);
    m_smallestHoleSplits = max2(Math::intChop(::log(m_tileSize / smallestHole) / ::log(2.f) + 0.5f), 0);

    // View volumes
    m_numViewVolumes = m_scene->getViewVolumeCount();
    if (!m_numViewVolumes)
        LOGW("No view volumes set, using scene bounds as view volume");

    m_viewVolumes.resize(m_numViewVolumes);
    for (int v = 0; v < m_numViewVolumes; v++)
    {
        const SceneVolume* vol = m_scene->getViewVolume(v);
        m_viewVolumes[v].aabb.set(AABB(vol->getMin(), vol->getMax()));

        UINT32 volName = vol->getID();
        m_viewVolumes[v].sceneVolIdx = v;

        float smallestOccluder;
        if (params.getVolumeParam(volName, ComputationParams::SMALLEST_OCCLUDER, smallestOccluder) && smallestOccluder > 0.f)
        {
            m_viewVolumes[v].cellLevel = max2(Math::intChop(::log(m_tileSize / smallestOccluder) / ::log(2.f) + 0.5f), 0);
            m_viewVolumes[v].smallestOccluder = smallestOccluder;
            m_viewVolumes[v].featureSize = smallestOccluder;
        }
        else
        {
            m_viewVolumes[v].cellLevel = -1;
            m_viewVolumes[v].smallestOccluder = -1.f;
            m_viewVolumes[v].featureSize = -1.f;
        }

        float smallestHole;
        if (params.getVolumeParam(volName, ComputationParams::SMALLEST_HOLE, smallestHole) && smallestHole > 0.f)
            m_viewVolumes[v].smallestHoleLevel = max2(Math::intChop(::log(m_tileSize / smallestHole) / ::log(2.f) + 0.5f), 0);
        else
            m_viewVolumes[v].smallestHoleLevel = -1;

        float bfLimit;
        if (params.getVolumeParam(volName, ComputationParams::BACKFACE_LIMIT, bfLimit) && bfLimit > 0.f)
            m_viewVolumes[v].backfaceLimit = bfLimit;
        else
            m_viewVolumes[v].backfaceLimit = -1.f;

        m_viewVolumes[v].name = volName;
    }

    // Calculate grid
    if (!calcGrid(m_iMin, m_iMax, sceneBounds, m_unitSize))
        return Builder::ERROR_INVALID_SCENE;

    if (!m_hasFilterAABB)
    {
        for (int i = 0; i < 3; i++)
        {
            while ((m_iMin[i] % m_unitsPerTile) != 0)
                m_iMin[i]--;
            while ((m_iMax[i] % m_unitsPerTile) != 0)
                m_iMax[i]++;
        }
    }

    Vector3i iGrid = m_iMax - m_iMin;

    UMBRA_ASSERT(!m_hasFilterAABB || float(m_iMin.i) * m_unitSize == filterAABB.getMin().x);
    UMBRA_ASSERT(!m_hasFilterAABB || float(m_iMin.j) * m_unitSize == filterAABB.getMin().y);
    UMBRA_ASSERT(!m_hasFilterAABB || float(m_iMin.k) * m_unitSize == filterAABB.getMin().z);
    UMBRA_ASSERT(!m_hasFilterAABB || float(m_iMax.i) * m_unitSize == filterAABB.getMax().x);
    UMBRA_ASSERT(!m_hasFilterAABB || float(m_iMax.j) * m_unitSize == filterAABB.getMax().y);
    UMBRA_ASSERT(!m_hasFilterAABB || float(m_iMax.k) * m_unitSize == filterAABB.getMax().z);

    // Computation string.

    {
        char str[1024];
        std::sprintf(str, "T %.1f SO %.1f SH %.3f BF %.0f F %x CS %.1f",
                m_tileSize,
                m_smallestOccluder,
                smallestHole,
                m_bfLimit,
                flags & (ComputationParams::DATA_ACCURATE_DILATION | ComputationParams::DATA_STRICT_VIEW_VOLUMES),
                clusterSize);
        m_computationString = str;
    }

    m_numEmptyTiles = 0;
    // Calculate nodes
    Builder::Error err = calcGridNodes();
    m_timer.stopTimer("TileGrid::create");

    if (err == Builder::SUCCESS)
    {
        double timerVal = m_timer.getTimerValue("TileGrid::create");

        LOGI("Tile grid %dx%dx%d (at %d,%d,%d) created for scene: %d tiles (%d empties not included) in %.3f seconds",
             iGrid.i, iGrid.j, iGrid.k, m_iMin.i, m_iMin.j, m_iMin.k, getNumNodes(), m_numEmptyTiles, timerVal);
    }
    return err;
}

inline void TileGrid::calcNodeAABBs(const Vector3i& iNodeMin, const Vector3i& iNodeMax, SIMDAABB& nodeAABB, SIMDAABB& inflatedAABB, SIMDAABB& backfaceAABB)
{
    nodeAABB.set(AABB(Vector3((float)iNodeMin.i, (float)iNodeMin.j, (float)iNodeMin.k) * m_unitSize,
                 Vector3((float)iNodeMax.i, (float)iNodeMax.j, (float)iNodeMax.k) * m_unitSize));

    inflatedAABB = nodeAABB.inflated(m_tileInflation);
    backfaceAABB = nodeAABB.inflated(m_bfDistance);
}

Builder::Error TileGrid::calcGridNodes (void)
{
    m_transformedVertices.setAllocator(m_platform.allocator);
    m_transformedVertices.reset(m_scene->getObjectCount());
    m_objVec.setAllocator(m_platform.allocator);
    m_objVec.resize(m_scene->getObjectCount(), false, false);

    SIMDAABB rootAABB, inflatedRootAABB, backfaceRootAABB;
    calcNodeAABBs(m_iMin, m_iMax, rootAABB, inflatedRootAABB, backfaceRootAABB);
    rootAABB.grow(inflatedRootAABB);
    rootAABB.grow(backfaceRootAABB);

    // Collect objects and triangles, transform vertices
    Array<Obj>      objects  (m_platform.allocator);
    for (int objSceneIdx = 0; objSceneIdx < m_scene->getObjectCount(); objSceneIdx++)
    {
        const SceneObject* object = m_scene->getObject(objSceneIdx);
        // Skip objects that don't intersect the computation AABB
        Vector3 mn, mx;
        object->getBounds(mn, mx);
        if (!SIMDAABB(AABB(mn, mx)).intersects(rootAABB))
            continue;

        const SceneModel*   model       = object->getModel();
        int                 numVerts    = model->getVertexCount();
        int                 numTris     = model->getTriangleCount();

        if (!numVerts || !numTris)
            continue;

        Array<Vector3>& vCache = m_transformedVertices[objSceneIdx];
        vCache.setAllocator(m_platform.allocator);
        vCache.reset(numVerts);

        // Transform
        Matrix4x4 publicMatrix;
        object->getMatrix(publicMatrix, MF_ROW_MAJOR);
        Matrix4x3 matrix = publicMatrix.get4x3Matrix();

        if (matrix.isIdentity())
            for (int v = 0; v < numVerts; v++)
                vCache[v] = model->getVertices()[v];
        else
            for (int v = 0; v < numVerts; v++)
            {
                Vector3 vert = model->getVertices()[v];
                vCache[v] = matrix.transform(vert);
            }

        AABB objAABB = ImpScene::getImplementation(const_cast<SceneObject*>(object))->getAABB();
        Obj obj;
        obj.aabb.set(objAABB);
        obj.flags = object->getFlags();
        obj.objSceneIdx = objSceneIdx;
        objects.pushBack(obj);
    }

    // Set up pointer arrays for sorting
    int numObjs = objects.getSize();
    Array<const Obj*> objPtrs(numObjs, m_platform.allocator);
    Array<const Obj*> volObjPtrs(m_platform.allocator);
    for (int i = 0; i < numObjs; i++)
    {
        objPtrs[i] = &objects[i];
        if (m_scene->getObject(objects[i].objSceneIdx)->getFlags() & SceneObject::VOLUME)
            volObjPtrs.pushBack(&objects[i]);
    }

    int numVolumes = m_viewVolumes.getSize();
    Array<const Volume*> volumePtrs(numVolumes, m_platform.allocator);
    for (int i = 0; i < numVolumes; i++)
        volumePtrs[i] = &m_viewVolumes[i];

    SplitState ss(m_platform.allocator);
    ss.objs           = objPtrs.getPtr();
    ss.numObjs        = objPtrs.getSize();
    ss.volObjs        = volObjPtrs.getPtr();
    ss.numVolObjs     = volObjPtrs.getSize();
    ss.viewVolumes    = volumePtrs.getPtr();
    ss.numViewVolumes = volumePtrs.getSize();

    return calcGridNodesRec(ss, m_iMin, m_iMax);
}

void TileGrid::filterSplitState(SplitState& out, const SplitState& in, Vector3i mn, Vector3i mx)
{
    // Calculate AABBs.

    SIMDAABB nodeAABB, inflatedAABB, backfaceAABB;
    calcNodeAABBs(mn, mx, nodeAABB, inflatedAABB, backfaceAABB);
    UMBRA_ASSERT(backfaceAABB.contains(inflatedAABB));

    // Prepare output.

    out.objs = in.objs;
    out.numObjs = 0;
    out.volObjs = in.volObjs;
    out.numVolObjs = 0;
    out.viewVolumes = in.viewVolumes;
    out.numViewVolumes = 0;
    out.triangles = in.triangles;
    out.hasOccluders = false;

    // Intersect objects. Only fully contained objects are kept as is and others are
    // opened to set of triangles.

    int numIntersectingObjects = 0;

    for (int i = 0; i < in.numObjs; i++)
    {
        if (inflatedAABB.contains(out.objs[i]->aabb))
        {
            if (out.objs[i]->flags & SceneObject::OCCLUDER)
                out.hasOccluders = true;

            swap2(out.objs[i], out.objs[out.numObjs]);
            out.numObjs++;
        }
        else if (backfaceAABB.intersects(out.objs[i]->aabb))
            numIntersectingObjects++;
    }

    if (numIntersectingObjects > 0)
    {
        out.newTriangles.reset(numIntersectingObjects);

        int j = 0;
        for (int i = 0; i < in.numObjs; i++)
        {
            if (!inflatedAABB.contains(out.objs[i]->aabb) && backfaceAABB.intersects(out.objs[i]->aabb))
            {
                int idx = out.objs[i]->objSceneIdx;

                const SceneObject* object = m_scene->getObject(idx);
                const SceneModel* model = object->getModel();

                int n = model->getTriangleCount();

                out.newTriangles[j].reset(n);
                for (int k = 0; k < n; k++)
                    out.newTriangles[j][k] = k;

                TriangleList tl;
                tl.objSceneIdx = idx;
                tl.flags       = object->getFlags();
                tl.n           = n;
                tl.tris        = out.newTriangles[j].getPtr();
                tl.aabb        = out.objs[i]->aabb;

                out.triangles.pushBack(tl);

                j++;
            }
        }

        // Sort occlusive objects first to avoid intersection tests.

        quickSort(out.triangles.getPtr(), out.triangles.getSize());

        UMBRA_ASSERT(j == numIntersectingObjects);
    }

    // Intersect volume objects (just AABB).

    for (int i = 0; i < in.numVolObjs; i++)
        if (inflatedAABB.intersects(out.volObjs[i]->aabb))
        {
            swap2(out.volObjs[i], out.volObjs[out.numVolObjs]);
            out.numVolObjs++;
        }

    // Intersect view volumes.

    for (int i = 0; i < in.numViewVolumes; i++)
        if (nodeAABB.intersects(out.viewVolumes[i]->aabb))
        {
            swap2(out.viewVolumes[i], out.viewVolumes[out.numViewVolumes]);
            out.numViewVolumes++;
        }

    // Intersect triangles.

    for (int i = 0; i < out.triangles.getSize(); i++)
    {
        TriangleList& tl = out.triangles[i];
        const Vector3i* triangles = m_scene->getObject(tl.objSceneIdx)->getModel()->getTriangles();

        SIMDAABB aabb = nodeAABB;

        if (tl.flags & SceneObject::OCCLUDER)
            aabb.grow(backfaceAABB);

        if (tl.flags & SceneObject::TARGET)
            aabb.grow(inflatedAABB);

        if (!aabb.intersects(tl.aabb))
        {
            out.triangles.removeSwap(i);
            i--;
            continue;
        }

        int k = 0;
        for (int j = 0; j < tl.n; j++)
        {
            const Array<Vector3>& vCache = m_transformedVertices[tl.objSceneIdx];
            Vector3i ind = triangles[tl.tris[j]];
            Vector3 a = vCache[ind.i];
            Vector3 b = vCache[ind.j];
            Vector3 c = vCache[ind.k];

            if (intersectAABBTriangleSIMD(aabb.m_mn, aabb.m_mx, a, b, c))
            {
                if (!out.hasOccluders && (tl.flags & SceneObject::OCCLUDER))
                {
                    if (intersectAABBTriangleSIMD(inflatedAABB.m_mn, inflatedAABB.m_mx, a, b, c))
                        out.hasOccluders = true;
                }

                swap2(tl.tris[j], tl.tris[k]);
                k++;
            }
        }

        // NOTE: surprisingly it didn't seem to be worth updating new AABB here

        tl.n = k;

        if (tl.n == 0)
        {
            out.triangles.removeSwap(i);
            i--;
        }
    }
}

static int adjustLevel(int lvl, int size, int unitsPerTile)
{
    if (size < unitsPerTile)
    {
        int d = size;

        while (d < unitsPerTile)
        {
            lvl--;
            d *= 2;
        }

        lvl = max2(0, lvl);
    }

    return lvl;
}

Builder::Error TileGrid::calcGridNodesRec (const SplitState& ssIn, Vector3i iNodeMin, Vector3i iNodeMax)
{
    SIMDAABB nodeAABB, inflatedAABB, backfaceAABB;
    calcNodeAABBs(iNodeMin, iNodeMax, nodeAABB, inflatedAABB, backfaceAABB);

    SplitState ss(m_platform.allocator);
    filterSplitState(ss, ssIn, iNodeMin, iNodeMax);

    Vector3i size = iNodeMax - iNodeMin;
    UMBRA_ASSERT(size.i > 0 && size.j > 0 && size.k > 0);

    // Calculate split bias for tiles not intersecting
    int splitBias = 0;
    if (m_numViewVolumes && !ss.hasViewVolumes())
        splitBias = calcSplitBias(nodeAABB);

    // Skip empty outside tiles.
    // Empty inside tiles are generated so that tome generation can tell apart inside empty
    // space from outside empty space.

    if (ss.isEmpty() && m_numViewVolumes)
    {
        m_numEmptyTiles++;
        return Builder::SUCCESS;
    }

    // Inner node?

    int minSize = m_unitsPerTile << splitBias;
    bool isMinSize = (size.i <= minSize) && (size.j <= minSize) && (size.k <= minSize);
    bool isCube = (size.i == size.j) && (size.j == size.k);

    isCube = true; // Always a cube, it doesn't matter if it is not.

    if (max2(size.i, max2(size.j, size.k)) / min2(size.i, min2(size.j, size.k)) > 8) // Must be close enough to a cube.
        isCube = false;

    // non-empty tiles that are larger than minsize or not cubes need to split

    bool doSplit = (ss.hasOccluders && (!isCube || !isMinSize)) ||
        !isPowerOfTwo(size.i) || !isPowerOfTwo(size.j) || !isPowerOfTwo(size.k);

    // Also split if volume parameter override would yield too deep cell grid.

    if ((size.i > 1 || size.j > 1 || size.k > 1) && ss.hasOccluders && ss.numViewVolumes > 0)
    {
        int localCellLevel = m_cellSplits;

        for (int i = 0; i < ss.numViewVolumes; i++)
        {
            const Volume* vol = ss.viewVolumes[i];
            if (vol->cellLevel > localCellLevel)
                localCellLevel = vol->cellLevel;
        }

        localCellLevel = adjustLevel(localCellLevel, size.i, m_unitsPerTile);

        if (localCellLevel > m_cellSplits)
            doSplit = true;
    }

    if (doSplit)
    {
        int axis = (size.i >= size.j) ? ((size.i >= size.k) ? 0 : 2) : ((size.j >= size.k) ? 1 : 2);

        // HACK: when power of two is limiting this from becoming a leaf, try to split non-power of twos

        if (!((ss.hasOccluders && (!isCube || !isMinSize))))
        {
            int npotAxises[3] = { -1, -1, -1 };
            int n = 0;

            if (!isPowerOfTwo(size.i))
                npotAxises[n++] = 0;
            if (!isPowerOfTwo(size.j))
                npotAxises[n++] = 1;
            if (!isPowerOfTwo(size.k))
                npotAxises[n++] = 2;

            if (n == 1)
                axis = npotAxises[0];
            else if (n == 2)
                axis = size[npotAxises[0]] < size[npotAxises[1]] ? npotAxises[1] : npotAxises[0];
            else if (n == 3)
            {
                axis = size[npotAxises[0]] < size[npotAxises[1]] ?
                  (size[npotAxises[1]] < size[npotAxises[2]] ? npotAxises[2] : npotAxises[1]) :
                  (size[npotAxises[0]] < size[npotAxises[2]] ? npotAxises[2] : npotAxises[0]);
            }
        }

        // Split and return
        int mid = (iNodeMin[axis] + iNodeMax[axis]) / 2;

        while ((mid - iNodeMin[axis]) % m_unitsPerTile != 0)
            mid--;

        while (!isPowerOfTwo(mid - iNodeMin[axis]))
            mid++;

        Vector3i iMin = iNodeMin;
        Vector3i iMax = iNodeMax;

        iMax[axis] = mid;
        Builder::Error err = calcGridNodesRec(ss, iMin, iMax);
        if (err != Builder::SUCCESS)
            return err;

        iMin[axis] = mid;
        iMax[axis] = iNodeMax[axis];
        err = calcGridNodesRec(ss, iMin, iMax);

        return err;
    }

    // Leaf

    // Dimensions must be powers of two
    UMBRA_ASSERT(isPowerOfTwo(size.i) && isPowerOfTwo(size.j) && isPowerOfTwo(size.k));
    // Non-empty leaves must be cubes

    UMBRA_ASSERT(!ss.hasOccluders || isCube);

    Node node;
    node.iMin = iNodeMin;
    node.iMax = iNodeMax;
    node.targetAABB = inflatedAABB.get();
    node.occluderAABB = backfaceAABB.get();
    node.cgp.aabb = nodeAABB.get();
    node.cgp.visualizations = m_compVisualizations;
    node.cgp.strictViewVolumes = m_strictViewVolumes;
    node.cgp.accurateDilation = m_compAccurateDilation;

    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, bfDistance);
    SIMDStoreAligned(m_bfDistance, &bfDistance.x);

    node.cgp.bfDistance = bfDistance.x;
    node.cgp.featureSize = m_featureSize * ( 1 << splitBias );

    // per-volume overrides

    node.cgp.cellLevel = m_cellSplits;
    node.cgp.smallestHoleLevel = m_smallestHoleSplits;
    node.cgp.bfLimit = m_bfLimit;

    for (int i = 0; i < ss.numViewVolumes; i++)
    {
        const Volume* vol = ss.viewVolumes[i];
        // check if there are per-volume overrides

        UMBRA_ASSERT(node.cgp.cellLevel >= 0);
        if (vol->cellLevel > node.cgp.cellLevel)
            node.cgp.cellLevel = vol->cellLevel;

        UMBRA_ASSERT(node.cgp.smallestHoleLevel >= 0);
        if (vol->smallestHoleLevel > node.cgp.smallestHoleLevel)
            node.cgp.smallestHoleLevel = vol->smallestHoleLevel;

        UMBRA_ASSERT(node.cgp.featureSize >= 0.f);
        if (vol->featureSize > 0.f && vol->featureSize < node.cgp.featureSize)
            node.cgp.featureSize = vol->featureSize;

        UMBRA_ASSERT(node.cgp.bfLimit > 0.f);
        if (vol->backfaceLimit > node.cgp.bfLimit)
            node.cgp.bfLimit = vol->backfaceLimit;

            // \todo rest of parameters
    }

    node.cgp.smallestHoleLevel = max2(node.cgp.smallestHoleLevel, node.cgp.cellLevel);

    // If tile is smaller than it should, adjust levels accordingly.

    node.cgp.cellLevel = adjustLevel(node.cgp.cellLevel, size.i, m_unitsPerTile);
    node.cgp.smallestHoleLevel = adjustLevel(node.cgp.smallestHoleLevel, size.i, m_unitsPerTile);

    // Fill objects
    Set<int> intersectingObjects(m_platform.allocator);
    for (int i = 0; i < ss.numObjs; i++)
        intersectingObjects.insert(ss.objs[i]->objSceneIdx);
    for (int i = 0; i < ss.triangles.getSize(); i++)
        intersectingObjects.insert(ss.triangles[i].objSceneIdx);
    for (int i = 0; i < ss.numVolObjs; i++)
        intersectingObjects.insert(ss.volObjs[i]->objSceneIdx);

    node.intersectingObjects.setAllocator(m_platform.allocator);
    intersectingObjects.getArray(node.intersectingObjects);

    node.intersectingVolumes.setAllocator(m_platform.allocator);
    node.intersectingVolumes.resize(ss.numViewVolumes);
    for (int i = 0; i < ss.numViewVolumes; i++)
        node.intersectingVolumes[i] = ss.viewVolumes[i]->sceneVolIdx;

    int numObjects = node.intersectingObjects.getSize();
    if (numObjects)
        quickSort(node.intersectingObjects.getPtr(), 0, numObjects); // \todo sort by name
    if (ss.numViewVolumes)
        quickSort(node.intersectingVolumes.getPtr(), 0, ss.numViewVolumes); // \todo sort by name

    m_nodes.pushBack(node);
    return Builder::SUCCESS;
}

int TileGrid::calcSplitBias (const SIMDAABB& aabb)
{
    int numVols = m_numViewVolumes;
    UMBRA_ASSERT(numVols);

    int bias = 0;

    float distance = m_viewVolumes[0].aabb.getDistance(aabb);
    for (int v = 1; v < numVols; v++)
        distance = min2(m_viewVolumes[v].aabb.getDistance(aabb), distance);

    // bias starting from 4*smallestOccluder distance from nearest view volume,
    // increases once distance doubles
    float dTile = distance / (4.f * m_smallestOccluder) - 1.f;
    if (dTile >= 1.f)
        bias = int(::log(dTile) / ::log(2.f));  // log2

    if (distance > 0)
        bias += 1;

    UMBRA_ASSERT(0 <= bias && bias <= 31);
    return bias;
}

void TileGrid::fillVolumes (Array<ViewVolume>& volumes, int nodeIdx) const
{
    UMBRA_ASSERT(m_viewVolumes.getSize() == m_scene->getViewVolumeCount());
    const Node& node = m_nodes[nodeIdx];

    if (m_viewVolumes.getSize() == 0)
    {
        // Set implicit default view volume to all nodes
        ViewVolume dstVolume;
        dstVolume.id              = 0;
        dstVolume.cellSplits      = node.cgp.cellLevel;
        dstVolume.aabb            = node.targetAABB;
        dstVolume.backfaceLimit   = node.cgp.bfLimit;
        dstVolume.isClusterMarker = false;
        volumes.pushBack(dstVolume);
        return;
    }

    int numVols = node.intersectingVolumes.getSize();
    for (int i = 0; i < numVols; i++)
    {
        int volIdx = node.intersectingVolumes[i];
        const ImpVolume* volume = ImpScene::getImplementation(const_cast<SceneVolume*>(m_scene->getViewVolume(volIdx)));
        AABB volAABB = volume->getAABB();
        const AABB& tileAABB = node.targetAABB; // \todo or real (non-inflated) tile bounds?

        volAABB.clamp(tileAABB);    // intersect with tile

        UMBRA_ASSERT(volume->getID() == m_viewVolumes[volIdx].name);

        float smallestOccluder = m_viewVolumes[volIdx].smallestOccluder;
        if (smallestOccluder <= 0.f)
            smallestOccluder = m_smallestOccluder;

        float volBfLimit = m_viewVolumes[volIdx].backfaceLimit;
        if (volBfLimit < 0.f)
            volBfLimit = m_bfLimit;

        // Calculate cell splits (log2(tileSize/smallestOccluder))
        Vector3i splits;
        for (int i = 0; i < 3; i++)
            splits[i] = max2(Math::intChop(::log(tileAABB.getDimensions()[i] / smallestOccluder) / ::log(2.f) + 0.5f), 0);

        // Append to tile's volumes
        ViewVolume dstVolume;
        dstVolume.id              = volume->getID();
        dstVolume.cellSplits      = splits[0]; // any
        dstVolume.aabb            = volAABB;
        dstVolume.backfaceLimit   = volBfLimit;
        dstVolume.isClusterMarker = false;

        // \todo [Hannu] hack: zero-volume volumes are cluster hacks
        if (dstVolume.aabb.getVolume() == 0.f)
            dstVolume.isClusterMarker = true;

        UMBRA_ASSERT(dstVolume.aabb.isOK());

        volumes.pushBack(dstVolume);
    }
}

void TileGrid::fillBlock (GeometryBlock& block, int blockIdx) const
{
    const Node& node = m_nodes[blockIdx];
    block.setTargetAABB(node.targetAABB);
    block.setOccluderAABB(node.occluderAABB);

    fillVolumes(block.getViewVolumes(), blockIdx);
    /* \todo [antti 27.6.2012]: should sort based on user id instead,
       why should scene insertion order matter? */

    int numObjs = node.intersectingObjects.getSize();


    for (int o = 0; o < numObjs; o++)
    {
        int idx = node.intersectingObjects[o];
        block.importObject(m_scene->getObject(idx), m_transformedVertices[idx]);
    }
}

#endif // UMBRA_EXCLUDE_COMPUTATION
