// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRACOMPUTATIONPARAMS_HPP
#define UMBRACOMPUTATIONPARAMS_HPP

/*!
 * \file    umbraComputationParams.hpp
 * \brief   Umbra computation parameters
 */

#include "umbraDefs.hpp"

namespace Umbra
{

class Scene;
class OutputStream;
class InputStream;
class Allocator;

#define UMBRA_COMPUTATION_ARGS_SIZE 128

/*!
 * \brief   A container for computation parameters
 */
class ComputationParams
{
public:

    /*!
     * \brief   Float-type scene parameters. These affect the computation time and output quality.
     *
     * \sa      ComputationParams::setParam
     * \sa      ComputationParams::getParam
     * \sa      ComputationParams::getParamLimits
     */
    enum ParamName
    {
        //
        // INPUT PARAMETERS
        //

        /*! \brief  Smallest occluder size. (Type: float)
         *
         * Smallest occluder parameter defines roughly the size of features that
         * should be used for occlusion. Larger values produces less data and
         * faster occlusion queries, while smaller values produce better occlusion.
         *
         * \note    This parameters was VIEWCELL_SIZE in Umbra 3.0.X
         */
        SMALLEST_OCCLUDER = 1,

        /*! \brief  Backface limit percentage. (Type: float)
         *
         * This value is used to remove bad detail from output by testing
         * backfaces. The idea is that typically valid camera positions cannot
         * see much backfaces. Typically geometry under terrain and inside
         * solid objects is removed. Value of 100 disables backface testing.
         */
        BACKFACE_LIMIT,

        /*! \brief  Smallest acknowledged hole. (Type: float)
         *
         * This value specifies smallest detail level that is used in
         * visibility computation. Smaller holes or cracks than this don't leak
         * visibility. Internally, this parameter defines the dimensions of the
         * voxels that the geometry is turned into.
         *
         * The value of this parameter can not exceed the value of the
         * SMALLEST_OCCLUDER parameter. Attempting to do so will result in the
         * parameter value being silently clamped to the value of SMALLEST_OCCLUDER.
         */
        SMALLEST_HOLE,

        /*! \brief  Tile size. (Type: float)
         *
         * This parameter is obsolete.
         */
        TILE_SIZE,

        /*! \brief  Cluster size. (Type: float)
         *
         * Cluster size is the cell size for connectivity queries.
         * TODO: explain how this affects occlusion queries
         */
        CLUSTER_SIZE,

        /*! \brief  Hierarchy detail. (Type: float)
         *
         * Cell graph hierarchy computation detail. Zero disables detailed
         * hierarchy. One is typically a good value. Larger values increases
         * computation time.
         */
        HIERARCHY_DETAIL,

        //
        // OUTPUT (TOME GENERATION) PARAMETERS
        //

        /*! \brief Tome generation output flags. (Type: UINT32)
         *
         * This parameter is used by tome generation only.
         *
         * Specifies what data the generated Tome contains. Specified as a combination
         * of ComputationParams::DataFlags.
         *
         * \sa ComputationParams::DataFlags
         */
        OUTPUT_FLAGS,

        /*! \brief (OPTIONAL) Minimum accurate distance (Type: float)
         *
         * This parameter is used by tome generation only.
         *
         * The value of this parameter defines the minimum possible value
         * of the "accurate occlusion threshold" parameter for the
         * Umbra visibility query. A larger value for this parameter has
         * the effect of reducing tome size, as portals between occlusion
         * data hierarchy levels can be pre-culled in tome generation.
         *
         * A value of zero or smaller will be translated to a default
         * setting of four times the minimum smallest occluder parameter
         * used in generating the tome.
         *
         * Note that the Umbra visibility query will use this value also
         * as the default value (when not explicitly specified by the
         * caller to umbraQueryPortalVisibility()).
         */
         MINIMUM_ACCURATE_DISTANCE,

        //
        // GROUPING PARAMETERS
        //

        /*! \brief Group cost when doing object grouping (Type: float)
         *
         * Affects the eagerness of object grouping. Positive value enables
         * object grouping for output Tome. This reduces runtime overhead at
         * the expense of culling quality. If object grouping is enabled, a
         * single index maps to multiple object IDs.
         *
         * \sa OBJECT_GROUP_COST
         */
        OBJECT_GROUP_COST,

        /*! \brief (OPTIONAL) Reference world size (Type: float3)
         *
         * By default the object groups are optimized relative to the
         * scene AABB. Passing in a reference world size, the grouping
         * is optimized with respect to the world instead of the scene.
         *
         * This also ensures that grouping results are consistent with
         * different scenes in the world.
         *
         * The world size must be big enough to encompass the scene.
         *
         * \sa WORLD_SIZE
         */
        WORLD_SIZE
    };

