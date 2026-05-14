#if !defined(UMBRA_EXCLUDE_COMPUTATION)

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2007-2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Cell generator
 *
 */

#include "umbraCellGenerator.hpp"
#include "umbraGeometryBlock.hpp"
#include "umbraSort.hpp"
#include "umbraComputationTile.hpp"
#include "umbraWeightedSampler.hpp"
#include "umbraRandom.hpp"
#include "umbraRT.hpp"
#include "umbraIntersectExact.hpp"
#include "umbraSet.hpp"
#include "umbraUnionFind.hpp"
#include "umbraLogger.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraClipPolygon.hpp"
#include <standard/Sampling.hpp>
#include <standard/MigrateFromCommon.hpp>
#include <optimizer/DebugCollector.hpp>
#include <optimizer/VisualizeHelper.hpp>

#include <stdio.h>

#define LOGE(...) UMBRA_LOG_E(m_platform.logger, __VA_ARGS__)
#define LOGI(...) UMBRA_LOG_I(m_platform.logger, __VA_ARGS__)
#define LOGW(...) UMBRA_LOG_W(m_platform.logger, __VA_ARGS__)
#define LOGD(...) UMBRA_LOG_D(m_platform.logger, __VA_ARGS__)

#define GATE_PLANE_EPSILON       0.001f
#define PLANE_DEGENERATE_EPSILON 1.e-20f

using namespace Umbra;

namespace Umbra {

/* \todo [antti 14.2.2013]: move these to umbraGeometry */

bool onSamePlane(const Vector4& pleq, const Triangle& tri)
{
    if (dot(pleq, tri.a) != 0.f)
        return false;
    if (dot(pleq, tri.b) != 0.f)
        return false;
    if (dot(pleq, tri.c) != 0.f)
        return false;
    return true;
}

float distanceToPlane(const Vector4& pleq, const Triangle& tri)
{
    float d0 = dot(pleq, tri.a);
    float d1 = dot(pleq, tri.b);
    float d2 = dot(pleq, tri.c);

    if ((d0 * d1 < 0.f) || (d0 * d2 < 0.f))
        return 0.f;

    float ret = Math::fabs(d0);
    ret = min2(ret, Math::fabs(d1));
    ret = min2(ret, Math::fabs(d2));
    return ret;
}

}

CellGenerator::CellGenerator(
    const PlatformServices& platform,
    CellGraph& output,
    const GeometryBlock& gb,
    const CellGeneratorParams& params,
    const RayTracer* rt,
    const AABB& topLevelAABB,
    DebugCollector& dc)
:   m_platform(platform),
    m_geometryBlock(gb),
    m_topLevelAABB(topLevelAABB),
    m_rayTracer(rt),
    m_rayTracerTraversal(*rt),
    m_params(params),
    m_cellGraph(output),
    m_backFaceRatio(platform.allocator),
    m_voxelTree(platform.allocator),
    m_gateTrianglesPerVoxel(platform.allocator),
    m_octantStack(platform.allocator),
    m_triangleStack(platform.allocator),
    m_gatePlanes(platform.allocator),
    m_gateTriangles(platform.allocator),
    m_inputGateTriangles(platform.allocator),
    m_triangleToPlane(platform.allocator),
    m_objMapping(platform.allocator),
    m_removedCells(0, platform.allocator),
    m_dc(dc)
{
    // \todo [Hannu] assert that ray tracer's aabb is within bfDistance
    // \todo [Hannu] assert that targets are within smallest hole (target inflation)
    UMBRA_ASSERT(gb.getOccluderAABB().contains(params.aabb));
    UMBRA_ASSERT(gb.getTargetAABB().contains(params.aabb));
    UMBRA_ASSERT(params.aabb.isOK());
    m_targetInflation = m_params.aabb.getDimensions() / float(1 << m_params.smallestHoleLevel) * 1.01f;
    m_smallestHoleSize = m_params.aabb.getMaxAxisLength() / float(1 << m_params.smallestHoleLevel) * 1.0f;
}

CellGenerator::~CellGenerator()
{
}

bool CellGenerator::intersectsWithArea(const AABB& aabb, const GateTri& tri)
{
    Vector4 pleqs[6];
    aabb.getPlaneEquations(pleqs);
    int N = tri.poly.getSize();

    UMBRA_ASSERT(tri.poly.getSize() < MAX_POLY_SIZE);
    if (tri.poly.getSize() >= MAX_POLY_SIZE)
        return true;

    AABB pAABB;

    for (int i = 0; i < tri.poly.getSize(); i++)
    {
        m_tempPolygon[i] = tri.poly[i];
        pAABB.grow(tri.poly[i]);
    }

    if (!aabb.intersectsWithArea(pAABB))
        return false;

    for (int i = 0; i < 6; i++)
    {
        if (i & 1)
            N = clipPolygonPlane(m_tempPolygon, m_tempClipped, pleqs[i], N);
        else
            N = clipPolygonPlane(m_tempClipped, m_tempPolygon, pleqs[i], N);
    }

    return N >= 3;
}

void CellGenerator::tessellate(const Array<Vector3>& input, Array<Array<Vector3> >& output, const SubdivisionTree::Node* node)
{
    if (node->isLeaf())
    {
        output.pushBack(input);
        return;
    }

    const SubdivisionTree::PlaneNode* plane = node->getPlane();
    const Vector4& pleq = plane->getPleq();

    // Use epsilon here?
    if (onSamePlane(pleq, Triangle(input[0], input[1], input[2])))
    {
        output.pushBack(input);
        return;
    }

    // Front of plane

    Array<Vector3> frontOut(input.getSize() * 2, getAllocator());
    int frontN = clipPolygonPlane(&frontOut[0], &input[0], pleq, input.getSize());
    UMBRA_ASSERT(frontN <= input.getSize() * 2);
    if (frontN >= 3)
    {
        frontOut.resize(frontN);
        tessellate(frontOut, output, plane->getRight());
    }

    // back of plane

    Array<Vector3> backOut(input.getSize() * 2, getAllocator());
    int backN = clipPolygonPlane(&backOut[0], &input[0], -pleq, input.getSize());
    UMBRA_ASSERT(backN <= input.getSize() * 2);
    if (backN >= 3)
    {
        backOut.resize(backN);
        if (frontOut != backOut)
            tessellate(backOut, output, plane->getLeft());
    }
}

void CellGenerator::collectIntersectingTriangles (int inputGateIdx, Array<PlaneTriangle>& out)
{
    int triIdx = m_inputGateTriangles[inputGateIdx];
    AABB triAABB = m_geometryBlock.getTriangleAABB(triIdx);
    UINT32 gateId = m_geometryBlock.getTriangleObject(triIdx).getId();
    Vector4 plane = m_gatePlanes[m_triangleToPlane[inputGateIdx]];
    Triangle tri;
    m_geometryBlock.getVertices(triIdx, tri.a, tri.b, tri.c);

    Vector3 range = m_targetInflation * 2;

    for (int i = 0; i < m_geometryBlock.getTriangleCount(); i++)
    {
        const ObjectParams& p = m_geometryBlock.getTriangleObject(i);
        if (p.isGate())
            continue;
        if (!p.isOccluder())
            continue;
        if (p.getId() == gateId)
            continue;

        // grow AABB in order to tesselate with triangles that are closer than voxel
        AABB otherAABB = m_geometryBlock.getTriangleAABB(i);
        otherAABB.inflate(range);

        if (!triAABB.intersects(otherAABB))
            continue;

        Triangle otherTri;
        m_geometryBlock.getVertices(i, otherTri.a, otherTri.b, otherTri.c);

        if (distanceToPlane(plane, otherTri) > range[0])
            continue;
        Vector4 otherPlane = getPlaneEquation(otherTri.a, otherTri.b, otherTri.c);
        if (otherPlane.xyz().lengthSqr() < PLANE_DEGENERATE_EPSILON)
            continue;
        if (!intersect(otherPlane, tri))
            continue;
        out.pushBack(PlaneTriangle(otherPlane, otherTri));
    }

    for (int i = 0; i < m_inputGateTriangles.getSize(); i++)
    {
        int otherIdx = m_inputGateTriangles[i];
        const ObjectParams& p = m_geometryBlock.getTriangleObject(otherIdx);
        UMBRA_ASSERT(p.isGate());
        if (p.getId() == gateId)
            continue;

        // grow AABB in order to tesselate with triangles that are closer than voxel
        AABB otherAABB = m_geometryBlock.getTriangleAABB(otherIdx);
        otherAABB.inflate(range);

        if (!triAABB.intersects(otherAABB))
            continue;

        Triangle otherTri;
        m_geometryBlock.getVertices(otherIdx, otherTri.a, otherTri.b, otherTri.c);

        if (distanceToPlane(plane, otherTri) > range[0])
            continue;
        Vector4 otherPlane = m_gatePlanes[m_triangleToPlane[i]];
        if (otherPlane.xyz().lengthSqr() < PLANE_DEGENERATE_EPSILON)
            continue;
        if (!intersect(otherPlane, tri))
            continue;
        out.pushBack(PlaneTriangle(otherPlane, otherTri));
    }
}

// Collect voxelization input into m_triangleStack.
// Collect gate triangles into m_gateTriangles and gate planes into m_gatePlanes.

