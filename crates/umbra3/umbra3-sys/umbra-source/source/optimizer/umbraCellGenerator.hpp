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
 * \brief   Umbra cell generator
 *
 */

#include "umbraCellGraph.hpp"
#include "umbraGeometryBlock.hpp"
#include "umbraHash.hpp"
#include "umbraExtCellGraph.hpp"
#include "umbraSet.hpp"
#include "umbraRT.hpp"
#include "umbraObjectPool.hpp"
#include "umbraSubdivisionTree.hpp"

namespace Umbra
{

class RayTracer;
class GeometryBlock;
class DebugCollector;
class VisualizeHelper;

struct CellGeneratorParams
{
    CellGeneratorParams() : bfLimit(100.f), bfDistance(0.f), debugMask(0), visualizations(false), strictViewVolumes(false), accurateDilation(false) {}

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, aabb);
        stream(op, cellLevel);
        stream(op, smallestHoleLevel);
        stream(op, bfLimit);
        stream(op, bfDistance);
        stream(op, featureSize);
        stream(op, debugMask);
        stream(op, visualizations);
        stream(op, strictViewVolumes);
        stream(op, accurateDilation);
    }

    AABB            aabb;
    int             cellLevel;
    int             smallestHoleLevel;
    float           bfLimit;
    float           bfDistance;
    float           featureSize;
    unsigned int    debugMask;
    bool            visualizations;
    bool            strictViewVolumes;
    bool            accurateDilation;
};

class CellGenerator
{
public:
    CellGenerator(const PlatformServices& platform, CellGraph&, const GeometryBlock&,
            const CellGeneratorParams&, const RayTracer*, const AABB& topLevelAABB, DebugCollector& dc);
    ~CellGenerator();

    void perform();

    CellGraph&          getCellGraph(void)  { return m_cellGraph; }
    Allocator*          getAllocator(void)  { return m_platform.allocator; }

    struct GateTri
    {
        GateTri(void) : frontCellIdx(-1), backCellIdx(-1), planeIndex(-1) { }

        Array<Vector3>  poly;
        Set<int>        gateIDs;
        Vector3         ref;
        int             frontCellIdx;
        int             backCellIdx;
        int             planeIndex;

        void setAllocator (Allocator* heap)
        {
            poly.setAllocator(heap);
            gateIDs.setAllocator(heap);
        }
    };

private:

    struct PlaneTriangle
    {
        PlaneTriangle(void) {}
        PlaneTriangle(const Vector4& pleq, const Triangle& tri)
            : plane(pleq), tri(tri) {}

        Vector4 plane;
        Triangle tri;
    };

    // Steps.

    void            collectTriangles        (void);
    void            collectIntersectingTriangles (int idx, Array<PlaneTriangle>& out);
    void            split                   (const AABB& aabb, int tris, int n, int depth, VoxelConstructor& vc);
    void            floodFill               (const AABB& aabb, VoxelTraversal& vt, int depth, int fillDepth, int& idx);
    void            floodFill2              (VoxelTraversal& parent, VoxelTraversal& vt, int& idx);
    void            collectPortals          (const AABB& aabb, VoxelTraversal& vt);
    void            gateVoxelPortalExpansion(const AABB& aabb, VoxelTraversal& vt);
    void            makePortalsTwoWay       (void);
    void            mirrorGatePortals       (void);
    bool            rayTestUserPortals      (const Vector3& a, const Vector3&b, const GateTri& sourceTri, Set<int>& overlapGates, bool allowOverlap);
    void            assignTriangleToGateNodes(const GateTri& tri, int idx, const AABB& triAABB, const AABB& voxelAABB, VoxelTraversal& vt);
    bool            validateRefPoint        (const GateTri& tri, const AABB& aabb, VoxelTraversal& vt);
    void            findTriangleCells       (GateTri& tri, const AABB& refAABB, const AABB& aabb, VoxelTraversal& vt, float& bestFrontDistance, float& bestBackDistance, bool allowOverlap, Set<int>& frontOverlapGates, Set<int>& backOverlapGates);
    bool            intersectLineSegment    (const Vector3& a, const Vector3& b, const AABB& aabb, VoxelTraversal& vt);
    void            computeCellAABBs        (const AABB& aabb, VoxelTraversal& vt);
    void            collectTargets          (const AABB& aabb, VoxelTraversal& vt, int* tris, int n);
    void            collectVolumes          (const AABB& aabb);
    void            markVolume              (const AABB& aabb, VoxelTraversal& vt, const RayTracerTraversal& rt, int idx, BitVector& seenCells);
    void            setVolumeOutsideness    (const AABB& aabb, VoxelTraversal& vt, int* vols, int n2);
    void            testVolumeOutsideness   (const AABB& aabb, VoxelTraversal& vt, int* vols, int n);
    void            testVoxelBackface       (const AABB& aabb, VoxelTraversal& vt, int depth, int fillDepth);
    bool            testCellBackface        (const AABB& aabb, VoxelTraversal& vt, int vc);
    void            markForceReachables     (const AABB& aabb, VoxelTraversal& vt, int* vols, int n);
    void            voxelDilation           (VoxelTraversal& vt, const AABB& aabb, int depth);
    void            voxelDilationCell       (VoxelTraversal& vt, const AABB& aabb, UINT32 mask);
    void            findClosestInsideCell   (VoxelTraversal& vt, const AABB& aabb, const AABB& tgt, const VoxelTraversal& vtRef, int& best, int& bestDistance);
    void            voxelDilationInsideCell (VoxelTraversal& vt, const AABB& aabb);

