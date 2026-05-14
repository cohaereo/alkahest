// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRATOME_HPP
#define UMBRATOME_HPP

/*!
 * \file    umbraTome.hpp
 * \brief   Umbra tome interface
 */

#include "umbraDefs.hpp"
#include "umbraPlatform.hpp"

#define UMBRA_TOMECOLLECTION_SIZE 128

namespace Umbra
{

/*!
 * \brief   The data container for visibility and spatial connectivity data.
 *
 * See \ref umbradata for a description of the Tome data format.
 *
 * The output of the Umbra Optimizer computation is a Tome object for the
 * input scene. To make use of Umbra Runtime queries the Tome data corresponding
 * to the scene must be loaded by the runtime system and introduced to a
 * Query context via Query::init().
 *
 * The Tome object wraps a read-only data buffer of size Tome::getSize().
 * There is no serialization or deserialization needed for the data, the Tome
 * can simply be stored by storing the data array starting at the memory pointed
 * to by the Tome pointer. The data buffer is internally accessed as an array of
 * 32-bit elements. When the endianness of the runtime environment differs from
 * the endianness of the computation platform, an endianness swap needs to be
 * performed for the data array. Tome::swapEndianness() is provided for this
 * purpose. Additionally, the data start must be aligned to a 16-byte boundary.
 * Tome::getStatus() can be used to confirm that these requirements are met.
 *
 * The TomeLoader utility is provided for convenient loading of the Tome data.
 *
 * The Umbra runtime also supports queries involving multiple Tome data sets.
 * This is accomplished via the TomeCollection object that manages references
 * to the active Tome instances.
 */
class Tome
{
public:

    /** Tome self-check status */
    enum Status
    {
        /** The Tome is ok */
        STATUS_OK,
        /** The Tome has not been initialized, returned for a NULL pointer */
        STATUS_UNINITIALIZED,
        /** The Tome data doesn't look like Tome data */
        STATUS_CORRUPT,
        /** The Tome is of an older version than the code, a version update operation is needed */
        STATUS_OLDER_VERSION,
        /** The Tome is of a newer version than the code, the runtime code needs updating */
        STATUS_NEWER_VERSION,
        /** The Tome data is in wrong endianness */
        STATUS_BAD_ENDIAN,
        /** The tome data start is not 16-byte aligned */
        STATUS_BAD_ALIGN,
        /** The buffer provided does not contain all of the Tome data */
        STATUS_OUT_OF_MEMORY
    };

    /** Statistics for the Tome */
    enum Statistic
    {
        /** The total size of the Tome data */
        STAT_TOTAL_DATA_SIZE,
        /** The total size of portal graph data */
        STAT_PORTAL_DATA_SIZE,
        /** Size of cell locating view tree */
        STAT_VIEWTREE_SIZE,
        /** Size of cell accurate find data structure */
        STAT_ACCURATE_LOCATION_SIZE,
        /** Size of portal geometry data */
        STAT_PORTAL_GEOMETRY_SIZE,
        /** Size of object data */
        STAT_OBJECT_DATA_SIZE,
        /** The total size of per-tile overhead data */
        STAT_TILE_COMMON_DATA_SIZE,
        /** Cluster graph data size */
        STAT_CLUSTER_GRAPH_DATA_SIZE,
        /** Tome matching data size */
        STAT_MATCHING_DATA_SIZE
    };

    /*!
     * \brief   Capabilities of the Tome.
     */
    enum Capability
    {
        /** The Tome can be used to built TomeCollections */
        CAPABILITY_TOMECOLLECTION_INPUT     = (1 << 0),
        /** The Tome can be used in connectivity queries */
        CAPABILITY_CONNECTIVITY_QUERIES     = (1 << 1),
        /*! The Tome has per-object static optimization data 
          * (ComputationParams::DATA_OBJECT_OPTIMIZATIONS) 
          */
        CAPABILITY_OBJECT_OPTIMIZATIONS     = (1 << 2)
    };

