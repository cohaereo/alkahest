// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRASCENE_HPP
#define UMBRASCENE_HPP

/*!
 * \file    umbraScene.hpp
 * \brief   Umbra scene representation
 */

#include "umbraDefs.hpp"
#include "umbraPlatform.hpp"

namespace Umbra
{

/*!
 * \brief   A read-only class that represents a reusable model.
 *
 * \sa      SceneObject
 * \sa      Scene::insertModel
 * \sa      Scene::insertObject
 * \sa      Scene::getModel
 * \sa      Scene::getModelCount
 * \sa      SceneObject::getModel
 */
class SceneModel
{
public:

    /*!
     * \brief   Gets model vertex count.
     *
     * \return  Number of vertices in the model.
     */
    UMBRADEC int                    getVertexCount      (void) const;

    /*!
     * \brief   Gets model vertices.
     *
     * \return  A pointer to an array of vertices.
     */
    UMBRADEC const Umbra::Vector3*  getVertices         (void) const;

    /*!
     * \brief   Gets model triangle count.
     *
     * \return  Number of triangles in the model.
     */
    UMBRADEC int                    getTriangleCount    (void) const;

    /*!
     * \brief   Gets model triangles.
     *
     * \return  A pointer to an array of triangles.
     */
    UMBRADEC const Umbra::Vector3i* getTriangles        (void) const;

    /*!
     * \brief   Gets model index.
     *
     * \return  Index of the model in the scene.
     */
    UMBRADEC int                    getIndex            (void) const;

    /*!
     * \brief   Gets the custom estimated rendering cost of the model.
     *
     * \return  The user-defined rendering cost of the model, if one was set.
     *          0.0 otherwise.
     */
    UMBRADEC float                  getEstimatedCost     (void) const;
private:
    SceneModel   (class ImpModel* imp);
    ~SceneModel  (void);

    ImpModel*    m_imp;
    friend class ImpScene;
    friend class ImpObject;
};

/*!
 * \brief   A class that represents a single object instance in the scene.
 *
 * \sa      SceneModel
 * \sa      Scene::insertObject
 * \sa      Scene::getObject
 * \sa      Scene::getObjectCount
 * \sa      Scene::getObjectByID
 */
class SceneObject
{
public:

    /*!
     * \brief   Object type flags.
     *
     * OCCLUDER objects are used to hide other geometry. Occluders should be
     * opaque and ideally defined so that they actually block the visibility
     * between different areas of the scene.
     *
     * TARGET objects can be hidden by occluders. Most of the objects in the
     * scene should be targets.
     *
     * GATE objects can be opened and closed at runtime so that their effect
     * on visibility changes. Typically windows and doors are tagged as gates.
     *
     * VOLUME objects have a solid interior as well as a solid surface. This
     * is useful for large target objects that may contain the camera, such as
     * lighting volumes. A query will show a large volume target object as
     * visible for a camera in the inside even if the line of sight to the
     * object's surface is occluded, while a similar non-volume target may be
     * occluded. Due to the way the inside volume of the object is determined,
     * volume objects should always have models that have no holes in their
     * surface and have all surface polygons facing towards the outside of the
     * object. If possible, volume object models should be convex.
     *
     * \note    OCCLUDER and GATE are mutually exclusive.
     *
     * \sa      SceneObject::getFlags
     */
    enum Flags
    {
        OCCLUDER        = (1<<0), /*!< The object is an occluder. */
        TARGET          = (1<<1), /*!< The object can be occluded. */
        GATE            = (1<<2), /*!< The object is a gate which can be opened and closed at runtime. */
        VOLUME          = (1<<3)  /*!< The object's entire interior volume acts as a target. */
    };

    /*!
     * \brief   Gets transformation matrix of the object.
     *
     * \param   mtx Variable to store the result.
     * \param   mf  Matrix ordering.
     */
    UMBRADEC void                   getMatrix           (Umbra::Matrix4x4& mtx, MatrixFormat mf = MF_COLUMN_MAJOR) const;

    /*!
     * \brief   Gets model of the object.
     *
     * \return  Model of the object.
     */
    UMBRADEC const SceneModel*      getModel            (void) const;

    /*!
     * \brief   Gets type flags of the object.
     *
     * \return  Type flags of the object.
     */
    UMBRADEC unsigned int           getFlags            (void) const;

    /*!
     * \brief   Gets ID of the object.
     *
     * \return  ID of the object.
     */
    UMBRADEC unsigned int           getID               (void) const;

    /*!
     * \brief   Get object triangle winding order.
     *
     * \return  TriangleWinding Object triangle winding.
     */
    UMBRADEC TriangleWinding        getTriangleWinding  (void) const;

    /*!
     * \brief   Get the axis-aligned bounds of this object
     */
    UMBRADEC void                   getBounds           (Vector3& mn, Vector3& mx) const;

    /*!
     * \brief   Get object draw distance limits.
     *
     * \return  A 2-element vector (minimum distance, maximum distance)
     */
    UMBRADEC Vector2                getDrawDistance     (void) const;