void CellGenerator::collectTriangles(void)
{
    m_triangleStack.clear();
    m_inputGateTriangles.clear();

    for (int i = 0; i < m_geometryBlock.getTriangleCount(); i++)
    {
        if (!m_geometryBlock.getTriangleObject(i).isOccluder() &&
            !m_geometryBlock.getTriangleObject(i).isGate())
            continue;

        Triangle tri;
        m_geometryBlock.getVertices(i, tri.a, tri.b, tri.c);

        // only triangles intersecting the computation aabb are interesting here

        if (!intersectAABBTriangle(m_params.aabb, tri.a, tri.b, tri.c))
            continue;

        if (m_geometryBlock.getTriangleObject(i).isGate())
        {
            // skip gate triangles exactly on planes of negative facing faces of tile AABB
            // in order to avoid adding the same gate to both tiles sharing the face

            bool onFace = false;
            for (int axis = 0; axis < 3; axis++)
            {
                if (onSamePlane(m_params.aabb.getPlaneEq(axis << 1), tri))
                {
                    onFace = true;
                    break;
                }
            }
            if (onFace)
                continue;

            m_inputGateTriangles.pushBack(i);
        }

        // add to voxelization stack (both occluder and gate tris)
        m_triangleStack.pushBack(i);
    }

    //
    // Find planes for triangles
    //

    m_gatePlanes.clear();
    m_triangleToPlane.reset(m_inputGateTriangles.getSize());

    for (int i = 0; i < m_inputGateTriangles.getSize(); i++)
    {
        int triIdx = m_inputGateTriangles[i];
        const GeometryBlock::Triangle& inputTri = m_geometryBlock.getTriangle(triIdx);
        UINT32 gateId = m_geometryBlock.getObject(inputTri.m_objectIdx).getId();

        int planeIdx = -1;

        for (int j = 0; j < i; j++)
        {
            const GeometryBlock::Triangle& otherTri = m_geometryBlock.getTriangle(m_inputGateTriangles[j]);

            // Belongs to same gate object?
            if (m_geometryBlock.getObject(otherTri.m_objectIdx).getId() != gateId)
                continue;

            // Is connected -- i.e. shares two vertices?
            int commonVerts = 0;
            int uniqueVtx = 0;
            for (int k = 0; k < 3; k++)
            for (int l = 0; l < 3; l++)
            {
                if (inputTri.m_vertices[k] == otherTri.m_vertices[l])
                {
                    commonVerts++;
                    if (uniqueVtx == k)
                        uniqueVtx++;
                    continue;
                }
            }

            if (commonVerts < 2)
                continue;
            if (commonVerts == 3)
            {
                planeIdx = m_triangleToPlane[j];
                break;
            }

            // Is (roughly) on same plane?

            Vector3 v = m_geometryBlock.getVertex(inputTri.m_vertices[uniqueVtx]);
            if (Math::fabs(dot(m_gatePlanes[m_triangleToPlane[j]], v)) < GATE_PLANE_EPSILON)
            {
                planeIdx = m_triangleToPlane[j];
                break;
            }
        }

        if (planeIdx == -1)
        {
            Vector4 plane = getNormalizedPlaneEquation(
                    m_geometryBlock.getVertex(inputTri.m_vertices.i),
                    m_geometryBlock.getVertex(inputTri.m_vertices.j),
                    m_geometryBlock.getVertex(inputTri.m_vertices.k));

            planeIdx = m_gatePlanes.getSize();
            m_gatePlanes.pushBack(plane);
        }

        m_triangleToPlane[i] = planeIdx;
    }

    //
    // Tessellate the hell out of the gate triangles
    //

    m_gateTriangles.clear();
    Array<Array<Vector3> > polygons[2];
    polygons[0].setAllocator(getAllocator());
    polygons[1].setAllocator(getAllocator());
    int activeSet = 0;

    for (int i = 0; i < m_inputGateTriangles.getSize(); i++)
    {
        int triIdx = m_inputGateTriangles[i];
        const GeometryBlock::Triangle& inputTri = m_geometryBlock.getTriangle(triIdx);
        UINT32 gateId = m_geometryBlock.getObject(inputTri.m_objectIdx).getId();

        Array<Vector3> inputVerts(3, getAllocator());
        AABB triAABB;
        for (int v = 0; v < 3; v++)
        {
            inputVerts[v] = m_geometryBlock.getVertex(inputTri.m_vertices[v]);
            triAABB.grow(inputVerts[v]);
        }
        polygons[activeSet].clear();
        polygons[activeSet].pushBack(inputVerts);

        // Split with cell planes

        for (int axis = 0; axis < 3; axis++)
        {
            int splitLevel = m_params.cellLevel;
            float tileSize = m_params.aabb.getAxisLength(axis);
            float cellSize = tileSize / (float)(1 << splitLevel);
            int range = (1 << splitLevel);
            float pos = m_params.aabb.getMin()[axis];

            for (int t = 0; t <= range; t++, pos += cellSize)
            {
                Vector4 pleq;
                pleq.set(0, 0, 0, pos);
                pleq[axis] = -1.f;

                if (pleq.w < triAABB.getMin()[axis] || pleq.w > triAABB.getMax()[axis])
                    continue;

                polygons[activeSet ^ 1].clear();

                for (int k = 0; k < polygons[activeSet].getSize(); k++)
                {
                    const Array<Vector3>& input = polygons[activeSet][k];

                    Array<Vector3> frontOut(input.getSize() * 2, getAllocator());

                    // drop triangles outside of computation bounds

                    if (t != 0)
                    {
                        int frontN = clipPolygonPlane(&frontOut[0], &input[0], pleq, input.getSize());
                        if (frontN >= 3)
                        {
                            frontOut.resize(frontN);
                            polygons[activeSet ^ 1].pushBack(frontOut);
                        }
                    }

                    if (t != range)
                    {
                        Array<Vector3> backOut(input.getSize() * 2, getAllocator());
                        int backN = clipPolygonPlane(&backOut[0], &input[0], -pleq, input.getSize());
                        if (backN >= 3)
                        {
                            backOut.resize(backN);
                            if (frontOut != backOut)
                                polygons[activeSet ^ 1].pushBack(backOut);
                        }
                    }
                }

                activeSet ^= 1;
            }
        }

        // Split with intersecting triangles
        // \todo optimize! this is O(N^2)
        // \todo not robust when triangle is almost parallel to splitting plane
        // \todo todo split when close to triangle plane, project edges on plane to generate clipping planes from them

        Array<PlaneTriangle> intersectingTriangles(getAllocator());
        collectIntersectingTriangles(i, intersectingTriangles);

        SubdivisionTree st(getAllocator());
        st.setRoot(buildTessellationBSPTree(st, intersectingTriangles));

        if (st.getRoot())
        {
            polygons[activeSet ^ 1].clear();

            for (int j = 0; j < polygons[activeSet].getSize(); j++)
                tessellate(polygons[activeSet][j], polygons[activeSet ^ 1], st.getRoot());

            activeSet ^= 1;
        }

        // Output tessellated gate triangles

        for (int j = 0; j < polygons[activeSet].getSize(); j++)
        {
            GateTri out;
            out.setAllocator(getAllocator());
            out.gateIDs.insert(gateId);
            out.planeIndex = m_triangleToPlane[i];
            out.poly = polygons[activeSet][j];

            UMBRA_ASSERT(intersectsWithArea(m_params.aabb, out));
            m_gateTriangles.pushBack(out);
        }
    }
}

void CellGenerator::perform()
{
    m_cellGraph.setAABB(m_params.aabb);

    m_cellGraph.setPortalExpand(m_params.aabb.getMinAxisLength() / float(1 << m_params.smallestHoleLevel));

    // Collect triangles

    collectTriangles();

    // Split.

    VoxelConstructor vc(m_voxelTree);
    split(m_params.aabb, 0, m_triangleStack.getSize(), 0, vc);

    if (m_dc.pushActive("solid_voxels"))
    {
        VoxelTraversal vt(m_voxelTree);
        {
            VisualizeHelper vh(m_dc);
            visualizeSolidVoxels(m_params.aabb, vt, vh);
        }
        m_dc.popActive();
    }

    // Set voxels outside according to view volumes.

    if (m_params.strictViewVolumes)
    {
        Array<int> volumes(m_geometryBlock.getViewVolumeCount(), getAllocator());

        for (int i = 0; i < m_geometryBlock.getViewVolumeCount(); i++)
            volumes[i] = i;

        VoxelTraversal vt(m_voxelTree);
        setVolumeOutsideness(m_params.aabb, vt, volumes.getPtr(), volumes.getSize());
    }

    // Set voxels outside by doing backface testing.

    if (m_rayTracer)
    {
        int backfaceLevel = m_params.cellLevel + 3;
        backfaceLevel = min2(backfaceLevel, m_params.smallestHoleLevel-2);
        backfaceLevel = max2(backfaceLevel, m_params.cellLevel);

        VoxelTraversal vt(m_voxelTree);
        testVoxelBackface(m_params.aabb, vt, 0, backfaceLevel);
    }

    // Flood-fill cells.

    {
        VoxelIterator iter(m_voxelTree);
        while (iter.next())
            iter.get().setTmp(0);

        int cells = 0;
        VoxelTraversal vt(m_voxelTree);
        floodFill(m_params.aabb, vt, 0, m_params.cellLevel, cells);

        if (cells > 0)
            m_cellGraph.addCell(cells);
    }

    // Compute cell AABBs.

    {
        VoxelTraversal vt(m_voxelTree);
        computeCellAABBs(m_params.aabb, vt);
    }

    // Find front and back cells for gate triangles

    for (int t = 0; t < m_gateTriangles.getSize(); t++)
    {
        GateTri tri = m_gateTriangles[t];
        AABB triAABB;
        Vector3 triCenter;

        Random random;
        VoxelTraversal vt(m_voxelTree);

        triCenter = Vector3(0,0,0);
        for (int i = 0; i < tri.poly.getSize(); i++)
        {
            triCenter += tri.poly[i];
            triAABB.grow(tri.poly[i]);
        }
        triCenter /= (float)tri.poly.getSize();
        tri.ref = triCenter;
        int bestFrontCell = -1;
        int bestBackCell = -1;
        int numTries = 2;
        float v = 4.f * m_params.aabb.getMaxAxisLength() / float(1 << m_params.smallestHoleLevel);
        bool allowOverlap = false;
        Set<int> frontOverlapGates(getAllocator());
        Set<int> backOverlapGates(getAllocator());

        while (numTries--)
        {
            if (m_params.aabb.contains(tri.ref) && validateRefPoint(tri, m_params.aabb, vt))
            {
                float bestFrontDistance = v;
                float bestBackDistance = v;
                tri.frontCellIdx = -1;
                tri.backCellIdx = -1;
                frontOverlapGates.removeAll();
                backOverlapGates.removeAll();
                // \todo this is not very safe! it would be better to traverse towards the ref point and find closest node
                findTriangleCells(tri, triAABB, m_params.aabb, vt, bestFrontDistance, bestBackDistance, allowOverlap, frontOverlapGates, backOverlapGates);
                if ((tri.frontCellIdx != -1) && (tri.backCellIdx != -1))
                {
                    bestFrontCell = tri.frontCellIdx;
                    bestBackCell = tri.backCellIdx;
                }
            }

            if (bestFrontCell != -1 && bestBackCell != -1)
                break;

            allowOverlap = true;
        }

        if (bestFrontCell == bestBackCell)
            continue;

        assignTriangleToGateNodes(tri, t, triAABB, m_params.aabb, vt);

        m_gateTriangles[t].frontCellIdx = bestFrontCell;
        m_gateTriangles[t].backCellIdx = bestBackCell;
        m_gateTriangles[t].gateIDs |= frontOverlapGates;
        m_gateTriangles[t].gateIDs |= backOverlapGates;

        UMBRA_ASSERT(m_gateTriangles[t].gateIDs.getSize() > 0);
    }

    // Initialize border gate cells "removed". Border gate cells that are
    // connected to the graph are cleared from this vector in the code below.
    // Removed cells are treated as solid in the matching tree and will always
    // remain "outside" so that they don't end up in the view tree and will
    // eventually be stripped by reachability.

    {
        m_removedCells.resize(m_cellGraph.getCellCount(), true, false);
        VoxelIterator iter(m_voxelTree);
        while (iter.next())
        {
            if (iter.get().isBorderGate())
                m_removedCells.set(iter.get().getBorderGateData().getBorderCellIndex());
        }
    }

    // Compute portals and gates.

    {
        VoxelTraversal vt(m_voxelTree);
        collectPortals(m_params.aabb, vt);
    }

    // Create gate portals

    for (int t = 0; t < m_gateTriangles.getSize(); t++)
    {
        GateTri& tri = m_gateTriangles[t];
        if (tri.frontCellIdx < 0 || tri.backCellIdx < 0 || tri.frontCellIdx == tri.backCellIdx)
            continue;

        int cell1 = min2(tri.frontCellIdx, tri.backCellIdx);
        int cell2 = max2(tri.frontCellIdx, tri.backCellIdx);

        CellGraph::Cell& c = m_cellGraph.getCell(cell1);
        CellGraph::GatePortal* existingPortal = NULL;

        for (int i = 0; i < c.getGatePortalCount(); i++)
        {
            CellGraph::GatePortal* p = &c.getGatePortal(i);
            if (p->isGate() &&
                (p->getGateIDs() == tri.gateIDs) &&
                (p->getTarget() == cell2))
            {
                existingPortal = p;
                break;
            }
        }

        if (!existingPortal)
        {
            // Add new portal.

            CellGraph::GatePortal newPortal;
            newPortal.setAllocator(getAllocator());
            newPortal.setGates(tri.gateIDs);
            newPortal.setTarget(cell2);
            c.addGatePortal(newPortal);
            existingPortal = &c.getGatePortal(c.getGatePortalCount()-1);
            m_removedCells.clear(cell1);
            m_removedCells.clear(cell2);
        }

        existingPortal->addHullVertices(m_gatePlanes[tri.planeIndex], tri.poly);
    }

    m_cellGraph.simplifyPortalHulls();
    mirrorGatePortals();
    makePortalsTwoWay();

    // Expand portals by neighboring gate voxels

    if (m_gateTriangles.getSize())
    {
        VoxelTraversal vt(m_voxelTree);
        gateVoxelPortalExpansion(m_params.aabb, vt);
        makePortalsTwoWay();
    }

    // Collect targets.

    {
        Array<int> triangles(getAllocator());

        for (int i = 0; i < m_geometryBlock.getTriangleCount(); i++)
        {
            if (m_geometryBlock.getTriangleObject(i).isTarget())
                triangles.pushBack(i);
        }

        const int N = 1024; // chunked to keep vertex data in cache
        for (int j = 0; j < triangles.getSize(); j += N)
        {
            VoxelTraversal vt(m_voxelTree);
            collectTargets(m_params.aabb, vt, triangles.getPtr()+j, min2(N, triangles.getSize()-j));
        }

        collectVolumes(m_params.aabb);
    }

    // Compute face matching tree.

    for (int i = 0; i < 6; i++)
    {
        SubdivisionTree st(getAllocator());

        VoxelTraversal vt(m_voxelTree);
        st.setRoot(buildMatchingTree(st, vt, i));
        AABB aabb2 = m_params.aabb;
        aabb2.flattenToFace(i);
        st.setAABB(aabb2);

        SubdivisionTreeUtils stu(st);

        st.setRoot(stu.collapse(st.getRoot(), false));

        m_cellGraph.getMatchingTree(i).serialize(st);
    }

    // Compute initial insides with view volumes.
    if (m_params.strictViewVolumes)
    {
        // Default all cells to outside
        int numCells = m_cellGraph.getCellCount();
        for (int i = 0; i < numCells; i++)
            m_cellGraph.getCell(i).setOutside(true);

        Array<int> volumes(m_geometryBlock.getViewVolumeCount(), getAllocator());

        for (int i = 0; i < m_geometryBlock.getViewVolumeCount(); i++)
            volumes[i] = i;

        VoxelTraversal vt(m_voxelTree);
        testVolumeOutsideness(m_params.aabb, vt, volumes.getPtr(), volumes.getSize());
    }

    // Filter out some insides with backface testing.

    if (m_rayTracer)
    {
        m_backFaceRatio.resize(m_cellGraph.getCellCount());
        VoxelTraversal vt(m_voxelTree);

        for (int i = 0; i < m_cellGraph.getCellCount(); i++)
        {
            if (!m_cellGraph.getCell(i).isOutside() && testCellBackface(m_params.aabb, vt, i))
            {
                m_cellGraph.getCell(i).setOutside(true);
            }
        }
    }

    // Set single insides as outsides.

    for (int i = 0; i < m_cellGraph.getCellCount(); i++)
    {
        CellGraph::Cell& c = m_cellGraph.getCell(i);
        if (c.isOutside())
            continue;

        if (m_params.aabb.getMin().x == c.getAABB().getMin().x ||
            m_params.aabb.getMin().y == c.getAABB().getMin().y ||
            m_params.aabb.getMin().z == c.getAABB().getMin().z ||
            m_params.aabb.getMax().x == c.getAABB().getMax().x ||
            m_params.aabb.getMax().y == c.getAABB().getMax().y ||
            m_params.aabb.getMax().z == c.getAABB().getMax().z)
        {
            continue;
        }

        int j;
        for (j = 0; j < c.getPortalCount(); j++)
            if (!m_cellGraph.getCell(c.getPortal(j).getTarget()).isOutside())
                break;
        if (j == c.getPortalCount())
            c.setOutside(true);
    }

    // Mark force reachables using cluster marker view volumes.

    {
        Array<int> volumes(getAllocator());

        for (int i = 0; i < m_geometryBlock.getViewVolumeCount(); i++)
            if (m_geometryBlock.getViewVolume(i).isClusterMarker)
                volumes.pushBack(i);

        VoxelTraversal vt(m_voxelTree);
        markForceReachables(m_params.aabb, vt, volumes.getPtr(), volumes.getSize());
    }

    // Visualize backface voxels

    if (m_dc.pushActive("backface_voxels"))
    {
        VoxelTraversal vt(m_voxelTree);
        {
            VisualizeHelper vh(m_dc);
            visualizeBackface(m_params.aabb, vt, vh);
        }
        m_dc.popActive();
    }

    // Dilate.

    if (!m_params.accurateDilation)
    {
        VoxelIterator iter(m_voxelTree);
        while (iter.next())
            iter.get().setTmp(0);
        VoxelTraversal vt(m_voxelTree);
        voxelDilation(vt, m_params.aabb, 0);
    }
    else
    {
        VoxelIterator iter(m_voxelTree);
        while (iter.next())
            iter.get().setTmp(0);

        VoxelTraversal vt(m_voxelTree);
        voxelDilationInsideCell(vt, m_params.aabb);
    }

    // New subdivision stuff for view tree.

    {
        SubdivisionTree st(getAllocator());

        VoxelTraversal vt(m_voxelTree);
        st.setRoot(buildViewTree(st, vt, 0));
        st.setAABB(m_params.aabb);

        SubdivisionTreeUtils stu(st);

        // If there are plane nodes, try to collapse them with negative leaves first.
        st.setRoot(stu.collapsePlanesToNegatives(st.getRoot()));
        st.setRoot(stu.collapse(st.getRoot(), true));

        m_cellGraph.getViewTree().serialize(st);

        // Test node unification and serialization.

#ifdef UMBRA_DEBUG
        st.setRoot(SubdivisionTreeUtils::unifyNodes(getAllocator(), st.getRoot()));

        SubdivisionTreeSerialization sts(getAllocator());
        sts.serialize(st);

        SubdivisionTree st2(getAllocator());
        sts.deserialize(st2);
        SubdivisionTreeUtils::sanityCheck(st2.getRoot(), false);

        bool ret = SubdivisionTreeUtils::compareNodes(st.getRoot(), st2.getRoot());
        UMBRA_ASSERT(ret);
#endif
    }

    m_cellGraph.computeOutsideness();
    m_cellGraph.checkConsistency(CellGraph::BIDI | CellGraph::RAW);
}

