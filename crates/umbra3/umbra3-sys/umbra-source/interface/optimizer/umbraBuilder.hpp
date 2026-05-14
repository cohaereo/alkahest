// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRABUILDER_HPP
#define UMBRABUILDER_HPP

/*!
 * \file
 * \brief   Computation interface
 */

#include "umbraDefs.hpp"
#include "umbraPlatform.hpp"
#include "umbraComputationParams.hpp"

namespace Umbra
{

class Tome;
class Scene;
class TomeGenerator;
class TileInputSet;

/*!
 * \brief   The per-tile computation input data.
 *
 * Holds all input data and parameters for carrying out the portal graph generation for
 * the part of the scene contained in the AABB of this tile.
 */
class TileInput
{
public:
    UMBRADEC                TileInput (void);
    UMBRADEC                ~TileInput (void);

    /*! \brief  Serialize this object for storage / transfer / byte data inspection */
    UMBRADEC bool           serialize (OutputStream& out) const;

    /*! \brief  Tile input equality test for tile computation caching.
     *          Does a byte-by-byte memory compare of the data */
    UMBRADEC bool           equals (const TileInput& other) const;

    /*! \brief  Get the AABB of the part of the scene that this TileInput represents */
    UMBRADEC void           getAABB (Vector3& mn, Vector3& mx) const;

    /*! \brief  Tells if TileInput is empty. Empty tiles don't need TileResult. */
    UMBRADEC bool           isEmpty (void) const;

    /*! \brief  Get TileInput's hash value as string of hex values. Can be used for 
     *  caching results. The returned pointer has the same lifetime as TileInput. */
    UMBRADEC const char*    getHashValue (void) const;

private:
                            TileInput (const TileInput&); // not allowed
    TileInput&              operator= (const TileInput&); // not allowed

    class ImpTileInput* m_imp;

    friend class Builder;
    friend class TileInputSet;
};

/*!
 * \brief   The result data of per-tile computation.
 *
 * The result data can be turned into a TomeTile for runtime use with TomeGenerator.
 */
class TileResult
{
public:
    UMBRADEC                TileResult (void);
    UMBRADEC                ~TileResult (void);

    /*! \brief  Serialize this object for storage / transfer */
    UMBRADEC bool           serialize (OutputStream& out) const;

    /*! \brief  Strip target objects by object ID */
    UMBRADEC bool           stripObjects (const uint32_t* idArray, int idArrayLen);

private:
                            TileResult (const TileResult&); // not allowed
    TileResult&             operator=  (const TileResult&); // not allowed

    class ImpTileResult* m_imp;

    friend class Builder;
    friend class TomeGenerator;
};

/*!
 * \brief   The computation context.
 */
class Builder
{
public:

    enum Error
    {
        SUCCESS = 0,
        ERROR_EMPTY_ITERATOR,
        ERROR_PARAM,
        ERROR_OUT_OF_MEMORY,
        ERROR_LICENSE_KEY,
        ERROR_INVALID_SCENE
    };

    UMBRADEC                Builder             (void);
    UMBRADEC                Builder             (const PlatformServices& services);
    UMBRADEC                ~Builder            (void);

    UMBRADEC void           init                (const PlatformServices& services);

    /*!
     * \brief   Get required input geometry bounds for computation of a Tome.
     */
    static UMBRADEC Error   getGeometryBounds   (Vector3& geomMn, Vector3& geomMx, const ComputationParams&, const Vector3& tomeMn, const Vector3& tomeMx);

    /*!
     * \brief  Initialize computation for the given Scene and optional computation bounds.
     *
     * This is the first stage of Umbra computation. When doing incremental computation this
     * first stage must be run whenever there is any change to the input Scene.
     *
     * The result of initialization is a set of TileInput objects, representing the input for
     * independently computable and cacheable entities. The initialization process creates a
     * suitable tile subdivision of the input scene and splits the scene data into TileInput
     * entities for the resulting subdivision leaves.
     *
     * The user can opt to only retrieve a subset of the TileInput object for the scene by
     * giving axis aligned bounds for the geometry to consider in the computation. Note that
     * the resulting TileInput objects will represent a significantly larger part of the scene
     * than only that enclosed within the given bounds. The intended use of the computation
     * bounds is to skip recreation of TileInput objects when previous objects are available
     * and the extent of the change from the previous call is limited and known.
     *
     * The (mn,mx) parameters are used limiting the computation to specific bounds, for the
     * purpose of creating Tomes that correspond exactly to a given volume. This is required
     * for the ability to bind separately computed Tomes together in TomeCollection::build().
     *
     * NOTE: The computation bounds must align to the "smallest occluder" computation parameter.
     * ERROR_PARAM is produced if this requirement is not met. This requirement will be
     * lifted in a future update.
     */
    UMBRADEC Error          split               (TileInputSet& res, Scene* scene, const ComputationParams&, const Vector3& mn, const Vector3& mx);