    /*!
     * \brief   Get object distance computation bounds.
     *
     * \note    This will return an invalid AABB when distance bounds have not
     *          been set by the user.
     */
    UMBRADEC void                   getDistanceBound    (Vector3& mn, Vector3& mx) const;

private:
    SceneObject  (class ImpObject* imp);
    ~SceneObject (void);

    ImpObject*   m_imp;
    friend class ImpScene;
};

/*!
 * \brief   A read-only class to access the view volumes that have been
 *          inserted to the Scene. View volumes are always axis aligned
 *          bounding boxes.
 *
 * \sa      Scene::insertViewVolume
 * \sa      Scene::getViewVolume
 * \sa      Scene::getViewVolumeCount
 * \sa      Scene::deleteViewVolume
 */
class SceneVolume
{
public:
    /*!
     * \brief   Gets minimum coordinates of the view volume.
     *
     * \return  Minimum coordinates of the view volume.
     */
    UMBRADEC const Umbra::Vector3&  getMin              (void) const;

    /*!
     * \brief   Gets maximum coordinates of the view volume.
     *
     * \return  Maximum coordinates of the view volume.
     */
    UMBRADEC const Umbra::Vector3&  getMax              (void) const;

    /*!
     * \brief   Gets ID of the volume.
     *
     * \return  ID of the volume.
     */
    UMBRADEC unsigned int           getID               (void) const;

private:
    SceneVolume  (class ImpVolume* imp);
    ~SceneVolume (void);

    ImpVolume*   m_imp;
    friend class ImpScene;
};

/*!
 * \brief   Scene is Umbra Optimizer's internal representation of a 3D scene
 *
 * Umbra Optimizer computes visibility using the Scene. Only objects
 * that are added to the Scene will be included in the results.
 *
 */
class Scene
{
public:

    /*!
     * \brief   Creates a new scene.
     *
     * \param   fileName Name of a scene file. If this is NULL an empty scene is constructed.
     *
     * \return  A pointer to the newly constructed scene.
     *
     * \sa Scene::release
     */
    UMBRADEC static Scene* UMBRACALL    create              (const char* fileName = NULL);

    /*!
     * \brief   Deserializes the Scene from an InputStream
     *
     * \param   in  InputStream from which the Scene is read
     *
     * \return  A pointer to the created scene.
     *
     * \sa Scene::release
     * \sa Scene::serialize
     */
    UMBRADEC static Scene* UMBRACALL    create              (InputStream& in);

    /*!
     * \brief   Releases a scene that has been previously created with Scene::create.
     *
     * \sa Scene::create
     */
    UMBRADEC void                       release             (void);

    /*!
     * \brief   Exports the Scene to a file.
     *
     * \param   fileName    The file where the scene is stored into.
     */
    UMBRADEC bool                       writeToFile         (const char* fileName);

    /*!
     * \brief  Serialize the Scene for storage / transfer / byte data inspection.
     *         Scene::create(InputStream&) to deserialize.
     *
     * \param   out     Output stream
     *
     * \return  true when successful, false otherwise
     *
     * \sa Scene::create
     */
    UMBRADEC bool                       serialize           (OutputStream& out) const;

    /*!
     * \brief   Inserts a model into the Scene.
     *
     * \param   vertices        Model vertices.
     * \param   indices         Model indices.
     * \param   vertexCount     Model vertex count (size of the vertices array).
     * \param   triangleCount   Model triangle count (size of the triangles array).
     * \param   vertexStride    Number of bytes between consecutive vertices. Zero
     *                          indicates a tightly packed vertex array.
     * \param   estimatedCost   Estimated rendering cost of the model for computing object
     *                          grouping in the scene. Value 0.0 means that the model
     *                          should use a default value derived from its geometry instead
     *                          of a custom cost.
     *
     * \return  A pointer to a SceneModel object or NULL on failure.
     *
     * \note    A SceneModel contains transform-independent object geometry.
     *          A single SceneModel can be referenced by multiple SceneObjects.
     *
     * \note    The size of the index array has to be at least triangleCount,
     *          as each triangle is naturaly composed of three indices.
     */
    UMBRADEC const SceneModel*          insertModel         (const float*           vertices,
                                                             const uint32_t*        indices,
                                                             int                    vertexCount,
                                                             int                    triangleCount,
                                                             int                    vertexStride = 0,
                                                             float                  estimatedCost = 0.0f);

    /*! \overload
     */
    UMBRADEC const SceneModel*          insertModel         (const float*           vertices,
                                                             const uint16_t*        indices,
                                                             int                    vertexCount,
                                                             int                    triangleCount,
                                                             int                    vertexStride = 0,
                                                             float                  estimatedCost = 0.0f);

    /*!
     * \brief   Gets the number of models in the scene.
     *
     * \return  Number of models in the scene
     *
     * \sa SceneModel
     * \sa Scene::insertModel
     * \sa Scene::getModel
     */
    UMBRADEC int                        getModelCount       (void)              const;