void CellGenerator::split(const AABB& aabb, int triangleOfs, int numTriangles, int depth, VoxelConstructor& vc)
{
    bool doSplit = true;

    if (numTriangles == 0)
    {
#if 1
        // Split empty space if there are view volumes.

        if (depth < m_params.cellLevel-1)
        {
            bool intersectsViewVolume = false;

            for (int i = 0; i < m_geometryBlock.getViewVolumeCount(); i++)
                if (m_geometryBlock.getViewVolume(i).aabb.intersects(aabb))
                {
                    intersectsViewVolume = true;
                    break;
                }

            if (!intersectsViewVolume)
                doSplit = false;
        }
        else
#endif
            doSplit = false;
    }

    //doSplit = true;

    if (depth >= m_params.smallestHoleLevel)
        doSplit = false;

    if (doSplit)
    {
        // Make 8-bit octant mask for each triangle.

        int octantPos = m_octantStack.getSize();
        m_octantStack.resize(octantPos + numTriangles);

        for (int i = 0; i < numTriangles; i++)
        {
            Vector3 a, b, c;
            m_geometryBlock.getVertices(m_triangleStack[triangleOfs + i], a, b, c);
            m_octantStack[octantPos + i] = triangleOctants(aabb, a, b, c);
        }

        // Split into octants, gathering per octant triangles into triangle stack

        VoxelConstructor vc2 = vc.split();
        Vector3 p = aabb.getCenter();
        int triangleStart = triangleOfs + numTriangles;

        for (int octant = 0; octant < 8; octant++)
        {
            AABB aabbChild = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (octant & (1 << axis))
                    aabbChild.setMin(axis, p[axis]);
                else
                    aabbChild.setMax(axis, p[axis]);

            m_triangleStack.resize(triangleStart);
            for (int i = 0; i < numTriangles; i++)
            {
                if (m_octantStack[octantPos + i] & (1 << octant))
                {
                    int idx = m_triangleStack[triangleOfs + i];
                    m_triangleStack.pushBack(idx);
                    Vector3 a, b, c;
                    m_geometryBlock.getVertices(idx, a, b, c);
                    UMBRA_ASSERT(intersectAABBTriangle(aabbChild, a, b, c));
                }
            }
            split(aabbChild, triangleStart, m_triangleStack.getSize() - triangleStart, depth + 1, vc2);
        }
    }
    else
    {
        VoxelTree::LeafType type = VoxelTree::EMPTY;
        UINT32 borderFaceMask = 0;

        if (numTriangles)
        {
            // Solid if any of the triangles are occluders.

            for (int i = 0; i < numTriangles; i++)
            {
                if (m_geometryBlock.getTriangleObject(m_triangleStack[triangleOfs + i]).isOccluder())
                {
                    type = VoxelTree::SOLID;
                    break;
                }
            }

            if (type != VoxelTree::SOLID)
            {
                type = VoxelTree::GATE;
                bool intersect = false;

                for (int face = 0; face < 6 && !intersect; face++)
                {
                    AABB nodeAABB = aabb;
                    nodeAABB.flattenToFace(face);
                    AABB tileAABB = m_params.aabb;
                    tileAABB.flattenToFace(face);

                    if (tileAABB.contains(nodeAABB))
                    {
                        // on this face
                        int axis = getFaceAxis(face);
                        float faceCoord = nodeAABB.getMin()[axis];

                        for (int i = 0; i < numTriangles; i++)
                        {
                            int idx = m_triangleStack[triangleOfs + i];
                            Vector3 a, b, c;
                            m_geometryBlock.getVertices(idx, a, b, c);

                            if (intersectAABBTriangle(nodeAABB, a, b, c) &&
                                (a[axis] != faceCoord ||
                                 b[axis] != faceCoord ||
                                 c[axis] != faceCoord))
                            {
                                intersect = true;
                                break;
                            }
                        }

                        borderFaceMask |= (1 << face);
                    }
                }

                if (!intersect && borderFaceMask)
                    type = VoxelTree::BORDER_GATE;
            }
        }

        VoxelTree::LeafData& ld = vc.terminate(type);
        if (type == VoxelTree::BORDER_GATE)
            ld.getBorderGateData().setFaceMask(borderFaceMask);
    }
}

void CellGenerator::floodFill(const AABB& aabb, VoxelTraversal& vt, int depth, int fillDepth, int& idx)
{
    if (!vt.isLeaf() && depth < fillDepth)
    {
        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            floodFill(octreeSplit(aabb, i), vt2, depth+1, fillDepth, idx);
            vt2.nextSibling();
        }

        return;
    }

    VoxelTraversal vt2 = vt.cutParentLink(); // Cut parent link to limit flood-filling.

    // Flood-filling does both dfs and bfs filling. tmp value in voxels indicates that
    //  0) still to operate
    //  1) continue filling here
    //  2) done, don't touch

    VoxelIterator iter(vt2);
    while (iter.next())
        iter.get().setTmp(0);
    floodFill2(vt2, vt2, idx);
}

void CellGenerator::floodFill2(VoxelTraversal& parent, VoxelTraversal& vt, int& idx)
{
    if (vt.isLeaf())
    {
        VoxelTree::LeafData& ld = vt.getLeafData();
        if (ld.getTmp() == 2)
            return;
        UMBRA_ASSERT(ld.getTmp() == 0);

        if (!ld.isEmpty() && !ld.isBorderGate())
        {
            ld.setTmp(2);
            return;
        }

        ld.setTmp(1);
        ld.setViewCellIndex(idx++);

        for (;;)
        {
            bool done;
            do
                done = repFloodFill(parent);
            while (!done);

            if (floodFillGateEdgeConnectivity(parent, idx))
                break;
        }
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            floodFill2(parent, vt2, idx);
            vt2.nextSibling();
        }
    }
}