    // Utils.

    bool            repFloodFill    (VoxelTraversal& vt);
    bool            floodFillGateEdgeConnectivity(VoxelTraversal& vt, int cellIdx);
    void            collectInsideAABBs(const AABB& aabb, VoxelTraversal& vt, int vc, Array<AABB>& aabbs);
    void            collectAABBs    (const AABB& aabb, VoxelTraversal& vt, int vc, Array<AABB>& aabbs);
    int             filterViewVolumes(const AABB& aabb, int* vols, int m);
    int             filterTriangles (const AABB& aabb, int* tris, int count);
    bool            clustersMatch   (VoxelTraversal& vt, int cluster);

    int             findInsideNeighbor (VoxelTraversal& vt, const AABB& aabb, const AABB& tgt, int exclude);

    bool            intersectsWithArea(const AABB& aabb, const GateTri& tri);

    SubdivisionTree::Node* buildViewTree(SubdivisionTree& st, VoxelTraversal& vt, int depth);
    SubdivisionTree::Node* buildMatchingTree(SubdivisionTree& st, VoxelTraversal& vt, int face);
    SubdivisionTree::Node* buildBSPTree(SubdivisionTree& st, const Array<int>& tris);
    SubdivisionTree::Node* buildBSPTree(SubdivisionTree& st, Set<int>& used, const Array<int>& tris, int cellIdx);
    SubdivisionTree::Node* buildTessellationBSPTree(SubdivisionTree& st, const Array<PlaneTriangle>& tris);

    void            tessellate      (const Array<Vector3>& input, Array<Array<Vector3> >& output, const SubdivisionTree::Node* node);

    void visualizeSolidVoxels(const AABB& aabb, VoxelTraversal& vt, VisualizeHelper& vh);
    void visualizeBackface(const AABB& aabb, VoxelTraversal& vt, VisualizeHelper& vh);

    // Flood-filler for empty voxels.

    template<int N = -1>
    struct EmptyFloodFiller
    {
        EmptyFloodFiller(VoxelTraversal& vt, int idx, int depth) : m_vcIdx(idx), m_depth(depth)
        {
            VoxelTree::LeafData& ld = vt.getLeafData();
            UMBRA_ASSERT(ld.getTmp() == 1);
            ld.setTmp(2);

            m_finished = true;
        }

        bool collect(VoxelTraversal& vt, int, int)
        {
            // \todo [Hannu] optimize: fill over if tmp == 1, are there problems with that?

            VoxelTree::LeafData& ld = vt.getLeafData();
            if (!ld.isEmpty() || ld.getTmp() > 0)
                return true;

            ld.setViewCellIndex(m_vcIdx);

            if (N < 0 || m_depth < N)
            {
                ld.setTmp(1);

                EmptyFloodFiller ff(vt, m_vcIdx, m_depth+1);
                NeighborFinder<EmptyFloodFiller>(ff).find(vt);
                if (!ff.m_finished)
                    m_finished = false;
            }
            else
            {
                // Recursion limit: mark that flood-filling should be continued here.

                ld.setTmp(1);
                m_finished = false;
            }

            return true;
        }

        void done(int) {}