    /** Query an integer statistics value */
    int                             getStatistic        (Statistic s) const;
    /** Perform a self-check on the Tome data */
    Status                          getStatus           (void) const;
    /** Get the version number of the Tome data format */
    int                             getVersion          (void) const;
    /** Get the total size of the Tome data, in bytes. Equivalent to getStatistic(STAT_TOTAL_DATA_SIZE). */
    uint32_t                        getSize             (void) const;
    /** Check the Tome for corruption */
    Status                          checkCorruption     (void) const;

    /** Retrieve the spatial extents for a target object */
    void                            getObjectBounds     (int objIndex, Vector3& mn, Vector3& mx) const;
    /** Retrieve the target object count */
    int                             getObjectCount      (void) const;
    /** Retrieve the cluster count */
    int                             getClusterCount     (void) const;
    /** Retrieve the cluster portal count */
    int                             getClusterPortalCount (void) const;
    /** Retrieve the gate object count */
    int                             getGateCount        (void) const;
    /** Retrieve the cell count */
    int                             getCellCount        (void) const;
    /** Retrieve the tile count */
    int                             getTileCount        (void) const;
    /** Get axis-aligned bounds for tome */
    void                            getBounds           (Vector3& mn, Vector3& mx) const;

    /*!
     * \brief   Returns the group of user object IDs set by Umbra::Scene::insertObject()
     *          that correspond to a given target object index. One target object
     *          may correspond to multiple user objects in case the Tome uses
     *          object grouping.
     *
     *          You can query the size of the user object group for a target object index
     *          with getObjectUserIDs(objIndex, NULL, 0).
     *
     * \param   objIndex    Index of a target object in the Tome's internal mapping
     * \param   ids         A pointer to the array where the matching user object IDs
     *                      will be written. May be NULL, in which case nothing will
     *                      be written
     * \param   max         The maximum number of elements that may be written to ids
     *
     * \return  The number of user object IDs that match to the Umbra::Tome target
     *          object index
     *
     * \note    This method treats each objIndex as a single-object group for Tomes that
     *          do not group objects.
     */
    int                             getObjectUserIDs    (int objIndex, uint32_t* ids, int max) const;

    /*!
     * \brief   Returns the user ID set by Umbra::Scene::insertObject() for
     *          for a given object index in a Tome which does not group objects.
     * \param   objIndex    Index of a target object in the Tome's internal mapping
     *
     * \return  The Umbra::SceneObject user ID that corresponds to the given
     *          Umbra::Tome target object index in a Tome that does not group objects
     *
     * \note    This method is deprecated, please use getObjectUserIDs instead. This
     *          method must not be called with a Tomes that groups objects.
     */
    uint32_t                         getObjectUserID     (int objIndex) const;

    /*!
     * \brief   Returns the user ID set by Umbra::Scene::insertObject() for
     *          a given gate index.
     *
     * \param   gateIndex    Index of a gate in the internal Tome mapping.
     *
     * \return  The Umbra::SceneObject user ID that corresponds to the given
     *          Umbra::Tome gate index
     */
    uint32_t                        getGateUserID       (int gateIndex) const;

    /** Get the internal Tome index for the target object identified by the user ID */
    int                             findObjectIndex     (uint32_t id) const;

    /** Get the internal Tome index for the gate identified by the user ID */
    int                             findGateIndex       (uint32_t id) const;

    /** Get the size of the GateStateVector for the gates in this Tome, in bytes */
    size_t                          getGateStateSize    (void) const;

    /** Get axis aligned bounds of cluster */
    void                            getClusterBounds    (int idx, Vector3& mn, Vector3& mx) const;

    /*!
     * \brief   Perform in-place initialization for the Tome data in memory
     *
     * This is convenience functionality that simply casts the passed in memory
     * pointer into a Tome pointer and invokes the self-check functionality
     * implemented by Tome::getStatus()
     *
     * \param   tome        The memory buffer cast into a Tome pointer
     * \param   buf         A memory buffer containing Tome data
     * \param   size        size of the memory buffer, in bytes
     * \return              Status of the Tome
     */
    static Status UMBRACALL         init                (const Tome** tome, const uint8_t* buf, size_t size);

    /*!
     * \brief   Swap the endianness of the Tome data
     *
     * \param   data        Tome data
     * \param   size        size of the Tome data
     */
    static void UMBRACALL           swapEndianness      (Tome* data, size_t size);