bool CellGenerator::repFloodFill(VoxelTraversal& vt)
{
    if (vt.isLeaf())
    {
        VoxelTree::LeafData& ld = vt.getLeafData();

        if (ld.getTmp() != 1)
            return true;

        if (ld.isEmpty())
        {
            EmptyFloodFiller<8> ff(vt, ld.getViewCellIndex(), 0);
            NeighborFinder<EmptyFloodFiller<8> >(ff).find(vt);
            return ff.m_finished;
        }
        else if (ld.isBorderGate())
        {
            VoxelTree::BorderGateData& bgd = ld.getBorderGateData();
            BorderGateFloodFiller<8> ff(vt, bgd.getFaceMask(), bgd.getBorderCellIndex(), 0);
            NeighborFinder<BorderGateFloodFiller<8> >(ff).find(vt);
            return ff.m_finished;
        }
        else
        {
            UMBRA_ASSERT(0);
            return true;
        }
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();

        bool finished = true;

        for (int i = 0; i < 8; i++)
        {
            if (!repFloodFill(vt2))
                finished = false;
            vt2.nextSibling();
        }

        return finished;
    }
}

bool CellGenerator::floodFillGateEdgeConnectivity(VoxelTraversal& vt, int cellIdx)
{
    if (vt.isLeaf())
    {
        VoxelTree::LeafData& ld = vt.getLeafData();

        if (!ld.isGate() || ld.isBorderGate())
            return true;

        FindCellPerFace fcpf;
        NeighborFinder<FindCellPerFace>(fcpf).find(vt);

        bool finished = true;

        for (int f = 0; f < 6; f++)
        {
            if (fcpf.m_faces[f] < 0)
                continue;

            int cell = fcpf.m_faces[f];

            for (int f2 = 0; f2 < 6; f2++)
            {
                if (f2 == f || (f2^1) == f)
                    continue;

                UMBRA_ASSERT(fcpf.m_faces[f2] == -1 || fcpf.m_faces[f2] == cell);

                if (fcpf.m_faces[f2] == cell)
                    continue;

                SetCellForFace scff(f2, cell);
                NeighborFinder<SetCellForFace>(scff).find(vt);

                if (scff.m_found)
                    finished = false;
            }
        }

        return finished;
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();

        bool finished = true;

        for (int i = 0; i < 8; i++)
        {
            if (!floodFillGateEdgeConnectivity(vt2, cellIdx))
                finished = false;
            vt2.nextSibling();
        }

        return finished;
    }
}

void CellGenerator::collectPortals(const AABB& aabb, VoxelTraversal& vt)
{
    if (vt.isLeaf())
    {
        if (vt.getLeafData().isEmpty() || vt.getLeafData().isBorderGate())
        {
            PortalCollector pc(m_cellGraph, aabb, vt);
            NeighborFinder<PortalCollector>(pc).find(vt);
        }
    }
    else
    {
        Vector3 p = aabb.getCenter();
        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            collectPortals(aabb2, vt2);

            vt2.nextSibling();
        }
    }
}

namespace Umbra
{
    struct PortalExpansion
    {
        PortalExpansion(CellGenerator& gen, const AABB& aabb, VoxelTraversal& vt)
            : m_gen(gen), m_aabb(aabb), m_exactIndex(vt.getLeafData().getExactIndex()), m_connectedCells(gen.getAllocator())
        {
        }

        int findConnectedCells()
        {
            const Array<int>& gateTris = m_gen.m_gateTrianglesPerVoxel[m_exactIndex];
            for (int i = 0; i < gateTris.getSize(); i++)
            {
                int cell = m_gen.m_gateTriangles[gateTris[i]].backCellIdx;
                if (cell != -1)
                    m_connectedCells.insert(cell);
                cell = m_gen.m_gateTriangles[gateTris[i]].frontCellIdx;
                if (cell != -1)
                    m_connectedCells.insert(cell);
            }
            return m_connectedCells.getSize();
        }

        bool collect(VoxelTraversal& vt, int face, int)
        {
            VoxelTree::LeafData& ld = vt.getLeafData();

            if (!ld.hasViewCell())
                return true;
            int cell = ld.getViewCellIndex();

            // are the neighbouring voxels directly connected?
            if (m_connectedCells.contains(cell))
                return true;

            float z = m_aabb.getFaceDist(face ^ 1);
            Vector4 rect = m_aabb.getFaceRect(face);
            CellGraph::Cell& c = m_gen.m_cellGraph.getCell(cell);
            for (int i = 0; i < c.getRectPortalCount(); i++)
            {
                CellGraph::RectPortal& p = c.getRectPortal(i);
                if (p.getFace() == face && p.getZ() == z && m_connectedCells.contains(p.getTarget()))
                {
                    Vector4 r = p.getRect();
                    r.x = min2(r.x, rect.x);
                    r.y = min2(r.y, rect.y);
                    r.z = max2(r.z, rect.z);
                    r.w = max2(r.w, rect.w);
                    // clamp portal to target cell AABB
                    r = rectIntersection(r, m_gen.m_cellGraph.getCell(p.getTarget()).getAABB().getFaceRect(face));
                    p.setRect(r);
                }
            }

            return true;
        }

        void done(int) {}

    private:
        PortalExpansion& operator= (const PortalExpansion&) { return *this; }

        CellGenerator& m_gen;
        AABB m_aabb;
        int m_exactIndex;
        Set<int> m_connectedCells;
    };
}

void CellGenerator::gateVoxelPortalExpansion(const AABB& aabb, VoxelTraversal& vt)
{
    if (vt.isLeaf())
    {
        if (!vt.getLeafData().isGate())
            return;
        if (vt.getLeafData().getExactIndex() == VoxelTree::INVALID_IDX)
            return;

        PortalExpansion pe(*this, aabb, vt);

        if (pe.findConnectedCells())
            NeighborFinder<PortalExpansion>(pe).find(vt);
    }
    else
    {
        Vector3 p = aabb.getCenter();
        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            gateVoxelPortalExpansion(aabb2, vt2);

            vt2.nextSibling();
        }
    }
}

void CellGenerator::makePortalsTwoWay()
{
    for (int i = 0; i < m_cellGraph.getCellCount(); i++)
    {
        CellGraph::Cell& c1 = m_cellGraph.getCell(i);

        for (int j = 0; j < c1.getRectPortalCount(); j++)
        {
            CellGraph::RectPortal& p1 = c1.getRectPortal(j);
            CellGraph::Cell& c2 = m_cellGraph.getCell(p1.getTarget());

            UMBRA_ASSERT(p1.getTarget() != i); // sanity check

            // Union to existing portal.

            bool found = false;

            for (int k = 0; k < c2.getRectPortalCount(); k++)
            {
                CellGraph::RectPortal& p2 = c2.getRectPortal(k);
                if (p2.getFace() == (p1.getFace()^1) && p2.getZ() == p1.getZ() && p2.getTarget() == i)
                {
                    Vector4 r = rectUnion(p1.getRect(), p2.getRect());
                    p1.setRect(r);
                    p2.setRect(r);
                    found = true;
                    break;
                }
            }

            // If existing portal wasn't found, add a new one to opposite direction.

            if (!found)
            {
                CellGraph::RectPortal p = p1;
                p.setTarget(i);
                if (!p.isGate())
                    p.setFace(p.getFace() ^ 1);
                c2.addRectPortal(p);
            }
        }
    }
}

void CellGenerator::mirrorGatePortals()
{
    for (int i = 0; i < m_cellGraph.getCellCount(); i++)
    {
        CellGraph::Cell& c1 = m_cellGraph.getCell(i);

        for (int j = 0; j < c1.getGatePortalCount(); j++)
        {
            CellGraph::GatePortal& p1 = c1.getGatePortal(j);
            CellGraph::Cell& c2 = m_cellGraph.getCell(p1.getTarget());

            UMBRA_ASSERT(p1.getTarget() != i); // sanity check

            // user portals only copied in one direction
            if (p1.getTarget() < i)
                continue;

            CellGraph::GatePortal p = p1;
            p.setTarget(i);
            c2.addGatePortal(p);
        }
    }
}

bool CellGenerator::rayTestUserPortals(const Vector3& a, const Vector3& b, const GateTri& sourceTri, Set<int>& overlapGates, bool allowOverlap)
{
    overlapGates.removeAll();

    UMBRA_UNREF(sourceTri);

    VoxelTraversal vt(m_voxelTree);

    if (intersectLineSegment(a, b, m_params.aabb, vt))
        return false;

    for (int i = 0; i < m_inputGateTriangles.getSize(); i++)
    {
        if (m_triangleToPlane[i] == sourceTri.planeIndex)
            continue;
        Triangle dest;
        m_geometryBlock.getVertices(m_inputGateTriangles[i], dest.a, dest.b, dest.c);
        bool isect = intersectLineSegmentTriangle(a, b, dest.a, dest.b, dest.c);

        if (isect)
        {
            if (allowOverlap)
            {
                overlapGates.insert(m_geometryBlock.getTriangleObject(m_inputGateTriangles[i]).getId());
            }
            else
            {
                return false;
            }
        }
    }

    return true;
}

void CellGenerator::assignTriangleToGateNodes(const GateTri& tri, int idx, const AABB& triAABB, const AABB& voxelAABB, VoxelTraversal& vt)
{
    if (!triAABB.intersects(voxelAABB))
        return;

    if (vt.isLeaf())
    {
        if (vt.getLeafData().isGate() && intersectsWithArea(voxelAABB, tri))
        {
            Array<int> cur(getAllocator());
            if (vt.getLeafData().getExactIndex() != VoxelTree::INVALID_IDX)
                cur = m_gateTrianglesPerVoxel[vt.getLeafData().getExactIndex()];
            cur.pushBack(idx);
            int exIdx = -1;
            for (int i = 0; i < m_gateTrianglesPerVoxel.getSize(); i++)
            {
                if (m_gateTrianglesPerVoxel[i] == cur)
                {
                    exIdx = i;
                    break;
                }
            }
            if (exIdx == -1)
            {
                exIdx = m_gateTrianglesPerVoxel.getSize();
                m_gateTrianglesPerVoxel.pushBack(cur);
            }
            vt.getLeafData().setExactIndex(exIdx);
        }
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = voxelAABB.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = voxelAABB;
            for (int axis = 0; axis < 3; axis++)
            {
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);
            }

            assignTriangleToGateNodes(tri, idx, triAABB, aabb2, vt2);
            vt2.nextSibling();
        }
    }
}

bool CellGenerator::validateRefPoint (const GateTri& tri, const AABB& aabb, VoxelTraversal& vt)
{
    UMBRA_ASSERT(aabb.contains(tri.ref));

    if (vt.isLeaf())
    {
        return !vt.getLeafData().isSolid();
    }
    else
    {
        Vector3 p = aabb.getCenter();
        int child = 0;
        AABB aabb2 = aabb;

        for (int axis = 0; axis < 3; axis++)
        {
            if (tri.ref[axis] < p[axis])
            {
                aabb2.setMax(axis, p[axis]);
            }
            else
            {
                child |= (1 << axis);
                aabb2.setMin(axis, p[axis]);
            }
        }

        VoxelTraversal vt2 = vt.child(child);
        return validateRefPoint(tri, aabb2, vt2);
    }
}

/* test if line segment intersects solid voxels */

bool CellGenerator::intersectLineSegment (const Vector3& a, const Vector3& b, const AABB& aabb, VoxelTraversal& vt)
{
    if (!intersectAABBLineSegment(aabb, a, b))
        return false;

    if (vt.isLeaf())
    {
        return vt.getLeafData().isSolid();
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
            {
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);
            }

            bool ret = intersectLineSegment(a, b, aabb2, vt2);
            if (ret)
                return true;
            vt2.nextSibling();
        }

        return false;
    }
}
       