        int  m_vcIdx;
        int  m_depth;
        bool m_finished;
    };

    template<int N = -1>
    struct BorderGateFloodFiller
    {
        BorderGateFloodFiller(VoxelTraversal& vt, UINT32 faceMask, int idx, int depth)
            : m_faceMask(faceMask), m_vcIdx(idx), m_depth(depth)
        {
            VoxelTree::LeafData& ld = vt.getLeafData();
            UMBRA_ASSERT(ld.getTmp() == 1);
            UMBRA_ASSERT(ld.getBorderGateData().getBorderCellIndex() == idx);
            UMBRA_ASSERT(faceMask != 0);
            ld.setTmp(2);

            m_finished = true;
        }

        bool collect(VoxelTraversal& vt, int, int)
        {
            // \todo [Hannu] optimize: fill over if tmp == 1, are there problems with that?

            VoxelTree::LeafData& ld = vt.getLeafData();

            if (!ld.isBorderGate() || ld.getTmp() > 0)
                return true;
            UINT32 otherMask = ld.getBorderGateData().getFaceMask();
            // no common faces?
            if ((otherMask & m_faceMask) == 0)
                return true;
            otherMask |= m_faceMask;
            // would contain faces in different directions?
            if ((otherMask & (otherMask >> 1) & 0x15) != 0)
                return true;
            m_faceMask = otherMask;
            ld.getBorderGateData().setBorderCellIndex(m_vcIdx);

            if (N < 0 || m_depth < N)
            {
                ld.setTmp(1);
                BorderGateFloodFiller ff(vt, m_faceMask, m_vcIdx, m_depth+1);
                NeighborFinder<BorderGateFloodFiller>(ff).find(vt);
                if (!ff.m_finished)
                    m_finished = false;
            }
            else
            {
                // Recursion limit: mark that flood-filling should be continued here.

                ld.setTmp(1);
                m_finished = false;
            }

            return true;
        }

        void done(int) {}

        int  m_faceMask;
        int  m_vcIdx;
        int  m_depth;
        bool m_finished;
    };

    struct FindCellPerFace
    {
        FindCellPerFace()
        {
            for (int i = 0; i < 6; i++)
                m_faces[i] = -1;
        }

        bool collect(VoxelTraversal& vt, int face, int size)
        {
            UMBRA_UNREF(size);
            UMBRA_ASSERT(size >= 0);

            VoxelTree::LeafData& ld = vt.getLeafData();
            if (ld.isEmpty() && ld.getTmp() >= 1)
            {
                UMBRA_ASSERT(m_faces[face] < 0);
                m_faces[face] = ld.getViewCellIndex();
            }

            return true;
        }

        void done(int) {}

        int m_faces[6];
    };

    struct SetCellForFace
    {
        SetCellForFace(int face, int cellIdx) : m_face(face), m_cellIdx(cellIdx)
        {
            m_found = false;
        }

        bool collect(VoxelTraversal& vt, int face, int)
        {
            if (face != m_face)
                return true;

            VoxelTree::LeafData& ld = vt.getLeafData();
            if (!ld.isEmpty() || ld.getTmp() > 0)
                return true;

            ld.setViewCellIndex(m_cellIdx);
            ld.setTmp(1);

            UMBRA_ASSERT(!m_found);
            m_found = true;

            return true;
        }

        void done(int) {}

        int m_face;
        int m_cellIdx;
        bool m_found;
    };

    struct PortalCollector
    {
        PortalCollector(CellGraph& cg, const AABB& aabb, VoxelTraversal& vt) : m_cellGraph(cg), m_aabb(aabb)
        {
            m_type = vt.getLeafData().getType();
            if (m_type == VoxelTree::BORDER_GATE)
                m_faceMask = vt.getLeafData().getBorderGateData().getFaceMask();
            m_viewCellIdx = vt.getLeafData().getViewCellIndex();
            UMBRA_ASSERT(m_viewCellIdx >= 0);
        }