    /*!
     * \brief   Gets a single model with a given index from the scene.
     *
     * \param   index   The index of the SceneModel to get.
     *
     * \return  A pointer to the SceneModel that corresponds to the given index.
     *
     * \sa SceneModel
     * \sa Scene::insertModel
     * \sa Scene::getModelCount
     */
    UMBRADEC const SceneModel*          getModel            (int index)         const;

    /*!
     * \brief   Inserts an object into the Scene.
     *
     * \param   model   A pointer to a SceneModel that the object references.
     * \param   mtx     The object's transformation matrix.
     * \param   id      The object's user ID. User ID's are used to identify the object.
     * \param   flags   Bitmask of flags.
     * \param   mf      Matrix ordering.
     * \param   winding Triangle winding for the given object.
     *
     * \return  A pointer to an object or NULL on failure.
     *
     * \note    A SceneObject represents a single instance of a SceneModel.
     *          A SceneObject includes a reference to a SceneModel and a transformation
     *          matrix that defines it's position in the 3D scene.
     *
     * \note    Same ID can be set for multiple OCCLUDER objects. For instance setting all OCCLUDER
     *          only objects to 0 to may make the ID scheme simpler. All TARGET and GATE objects
     *          need an unique ID.
     *
     * \note    The triangle count of the SceneObject does not have any runtime performance
     *          implications.
     *
     * \sa SceneObject::Flags
     */
    UMBRADEC const SceneObject*         insertObject        (const SceneModel*        model,
                                                             const Umbra::Matrix4x4&  mtx,
                                                             unsigned int             id,
                                                             unsigned int             flags,
                                                             MatrixFormat             mf = MF_COLUMN_MAJOR,
                                                             TriangleWinding          winding = WINDING_CCW,
                                                             const Vector2*           drawDistanceLimits = NULL,
                                                             const Vector3*           distanceBoundsMin = NULL,
                                                             const Vector3*           distanceBoundsMax = NULL);

    /*!
     * \brief   Gets the number of objects in the scene
     *
     * \return  Number of objects in the scene
     *
     * \sa SceneObject
     * \sa Scene::insertObject
     * \sa Scene::getObject
     * \sa Scene::getObjectByID
     */
    UMBRADEC int                        getObjectCount      (void)              const;

    /*!
     * \brief   Gets a single object with a given index from the scene.
     *
     * \param   index   The index of the Object to get
     *
     * \return  A pointer to the Object that corresponds to the given index
     *
     * \sa SceneObject
     * \sa Scene::insertObject
     * \sa Scene::getObjectCount
     * \sa Scene::getObjectByID
     */
    UMBRADEC const SceneObject*         getObject           (int index)         const;

    /*!
     * \brief   Define a single view volume for the Scene.
     *
     * The set of view volumes define the region from which visibility can be
     * calculated from, i.e., regions where camera can be. View volumes are
     * defined with axis aligned bounding boxes. The default view volume is the
     * whole scene.
     *
     * \param   mn  View volume min coordinate.
     * \param   mx  View volume max coordinate.
     * \param   id  View volume id.
     *
     * \note    A Scene can have multiple view volumes.
     * \note    The set of view volumes define the region where the application camera can move.
     *          If the application camera moves outside the region defined by the view volumes
     *          visibility information may not be correct.
     */
    UMBRADEC const SceneVolume*         insertViewVolume    (const Umbra::Vector3& mn, const Umbra::Vector3& mx, unsigned int id);

    /*!
     * \brief   Gets the number of view volumes in the scene
     *
     * \return  Number of view volumes in the scene
     *
     * \sa SceneVolume
     * \sa Scene::insertViewVolume
     * \sa Scene::getViewVolume
     */
    UMBRADEC int                        getViewVolumeCount  (void)              const;

    /*!
     * \brief   Gets a single view volume with a given index from the scene.
     *
     * \param   index   The index of the view volume to get.
     *
     * \return  A pointer to the view volume that corresponds to the given index.
     *
     * \sa SceneVolume
     * \sa Scene::insertViewVolume
     * \sa Scene::getViewVolumeCount
     */
    UMBRADEC const SceneVolume*         getViewVolume       (int index)         const;

    /*!
     * \brief   Insert seed point for connectivity analysis. Cells which are not reachable from any seed point will be removed.
     *          If no seed points are present, the default behaviour is to remove cell clusters that are smaller than the largest connected cluster.
     * \param   p   Seed point
     *
     */
    UMBRADEC void                       insertSeedPoint     (const Vector3& p);

    /*!
     * \brief   Get axis aligned bounds of Scene geometry and view volumes
     */
    UMBRADEC void                       getBounds           (Vector3& mn, Vector3& mx) const;

private:
                                        Scene               (void);
                                        Scene               (class ImpScene* imp);
                                        Scene               (const Scene&); // not allowed
    Scene&                              operator=           (const Scene&); // not allowed
                                        ~Scene              (void);

    class ImpScene* m_imp;
    friend class ImpTask;
    friend class ImpScene;
};

} // namespace Umbra

#endif // UMBRASCENE_HPP