void CellGenerator::findTriangleCells (GateTri& tri, const AABB& refAABB, const AABB& aabb, VoxelTraversal& vt, 
    float& bestFrontDistance, float& bestBackDistance, bool allowOverlap, Set<int>& frontOverlapGates, Set<int>& backOverlapGates)
{
    // \todo should find only withing same cell AABB?
    // \todo measure distance from node to triangle

    float dist = aabb.getDistance(tri.ref);

    if (!refAABB.intersects(aabb) && dist > bestFrontDistance && dist > bestBackDistance)
        return;

    if (vt.isLeaf())
    {
        VoxelTree::LeafData& ld = vt.getLeafData();
        if (ld.isEmpty() || ld.isBorderGate())
        {
            int cellIdx = ld.getViewCellIndex();

            Vector3 refPoint = aabb.getCenter();
            
            if (ld.isBorderGate())
            {
                AABB faceBounds = aabb;

                for (int i = 0; i < 6; i++)
                {
                    if (ld.getBorderGateData().getFaceMask() & (1 << i))
                        faceBounds.flattenToFace(i);
                }
                Vector3 dir = faceBounds.getCenter() - refPoint;
                float len = dir.length();
                float scale = (len + (m_targetInflation[0] / 2.f)) / len;
                dir *= scale;
                refPoint += dir;
            }
        
            float d = dot(m_gatePlanes[tri.planeIndex], refPoint);

            if (d < 0.f && dist < bestFrontDistance)
            {
                Set<int> overlapGates(getAllocator());

                if (rayTestUserPortals(tri.ref, refPoint, tri, overlapGates, allowOverlap))
                {
                    tri.frontCellIdx = cellIdx;
                    bestFrontDistance = dist;
                    
                    frontOverlapGates = overlapGates;
                }
            }
            else if (d > 0.f && dist < bestBackDistance)
            {
                Set<int> overlapGates(getAllocator());

                if (rayTestUserPortals(tri.ref, refPoint, tri, overlapGates, allowOverlap))
                {
                    tri.backCellIdx = cellIdx;
                    bestBackDistance = dist;

                    backOverlapGates = overlapGates;
                }
            }
        }
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
            {
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);
            }

            findTriangleCells(tri, refAABB, aabb2, vt2, bestFrontDistance, bestBackDistance, allowOverlap, frontOverlapGates, backOverlapGates);
            vt2.nextSibling();
        }
    }
}

void CellGenerator::computeCellAABBs(const AABB& aabb, VoxelTraversal& vt)
{
    if (vt.isLeaf())
    {
        if (vt.getLeafData().hasViewCell())
        {
            int vcIdx = vt.getLeafData().getViewCellIndex();
            m_cellGraph.getCell(vcIdx).growAABB(aabb);
        }
    }
    else
    {
        Vector3 p = aabb.getCenter();
        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            computeCellAABBs(aabb2, vt2);

            vt2.nextSibling();
        }
    }
}

void CellGenerator::collectTargets(const AABB& aabb, VoxelTraversal& vt, int* tris, int m)
{
    // Filter triangles.

    AABB aabb3 = aabb;
    aabb3.inflate(m_targetInflation);

    // The need for target inflation might be larger at computation tile boundaries, if
    // the voxel size on the other side of the boundary is greater than our voxel size.
    // To circumvent this, we further inflate all the way up to the geometry block target
    // bounds at the boundaries.

    for (int axis = 0; axis < 3; axis++)
    {
        if (aabb.getMin()[axis] == m_topLevelAABB.getMin()[axis])
        {
            Vector3 mn = aabb3.getMin();
            mn[axis] = m_geometryBlock.getTargetAABB().getMin()[axis];
            aabb3.setMin(mn);
        }
        if (aabb.getMax()[axis] == m_topLevelAABB.getMax()[axis])
        {
            Vector3 mx = aabb3.getMax();
            mx[axis] = m_geometryBlock.getTargetAABB().getMax()[axis];
            aabb3.setMax(mx);
        }
    }

    int n = filterTriangles(aabb3, tris, m);

    if (!n)
        return;

    if (vt.isLeaf())
    {
        // Get cell index. Special case for failed gate voxels.

        int vcIdx = -1;

        if (vt.getLeafData().isGate() && !vt.getLeafData().isBorderGate() && vt.getLeafData().getExactIndex() == VoxelTree::INVALID_IDX)
        {
            CellFinder cf;
            NeighborFinder<CellFinder>(cf).find(vt);
            if (cf.m_idx < 0)
                return;

            vcIdx = cf.m_idx;
        }
        else
        {
            if (!vt.getLeafData().hasViewCell())
                return;
            vcIdx = vt.getLeafData().getViewCellIndex();
        }

        // \todo [Hannu] approximate better triangle AABB
        for (int i = 0; i < n; i++)
        {
            Vector3 a, b, c;
            m_geometryBlock.getVertices(tris[i], a, b, c);
            AABB aabb4;
            aabb4.grow(a);
            aabb4.grow(b);
            aabb4.grow(c);
            aabb4.clamp(aabb3);

            int gb_idx = m_geometryBlock.getTriangle(tris[i]).m_objectIdx;
            UINT32 id = m_geometryBlock.getObject(gb_idx).getId();
            int* targetIdx = m_objMapping.get(id);
            if (!targetIdx)
            {
                targetIdx = m_objMapping.insert(id, m_cellGraph.getTargetObjectCount());
                m_cellGraph.addTargetObject(m_geometryBlock.getObject(gb_idx));
            }
            m_cellGraph.getCell(vcIdx).addObject(*targetIdx, aabb4);
        }
    }
    else
    {
        Vector3 p = aabb.getCenter();

        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            collectTargets(aabb2, vt2, tris, n);

            vt2.nextSibling();
        }
    }
}

void CellGenerator::collectVolumes(const AABB& aabb)
{
    BitVector seenCells(m_cellGraph.getCellCount(), getAllocator());
    for (int i = 0; i < m_geometryBlock.getObjectCount(); i++)
    {
        if (!m_geometryBlock.getObject(i).isVolume())
            continue;

        // Create a raytracer for each volumetric object and iterate through
        // the scene looking for points that are inside the volume.

        seenCells.clearAll();
        RayTracer rt(m_platform);
        Array<RayTracer::Triangle> indices(getAllocator());
        for (int j = 0; j < m_geometryBlock.getTriangleCount(); j++)
        {
            const GeometryBlock::Triangle& tri = m_geometryBlock.getTriangle(j);
            if (tri.m_objectIdx != UINT32(i))
                continue;

            indices.pushBack(RayTracer::Triangle(tri.m_vertices));
        }
        rt.buildBVH(m_geometryBlock.getVertices().getPtr(), indices.getPtr(), m_geometryBlock.getVertices().getSize(), indices.getSize());
        RayTracerTraversal rtt(rt);

        VoxelTraversal vt(m_voxelTree);
        markVolume(aabb, vt, rtt, i, seenCells);
    }
}

void CellGenerator::markVolume(const AABB& aabb, VoxelTraversal& vt, const RayTracerTraversal& rt, int idx, BitVector& seenCells)
{
    // XXX: This function's code duplicates the voxel traversal logic in collectTargets

    // The voxel and the object have separate AABBs and can't intersect, bail out.
    if (!aabb.intersects(m_geometryBlock.getObject(idx).m_bounds))
        return;

    if (vt.isLeaf())
    {
        if (!vt.getLeafData().hasViewCell())
            return;
        int vcIdx = vt.getLeafData().getViewCellIndex();

        // Ensure that each cell is processed at most once.
        if (seenCells[vcIdx])
            return;
        seenCells.set(vcIdx);

        UINT32 id = m_geometryBlock.getObject(idx).getId();

        // Do we need to add the target object to the cell graph later?
        bool objectMapped = false;
        // The (eventual) index of the object in the cell graph.
        int targetIdx = m_cellGraph.getTargetObjectCount();
        // Grab an existing mapping for the object if one exists.
        int* ref = m_objMapping.get(id);
        if (ref)
        {
            objectMapped = true;
            targetIdx = *ref;
        }

        // Cell was not marked in seenCells, but the object nevertheless already
        // exists here. This happens with cells at the object's surface, objects
        // are added to them in the earlier processing phase of non-volumentric
        // objects.
        if (m_cellGraph.getCell(vcIdx).hasObject(targetIdx))
            return;

        // At this point, the current voxel's cell may be inside the object. Fire up the raytracer.

        // XXX: Only shooting from the first voxel in the cell is somewhat iffy.
        // A more robust implementation would have the traversal collect the
        // entire cell volume, then take random samples from it.
        Vector3 pos = aabb.getCenter();
        float dist; // Needed by raytrace, ignored otherwise.
        float maxDist = m_geometryBlock.getObject(idx).m_bounds.getDiagonalLength() * 2.f;

        const int nRays = 15;
        int backfaceHits = 0;

        for (int i = 0; i < nRays; i++)
        {
            // Shoot a random ray and see what it hits. All rays from inside a
            // closed convex object should hit a backface of the object.

            // Generate an equally dispersed set of vectors as ray directions
            // using Halton sequences.
            Vector3 dir = migrate(uniformPointOnSphere(migrate(Vector2f(haltonf<2>(i * 2), haltonf<2>(i * 2 + 1)))));

            // TODO: Make the raytracer support hitting the geometry multiple times and count the number
            // of times the ray intersects the geometry. This would let us handle nonconvex closed
            // objects as well, through the condition that a ray from inside the object will intersect
            // the surface an odd number of times.
            RayTracer::RayTraceResult hit = rt.rayTrace(pos, dir, maxDist, dist, NULL);

            if (hit == RayTracer::HIT_BACKFACE)
                backfaceHits++;
        }

        if (backfaceHits <= nRays / 2)
        {
            // The majority of the rays did not hit object backface, assume
            // we're on the outside of the object and return.
            return;
        }

        // Otherwise assume the voxel is inside the object. Link the cell to the object.
        AABB targetAABB = m_cellGraph.getCell(vcIdx).getAABB();
        targetAABB.clamp(m_geometryBlock.getObject(idx).m_bounds);

        // If the object is not mapped yet, add the mapping too.
        if (!objectMapped)
        {
            m_objMapping.insert(id, targetIdx);
            m_cellGraph.addTargetObject(m_geometryBlock.getObject(idx));
        }

        m_cellGraph.getCell(vcIdx).addObject(targetIdx, targetAABB);
    }
    else
    {
        // Iterate through the octree child voxels, maintaining the AABB for each.
        Vector3 p = aabb.getCenter();

        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            markVolume(aabb2, vt2, rt, idx, seenCells);

            vt2.nextSibling();
        }
    }

}

int CellGenerator::filterViewVolumes(const AABB& aabb, int* vols, int m)
{
    int i = 0, j = 0;

    for (; i < m; i++)
    {
        if (m_geometryBlock.getViewVolume(vols[i]).aabb.intersects(aabb))
        {
            if (i != j)
                swap2(vols[i], vols[j]);
            j++;
        }
    }

    return j;
}

void CellGenerator::markForceReachables(const AABB& aabb, VoxelTraversal& vt, int* vols, int n2)
{
    // Filter volumes.

    int n = filterViewVolumes(aabb, vols, n2);
    if (!n)
        return;

    if (vt.isLeaf())
    {
        if (!vt.getLeafData().hasViewCell())
            return;
        int vcIdx = vt.getLeafData().getViewCellIndex();
        if (m_removedCells.test(vcIdx))
            return;
        m_cellGraph.getCell(vcIdx).setForceReachable(true);
        m_cellGraph.getCell(vcIdx).setOutside(false);
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            markForceReachables(octreeSplit(aabb, i), vt2, vols, n);
            vt2.nextSibling();
        }
    }
}

