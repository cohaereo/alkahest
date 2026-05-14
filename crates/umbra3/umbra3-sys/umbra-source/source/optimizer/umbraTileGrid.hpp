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
 * \brief   Tile grid building
 *
 * TileGrid is responsible for implementing Builder::initialize(), i.e. taking
 * a Scene as input and generating a set of TileInput objects as output for
 * distributed computation.
 *
 * The operation roughly consists of:
 * 1. looping through the geometry, building object AABBs and validating input
 *    and figuring out the scene AABB (round up to next power of two!)
 * 2. splitting the scene in half along the longest current axis recursively,
 *    pushing the intersecting objects and view volumes downwards on the way.
 *    Note that for the occluder geometry intersection test the AABB needs to
 *    be inflated by the backface test distance.
 * 3. putting together TileInput objects for the KD-tree leaves when requested,
 *    by generating a GeometryBlock for the surroundings of the tile. Note that
 *    here we (probably) do a triangle-by-triangle intersection test against
 *    the inflated tile so not all objects that we have for the leaf necessarily
 *    make it to the final TileInput.
 */

#include "umbraVector.hpp"
#include "umbraAABB.hpp"
#include "umbraImpScene.hpp"
#include "umbraCellGenerator.hpp"
#include "umbraGeometryBlock.hpp"
#include "umbraTimer.hpp"
#include "umbraSIMD.hpp"
#include "optimizer/umbraBuilder.hpp"

namespace Umbra
{

class ComputationParams;

class TileGrid
{
public:

    TileGrid (const PlatformServices& platform);
    ~TileGrid ();

    Builder::Error  create          (const Scene* scene, const ComputationParams& params, const AABB& filterAABB); // empty by default
    int             getNumNodes     (void)                  const { return m_nodes.getSize(); }
    Vector3i        getIntMin       (int idx)               const { return m_nodes[idx].iMin; }
    Vector3i        getIntMax       (int idx)               const { return m_nodes[idx].iMax; }
    CellGeneratorParams getCellGeneratorParams(int idx) const { return m_nodes[idx].cgp; }
    float           getUnitSize     (void)                  const { return m_unitSize; }
    AABB            getFilterAABB   (void)                  const { return m_filterAABB; }
    void            fillBlock       (GeometryBlock& block, int idx) const;
    void            fillVolumes     (Array<ViewVolume>& dst, int idx) const;
    const String&   getComputationString() const { return m_computationString; }

    static bool calcGrid        (Vector3i& iMin, Vector3i& iMax, const AABB& bounds, float tileSize);

private:

    void reset (void);

    struct Node
    {
        Vector3i            iMin;
        Vector3i            iMax;
        AABB                targetAABB;
        AABB                occluderAABB;
        Array<int>          intersectingObjects;
        Array<int>          intersectingVolumes;
        CellGeneratorParams cgp;

        Node& operator=(const Node& b)
        {
            // I explicitly want them from the same heap
            intersectingObjects.setAllocator(b.intersectingObjects.getAllocator());
            intersectingVolumes.setAllocator(b.intersectingVolumes.getAllocator());

            iMin                = b.iMin;
            iMax                = b.iMax;
            targetAABB          = b.targetAABB;
            occluderAABB        = b.occluderAABB;
            intersectingObjects = b.intersectingObjects;
            intersectingVolumes = b.intersectingVolumes;
            cgp                 = b.cgp;

            return *this;
        }
    };

    class SIMDAABB
    {
    public:
        SIMDRegister m_mn;
        SIMDRegister m_mx;

        SIMDAABB (void) {}
        SIMDAABB (const SIMDRegister& mn, const SIMDRegister& mx)
        {
            m_mn = mn;
            m_mx = mx;
        }

        SIMDAABB (const AABB& aabb)
        {
            set(aabb);
        }

        void set (const AABB& aabb)
        {
            Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, mn) = Vector4(aabb.getMin(), 0.f);
            Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, mx) = Vector4(aabb.getMax(), 0.f);
            m_mn = SIMDLoadAligned(&mn.x);
            m_mx = SIMDLoadAligned(&mx.x);
        }