        bool collect(VoxelTraversal& vt, int face, int size)
        {
            if (size < 0) // Only accept larger or equal AABB.
                return true;

            VoxelTree::LeafData& ld = vt.getLeafData();

            if (ld.getType() != m_type)
                return true;

            // only create portals between cells on the same face
            if ((m_type == VoxelTree::BORDER_GATE) &&
                (m_faceMask != ld.getBorderGateData().getFaceMask()))
                return true;

            int otherViewCellIdx = ld.getViewCellIndex();
            if (otherViewCellIdx == m_viewCellIdx)
                return true;

            UMBRA_ASSERT(otherViewCellIdx >= 0);

            // Match existing portal.

            face ^= 1;

            float z = m_aabb.getFaceDist(face);
            Vector4 rect = m_aabb.getFaceRect(face);

            CellGraph::Cell& c = m_cellGraph.getCell(m_viewCellIdx);
            for (int i = 0; i < c.getRectPortalCount(); i++)
            {
                CellGraph::RectPortal& p = c.getRectPortal(i);
                if (p.getFace() == face && p.getZ() == z && p.getTarget() == otherViewCellIdx)
                {
                    Vector4 r = p.getRect();
                    r.x = min2(r.x, rect.x);
                    r.y = min2(r.y, rect.y);
                    r.z = max2(r.z, rect.z);
                    r.w = max2(r.w, rect.w);
                    p.setRect(r);
                    return true;
                }
            }

            // Add new portal.

            CellGraph::RectPortal p;
            p.setFace(face);
            p.setTarget(otherViewCellIdx);
            p.setZ(z);
            p.setRect(rect);
            c.addRectPortal(p);
            return true;
        }

        void done(int) {}

        CellGraph&  m_cellGraph;
        AABB        m_aabb;
        int         m_viewCellIdx;
        VoxelTree::LeafType m_type;
        UINT32      m_faceMask;

    private:
        PortalCollector& operator= (const PortalCollector&) { return *this; }
    };

    struct CellFinder
    {
        CellFinder() : m_idx(-1)
        {
        }

        bool collect(VoxelTraversal& vt, int, int)
        {
            UMBRA_ASSERT(m_idx < 0);

            VoxelTree::LeafData& ld = vt.getLeafData();
            if (!ld.hasViewCell())
                return true;

            m_idx = ld.getViewCellIndex();
            return false;
        }

        void done(int) {}

        int  m_idx;
    };

    // Input.

    PlatformServices            m_platform;
    const GeometryBlock&        m_geometryBlock;
    AABB                        m_topLevelAABB;
    const RayTracer*            m_rayTracer;
    RayTracerTraversal          m_rayTracerTraversal;
    CellGeneratorParams         m_params;

    // Output.

    CellGraph&                  m_cellGraph;

    // Misc data.

    Array<float>                m_backFaceRatio;
    VoxelTree                   m_voxelTree;

    Array<Array<int> >          m_gateTrianglesPerVoxel;

    Array<UINT8>                m_octantStack;
    Array<int>                  m_triangleStack;

    Array<Vector4>              m_gatePlanes;
    Array<GateTri>              m_gateTriangles;
    Array<int>                  m_inputGateTriangles;
    Array<int>                  m_triangleToPlane;

    Vector3                     m_targetInflation;

    Hash<UINT32, int>           m_objMapping;

    // this is a pretty ugly hack for being able to remove non-reachable
    // border gate cells
    BitVector                   m_removedCells;

    static const int MAX_POLY_SIZE = 256;

    Vector3                     m_tempPolygon[MAX_POLY_SIZE];
    Vector3                     m_tempClipped[MAX_POLY_SIZE];

    float                       m_smallestHoleSize;

    DebugCollector&             m_dc;

    CellGenerator& operator=(const CellGenerator&) { return *this; } // deny

    friend struct PortalExpansion;
};

class TopCellGenerator
{
public:
    TopCellGenerator(const PlatformServices& platform, const GeometryBlock&, const CellGeneratorParams&, Vector3i idim, const RayTracer*, DebugCollector& dc);
    ~TopCellGenerator();

    void perform();

    CellGraph&          getCellGraph(void) { return m_cellGraph; }
    Allocator*          getAllocator(void) { return m_platform.allocator; }

private:
    int compute(CellGraph& cg, const AABB& aabb, Vector3i idim, Vector3i level, int* tris, int numTris);

private:
    PlatformServices            m_platform;
    const GeometryBlock&        m_geometryBlock;
    Vector3i                    m_idim;
    const RayTracer*            m_rayTracer;
    CellGeneratorParams         m_params;
    DebugCollector&             m_dc;

    CellGraph                   m_cellGraph;

    TopCellGenerator& operator= (const TopCellGenerator&) { return *this; }
};

static inline void copyHeap (CellGenerator::GateTri* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

} // namespace Umbra