    /*!
     * \brief   Convert an older Tome version to the latest version.
     *
     * A new memory allocation will be made for the converted Tome by using the
     * Allocator callback.
     *
     * The old Tome data can be freed after the version update.
     *
     * \param   oldData     Tome data of older version, \ref Tome::getStatus()
     * \param   a           An Allocator implementation for doing a dynamic allocation for
     *                      the converted Tome. Pass in NULL to use the default implementation.
     * \return              A newly allocated up-to-date Tome instance, oldData if not version
     *                      conversion was necessary, NULL if version conversion was not possible
     */
    static const Tome* UMBRACALL    updateVersion       (const Tome* oldData, Allocator* a = NULL);

    /*!
     * \brief   Test whether the Tome supports given capabilities.
     *
     * \param   cap     A bit vector of capabilities to be tested
     * \return          true if all the required capabilities are supported, false otherwise
     */
    bool                        testCapability      (uint32_t cap) const;
};

/*!
 * \brief   A collection of Tome data objects
 *
 * A TomeCollection is an alternative input to the Umbra runtime queries.
 * It enables on demand streaming of individual Tome objects to reduce the
 * working set memory requirement in large continuous environments.
 *
 * The active set of in-memory Tome objects are passed to the collection via
 * the build() method. The TomeCollection object holds live references to the
 * memory addresses of the input Tomes (instead of taking copies of the data).
 * Therefore the Tome object may not be deallocated or moved until these
 * references are removed either by deleting the TomeCollection object or
 * by calling build() again with a different set of inputs.
 *
 * The TomeCollection object holds additional data that allows queries to
 * traverse between the input tomes. This data is generated as part of the build()
 * operation and placed in a single continuous memory blob dynamically allocated
 * from the allocator passed in init(). This memory is deallocated upon destruction
 * or upon the next build() call.
 */
class TomeCollection
{
public:

    /** Error codes from build(), serialize() and deserialize() */
    enum ErrorCode
    {
        /** The operation succeeded */
        SUCCESS = 0,
        /** Not enough memory available */
        ERROR_OUT_OF_MEMORY,
        /** Error in parameters */
        ERROR_INVALID_PARAM,
        /** One of the input tomes did not contain matching data, \sa ComputationParams::DATA_TOME_MATCH */
        ERROR_NO_MATCHING_DATA,
        /** One of the input tomes was not a valid tome */
        ERROR_CORRUPT_TOME,
        /** The input tomes provided overlap */
        ERROR_OVERLAPPING_TOMES,

        /** InputStream/OutputStream failure */
        ERROR_IO,
        /** Unsupported version */
        ERROR_OLDER_VERSION,
        /** Unsupported version */
        ERROR_NEWER_VERSION,
        /** Data doesn't look like serialized TomeCollection */
        ERROR_CORRUPTED,
        /** Unbuilt collection */
        ERROR_UNBUILT,
        /** Bad endianess */
        ERROR_BAD_ENDIAN,
        /** Invalid input tomes for deserialization */
        ERROR_INVALID_INPUT_TOMES
    };

    /** Constructor, optional Allocator */
    TomeCollection  (Allocator* allocator = NULL);
    /** Destructor */
    ~TomeCollection (void);

    /** (Re)initialize the TomeCollection object. Releases references and frees allocations. */
    void        init                (Allocator* allocator = NULL);

    /** (Re)initialize the TomeCollection object. Releases references and frees allocations.
      * This version writes the TomeCollection into the user provided buffer. If the size of the
      * buffer is insufficient, an error is generated. Note that a scratch allocator MUST be
      * passed in to build() when using this variant. */
    void        init                (void* buffer, size_t size);


