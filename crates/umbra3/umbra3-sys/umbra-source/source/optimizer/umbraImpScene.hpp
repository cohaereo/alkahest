#pragma once

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2007-2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Simple scene representation
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraArray.hpp"
#include "umbraMatrix.hpp"
#include "umbraString.hpp"
#include "umbraHash.hpp"
#include "umbraAABB.hpp"
#include "umbraSerializer.hpp"
#include "umbraSet.hpp"

#include "optimizer/umbraScene.hpp"

namespace Umbra
{

/*----------------------------------------------------------------------*//*!
 * \brief   Container for a 3D scene
 *//*----------------------------------------------------------------------*/

class ImpScene
{
public:

    // Enumeration of parameters as serialized in file

    enum Param
    {
        PARAM_SAFE_OCCLUDER_DISTANCE = 0,
        PARAM_PVS_CULL_DISTANCE,
        PARAM_DEFAULT_SMALLEST_OCCLUDER,
        PARAM_DEFAULT_TILE_SIZE,
        PARAM_DEFAULT_BACKFACE_LIMIT,
        PARAM_DEFAULT_CELL_SCALING_X,
        PARAM_DEFAULT_CELL_SCALING_Y,
        PARAM_DEFAULT_CELL_SCALING_Z,
        PARAM_SMALLEST_HOLE,
        PARAM_USER_GRID_MIN_X,
        PARAM_USER_GRID_MIN_Y,
        PARAM_USER_GRID_MIN_Z,
        PARAM_USER_GRID_MAX_X,
        PARAM_USER_GRID_MAX_Y,
        PARAM_USER_GRID_MAX_Z,
        PARAM_USER_GRID_SIZE_X,
        PARAM_USER_GRID_SIZE_Y,
        PARAM_USER_GRID_SIZE_Z,
        PARAM_TILE_SIZE_X,
        PARAM_TILE_SIZE_Y,
        PARAM_TILE_SIZE_Z,
        PARAM_LAST
    };

    enum IndexType
    {
        IT_UINT16,
        IT_UINT32
    };


                                    ImpScene                (void);
                                    ~ImpScene               (void);

    SceneModel*                     insertModel             (const float* vertices, const void* triangleVertices, int vertexCount, int triangleCount,
                                                             int vertexStride, IndexType indexType, float estimatedCost);
    SceneObject*                    insertObject            (const SceneModel* model, const Matrix4x4& mtx, unsigned int id, unsigned int flags, 
                                                             const Vector2* distanceLimits, const AABB* distanceBound, TriangleWinding winding);

    SceneVolume*                    insertViewVolume        (unsigned int id, const Vector3& mn, const Vector3& mx, const Vector3& scaling, float backfaceLimit, float smallestOccluder = 0.f);

    void                            clearViewVolumes        (void)                                          {m_viewVolumes.reset(0);}

    SceneModel*                     getModel                (int index) const                               { return m_models[index]; }
    int                             getModelCount           (void) const                                    { return m_models.getSize(); }
    const SceneObject*              getObject               (int index) const                               { return m_objects[index]; }
    int                             getObjectCount          (void) const                                    { return m_objects.getSize(); }

    int                             getViewVolumeCount      (void) const                                    { return m_viewVolumes.getSize(); }

    const SceneVolume*              getViewVolume           (int i) const                                   { return m_viewVolumes[i]; }

    void                            deleteViewVolume        (int i);
    void                            deleteViewVolume        (const SceneVolume*);

    const Array<SceneVolume*>&      getViewVolumes          (void) const                                    { return m_viewVolumes; }

    AABB                            getAABB                 (void) const;

    bool                            exportScene             (OutputStream& s);
    bool                            importScene             (InputStream* s);

    void                            ref                     (void)                                          { m_refCount++; }
    int                             unref                   (void)                                          { m_refCount--; return m_refCount; }

    bool                            containsObject          (Umbra::UINT32 id)                                 { return m_objectIDSet.contains(id); }

    static void                     ref                     (Scene* scene)                                  { scene->m_imp->ref(); }
    static ImpScene*                getImplementation       (Scene* scene)                                  { return scene->m_imp; }
    static const ImpScene*          getImplementation       (const Scene* scene)                            { return scene->m_imp; }
    static ImpObject*               getImplementation       (SceneObject* object)                           { return object->m_imp; }
    static const ImpObject*         getImplementation       (const SceneObject* object)                     { return object->m_imp; }
    static ImpModel*                getImplementation       (SceneModel* model)                             { return model->m_imp; }
    static const ImpModel*          getImplementation       (const SceneModel* model)                       { return model->m_imp; }
    static ImpVolume*               getImplementation       (SceneVolume* volume)                           { return volume->m_imp; }
    static const ImpVolume*         getImplementation       (const SceneVolume* volume)                     { return volume->m_imp; }

    static Vector3i                 getSplits               (const AABB& aabb, float size);

private:
                                    ImpScene                (const ImpScene&);
    ImpScene&                       operator=               (const ImpScene&);

    Array<SceneModel*>              m_models;               //!< Models contained in the scene
    Array<SceneObject*>             m_objects;              //!< Objects containted in the scene
    Array<SceneVolume*>             m_viewVolumes;          //!< List of scene volumes
    int                             m_refCount;             //!< Scene's create/release reference count
    Array<float>                    m_params;               //!< Array of float params
    Set<Umbra::UINT32>              m_objectIDSet;          //!< Set of object IDs for fast access
};

class ImpVolume
{
public:

