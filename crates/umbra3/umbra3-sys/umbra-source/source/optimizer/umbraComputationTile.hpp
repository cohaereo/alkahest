#pragma once

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Computation tile data containers
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraGeometryBlock.hpp"
#include "umbraCellGraph.hpp"
#include "umbraTileGrid.hpp"
#include "umbraBuildContext.hpp"
#include "umbraStats.hpp"
#include <optimizer/GraphicsContainer.hpp>
#include "umbraChecksum.hpp"

#define TILEINPUT_VERSION   36
#define TILERESULT_VERSION  TILEINPUT_VERSION

namespace Umbra
{

class ImpTileInput : public BuilderBase
{
public:

    ImpTileInput (BuildContext* ctx)
    :   BuilderBase(ctx),
        m_geometry(ctx->getPlatform().allocator),
        m_computationString(ctx->getPlatform().allocator),
        m_hash(ctx->getPlatform().allocator)
    {}

    const GeometryBlock& getGeometry (void) const { return m_geometry; }
    const CellGeneratorParams& getCellGeneratorParams (void) const { return m_cellGeneratorParams; }
    AABB getAABB (void) const;
    Vector3i getAABBMin() const { return m_aabbMin; }
    Vector3i getAABBMax() const { return m_aabbMax; }
    float getUnitSize() const { return m_unitSize; }
    const String& getComputationString() const { return m_computationString; }
    const char* getHash() const;

    bool serialize(OutputStream&) const;

    bool isEmpty() const
    {
        return m_geometry.isEmpty();
    }

    template<typename OP> void streamOp (OP& op)
    {
        UINT32 resultVersion = TILERESULT_VERSION;
        stream(op, resultVersion);
        op.require(resultVersion == TILERESULT_VERSION);
        stream(op, m_aabbMin);
        stream(op, m_aabbMax);
        stream(op, m_unitSize);
        stream(op, m_cellGeneratorParams);
        stream(op, m_geometry);
        stream(op, m_computationString);
    }

private:

    /* the axis aligned bounds of the tile to compute */
    Vector3i                m_aabbMin;
    Vector3i                m_aabbMax;
    float                   m_unitSize;

    /* computation parameters */
    CellGeneratorParams     m_cellGeneratorParams;

    /* The part of the scene occluder geometry available for this tile computation. */
    /* Possibly better to store objects and models instead? */
    GeometryBlock           m_geometry;

    /* Debugging string from TileGrid */
    String                  m_computationString;

    /* Hash value (not serialized) */
    mutable String          m_hash;

    friend class ImpTileInputSet;
};

class ImpTileInputSet : public BuilderBase
{
public:
    ImpTileInputSet (BuildContext* ctx):
      BuilderBase(ctx), m_grid(ctx->getPlatform()) { }

    bool get (ImpTileInput** tile, int idx);
    int size (void) const { return m_grid.getNumNodes(); }

    Builder::Error init (const Scene* scene, const ComputationParams& params, AABB filterAABB = AABB())
    {
        return m_grid.create(scene, params, filterAABB);
    }

private:
    TileGrid        m_grid;
};

class ImpTileResult : public BuilderBase
{
public:
    Vector3i          m_aabbMin;
    Vector3i          m_aabbMax;
    float             m_unitSize;
    float             m_featureSize;
    CellGraph         m_cellGraph;
    Stats             m_stats;
    Array<AABB>       m_viewVolume;
    String            m_computationString;
    GraphicsContainer m_graphics;

    ImpTileResult (BuildContext* ctx) :
        BuilderBase(ctx),
        m_cellGraph(ctx->getPlatform().allocator),
        m_stats(ctx->getPlatform().allocator),
        m_viewVolume(ctx->getPlatform().allocator),
        m_computationString(ctx->getPlatform().allocator),
        m_graphics(ctx->getMemory())
        {}

    template<typename OP> void streamOp (OP& op)
    {
        UINT32 resultVersion = TILERESULT_VERSION;
        stream(op, resultVersion);
        op.require(resultVersion == TILERESULT_VERSION);

        stream(op, m_aabbMin);
        stream(op, m_aabbMax);
        stream(op, m_unitSize);
        stream(op, m_featureSize);
        stream(op, m_cellGraph);
        stream(op, m_stats);
        stream(op, m_viewVolume);
        stream(op, m_computationString);
        stream(op, m_graphics);
    }
};

}