    /**
     *  \brief  Store references to active set of Tomes and build inter-tome traverse data
     *
     *  This operation prepares the TomeCollection for use in Umbra queries on the set of input Tomes.
     *  The complexity of the operation is expected to be "a couple of frames", it should not be done
     *  as a per-frame operation.
     *
     *  The input Tomes must have supplemental tome matching data present. The generation of this data
     *  is optional and is enabled via the ComputationParams::DATA_TOME_MATCH flag for
     *  ComputationParams::OUTPUT_FLAGS in the data computation. The presence of the matching data
     *  can be verified with Tome::testCapacility(CAPABILITY_TOMECOLLECTION_INPUT). In other regards the
     *  input Tomes do not differ from Tomes used in single-Tome query mode.
     *
     *  The input Tomes may not overlap. Empty space between Tomes is ok. The input Tomes must have been
     *  computed with identical computation parameters.
     *
     *  The build operation requires dynamic memory allocations. The optional scratchAllocator instance
     *  will be used for these allocations. The scratch allocations are freed before build() returns.
     *  If a scratchAllocator is not given, the TomeCollection allocator that is also used for the
     *  generated inter-tome data blob will be used.
     *
     *  The output of the build() operation - the inter-tome linking data - is placed in a single
     *  continuous blob of memory that is allocated during the operation using the TomeCollection allocator.
     *  As a special case, passing in a single Tome to build() does not require this allocation.
     *
     *  \param  tomes               Array of pointers to Tome objects, must be non-NULL if numTomes != 0
     *  \param  numTomes            Number of Tome objects
     *  \param  scratchAllocator    (Optional) Allocator instance for build scratch memory needs
     *  \param  previous            (Optional) Previously built TomeCollection for performing an incremental.
     *                              update. Incremental updates are faster. The old TomeCollection is not 
	 *                              modified.
     *  \return SUCCESS if operation succeeded
     */
    ErrorCode   build               (const Tome** tomes, int numTomes, Allocator* scratchAllocator = NULL,
                                     const TomeCollection* previous = NULL);

    /**
     *  \brief  Build with specified minimum empty space bounds
     *
     *  Use this variant to guarantee the validity of the data to given world space bounds. The default
     *  operation is to only allow queries within the combined bounds of the input Tomes.
     *
     *  \param  tomes               Array of pointers to Tome objects, must be non-NULL if numTomes != 0
     *  \param  numTomes            Number of Tome objects
     *  \param  mn                  Min corner of the empty bounds
     *  \param  mx                  Max corner of the empty bounds
     *  \param  scratchAllocator    (Optional) Allocator instance for build scratch memory needs
     *  \param  previous            (Optional) Previously built TomeCollection for performing an incremental.
     *                              update. Incremental updates are faster. The old TomeCollection is not 
	 *                              modified.
     *  \return SUCCESS if operation succeeded
     */
    ErrorCode   build               (const Tome** tomes, int numTomes, const Vector3& mn, const Vector3& mx,
                                     Allocator* scratchAllocator = NULL, const TomeCollection* previous = NULL);

    /** Get number of Tome objects referenced by this collection */
    int         getNumTomes         (void) const;
    /** Get a referenced Tome object by index */
    const Tome* getTome             (int idx) const;
    /** Get the size (in bytes) of the additional inter-tome traverse data contained in this collection */
    uint32_t    getSize             (void) const;
    /** Get number of total clusters in this collection, \sa Tome::getClusterCount() */
    int         getClusterCount     (void) const;
    /** Retrieve the cluster portal count, \sa Tome::getClusterPortalCount()  */
    int         getClusterPortalCount (void) const;
    /** Get number of total object indices in this collection, \sa Tome::getObjectCount() */
    int         getObjectCount      (void) const;
    /** Get number of gate object indices in this collection, \sa Tome::getGateCount() */
    int         getGateCount        (void) const;
    /** Map object index to user IDs, \sa Tome::getObjectUserIDs()  */
    int         getObjectUserIDs    (int objIndex, uint32_t* ids, int max) const;
    /** Map object index to user ID, \sa Tome::getObjectUserID()  */
    uint32_t    getObjectUserID     (int objIndex) const;
    /** Map user ID to object index, \sa Tome::findObjectIndex()  */
    int         findObjectIndex     (uint32_t id) const;
    /** Map gate index to user ID, \sa Tome::getGateUserID()  */
    uint32_t    getGateUserID       (int gateIndex) const;
    /** Map user ID to gate index, \sa Tome::findGateIndex()  */
    int         findGateIndex       (uint32_t id) const;
    /** Get the size of the GateStateVector for the gates in this TomeCollection, in bytes */
    size_t      getGateStateSize    (void) const;
    /** Retrieve the spatial extents for a target object */
    void        getObjectBounds     (int objIndex, Vector3& mn, Vector3& mx) const;
    /** Get axis-aligned bounds */
    void        getBounds           (Vector3& mn, Vector3& mx) const;