    UMBRADEC Error          split               (TileInputSet& res, Scene* scene, const ComputationParams&);

    /*!
     * \brief   Execute computation for a single Tile.
     *
     * The bulk of Umbra computation happens tile by tile in computeTile(). The tile computation
     * is deterministic with regard to the data carried in TileInput, it is safe to cache TileResult
     * objects based on data in TileInput objects. Tile by tile computation is also the means for
     * distributing Umbra computation: it is completely legal and valid to execute individual
     * computeTile() calls in a different Builder context, even on a different process or host.
     * Both the input object and the result object are serializable for transferring the computation
     * data between computation nodes.
     *
     * This call is synchronous, there is no way to abort the tile computation or query its status
     * while the call is executing.
     *
     * Tile computation is a relatively heavyweight operation and computing the complete set of tiles
     * for a large scene can take up to hours. To be able to get results for particular areas of interest
     * it is possible to create Tomes containing partially finished results. If this is desired, create
     * "dummy" results for tiles that have not been computed yet by calling computeTile() with passing
     * true for 'dummy'.
     */
    UMBRADEC Error          computeTile         (TileResult& out, const TileInput& in);

    /*
     * \brief   Initialize a TomeGenerator
     */
    UMBRADEC Error          join                (TomeGenerator& gen, const ComputationParams&);
    UMBRADEC Error          join                (TomeGenerator& gen, const ComputationParams&, const Vector3& mn, const Vector3& mx);

    /*
     * \brief   Deserialization of data elements
     */
    UMBRADEC Error          loadTileInput       (TileInput& elem, InputStream& in);
    UMBRADEC Error          loadTileResult      (TileResult& elem, InputStream& in);

private:
                      Builder   (const Builder&); // not allowed
    Builder&          operator= (const Builder&); // not allowed

    class ImpBuilder* m_imp;
};

/*!
 * \brief   The facility of creating Tomes from finished or partial computation
 *          results.
 */
class TomeGenerator
{
public:

    UMBRADEC                TomeGenerator       (void);
    UMBRADEC                ~TomeGenerator      (void);

    UMBRADEC Builder::Error addTileResult       (const TileResult& tile);

    /*
     * \brief   Set number of threads to use internally during LOD generation.
     *
     *          The default thread count is 1 (no threads launched).
     * \note    This function may change or be removed in the future!
     */
    UMBRADEC void           setNumThreadsExt    (int numThreads);

    /*
     * \brief   Set path for object optimization cache.
     *
     *          You are required to set this for using static object or shadow
     *          optimizations, an error will be triggered otherwise.
     *
     * \note    This function may change or be removed in the future!
     */
    UMBRADEC void           setCachePathExt     (const char* path);

    UMBRADEC Builder::Error getTomeSize         (uint32_t& size) const;
    UMBRADEC const Tome*    getTome             (uint8_t* buf, uint32_t bufSize) const;
    UMBRADEC float          getProgress         (void) const;

    UMBRADEC void           visualizeState      (class DebugRenderer*) const;

private:
                            TomeGenerator   (const TomeGenerator&); // not allowed
    TomeGenerator&          operator=       (const TomeGenerator&); // not allowed

    class ImpTomeGenerator* m_imp;
    friend class Builder;
};

/*!
 * \brief   A set of TileInput objects
 *
 */
class TileInputSet
{
public:
    UMBRADEC                TileInputSet    (void);
    UMBRADEC                ~TileInputSet   (void);

    UMBRADEC int            size            (void) const;
    UMBRADEC Builder::Error get             (TileInput& tile, int idx) const;

private:
                            TileInputSet    (const TileInputSet&); // not allowed
    TileInputSet&           operator=       (const TileInputSet&); // not allowed

    class ImpTileInputSet* m_imp;
    friend class Builder;
};

} // namespace Umbra

#endif // UMBRABUILDER_HPP