    /* SceneVolume interface */
    const Vector3&          getMin               (void) const { return (const Vector3&)m_aabb.getMin(); }
    const Vector3&          getMax               (void) const { return (const Vector3&)m_aabb.getMax(); }
    const Vector3&          getScaling           (void) const { return (const Vector3&)m_scaling; }
    float                   getBackfaceLimit     (void) const { return m_backfaceLimit; }
    float                   getSmallestOccluder  (void) const { return m_smallestOccluder; }

    /* Umbra internal */
                            ImpVolume           (void);
                            ImpVolume           (const AABB& aabb,
                                                 float smallestOccluder,
                                                 const Vector3& scaling,
                                                 float backfaceLimit,
                                                 unsigned int id);
                            ~ImpVolume          (void);
    const AABB&             getAABB             (void) const { return m_aabb; }
    unsigned int            getID               (void) const { return m_volumeID; }


    template <typename OP>  void                streamOp (OP& op);

private:
                            ImpVolume           (const ImpVolume&);
    ImpVolume&              operator=           (const ImpVolume&);

    AABB                    m_aabb;
    float                   m_smallestOccluder;
    Vector3                 m_scaling;
    float                   m_backfaceLimit;
    unsigned int            m_volumeID;
};

/*-------------------------------------------------------------------*//*!
    * \brief Storage for model within a scene
    *//*-------------------------------------------------------------------*/

class ImpModel
{
public:

    /* SceneModel interface */
    int                         getVertexCount      (void) const { return m_vertices.getSize(); }
    int                         getTriangleCount    (void) const { return m_triangles.getSize(); }
    const Vector3*              getVertices         (void) const { return m_vertices.getPtr(); }
    const Vector3i*             getTriangles        (void) const { return m_triangles.getPtr(); }
    int                         getIndex            (void) const { return m_index; }
    float                       getEstimatedCost    (void) const { return m_estimatedCost; }
    float                       getCost             (void) const { return getEstimatedCost() > 0.0f ? getEstimatedCost() : (float)(getVertexCount() + 1); }

    /* Umbra internal */
                                ImpModel            (void): m_vertices(), m_triangles(), m_index(-1), m_estimatedCost(0.0f) {}
                                ~ImpModel           (void);

    template<typename OP> void  streamOp            (OP& op);

    bool init (const float*, const void*, int vertexCount, int triangleCount,
               int vertexStride, ImpScene::IndexType indexType, float estimatedCost = 0.0f);

    void setIndex(int idx) { UMBRA_ASSERT(m_index == -1); m_index = idx; }

private:
                            ImpModel                (const ImpModel&);
    ImpModel&               operator=               (const ImpModel&);

    Array<Vector3>                      m_vertices;         //!< Vertex positions
    Array<Vector3i>                     m_triangles;        //!< Triangle vertex indices
    int                                 m_index;
    float                               m_estimatedCost;
};

/*-------------------------------------------------------------------*//*!
    * \brief   Object instantiated from SceneModel.
    *//*-------------------------------------------------------------------*/

class ImpObject
{
public:
    /* SceneObject interface */
    const Matrix4x4         getMatrix           (void) const;
    void                    setMatrix           (const Matrix4x4& m);
    const SceneModel*       getModel            (void) const { return m_model; }
    unsigned int            getFlags            (void) const { return m_flags; }
    void                    setFlags            (unsigned int f) { m_flags = f; }
    unsigned int            getID               (void) const { return m_objectID; }
    unsigned int            getSliceID          (void) const { return m_sliceID; }
    void                    setSliceID          (unsigned int s);
    TriangleWinding         getTriangleWinding  (void) const { return m_winding; }
    Vector2                 getDrawDistance     (void) const { return m_distanceLimits; }
    void                    getDistanceBound    (Vector3& mn, Vector3& mx) const { mn = m_distanceBound.getMin(); mx = m_distanceBound.getMax(); }

    /* Umbra internal */
                            ImpObject           (void): m_flags(0), m_sliceID(0), m_winding(WINDING_CCW) {}
                            ImpObject           (const SceneModel* model, const Matrix4x4& mtx, unsigned int id, unsigned int flags, 
                                                 const Vector2* distanceLimits, const AABB* distanceBound, TriangleWinding winding);
                            ~ImpObject          (void);
    void                    setMatrix           (const Matrix4x3& mtx) { m_matrix = mtx; }
    const Matrix4x3&        getMatrix4x3        (void) const { return m_matrix; }
    Vector3                 getVertex           (int i) const       { return getMatrix4x3().transform(((Vector3*)getModel()->getVertices())[i]); }

    void                    getAABB             (Umbra::Vector3& mn, Umbra::Vector3& mx) const { mn = m_aabb.getMin(); mx = m_aabb.getMax(); }
    const AABB&             getAABB             (void) const { return m_aabb; }

    void                    streamOp             (Serializer& op);
    void                    streamOp             (Deserializer& op);

private:
                            ImpObject           (const ImpObject&);
    ImpObject&              operator=           (const ImpObject&);
    void                    computeAABB         (void);

    Matrix4x3                       m_matrix;           //!< Transformation matrix
    unsigned int                    m_flags;            //!< Object flags
    unsigned int                    m_objectID;         //!< Object ID
    const SceneModel*               m_model;            //!< Pointer to model
    unsigned int                    m_sliceID;          //!< Slice ID.
    AABB                            m_aabb;
    TriangleWinding                 m_winding;
    Vector2                         m_distanceLimits;
    AABB                            m_distanceBound;
};

} // namespace Umbra