    /**
     *  \brief Get a local and tome index for a TomeCollection cluster.
     *
     *  The cluster lists outputted by e.g. Query::queryConnectedRegion contain global cluster
     *  indices that differ between TomeCollections initialized from different sets of Tomes
     *  This function allows obtaining a tome-local constant index for these global indices, and
     *  index of the containing tome
     *
     *  \param clusterGlobalIdx     Cluster index obtained using a TomeCollection connectivity
     *                              query
     *  \param tomeIdx              Receives the tome index for given global index
     *                              This index corresponds to the Tome array given to TomeCollection::build
     *  \param localIdx             Cluster index within the tome
     *
     */
    void        findClusterIndex    (int globalClusterIdx, int& tomeIdx, int& localIdx);

    /**
     *  \brief Get a local and tome index for a TomeCollection cluster portal.
     *
     *  Cluster indices are obtained from QueryExt::clusterPortals.
     *
     *  \sa getClusterIndex
     */
    void        findPortalIndex     (int globalClusterPortalIdx, int& tomeIdx, int& localIdx);

    /**
     *  \brief Serialize a built TomeCollection into OutputStream.
     *
     *  Storing the TomeCollection in serialized form is useful for avoiding TomeCollection build times at runtime.
     *
     *  Endianness of the data can be swapped by treating it as an array of 32-bit integers and swapping them.
     */
    ErrorCode   serialize           (OutputStream& stream) const;

    /**
     *  \brief Deserialize a TomeCollection from InputStream.
     *
     *  Useful for avoiding TomeCollection build times at runtime.
     *
     *  \note  Expects tomes to be in the same order as when TomeCollection::build was called!
     */
    ErrorCode   deserialize         (InputStream& stream, const Tome** tomes, int numTomes, Allocator* scratchAllocator = NULL);

    uint8_t m_mem[UMBRA_TOMECOLLECTION_SIZE];

private:
    TomeCollection(const TomeCollection&); // not allowed
    TomeCollection& operator= (const TomeCollection&); // not allowed

};

/*!
 * \brief   An utility class for instantiating Tome objects
 *
 * This utility uses a dynamic allocation for storing the Tome data into.
 * It is possible to initialize from a preloaded memory buffer without doing
 * further dynamic allocations, see Tome::getStatus() and Tome::init().
 *
 * The TomeLoader should be used when automatic version updating, endian swapping
 * and data aligning is desired.
 */
class TomeLoader
{
public:

    /*!
     * \brief   Creates an Umbra::Tome instance from a given input stream.
     *
     * \param   buffer  Input stream. This must contain the Tome data computed by the Umbra3
     *                  Optimizer.
     * \param   a       An optional allocator instance.
     * \return          Tome instance that can be used for runtime queries.
     */
    static const Tome* UMBRACALL load            (InputStream& input, Allocator* a = NULL);

    /*!
     * \brief   Creates an Umbra::Tome instance from a given data buffer.
     *
     * \param   buffer  Input buffer. This must contain the Tome data computed by the Umbra3
     *                  Optimizer.
     * \param   size    Size of the input buffer.
     * \param   a       An optional allocator instance.
     * \return          Tome instance that can be used for runtime queries.
     */
    static const Tome* UMBRACALL loadFromBuffer  (const uint8_t* buffer, size_t size, Allocator* a = NULL);

    /*!
     * \brief   Frees a given Umbra::Tome instance loaded by using the Umbra::TomeLoader loader methods.
     *
     * \param   tome    The Tome instance to free.
     * \param   a       An optional allocator instance, must match the Allocator used with
     *                  the load functions.
     */
    static void UMBRACALL       freeTome        (const Tome* tome, Allocator* a = NULL);
};

} // namespace Umbra
#endif // UMBRATOME_HPP
