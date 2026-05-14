/*!
 *
 * Umbra PVS
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

#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraPrivateDefs.hpp"
#include "umbraImpScene.hpp"
#include "umbraFileStream.hpp"
#include "umbraSerializer.hpp"
#include "umbraFloat.hpp"
#include "umbraMath.hpp"
#include "umbraHttp.hpp"
#include "umbraJson.hpp"

#include "optimizer/umbraScene.hpp"

using namespace Umbra;

/* Lowest supported scene version */
#define SCENE_MIN_VERSION 1

static const unsigned int SCENE_VERSION                     = 15;
static const unsigned int SCENE_MAGIC_NUMBER                = 0xbeefcafe;
static const unsigned int SCENE_CHUNK_MODEL                 = 0x0;
static const unsigned int SCENE_CHUNK_OBJECT                = 0x1;
static const unsigned int SCENE_CHUNK_TEXTURE_NAMES         = 0x2;
static const unsigned int SCENE_CHUNK_MATERIAL_MEMORY_COSTS = 0x3;
static const unsigned int SCENE_CHUNK_VIEW_VOLUME           = 0x4;
static const unsigned int SCENE_CHUNK_TARGET_VOLUME         = 0x5;
static const unsigned int SCENE_CHUNK_PARAMS                = 0x6;
static const unsigned int SCENE_CHUNK_NAME                  = 0x7;

// Vertex field flags
// Note that adding a new field doesn't require increasing version number.
// Note that only VERTEX_POSITION is currently in use!
static const unsigned int VERTEX_POSITION                   = (1<<0);

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