void CellGenerator::setVolumeOutsideness(const AABB& aabb, VoxelTraversal& vt, int* vols, int n2)
{
    // Filter volumes.

    int n = filterViewVolumes(aabb, vols, n2);

    if (vt.isLeaf())
    {
        vt.getLeafData().setOutside(n == 0);
    }
    else
    {
        Vector3 p = aabb.getCenter();

        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            setVolumeOutsideness(aabb2, vt2, vols, n);

            vt2.nextSibling();
        }
    }
}


void CellGenerator::testVolumeOutsideness(const AABB& aabb, VoxelTraversal& vt, int* vols, int n2)
{
    // Filter volumes.

    int n = filterViewVolumes(aabb, vols, n2);
    if (!n)
        return;

    if (vt.isLeaf())
    {
        if (!vt.getLeafData().hasViewCell())
            return;
        int vcIdx = vt.getLeafData().getViewCellIndex();
        if (m_removedCells.test(vcIdx))
            return;
        m_cellGraph.getCell(vcIdx).setOutside(false);
    }
    else
    {
        Vector3 p = aabb.getCenter();

        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            testVolumeOutsideness(aabb2, vt2, vols, n);

            vt2.nextSibling();
        }
    }
}

void CellGenerator::testVoxelBackface (const AABB& aabb, VoxelTraversal& vt, int depth, int fillDepth)
{
    // Traverse until fill depth.

    if (!vt.isLeaf() && depth < fillDepth)
    {
        VoxelTraversal vt2 = vt.firstChild();

        for (int i = 0; i < 8; i++)
        {
            testVoxelBackface(octreeSplit(aabb, i), vt2, depth+1, fillDepth);
            vt2.nextSibling();
        }
        return;
    }

    // Flood-fill backface cells.

    int cells = 0;
    floodFill(m_params.aabb, vt, depth, fillDepth, cells);

    // Sampling.

    for (int vc = 0; vc < cells; vc++)
    {
        const int NUM_SAMPLES = 64;
        float bfLimit = 50.f;//m_params.bfLimit;
        float bfDistance = m_smallestHoleSize * 4.f;

        if (m_params.bfLimit >= 100.f)
            return;

        // \todo [Hannu] collecting AABBs can be avoided by doing two passes on the tree
        Array<AABB> aabbs(getAllocator());
        collectInsideAABBs(aabb, vt, vc, aabbs);

        if (aabbs.getSize() == 0)
            continue;

        // Weight AABBs by their volume.

        WeightedSampler ws(getAllocator());
        ws.resize(aabbs.getSize());

        for (int i = 0; i < aabbs.getSize(); i++)
            ws.setWeight(i, aabbs[i].getVolume());

        // Sample.

        Random rand;
        int in = 0, out = 0;

        for (int i = 0; i < NUM_SAMPLES; i++)
        {
            double v = (i + rand.get()) / NUM_SAMPLES;
            int j = ws.pickSample(v);

            // Random position.
            // \todo [Hannu] use halton sequences instead of random

            AABB aabb = aabbs[j];
            Vector3f pos = aabb.getMin();
            pos.x += aabb.getDimensions().x * rand.get();
            pos.y += aabb.getDimensions().y * rand.get();
            pos.z += aabb.getDimensions().z * rand.get();

            // Random direction.

            float theta = haltonf<3>(i+3) * 2.f * 3.14159265f;

            Vector3f dir;
            dir.z = haltonf<2>(i+3) * 2.f - 1.f;
            dir.x = sqrtf(1.f - dir.z*dir.z) * cosf(theta);
            dir.y = sqrtf(1.f - dir.z*dir.z) * sinf(theta);

            // Do the query.

            float dist;
            RayTracer::RayTraceResult res = m_rayTracerTraversal.rayTrace(pos, dir, bfDistance, dist, 0);

            if (res == RayTracer::HIT_BACKFACE)
                out++;
            else if (res == RayTracer::HIT_FRONTFACE)
                in++;

            // Test if the cell cannot change inside/outside anymore.

            int samplesRemaining = NUM_SAMPLES-1 - i;

            if (in+out > 4 && (out+samplesRemaining) / float(in+out+samplesRemaining) * 100.f < bfLimit)
                break;

            if (in+out > 4 && out / float(in+out+samplesRemaining) * 100.f > bfLimit)
                break;
        }

        if (in+out > 4 && out / float(in+out) * 100.f > bfLimit)
        {
            VoxelIterator iter(vt);
            while (iter.next())
                if (iter.get().hasViewCell() && iter.get().getViewCellIndex() == vc)
                    iter.get().setOutside(true);
        }
    }
}

bool CellGenerator::testCellBackface (const AABB& aabb, VoxelTraversal& vt, int vc)
{
    UMBRA_ASSERT(m_rayTracer);

    const int NUM_SAMPLES = 200;
    float bfLimit = m_params.bfLimit;
    float bfDistance = m_params.bfDistance;

    if (bfLimit >= 100.f)
    {
        m_backFaceRatio[vc] = 0.f;
        return false;
    }

    // \todo [Hannu] collecting AABBs can be avoided by doing two passes on the tree
    Array<AABB> aabbs(getAllocator());
    collectAABBs(aabb, vt, vc, aabbs);

    // Weight AABBs by their volume.

    WeightedSampler ws(getAllocator());
    ws.resize(aabbs.getSize());

    for (int i = 0; i < aabbs.getSize(); i++)
        ws.setWeight(i, aabbs[i].getVolume());

    // Sample.

    Random rand;
    Vector3 tri[3];
    int in = 0, out = 0;
    int outThreshold = (int)((NUM_SAMPLES * bfLimit) / 100.f);
    int inThreshold = (int)((NUM_SAMPLES * (100.f - bfLimit)) / 100.f);

    for (int i = 0; i < NUM_SAMPLES; i++)
    {
        double v = (i + rand.get()) / NUM_SAMPLES;
        int j = ws.pickSample(v);

        // Random position.
        // \todo [Hannu] use halton sequences instead of random

        AABB aabb = aabbs[j];
        Vector3f pos = aabb.getMin();
        pos.x += aabb.getDimensions().x * rand.get();
        pos.y += aabb.getDimensions().y * rand.get();
        pos.z += aabb.getDimensions().z * rand.get();

        // Random direction.

        float theta = haltonf<3>(i+3) * 2.f * 3.14159265f;

        Vector3f dir;
        dir.z = haltonf<2>(i+3) * 2.f - 1.f;
        dir.x = sqrtf(1.f - dir.z*dir.z) * cosf(theta);
        dir.y = sqrtf(1.f - dir.z*dir.z) * sinf(theta);

        // Do the query.

        float dist;
        RayTracer::RayTraceResult res = m_rayTracerTraversal.rayTrace(pos, dir, bfDistance, dist, tri);
        if (res == RayTracer::NO_HIT)
        {
            continue;
        }
        else if (res == RayTracer::HIT_BACKFACE)
        {
            out++;
        }
        else if (res == RayTracer::HIT_FRONTFACE)
        {
            in++;
        }

        // If we reach enough outside samples, stop sampling now.

        if (out > outThreshold)
            return true;
        if (in > inThreshold)
            break;
    }

    int validSamples = in + out;
    if (!validSamples)
    {
        m_backFaceRatio[vc] = 0.f;
        return false;
    }

    float outPercent = 100.f * out / validSamples;
    if (outPercent > bfLimit)
        return true;

    // Inside cell, record backface ratio for voxel dilation
    m_backFaceRatio[vc] = outPercent;
    return false;
}

int CellGenerator::findInsideNeighbor(VoxelTraversal& vt, const AABB& aabb, const AABB& tgt, int exclude)
{
    if (!aabb.intersects(tgt))
        return -1;

    if (vt.isLeaf())
    {
        if (!vt.getLeafData().isEmpty() || vt.getLeafData().getTmp() != 0 || vt.getLeafData().isOutside())
            return -1;
        int idx = vt.getLeafData().getViewCellIndex();
        if (m_cellGraph.getCell(idx).isOutside())
            return -1;
        return idx;
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();
        for (int i = 0; i < 8; i++)
        {
            if (i != exclude)
            {
                AABB aabb2 = aabb;
                for (int axis = 0; axis < 3; axis++)
                    if (i & (1 << axis))
                        aabb2.setMin(axis, p[axis]);
                    else
                        aabb2.setMax(axis, p[axis]);

                // TODO: choose better neighbor (relative to tgt) first!
                int idx = findInsideNeighbor(vt2, aabb2, tgt, -1);
                if (idx != -1)
                    return idx;
            }
            vt2.nextSibling();
        }
    }
    return -1;
}

struct DilationTraverseData
{
    AABB aabb;
    bool hasInsides;

    static const DilationTraverseData* get(const VoxelTraversal& vt)
    {
        return (const DilationTraverseData*)vt.m_userData;
    }
};

void CellGenerator::voxelDilation(VoxelTraversal& vt, const AABB& aabb, int depth)
{
    if (!vt.isLeaf() && depth < m_params.cellLevel)
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            voxelDilation(vt2, aabb2, depth+1);
            vt2.nextSibling();
        }

        return;
    }

    VoxelTraversal vt2 = vt.cutParentLink();
    voxelDilationCell(vt2, aabb, 0x3F);
}

void CellGenerator::voxelDilationCell(VoxelTraversal& vt, const AABB& aabb, Umbra::UINT32 borderMask)
{
    if (vt.isLeaf())
    {
        if (!vt.getLeafData().isSolid())
            return;

        UINT32 faceMask = ~borderMask & 0x3F;
        int bestCell = -1;
        VoxelTraversal cur = vt;

        while ((bestCell == -1) && faceMask && cur.hasParent())
        {
            int idx = cur.indexInParent();
            for (int axis = 0; axis < 3; axis++)
            {
                UINT32 face = axis << 1;
                if (!(idx & (1 << axis)))
                    face |= 1;
                faceMask &= ~(1 << face);
            }

            cur = cur.parent();
            const DilationTraverseData* data2 = DilationTraverseData::get(cur);
            if (data2->hasInsides)
                bestCell = findInsideNeighbor(cur, data2->aabb, aabb, idx);
        }

        // no inside neighbors, no dilation
        if (bestCell < 0)
            return;

        vt.getLeafData().setType(VoxelTree::EMPTY);
        vt.getLeafData().setViewCellIndex(bestCell);
        vt.getLeafData().setTmp(1);
    }
    else
    {
        DilationTraverseData data;
        data.aabb = aabb;
        data.hasInsides = false;
        vt.m_userData = &data;

        // Are there inside cells in this subtree?

        if (!vt.hasParent() || DilationTraverseData::get(vt.parent())->hasInsides)
        {
            VoxelIterator iter(vt);
            while (iter.next())
            {
                if (iter.get().isEmpty() && !iter.get().isOutside() &&
                    !m_cellGraph.getCell(iter.get().getViewCellIndex()).isOutside())
                {
                    UMBRA_ASSERT(iter.get().getTmp() == 0);
                    data.hasInsides = true;
                    break;
                }
            }
        }

        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            UINT32 mask = borderMask;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                {
                    UINT32 face = axis << 1;
                    mask &= ~(1 << face);
                    aabb2.setMin(axis, p[axis]);
                }
                else
                {
                    UINT32 face = (axis << 1) | 1;
                    mask &= ~(1 << face);
                    aabb2.setMax(axis, p[axis]);
                }

            voxelDilationCell(vt2, aabb2, mask);
            vt2.nextSibling();
        }
    }
}