    /*!
     * \brief   Computation and output flags.
     */
    enum DataFlags
    {
        DATA_VISUALIZATIONS         = (1<<0),    /*!< Visualizations. */
        DATA_TOME_MATCH             = (1<<1),    /*!< Data for tome matching. */
        DATA_STRICT_VIEW_VOLUMES    = (1<<2),    /*!< Undefined culling results outside view volumes */
        DATA_ACCURATE_DILATION      = (1<<3),    /*!< Accurate voxel dilation. Increases data size but cell
                                                      location is in some cases more accurate near occluder geometry. */
        DATA_OBJECT_OPTIMIZATIONS   = (1<<4),    /*!< Compute per-object static optimizations. Increases culling
                                                      quality at expense of data size and computation time. */
        DATA_SHADOW_OPTIMIZATIONS   = (1<<5)     /*!< Compute per-object static optimizations for shadows only. */
    };

    /*!
     * \brief   Set parameters for visibility computation.
     *
     * \param   name    The name of the parameter to be set
     * \param   value   The value the parameter is to be set to
     *
     * \return  Returns true if successful, false otherwise.
     *          Currently, the only possible error condition occurs
     *          when the parameter type  doesn't match with the given
     *          value type.
     */
    UMBRADEC bool setParam (ParamName name, float           value);

    /*! \overload */
    UMBRADEC bool setParam (ParamName name, uint32_t value);

    /*! \overload */
    UMBRADEC bool setParam (ParamName name, const Umbra::Vector3& value);


    /*!
     * \brief   Set view volume -specific computation parameters
     *
     * \param   volume  The name of the view volume \sa Scene::insertViewVolume
     * \param   name    The name of the parameter to be set
     * \param   value   The value the parameter is to be set to
     *
     * \return  Returns true if successful, false otherwise.
     *          Currently, only SMALLEST_HOLE, SMALLEST_OCCLUDER,
     *          BACKFACE_LIMIT can be set on a per volume basis. 
     *          An error is returned when the specified parameter 
     *          isn't one that can be set on a per-volume basis.
     */
    UMBRADEC bool setVolumeParam (uint32_t volume, ParamName name, float value);


    /*!
     * \brief   Get float parameters for visibility computation.
     *
     * \param   name        The name of the parameter
     * \param   valueOut    The destination into which the value is written
     *
     * \return  Returns true if successful, false otherwise.
     *          Currently, the only possible error condition occurs
     *          when the parameter type  doesn't match with the given
     *          value type.
     */
    UMBRADEC bool getParam (ParamName name, float&          valueOut) const;

    /*! \overload */
    UMBRADEC bool getParam (ParamName name, uint32_t& valueOut) const;

    /*! \overload */
    UMBRADEC bool getParam (ParamName name, Umbra::Vector3&  valueOut) const;

    /*!
     * \brief   Get view volume -specific computation parameters
     *
     * \param   volume      The name of the view volume \sa Scene::insertViewVolume
     * \param   name        The name of the parameter to get
     * \param   valueOut    The returned value
     *
     * \return  Returns true if successful, false otherwise.
     *          The return value may be false if the given parameter
     *          has not been set for the given volume, or if there is
     *          no such volume in the first place, or if the given parameter
     *          cannot be set on a per volume basis.
     */
    UMBRADEC bool                                   getVolumeParam (uint32_t volume, ParamName name, float& valueOut) const;


    /*!
     * \brief   Get reasonable ranges for float parameters.
     *
     * \param   scene   The scene for which to obtain the parameter range
     * \param   name    The name of the parameter
     * \param   mn      Minimum value
     * \param   mx      Maximum value
     *
     * \return  Returns true if successful, false if the parameter type
     *          doesn't match with the given limit types.
     *
     * \note    Currently, only float parameter ranges can be queried.
     */
    UMBRADEC bool                                   getParamLimits (const Scene& scene, ParamName name, float& mn, float& mx) const;

    /*!
     * \brief   Write the object into a file.
     *
     * \param   filename    The filename into which the object is written
     *
     * \return  True on success, false otherwise
     */
    UMBRADEC bool                                   writeToFile     (const char* filename) const;

    /*!
     * \brief   Write the object into a stream.
     *
     * \param   out         The stream into which the object is written
     *
     * \return  True on success, false otherwise
     */
    UMBRADEC bool                                   writeToStream   (OutputStream& out) const;

    /*!
     * \brief   Read the object from a file.
     *
     * \param   filename    The filename from which the object is read
     *
     * \return  The read object on success, NULL if there was a failure
     */
    UMBRADEC static ComputationParams*  UMBRACALL   readFromFile    (const char* filename, Allocator* a = NULL);

    /*!
     * \brief   Read the object from a stream.
     *
     * \param   in          The stream from which the object is read
     *
     * \return  The read object on success, NULL if there was a failure
     */
    UMBRADEC static ComputationParams*  UMBRACALL   readFromStream  (InputStream& in, Allocator* a = NULL);

    /*!
     * \brief   Release the object created with readFromFile()
     */
    UMBRADEC void                                   release         (void);

    /** Default constructor for creating an empty ComputationParams instance */
    UMBRADEC ComputationParams (Allocator* a = NULL);
    UMBRADEC ComputationParams (const ComputationParams&, Allocator* a = NULL);
    UMBRADEC ComputationParams& operator=(const ComputationParams&);
    UMBRADEC ~ComputationParams ();

private:

    uint8_t m_imp[UMBRA_COMPUTATION_ARGS_SIZE];
};

}   // namespace Umbra

#endif