bool ImpModel::init(
    const float*        vertices,
    const void*         triangles,
    int                 vCount,
    int                 tCount,
    int                 vertexStride,
    ImpScene::IndexType indexType,
    float               estimatedCost)
{
    m_index = -1;
    m_estimatedCost = estimatedCost;
    m_vertices.clear();
    m_triangles.clear();

    if (!vertices || !triangles || vCount <= 0 || tCount <= 0)
        return false;

    if (!vertexStride)
        vertexStride = 3 * sizeof(float); // tight

    Hash<Vector3, int> map;
    const UINT8* triData = (const UINT8*)triangles;
    const UINT8* vertData = (const UINT8*)vertices;

    for (int i = 0; i < tCount; ++i)
    {
        Vector3i tri;
        if (indexType == ImpScene::IT_UINT16)
        {
            const UINT16* p = (const UINT16*)triData;
            tri = Vector3i(p[0], p[1], p[2]);
            triData += (3 * sizeof(UINT16));
        }
        else
        {
            const UINT32* p = (const UINT32*)triData;
            tri = Vector3i(p[0], p[1], p[2]);
            triData += (3 * sizeof(UINT32));
        }

        for (int j = 0; j < 3; ++j)
        {
            int origIdx = tri[j];
            // check that vertex index is valid
            if ((origIdx < 0) || (origIdx >= vCount))
                return false;
            Vector3 vtx;
            memcpy(&vtx[0], vertData + origIdx * vertexStride, 3 * sizeof(float));
            int idx = map.getDefault(vtx, m_vertices.getSize());
            if (idx == m_vertices.getSize())
            {
                // check that coordinates are ok
                if (!Float::isFinite(vtx.x) ||
                    !Float::isFinite(vtx.y) ||
                    !Float::isFinite(vtx.z))
                    return false;
                m_vertices.pushBack(vtx);
            }
            tri[j] = idx;
        }

        m_triangles.pushBack(tri);
    }
    return true;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpModel::~ImpModel (void)
{
    // nada
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

template<typename OP> void ImpModel::streamOp (OP& op)
{
    if (!op.minVersion(12))
    {
        int id;
        stream(op, id); // not used
    }

    int vertexCount = m_vertices.getSize();
    stream(op, vertexCount);
    if (OP::IsWrite)
        m_vertices.reset(vertexCount);

    int vertexFlags = VERTEX_POSITION;
    int elementSize = sizeof(Vector3);

    if (op.minVersion(8))
    {
        stream(op, vertexFlags);
        stream(op, elementSize);
    }
    else
    {
        elementSize += sizeof(Vector2) + sizeof(UINT32);
    }

    UMBRA_ASSERT(vertexFlags & VERTEX_POSITION);
    elementSize -= sizeof(Vector3);

    if (!elementSize)
    {
        stream(op, m_vertices, vertexCount);
    }
    else
    {
        for (int i = 0; i < vertexCount; i++)
        {
            stream(op, m_vertices[i]);
            op.skip(elementSize);
        }
    }

    if (op.minVersion(12))
    {
        stream(op, m_triangles);
    }
    else
    {
        int triangleCount = m_triangles.getSize();
        stream(op, triangleCount);
        if (OP::IsWrite)
            m_triangles.reset(triangleCount);
        for (int i = 0; i < triangleCount; i++)
        {
            stream(op, m_triangles[i]);
            if (!op.minVersion(12))
                op.skip(4);
        }
    }
    if (op.minVersion(15))
    {
        stream(op, m_estimatedCost);
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpObject::ImpObject(
    const SceneModel*   model,
    const Matrix4x4&    mtx,
    unsigned int        id,
    unsigned int        flags,
    const Vector2*      distanceLimits,
    const AABB*         distanceBound,
    TriangleWinding     winding)
:   m_matrix            (mtx.get4x3Matrix()),
    m_flags             (flags),
    m_objectID          (id),
    m_model             (model),
    m_sliceID           (0),
    m_winding           (winding),
    m_distanceLimits    (Vector2(0, -1.f)),
    m_distanceBound     ()
{
    computeAABB();
    if (distanceLimits)
        m_distanceLimits = *distanceLimits;
    if (distanceBound)
        m_distanceBound = *distanceBound;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpObject::~ImpObject (void)
{
    // nada
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpObject::streamOp (Serializer& op)
{
    Hash<const SceneModel*, int>* modelHash = (Hash<const SceneModel*, int>*)op.getOpaque();
    UMBRA_ASSERT(modelHash->contains(m_model));
    int modelIndex = *modelHash->get(m_model);

    stream(op, m_objectID);
    stream(op, modelIndex);
    stream(op, m_matrix);

    unsigned int outFlags = 0;
    if (m_flags & SceneObject::OCCLUDER)
        outFlags |= (1<<0);
    if (m_flags & SceneObject::TARGET)
        outFlags |= (1<<2);
    if (m_flags & SceneObject::GATE)
        outFlags |= (1<<4);
    if (m_flags & SceneObject::VOLUME)
        outFlags |= (1<<5);

    stream(op, outFlags);
    stream(op, m_sliceID);
    UINT32* w = (UINT32*)&m_winding;
    stream(op, *w);

    stream(op, m_distanceLimits);
    stream(op, m_distanceBound);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpObject::streamOp (Deserializer& op)
{
    Array<const SceneModel*>* modelArray = (Array<const SceneModel*>*)op.getOpaque();

    stream(op, m_objectID);
    int modelIndex = 0;
    stream(op, modelIndex);
    m_model = modelArray->operator[](modelIndex);
    stream(op, m_matrix);
    UINT32 inFlags = 0;
    stream(op, inFlags);
    m_flags = 0;

    if (inFlags & (1 << 0))
        m_flags |= SceneObject::OCCLUDER;
    if (inFlags & (1 << 2))
        m_flags |= SceneObject::TARGET;
    if (inFlags & (1 << 4))
        m_flags |= SceneObject::GATE;
    if (inFlags & (1 << 5))
        m_flags |= SceneObject::VOLUME;

    if (!op.minVersion(12) && op.minVersion(2))
    {
        // material ID, object cost
        op.skip(sizeof(UINT32) + sizeof(float));
    }

    if (op.minVersion(9))
        stream(op, m_sliceID);

    if (op.minVersion(11))
    {
        UINT32* w = (UINT32*)&m_winding;
        stream(op, *w);
    }

    computeAABB();

    if (op.minVersion(14))
    {
        stream(op, m_distanceLimits);
        stream(op, m_distanceBound);
    }
    else
    {
        m_distanceLimits = Vector2(0.f, -1.f);
        m_distanceBound = AABB();
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpObject::computeAABB (void)
{
    Matrix4x3 mat = getMatrix4x3();
    const SceneModel* m = getModel();

    int vertexCount = m->getVertexCount();

    if (!vertexCount)
        return;

    Vector3 v = mat.transform(m->getVertices()[0]);
    m_aabb.set(v, v);

    for (int i = 1; i < m->getVertexCount(); i++)
    {
        v = mat.transform(m->getVertices()[i]);
        m_aabb.grow(v);
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

const Matrix4x4 ImpObject::getMatrix(void) const
{
    Matrix4x4 mtx = m_matrix;
    return mtx;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpObject::setMatrix(const Matrix4x4& m)
{
    Matrix4x4 mtx = m;
    m_matrix = mtx.get4x3Matrix();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpObject::setSliceID(unsigned int id)
{
    m_sliceID = id;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpVolume::ImpVolume(
    const AABB&         aabb,
    float               smallestOccluder,
    const Vector3&      scaling,
    float               backfaceLimit,
    unsigned int        id)
:   m_aabb              (aabb),
    m_smallestOccluder  (smallestOccluder),
    m_scaling           (scaling),
    m_backfaceLimit     (backfaceLimit),
    m_volumeID          (id)
{
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpVolume::ImpVolume(void): m_smallestOccluder(0.f), m_scaling(Vector3(1.f, 1.f, 1.f)), m_backfaceLimit(-1.f), m_volumeID(0xffffffff) {}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpVolume::~ImpVolume (void)
{
    // nada
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

template <typename OP> void ImpVolume::streamOp (OP& op)
{
    stream(op, m_aabb);
    if (op.minVersion(4))
    {
        stream(op, m_smallestOccluder);
    }
    if (op.minVersion(5))
    {
        stream(op, m_scaling);
        stream(op, m_backfaceLimit);
    }
    if (op.minVersion(13))
    {
        stream(op, m_volumeID);
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpScene::ImpScene (void)
: m_refCount(1),
  m_params(ImpScene::PARAM_LAST)
{
    memset(m_params.getPtr(), 0, ImpScene::PARAM_LAST*sizeof(float));
}


/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ImpScene::~ImpScene (void)
{
    for (int i = 0; i < m_objects.getSize(); i++)
        UMBRA_HEAP_DELETE2(getAllocator(), SceneObject, m_objects[i]);
    for (int i = 0; i < m_models.getSize(); i++)
        UMBRA_HEAP_DELETE2(getAllocator(), SceneModel, m_models[i]);
    for (int i = 0; i < m_viewVolumes.getSize(); i++)
        UMBRA_HEAP_DELETE2(getAllocator(), SceneVolume, m_viewVolumes[i]);
}

/*----------------------------------------------------------------------*//*!
 * \brief   Exports the scene in .scene format
 * \param   s Reference to output stream
 * \return  boolean value indicating success
 *//*----------------------------------------------------------------------*/

bool ImpScene::exportScene (OutputStream& out)
{
    Serializer op(&out);

    stream(op, SCENE_MAGIC_NUMBER);
    stream(op, SCENE_VERSION);

    stream(op, SCENE_CHUNK_PARAMS);
    stream(op, m_params);

    Hash<const SceneModel*, int> modelHash;

    for (int i = 0; i < getModelCount(); i++)
    {
        const SceneModel* m = getModel(i);
        stream(op, SCENE_CHUNK_MODEL);
        stream(op, *m->m_imp);
        modelHash.insert(m, i);
    }

    op.setOpaque(&modelHash);

    for (int i = 0; i < getObjectCount(); i++)
    {
        const SceneObject* o = getObject(i);
        stream(op, SCENE_CHUNK_OBJECT);
        stream(op, *o->m_imp);
    }

    stream(op, SCENE_CHUNK_VIEW_VOLUME);
    stream(op, getViewVolumeCount());
    for (int i = 0; i < getViewVolumeCount(); i++)
    {
        const SceneVolume* v = getViewVolume(i);
        stream(op, *v->m_imp);
    }

    return op.isOk();
}

/*----------------------------------------------------------------------*//*!
 * \brief   Scene import
 *//*----------------------------------------------------------------------*/

bool ImpScene::importScene (InputStream* inStream)
{
    Deserializer op(inStream);
    UINT32 magic = 0;
    stream(op, magic);
    if (magic != SCENE_MAGIC_NUMBER)
        return false;
    UINT32 version = SCENE_VERSION;
    stream(op, version);
    if (version < SCENE_MIN_VERSION || version > SCENE_VERSION)
        return false;

    op.setVersion(version);
    op.setOpaque(&m_models);

    bool success = false;
    while (op.isOk())
    {
        int chunkId;
        stream(op, chunkId);

        // eof, load success
        if (!op.isOk())
        {
            success = true;
            break;
        }

        switch (chunkId)
        {
        case SCENE_CHUNK_TEXTURE_NAMES:
            {
                int total;
                stream(op, total);
                op.skip(total);
            }
            break;
        case SCENE_CHUNK_MODEL:
            {
                ImpModel* m = UMBRA_NEW(ImpModel);
                stream(op, *m);
                m->setIndex(m_models.getSize());
                m_models.pushBack(UMBRA_NEW(SceneModel, m));
                break;
            }
        case SCENE_CHUNK_OBJECT:
            {
                ImpObject* o = UMBRA_NEW(ImpObject);
                stream(op, *o);
                if (o->getFlags())
                    m_objects.pushBack(UMBRA_NEW(SceneObject, o));
                else
                    // these may happen for legacy scene files (target and view bounds)
                    UMBRA_DELETE(o);
            }
            break;
        case SCENE_CHUNK_MATERIAL_MEMORY_COSTS:
            {
                int total;
                stream(op, total);
                op.skip(total * (sizeof(UINT32) + sizeof(float)));
            }
            break;
        case SCENE_CHUNK_VIEW_VOLUME:
            {
                int total;
                stream(op, total);
                while (total--)
                {
                    ImpVolume* v = UMBRA_NEW(ImpVolume);
                    stream(op, *v);
                    m_viewVolumes.pushBack(UMBRA_NEW(SceneVolume, v));
                }
            }
            break;
        case SCENE_CHUNK_TARGET_VOLUME:
            {
                int total;
                stream(op, total);
                while (total--)
                {
                    ImpVolume dummy;
                    stream(op, dummy);
                    UMBRA_UNREF(dummy);
                }
            }
            break;
        case SCENE_CHUNK_NAME:
            {
                int total;
                stream(op, total);
                op.skip(total);
            }
            break;
        case SCENE_CHUNK_PARAMS:
            stream(op, m_params);
            {
                int p = m_params.getSize();
                m_params.resize(ImpScene::PARAM_LAST);
                for (int i = p; i < m_params.getSize(); i++)
                    m_params[i] = 0.f;
            }
            break;
        default:
            break;
        }
    }

    return success;
}


/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

SceneModel* ImpScene::insertModel(
    const float*        vertices,
    const void*         triangles,
    int                 vertexCount,
    int                 triangleCount,
    int                 vertexStride,
    ImpScene::IndexType indexType,
    float               estimatedCost)
{
    ImpModel* m = UMBRA_NEW(ImpModel);
    if (!m->init(vertices, triangles, vertexCount, triangleCount,
        vertexStride, indexType, estimatedCost))
    {
        UMBRA_DELETE(m);
        return NULL;
    }
    m->setIndex(m_models.getSize());
    SceneModel* apiModel = UMBRA_NEW(SceneModel, m);
    m_models.pushBack(apiModel);
    return apiModel;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

SceneObject* ImpScene::insertObject(
    const SceneModel*       model,
    const Matrix4x4&        mtx,
    unsigned int            id,
    unsigned int            flags,
    const Vector2*          distanceLimits,
    const AABB*             distanceBounds,
    TriangleWinding         winding)
{
    UMBRA_ASSERT(mtx[3] == Vector4(0.f, 0.f, 0.f, 1.f));

    ImpObject* o = UMBRA_NEW(ImpObject, model, mtx, id, flags, distanceLimits, distanceBounds, winding);
    SceneObject* apiObject = UMBRA_NEW(SceneObject, o);
    m_objects.pushBack(apiObject);

    m_objectIDSet.insert(id);
    return apiObject;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

SceneVolume* ImpScene::insertViewVolume(
    unsigned int id,
    const Vector3& mn,
    const Vector3& mx,
    const Vector3& scaling,
    float backfaceLimit,
    float smallestOccluder)
{
    Vector3 mn2 = mn;
    Vector3 mx2 = mx;

    if (mx2.x < mn2.x) swap(mx2.x, mn2.x);
    if (mx2.y < mn2.y) swap(mx2.y, mn2.y);
    if (mx2.z < mn2.z) swap(mx2.z, mn2.z);

    ImpVolume* v = UMBRA_NEW(ImpVolume, AABB(mn2, mx2), smallestOccluder, scaling, backfaceLimit, id);
    SceneVolume* apiVolume = UMBRA_NEW(SceneVolume, v);
    m_viewVolumes.pushBack(apiVolume);
    return apiVolume;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpScene::deleteViewVolume (const SceneVolume* volume)
{
    for (int i = 0; i <m_viewVolumes.getSize(); i++)
    {
        if (m_viewVolumes[i] == volume)
        {
            deleteViewVolume(i);
            return;
        }
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpScene::deleteViewVolume (int index)
{
    if (index < 0 || index >= m_viewVolumes.getSize())
        return;

    UMBRA_HEAP_DELETE2(getAllocator(), SceneVolume, m_viewVolumes[index]);

    for (int i = index; i < m_viewVolumes.getSize() - 1; i++)
        m_viewVolumes[i] = m_viewVolumes[i+1];

    m_viewVolumes.resize(m_viewVolumes.getSize() - 1);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

AABB ImpScene::getAABB (void) const
{
    AABB aabb;

    for (int i = 0; i < getObjectCount(); i++)
    {
        if (!(getObject(i)->getFlags() & (SceneObject::TARGET | SceneObject::GATE | SceneObject::OCCLUDER)))
            continue;
        aabb.grow(getObject(i)->m_imp->getAABB());
    }

    for (int i = 0; i < getViewVolumeCount(); i++)
    {
        aabb.grow(getViewVolume(i)->m_imp->getAABB());
    }

    return aabb;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Vector3i ImpScene::getSplits(const AABB& aabb, float size)
{
    Vector3i d;
    for (int i = 0; i < 3; i++)
        d[i] = max2(Math::intChop(::log(aabb.getDimensions()[i] / size) / ::log(2.f) + 0.5f), 0);
    return d;
}


/*---------------------------------------------------------------*//*!
 *  Public API
 *//*---------------------------------------------------------------*/

SceneModel::SceneModel(class ImpModel* imp) : m_imp(imp) {}
SceneModel::~SceneModel(void) {UMBRA_DELETE(m_imp);}

int SceneModel::getVertexCount(void) const
{
    return m_imp->getVertexCount();
}

const Umbra::Vector3* SceneModel::getVertices(void) const
{
    return m_imp->getVertices();
}

int SceneModel::getTriangleCount(void) const
{
    return m_imp->getTriangleCount();
}

const Umbra::Vector3i* SceneModel::getTriangles (void) const
{
    return m_imp->getTriangles();
}

int SceneModel::getIndex (void) const
{
    return m_imp->getIndex();
}

float SceneModel::getEstimatedCost (void) const
{
    return m_imp->getEstimatedCost();
}

SceneObject::SceneObject(class ImpObject* imp) : m_imp(imp) {}
SceneObject::~SceneObject() {UMBRA_DELETE(m_imp);}

void SceneObject::getMatrix (Umbra::Matrix4x4& mtx, MatrixFormat mf) const
{
    mtx = m_imp->getMatrix();
    if (mf != MF_ROW_MAJOR)
        mtx.transpose();
}

const SceneModel* SceneObject::getModel() const
{
    return m_imp->getModel();
}

unsigned int SceneObject::getFlags() const
{
    return m_imp->getFlags();
}

unsigned int SceneObject::getID() const
{
    return m_imp->getID();
}

TriangleWinding SceneObject::getTriangleWinding(void) const
{
    return m_imp->getTriangleWinding();
}

Vector2 SceneObject::getDrawDistance(void) const
{
    return m_imp->getDrawDistance();
}

void SceneObject::getDistanceBound(Vector3& mn, Vector3& mx) const
{
    return m_imp->getDistanceBound(mn, mx);
}

void SceneObject::getBounds(Vector3& mn, Vector3& mx) const
{
    return m_imp->getAABB(mn, mx);
}

const Umbra::Vector3& SceneVolume::getMin () const
{
    return m_imp->getMin();
}

const Umbra::Vector3& SceneVolume::getMax () const
{
    return m_imp->getMax();
}

unsigned int SceneVolume::getID (void) const
{
    return m_imp->getID();
}

SceneVolume::SceneVolume(class ImpVolume* imp) : m_imp(imp) {}
SceneVolume::~SceneVolume() {UMBRA_DELETE(m_imp);}

Scene::Scene() : m_imp(UMBRA_NEW(ImpScene)) {}
Scene::Scene(class ImpScene* imp) : m_imp(imp) {}
Scene::~Scene(void) { UMBRA_HEAP_DELETE2(getAllocator(), ImpScene, m_imp); }

Scene* Scene::create(const char* fileName)
{
    if (fileName)
    {
        FileInputStream inStream(fileName);
        if (!inStream.isOpen())
            return NULL;
        return create(inStream);
    }

    ImpScene* imp = UMBRA_NEW(ImpScene);
    if (!imp)
        return NULL;

    return UMBRA_NEW(Scene, imp);
}

Scene* Scene::create (InputStream& in)
{
    ImpScene* imp = UMBRA_NEW(ImpScene);
    if (!imp || !imp->importScene(&in))
        return NULL;

    return UMBRA_NEW(Scene, imp);
}

void Scene::release(void)
{
    if (this && m_imp && m_imp->unref() == 0)
        UMBRA_HEAP_DELETE2(getAllocator(), Scene, this);
}


const SceneModel* Scene::insertModel (const float* vertices, const Umbra::UINT16* indices, int vertexCount, int triangleCount, int vertexStride, float estimatedCost)
{
    return m_imp->insertModel(vertices, indices, vertexCount, triangleCount, vertexStride, ImpScene::IT_UINT16, estimatedCost);
}

const SceneModel* Scene::insertModel (const float* vertices, const Umbra::UINT32* indices, int vertexCount, int triangleCount, int vertexStride, float estimatedCost)
{
    return m_imp->insertModel(vertices, indices, vertexCount, triangleCount, vertexStride, ImpScene::IT_UINT32, estimatedCost);
}

const SceneObject* Scene::insertObject(
    const SceneModel* model, 
    const Matrix4x4& mtx_, 
    unsigned int id, 
    unsigned int flags, 
    MatrixFormat mf, 
    TriangleWinding winding,
    const Vector2* drawDistance,
    const Vector3* distBoundMn,
    const Vector3* distBoundMx)
{
    if (!model)
        return 0;

    if (flags & (SceneObject::TARGET | SceneObject::GATE))
        if (m_imp->containsObject(id))
            return 0;

    Matrix4x4 mtx = mtx_;
    if (mf == MF_COLUMN_MAJOR)
        mtx.transpose();

    float det = mtx.det();
    if (det == 0.f || !Float::isFinite(det))
        return 0;

    if (flags & ~(SceneObject::OCCLUDER | SceneObject::TARGET | SceneObject::GATE | SceneObject::VOLUME))
        return 0;

    // User portals cannot be occluders.
    if (flags & SceneObject::GATE)
        flags &= ~SceneObject::OCCLUDER;

    if ((distBoundMn == NULL) != (distBoundMx == NULL))
        return 0;

    AABB distanceBound;
    if (distBoundMn && distBoundMx)
        distanceBound = AABB(*distBoundMn, *distBoundMx);

    return m_imp->insertObject(model, mtx, id, flags, drawDistance,
        (distBoundMn && distBoundMx) ? &distanceBound : NULL, winding);
}

const SceneVolume* Scene::insertViewVolume(const Vector3& mn, const Vector3& mx, unsigned int id)
{
    return m_imp->insertViewVolume(
        id,
        mn,
        mx,
        Vector3(1.f, 1.f, 1.f),
        -1.f, -1.f);
}


void Scene::insertSeedPoint(const Vector3& p)
{
    m_imp->insertViewVolume(0xffffffff, p, p, Vector3(1.f, 1.f, 1.f), -1.f, -1.f);
}

void Scene::getBounds(Vector3& mn, Vector3& mx) const
{
    AABB aabb = m_imp->getAABB();
    mn = aabb.getMin();
    mx = aabb.getMax();
}

const SceneModel* Scene::getModel (int index) const
{
    if (index < 0 || index >= getModelCount())
        return 0;
    return m_imp->getModel(index);
}

int Scene::getModelCount (void) const
{
    return m_imp->getModelCount();
}

const SceneObject* Scene::getObject (int index) const
{
    if (index < 0 || index >= getObjectCount())
        return 0;
    return m_imp->getObject(index);
}

int Scene::getObjectCount (void) const
{
    return m_imp->getObjectCount();
}

int Scene::getViewVolumeCount (void) const
{
    return m_imp->getViewVolumeCount();
}

const SceneVolume* Scene::getViewVolume (int i) const
{
    if (i < 0 || i >= getViewVolumeCount())
        return 0;
    return m_imp->getViewVolume(i);
}

bool Scene::writeToFile(const char* fileName)
{
    UMBRA_ASSERT(fileName);
    FileOutputStream stream(fileName);
    return m_imp->exportScene(stream);
}

bool Scene::serialize (OutputStream& out) const
{
    return m_imp->exportScene(out);
}

#endif