        AABB get (void) const
        {
            Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, mn);
            Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, mx);

            SIMDStoreAligned(m_mn, &mn.x);
            SIMDStoreAligned(m_mx, &mx.x);
            return AABB(mn.xyz(), mx.xyz());
        }

        float getDistance (const SIMDAABB& aabb) const
        {
            // \todo real SIMD implementation
            return get().getDistance(aabb.get());
        }
        
        void grow (const SIMDAABB& aabb)
        {
            AABB aabb2 = get();
            aabb2.grow(aabb.get());
            set(aabb2);
        }

        SIMDAABB inflated (const SIMDRegister& v) const
        {
            SIMDAABB aabb(SIMDSub(m_mn, v), SIMDAdd(m_mx, v));
            return aabb;
        }

        bool intersects (const SIMDAABB& b) const
        {
            SIMDRegister ca = SIMDCompareGE(b.m_mx, m_mn);  // ca = m_mn <= b.m_mx
            SIMDRegister cb = SIMDCompareGE(m_mx, b.m_mn);  // cb = m_mx >= b.m_mn
            SIMDRegister r  = SIMDBitwiseAnd(ca, cb);   // r  = ca & cb

            int m = SIMDExtractSignBits(r);             // one bit per component
            bool intersects = !(~m & 7);                // intersect if r.xyz all true, ignore w

            UMBRA_ASSERT(intersects == get().intersects(b.get()));
            return intersects;
        }

        bool contains (const SIMDAABB& b) const
        {
            SIMDRegister cmn = SIMDCompareGE(b.m_mn, m_mn);     // cmn = a.min <= b.min
            SIMDRegister cmx = SIMDCompareGE(m_mx, b.m_mx);     // cmx = a.max >= b.max
            SIMDRegister r   = SIMDBitwiseAnd(cmn, cmx);    // r = cmn && cmx

            int m = SIMDExtractSignBits(r);
            bool contains = !(~m & 7);                      // contains if r.xyz all true, ignore w

            UMBRA_ASSERT(contains == get().contains(b.get()));
            return contains;
        }
    };

    // Object
    class Obj
    {
    public:
        SIMDAABB aabb;
        UINT32   flags;
        int      objSceneIdx;
    };

    class Volume
    {
    public:
        ~Volume () {}

        SIMDAABB    aabb;

        UINT32      name;
        int         sceneVolIdx;
        int         cellLevel;
        int         smallestHoleLevel;

        float       backfaceLimit;
        float       smallestOccluder;
        float       featureSize;
        UINT32      padding[1];
    };

    struct TriangleList
    {
        int      objSceneIdx;
        UINT32   flags;
        int*     tris;
        int      n;
        SIMDAABB aabb;

        bool operator<(const TriangleList& tl) const
        {
            if ((flags & SceneObject::OCCLUDER) == (tl.flags & SceneObject::OCCLUDER))
                return objSceneIdx < tl.objSceneIdx;
            return (flags & SceneObject::OCCLUDER) > (tl.flags & SceneObject::OCCLUDER);
        }

        bool operator>(const TriangleList& tl) const
        {
            return !(*this < tl);
        }
    };

    struct SplitState
    {
        SplitState(Allocator* a) : triangles(a), newTriangles(a), hasOccluders(false) {}

        const Obj**         objs;
        int                 numObjs;
        const Obj**         volObjs;
        int                 numVolObjs;
        const Volume**      viewVolumes;
        int                 numViewVolumes;
        Array<TriangleList> triangles;
        Array<Array<int> >  newTriangles;
        bool                hasOccluders;

        bool isEmpty() const
        {
            return numObjs == 0 && numVolObjs == 0 && numViewVolumes == 0 && triangles.getSize() == 0;
        }

        bool hasGeometry() const
        {
            return numObjs > 0 || triangles.getSize() > 0;
        }

        bool hasViewVolumes() const
        {
            return numViewVolumes > 0;
        }
    };

    TileGrid& operator= (const TileGrid&);  // disallowed

    int partitionTriangles  (const Triangle** triangles, int numTriangles,
                             const BitVector& intersectingObjs,
                             const SIMDAABB& aabb, const SIMDAABB& backfaceAABB,
                             int& numOccluders, int& numTargets);

    // \todo make template general enough to be able to partition triangles too
    template<class T> int partitionAABB (const T** arr, int num, const SIMDAABB& aabb)
    {
        int i, j;
        for (i = 0, j = 0; i < num; i++)
        {
            const T* obj = arr[i];
            if (obj->aabb.intersects(aabb))
            {
                if (i != j)
                    swap2(arr[i], arr[j]);
                j++;
            }
        }
        return j;
    }

    Builder::Error calcGridNodes     (void);
    void           filterSplitState  (SplitState& out, const SplitState& in, Vector3i mn, Vector3i mx);
    Builder::Error calcGridNodesRec  (const SplitState& ss,
                                      Vector3i           iMin,
                                      Vector3i           iMax);

    int                     calcSplitBias       (const SIMDAABB& aabb);
    inline void             calcNodeAABBs       (const Vector3i& mn, 
                                                 const Vector3i& mx, 
                                                 SIMDAABB& nodeAABB, 
                                                 SIMDAABB& inflatedAABB, 
                                                 SIMDAABB& backfaceAABB);

    PlatformServices        m_platform;
    Timer                   m_timer;
    Vector3i                m_iMin;
    Vector3i                m_iMax;
    SIMDRegister            m_bfDistance;
    float                   m_tileSize;
    float                   m_unitSize;
    int                     m_unitsPerTile;
    float                   m_featureSize;
    float                   m_smallestOccluder;
    SIMDRegister            m_tileInflation;
    int                     m_cellSplits;
    int                     m_smallestHoleSplits;
    float                   m_bfLimit;
    const Scene*            m_scene;
    Array<Node>             m_nodes;
    int                     m_numViewVolumes;
    Array<Volume>           m_viewVolumes;
    Array<Array<Vector3> >  m_transformedVertices;
    int                     m_numEmptyTiles;
    bool                    m_compVisualizations;
    bool                    m_strictViewVolumes;
    bool                    m_compAccurateDilation;	
    bool                    m_hasFilterAABB;
    AABB                    m_filterAABB;
    BitVector               m_objVec;
    String                  m_computationString;
};

} // namespace Umbra