void CellGenerator::collectInsideAABBs(const AABB& aabb, VoxelTraversal& vt, int vc, Array<AABB>& aabbs)
{
    if (vt.isLeaf())
    {
        if (!vt.getLeafData().hasViewCell() || vt.getLeafData().isOutside())
            return;

        if (vt.getLeafData().getViewCellIndex() == vc)
            aabbs.pushBack(aabb);
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            collectInsideAABBs(aabb2, vt2, vc, aabbs);

            vt2.nextSibling();
        }
    }
}

void CellGenerator::findClosestInsideCell(VoxelTraversal& vt, const AABB& aabb, const AABB& tgt, const VoxelTraversal& vtRef, int& best, int& bestDistance)
{
    if (!aabb.intersects(tgt))
        return;

    if (vt.isLeaf())
    {
        if (!vt.getLeafData().isEmpty() || vt.getLeafData().getTmp() != 0 || vt.getLeafData().isOutside())
            return;

        int idx = vt.getLeafData().getViewCellIndex();
        if (m_cellGraph.getCell(idx).isOutside())
            return;

        int dist = VoxelTraversal::pathDistance(vt, vtRef);

        if (best == -1 || dist < bestDistance)
        {
            best = idx;
            bestDistance = dist;
        }
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();
        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            findClosestInsideCell(vt2, aabb2, tgt, vtRef, best, bestDistance);
            vt2.nextSibling();
        }
    }
}

void CellGenerator::voxelDilationInsideCell(VoxelTraversal& vt, const AABB& aabb)
{
    if (vt.isLeaf())
    {
        if (!vt.getLeafData().isSolid())
            return;

        int cellIdx = -1;
        int distance = INT_MAX;

        VoxelTraversal vt2(m_voxelTree);
        findClosestInsideCell(vt2, m_params.aabb, aabb, vt, cellIdx, distance);

        if (cellIdx < 0)
            return;

        vt.getLeafData().setType(VoxelTree::EMPTY);
        vt.getLeafData().setViewCellIndex(cellIdx);
        vt.getLeafData().setTmp(1);
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            voxelDilationInsideCell(vt2, aabb2);
            vt2.nextSibling();
        }
    }
}

void CellGenerator::collectAABBs(const AABB& aabb, VoxelTraversal& vt, int vc, Array<AABB>& aabbs)
{
    if (m_cellGraph.getCell(vc).getAABB().isOK() && !aabb.intersects(m_cellGraph.getCell(vc).getAABB()))
        return;

    if (vt.isLeaf())
    {
        if (!vt.getLeafData().hasViewCell() ||
            (vt.getLeafData().getViewCellIndex() != vc))
            return;

        aabbs.pushBack(aabb);
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            collectAABBs(aabb2, vt2, vc, aabbs);

            vt2.nextSibling();
        }
    }
}

int CellGenerator::filterTriangles(const AABB& aabb, int* tris, int count)
{
    int i = 0, j = 0;

    for (; i < count; i++)
    {
        Vector3 a, b, c;
        m_geometryBlock.getVertices(tris[i], a, b, c);

        if (intersectAABBTriangle_Fast(aabb, a, b, c))
        {
            if (i != j)
                swap2(tris[i], tris[j]);
            j++;
        }
    }

    return j;
}

bool CellGenerator::clustersMatch (VoxelTraversal& vt, int cluster)
{
    VoxelIterator iter(vt);
    while (iter.next())
    {
        VoxelTree::LeafData& ld = iter.get();
        if (!ld.isEmpty())
            continue;
        int idx = ld.getViewCellIndex();
        if (m_cellGraph.getCell(idx).isOutside())
            continue;
        if (m_cellGraph.getCell(idx).getClusters()[0] != cluster)
            return false;
    }
    return true;
}

namespace Umbra {

enum
{
    IN_FRONT = (1 << 0),
    IN_BACK  = (1 << 1)
};

static UINT32 planeTest (const CellGenerator::GateTri& tri, const Vector4& pleq, float epsilon)
{
    UINT32 ret = 0;
    for (int i = 0; i < tri.poly.getSize(); i++)
    {
        float d = dot(pleq, tri.poly[i]);
        if (d < -epsilon)
            ret |= IN_FRONT;
        if (d > epsilon)
            ret |= IN_BACK;
    }
    return ret;
}

static UINT32 planeTest (const Triangle& tri, const Vector4& pleq, const Triangle& origTri)
{
    UINT32 ret = 0;
    for (int i = 0; i < 3; i++)
    {
        if (tri[i] == origTri[0] || tri[i] == origTri[1] || tri[i] == origTri[2])
            continue;
        float d = dot(pleq, tri[i]);
        if (d < 0.f)
            ret |= IN_FRONT;
        if (d > 0.f)
            ret |= IN_BACK;
    }
    return ret;
}

static UINT32 planeTest (const Triangle& tri, const Vector4& pleq)
{
    UINT32 ret = 0;
    for (int i = 0; i < 3; i++)
    {
        float d = dot(pleq, tri[i]);
        if (d < 0.f)
            ret |= IN_FRONT;
        if (d > 0.f)
            ret |= IN_BACK;
    }
    return ret;
}

}

SubdivisionTree::Node* CellGenerator::buildBSPTree(SubdivisionTree& st, Set<int>& used, const Array<int>& tris, int cellIdx)
{
    if (!tris.getSize())
    {
        SubdivisionTree::LeafNode* n = st.newLeaf();
        if ((cellIdx >= 0) && m_cellGraph.getCell(cellIdx).isOutside())
            cellIdx = -1;
        n->setIndex(cellIdx);
        return n;
    }

    int splitIdx = tris[0];
    const GateTri& splitter = m_gateTriangles[splitIdx];

    Vector4 pleq = m_gatePlanes[splitter.planeIndex];
    SubdivisionTree::PlaneNode* node = st.newPlane();
    node->setPleq(pleq);

    Array<int> leftTris(getAllocator());  // negative / front
    Array<int> rightTris(getAllocator()); // positive / back

    used.insert(splitter.planeIndex);

    for (int i = 1; i < tris.getSize(); i++)
    {
        if (used.contains(m_gateTriangles[tris[i]].planeIndex))
            continue;
        UINT32 side = planeTest(m_gateTriangles[tris[i]], pleq, GATE_PLANE_EPSILON);
        if (side & IN_FRONT)
            leftTris.pushBack(tris[i]);
        if (side & IN_BACK)
            rightTris.pushBack(tris[i]);
    }

    node->setLeft(buildBSPTree(st, used, leftTris, splitter.frontCellIdx));
    node->setRight(buildBSPTree(st, used, rightTris, splitter.backCellIdx));

    used.remove(splitter.planeIndex);

    return node;
}

SubdivisionTree::Node* CellGenerator::buildTessellationBSPTree(SubdivisionTree& st, const Array<PlaneTriangle>& tris)
{
    if (!tris.getSize())
    {
        SubdivisionTree::LeafNode* n = st.newLeaf();
        return n;
    }

    // With huge triangle counts, try to do axis aligned splits first to make things faster.

    if (tris.getSize() > 500)
    {
        AABB aabb;
        for (int i = 0; i < tris.getSize(); i++)
        {
            aabb.grow(tris[i].tri.a);
            aabb.grow(tris[i].tri.b);
            aabb.grow(tris[i].tri.c);
        }

        int   bestAxis = -1;
        float bestPos  = 0.f;
        float bestCost = FLT_MAX;

        for (int axis = 0; axis < 3; axis++)
        {
            const int SAMPLES = 5;

            for (int j = 0; j < SAMPLES; j++)
            {
                float p = aabb.getMin()[axis] + (aabb.getMax()[axis] - aabb.getMin()[axis]) * (j+1) / float(SAMPLES + 1);

                int numLeft  = 0;
                int numRight = 0;

                for (int k = 0; k < tris.getSize(); k++)
                {
                    float mn = min2(min2(tris[k].tri.a[axis], tris[k].tri.b[axis]), tris[k].tri.c[axis]);
                    float mx = max2(max2(tris[k].tri.a[axis], tris[k].tri.b[axis]), tris[k].tri.c[axis]);

                    if (mx >= p)
                        numRight++;
                    if (mn <= p)
                        numLeft++;
                }

                if (numLeft >= tris.getSize()-32 || numRight >= tris.getSize()-32)
                    continue;

                float cost = float(numLeft) * numLeft + float(numRight) * numRight;

                if (cost < bestCost)
                {
                    bestAxis = axis;
                    bestPos  = p;
                    bestCost = cost;
                }
            }
        }

        if (bestAxis >= 0)
        {
            Vector4 pleq;
            pleq[bestAxis] = 1.f;
            pleq.w         = -bestPos;

            SubdivisionTree::PlaneNode* node = st.newPlane();
            node->setPleq(pleq);

            Array<PlaneTriangle> leftTris(getAllocator());  // negative / front
            Array<PlaneTriangle> rightTris(getAllocator()); // positive / back

            for (int i = 0; i < tris.getSize(); i++)
            {
                const PlaneTriangle& tri2 = tris[i];
                UINT32 side = planeTest(tri2.tri, pleq);
                if (side & IN_FRONT)
                    leftTris.pushBack(tris[i]);
                if (side & IN_BACK)
                    rightTris.pushBack(tris[i]);
            }

            node->setLeft(buildTessellationBSPTree(st, leftTris));
            node->setRight(buildTessellationBSPTree(st, rightTris));

            return node;
        }
    }


    // Find the best splitting candiate.

    int splitIdx = 0;
    int numIntersecting = tris.getSize();

    for (int k = 0; k < tris.getSize(); k++)
    {
        int i = intShuffler(k, tris.getSize());

        const PlaneTriangle& tri = tris[i];

        int n = 0;
        for (int j = 0; j < tris.getSize(); j++)
        {
            if (i == j)
                continue;

            const PlaneTriangle& tri2 = tris[j];
            if (planeTest(tri2.tri, tri.plane, tri.tri) == (IN_FRONT | IN_BACK))
            {
                n++;

                if (n >= numIntersecting) // Early exit
                    break;
            }
        }

        if (n < numIntersecting)
        {
            splitIdx = i;
            numIntersecting = n;
        }
        if (numIntersecting == 0)
            break;
    }

    // Split triangles using new plane node.

    const PlaneTriangle& splitter = tris[splitIdx];
    SubdivisionTree::PlaneNode* node = st.newPlane();
    node->setPleq(splitter.plane);

    Array<PlaneTriangle> leftTris(getAllocator());  // negative / front
    Array<PlaneTriangle> rightTris(getAllocator()); // positive / back

    for (int i = 0; i < tris.getSize(); i++)
    {
        if (i == splitIdx)
            continue;

        const PlaneTriangle& tri2 = tris[i];
        UINT32 side = planeTest(tri2.tri, splitter.plane, splitter.tri);
        if (side & IN_FRONT)
            leftTris.pushBack(tris[i]);
        if (side & IN_BACK)
            rightTris.pushBack(tris[i]);
    }

    node->setLeft(buildTessellationBSPTree(st, leftTris));
    node->setRight(buildTessellationBSPTree(st, rightTris));

    return node;
}

SubdivisionTree::Node* CellGenerator::buildBSPTree(SubdivisionTree& st, const Array<int>& indices)
{
    Set<int> used(getAllocator());
    return buildBSPTree(st, used, indices, -1);
}

SubdivisionTree::Node* CellGenerator::buildViewTree(SubdivisionTree& st, VoxelTraversal& vt, int depth)
{
    int validExact = -1;
    int validCell = -1;

    VoxelIterator iter(vt);
    while (iter.next())
    {
        VoxelTree::LeafData& ld = iter.get();
        if (ld.isOutside())
            continue;

        if (ld.isGate() && (ld.getExactIndex() != VoxelTree::INVALID_IDX))
        {
            if (validCell != -1)
                break;
            if (validExact == -1)
                validExact = ld.getExactIndex();
            else if (ld.getExactIndex() != validExact)
                break;
        }
        else if (ld.isEmpty())
        {
            UMBRA_ASSERT(ld.getViewCellIndex() != VoxelTree::INVALID_IDX);
            if (validExact != -1)
                break;
            int cellIdx = ld.getViewCellIndex();
            if (m_cellGraph.getCell(cellIdx).isOutside())
                continue;
            if (validCell == -1)
                validCell = cellIdx;
            else if (cellIdx != validCell)
                break;
        }
    }

    // Make leaf.

    if (vt.isLeaf() || !iter.hasMore())
    {
        if (validExact != -1)
            return buildBSPTree(st, m_gateTrianglesPerVoxel[validExact]);

        SubdivisionTree::LeafNode* node = st.newLeaf();
        node->setIndex(validCell);
        return node;
    }

    // Inner.

    SubdivisionTree::MedianNode* splits[7];

    for (int i = 0; i < 7; i++)
        splits[i] = st.newMedian();

    splits[0]->setAxis(2);
    splits[1]->setAxis(1);
    splits[2]->setAxis(1);
    splits[3]->setAxis(0);
    splits[4]->setAxis(0);
    splits[5]->setAxis(0);
    splits[6]->setAxis(0);

    splits[0]->setLeft(splits[1]);
    splits[0]->setRight(splits[2]);

    splits[1]->setLeft(splits[3]);
    splits[1]->setRight(splits[4]);
    splits[2]->setLeft(splits[5]);
    splits[2]->setRight(splits[6]);

    VoxelTraversal vt2 = vt.firstChild();

    for (int i = 0; i < 8; i++)
    {
        if (i & 1)
            splits[3 + i/2]->setRight(buildViewTree(st, vt2, depth+1));
        else
            splits[3 + i/2]->setLeft(buildViewTree(st, vt2, depth+1));

        vt2.nextSibling();
    }

    return splits[0];
}

SubdivisionTree::Node* CellGenerator::buildMatchingTree(SubdivisionTree& st, VoxelTraversal& vt, int face)
{
    int idx = VoxelTree::INVALID_IDX;
    VoxelIterator iter(vt);
    while (iter.next())
    {
        VoxelTree::LeafData& ld = iter.get();
        int cur = -1;
        if (ld.hasViewCell() && !m_removedCells.test(ld.getViewCellIndex()))
            cur = ld.getViewCellIndex();
        if (idx == VoxelTree::INVALID_IDX)
            idx = cur;
        else if (cur != idx)
            break;
    }
    UMBRA_ASSERT(idx != VoxelTree::INVALID_IDX);
    if (vt.isLeaf() || !iter.hasMore())
    {
        SubdivisionTree::LeafNode* node = st.newLeaf();
        node->setIndex(idx);
        return node;
    }

    // Table for each face. First split axises, then voxel tree child indices.
    // TODO: better logic

    static const int table[6][6] = {
        { 2, 1, 0, 2, 4, 6 },
        { 2, 1, 1, 3, 5, 7 },
        { 2, 0, 0, 1, 4, 5 },
        { 2, 0, 2, 3, 6, 7 },
        { 1, 0, 0, 1, 2, 3 },
        { 1, 0, 4, 5, 6, 7 },
    };

    SubdivisionTree::MedianNode* node0 = st.newMedian();
    SubdivisionTree::MedianNode* node1 = st.newMedian();
    SubdivisionTree::MedianNode* node2 = st.newMedian();

    node0->setAxis(table[face][0]);
    node1->setAxis(table[face][1]);
    node2->setAxis(table[face][1]);

    node0->setLeft(node1);
    node0->setRight(node2);

    VoxelTraversal vt2 = vt.child(table[face][2]);
    node1->setLeft(buildMatchingTree(st, vt2, face));
    vt2 = vt.child(table[face][3]);
    node1->setRight(buildMatchingTree(st, vt2, face));
    vt2 = vt.child(table[face][4]);
    node2->setLeft(buildMatchingTree(st, vt2, face));
    vt2 = vt.child(table[face][5]);
    node2->setRight(buildMatchingTree(st, vt2, face));

    return node0;
}

namespace Umbra
{
    struct VisualizeSolid
    {
        VisualizeSolid(VisualizeHelper& vh, const AABB& aabb)
            : m_vh(vh), m_aabb(aabb), m_mask(0)
        {
        }

        bool collect(VoxelTraversal& vt, int face, int)
        {
            m_mask |= 1 << face;

            VoxelTree::LeafData& ld = vt.getLeafData();

            if (ld.isSolid())
                return true;

            float z = m_aabb.getFaceDist(face^1);
            AARectf rect = getBoxFaceRect(migrate(m_aabb), face);

            m_vh.voxelFace(face, z, rect, 0);

            return true;
        }

        void done(int) {}

        VisualizeHelper& m_vh;
        AABB             m_aabb;
        int              m_mask;

        VisualizeSolid& operator= (const VisualizeSolid&) { return *this; }
    };
}

void CellGenerator::visualizeSolidVoxels(const AABB& aabb, VoxelTraversal& vt, VisualizeHelper& vh)
{
    if (vt.isLeaf())
    {
        if (!vt.getLeafData().isSolid())
            return;

        VisualizeSolid vb(vh, aabb);
        NeighborFinder<VisualizeSolid>(vb).find(vt);
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            visualizeSolidVoxels(aabb2, vt2, vh);

            vt2.nextSibling();
        }
    }
}

namespace Umbra
{
    struct VisualizeBackface
    {
        VisualizeBackface(VisualizeHelper& vh, const AABB& aabb, const CellGraph& cg)
            : m_vh(vh), m_aabb(aabb), m_cg(cg), m_mask(0)
        {
        }

        bool collect(VoxelTraversal& vt, int face, int)
        {
            m_mask |= 1 << face;

            VoxelTree::LeafData& ld = vt.getLeafData();

            if (!ld.hasViewCell())
                return true;

            int cell = ld.getViewCellIndex();

            int color = m_cg.getCell(cell).isOutside() ? 0 : 1;

            if (color != 0)
            {
                return true;
            }

            float z = m_aabb.getFaceDist(face^1);
            AARectf rect = getBoxFaceRect(migrate(m_aabb), face);

            m_vh.voxelFace(face, z, rect, 0);

            return true;
        }

        void done(int) {}

        VisualizeHelper& m_vh;
        AABB             m_aabb;
        const CellGraph& m_cg;
        int              m_mask;

        VisualizeBackface& operator= (const VisualizeBackface&) { return *this; }
    };
}

void CellGenerator::visualizeBackface(const AABB& aabb, VoxelTraversal& vt, VisualizeHelper& vh)
{
    if (vt.isLeaf())
    {
        if (!vt.getLeafData().isSolid())
            return;

        VisualizeBackface vb(vh, aabb, m_cellGraph);
        NeighborFinder<VisualizeBackface>(vb).find(vt);

        // Render computation tile boundary as unknown.

#if 0
        for (int i = 0; i < 6; i++)
            if (!(vb.m_mask & (1 << i)))
            {
                int color = 2;

                float z = aabb.getFaceDist(i);
                Vector4 rect = aabb.getFaceRect(i);

                vv.insert(i, z, rect, color);
            }
#endif
    }
    else
    {
        VoxelTraversal vt2 = vt.firstChild();
        Vector3 p = aabb.getCenter();

        for (int i = 0; i < 8; i++)
        {
            AABB aabb2 = aabb;
            for (int axis = 0; axis < 3; axis++)
                if (i & (1 << axis))
                    aabb2.setMin(axis, p[axis]);
                else
                    aabb2.setMax(axis, p[axis]);

            visualizeBackface(aabb2, vt2, vh);

            vt2.nextSibling();
        }
    }
}


//
// TopCellGenerator
//

TopCellGenerator::TopCellGenerator(const PlatformServices& platform, const GeometryBlock& gb, const CellGeneratorParams& cgp, Vector3i idim, const RayTracer* rt, DebugCollector& dc)
:   m_platform(platform),
    m_geometryBlock(gb),
    m_idim(idim),
    m_rayTracer(rt),
    m_params(cgp),
    m_dc(dc),
    m_cellGraph(platform.allocator)
{
}

TopCellGenerator::~TopCellGenerator()
{
}

void TopCellGenerator::perform()
{
    UMBRA_ASSERT(isPowerOfTwo(m_idim.i));
    UMBRA_ASSERT(isPowerOfTwo(m_idim.j));
    UMBRA_ASSERT(isPowerOfTwo(m_idim.k));

    // Compute.

    Vector3i dim = m_idim; // make dimensions bigger (only shape matters, not quantity)
    while (max2(dim.i, max2(dim.j, dim.k)) < 1 << 30)
    {
        dim.i *= 2;
        dim.j *= 2;
        dim.k *= 2;
    }

    Array<int> tris(m_platform.allocator);
    for (int i = 0; i < m_geometryBlock.getTriangleCount(); i++)
        tris.pushBack(i);

    if (compute(m_cellGraph, m_params.aabb, dim, Vector3i(0, 0, 0), tris.getPtr(), tris.getSize()) > 1)
    {
        SubdivisionTree st(m_platform.allocator);
        m_cellGraph.getViewTree().deserialize(st);
        st.setRoot(SubdivisionTreeUtils(st).collapse(st.getRoot(), true));
        m_cellGraph.getViewTree().serialize(st);
    }

    // Checks.

    m_cellGraph.checkConsistency(CellGraph::BIDI | CellGraph::RAW);
}

int TopCellGenerator::compute(CellGraph& cg, const AABB& aabb, Vector3i idim, Vector3i level, int* tris, int numTris2)
{
    // Filter intersecting triangles.

    int numTris = 0;
    for (int i = 0; i < numTris2; i++)
    {
        Triangle tri;
        m_geometryBlock.getVertices(tris[i], tri.a, tri.b, tri.c);

        if (intersectAABBTriangle(aabb, tri.a, tri.b, tri.c))
            swap2(tris[i], tris[numTris++]);
    }

    bool occluders = false;

    for (int i = 0; i < numTris; i++)
        if (m_geometryBlock.getTriangleObject(tris[i]).isOccluder() ||
            m_geometryBlock.getTriangleObject(tris[i]).isGate())
        {
            occluders = true;
            break;
        }

    // Must be a cube for CellGenerator.
    // Also we must have either reached the cell level or smallest-hole level should not be further than 9 splits.

    int l = min2(level.i, min2(level.j, level.k));

    if (!occluders ||
        (idim.i == idim.j && idim.i == idim.k &&
         (idim.i <= 1 || m_params.cellLevel - l <= 0 || m_params.smallestHoleLevel - l <= 9)))
    {
        CellGeneratorParams p = m_params;

        p.aabb = aabb;
        p.cellLevel = max2(m_params.cellLevel - l, 0);
        p.smallestHoleLevel = max2(m_params.smallestHoleLevel - l, 0);

        // TODO: pass only intersected triangles to cellgen?
        // Note: not as is, because we need triangles intersecting inflated bounds

        CellGenerator(m_platform, cg, m_geometryBlock, p, m_rayTracer, m_params.aabb, m_dc).perform();
        return 1;
    }

    int axis = getLongestAxis(Vector3i(idim.k, idim.j, idim.i)); // Hack to pick z first when same.
    axis = 2 - axis;

    idim[axis] /= 2;

    level[axis]++;

    AABB leftAABB, rightAABB;
    aabb.splitHalf(axis, leftAABB, rightAABB);

    int ret = compute(cg, leftAABB, idim, level, tris, numTris);

    CellGraph right(m_platform.allocator);
    ret += compute(right, rightAABB, idim, level, tris, numTris);

    cg.joinRight(right, true, m_params.featureSize);
    return ret;
}

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)
