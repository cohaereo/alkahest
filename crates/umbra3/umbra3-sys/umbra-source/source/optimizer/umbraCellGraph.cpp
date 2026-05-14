#if !defined(UMBRA_EXCLUDE_COMPUTATION)

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
 * \brief   Cells and portals graph
 *
 */

#include "umbraCellGraph.hpp"
#include "umbraUnionFind.hpp"
#include "umbraConvexHull.hpp"
#include "umbraExtCellGraph.hpp"

using namespace Umbra;

CellGraph::CellGraph(Allocator* a)
:   m_viewTree(a),
    m_cells(a),
    m_targetObjs(a),
    m_portalExpand()
{
    for (int i = 0; i < (int)UMBRA_ARRAY_SIZE(m_matchingTree); i++)
        m_matchingTree[i].setAllocator(a);
}

void CellGraph::Cell::addObject (int idx, const AABB& aabb)
{
    AABB* cur = m_objects.get(idx);
    if (cur)
        cur->grow(aabb);
    else
        m_objects.insert(idx, aabb);
}

void CellGraph::Cell::removeObject (int idx)
{
    UMBRA_ASSERT(m_objects.contains(idx));
    m_objects.remove(idx);
}

void CellGraph::checkConsistency(Umbra::UINT32 flags) const
{
    UMBRA_UNREF(flags);

#ifdef UMBRA_DEBUG
    // Check bidirectionality.

    if (flags & BIDI)
    {
        for (int i = 0; i < getCellCount(); i++)
        {
            const CellGraph::Cell& c1 = getCell(i);

            for (int j = 0; j < c1.getPortalCount(); j++)
            {
                const CellGraph::Portal& p1 = c1.getPortal(j);
                const CellGraph::Cell& c2 = getCell(p1.getTarget());

                bool found = false;

                for (int k = 0; k < c2.getPortalCount(); k++)
                {
                    const CellGraph::Portal& p2 = c2.getPortal(k);

                    if (p1.isGate() || p2.isGate()) // TODO: check gates
                        continue;

                    if (p2.getTarget() != i || p1.getRectPortal().getFace() != (p2.getRectPortal().getFace()^1) || p1.getRectPortal().getZ() != p2.getRectPortal().getZ() || p1.getRectPortal().getRect() != p2.getRectPortal().getRect())
                        continue;

                    UMBRA_ASSERT(!found);
                    found = true;
                }

                UMBRA_UNREF(found);
                UMBRA_ASSERT(p1.isGate() || found);
            }
        }
    }

    // Portals must not overlap other cells.

    if (flags & RAW)
    {
        for (int i = 0; i < getCellCount(); i++)
        {
            const CellGraph::Cell& c1 = getCell(i);

            AABB portalAABB;
            for (int j = 0; j < c1.getRectPortalCount(); j++)
                portalAABB.grow(c1.getRectPortal(j).getAABB());

            if (!portalAABB.isOK())
                continue;

            for (int j = 0; j < c1.getRectPortalCount(); j++)
            {
                const CellGraph::RectPortal& p1 = c1.getRectPortal(j);
                const CellGraph::Cell& c2 = getCell(p1.getTarget());

                if (p1.isGate())
                    continue;

                UMBRA_ASSERT(!c1.getAABB().intersectsWithVolume(c2.getAABB()));

                AABB portalAABB2;
                for (int k = 0; k < c2.getRectPortalCount(); k++)
                    portalAABB2.grow(c2.getRectPortal(k).getAABB());

                UMBRA_ASSERT(!portalAABB2.isOK() || !portalAABB.intersectsWithVolume(portalAABB2));
            }
        }
    }

    // Check that view and matching trees make sense.

    if (!m_viewTree.isEmpty())
    {
        SubdivisionTree vst(getAllocator());
        m_viewTree.deserialize(vst);

        UMBRA_ASSERT(getAABB() == vst.getAABB());

        SubdivisionTree::LeafIterator iter;
        for (vst.iterate(iter); !iter.end(); iter.next())
        {
            UMBRA_ASSERT(iter.leaf()->getIndex() >= -1);
            UMBRA_ASSERT(iter.leaf()->getIndex() < getCellCount());
        }

        UMBRA_ASSERT(!SubdivisionTreeUtils(vst).hasInvalidNodes());
    }

    for (int face = 0; face < 6; face++)
    {
        if (getMatchingTree(face).isEmpty())
            continue;

        SubdivisionTree st(getAllocator());
        getMatchingTree(face).deserialize(st);

        SubdivisionTree::Iterator iter;
        for (st.iterate(iter); !iter.end(); iter.next())
            UMBRA_ASSERT(!iter.node()->isLeaf() || iter.node()->getLeaf()->getIndex() < getCellCount());

        UMBRA_ASSERT(st.getAABB().isOK());
        UMBRA_ASSERT(st.getAABB().isFlat());

        AABB aabb = getAABB();
        aabb.flattenToFace(face);

        UMBRA_ASSERT(st.getAABB() == aabb);

        UMBRA_ASSERT(!SubdivisionTreeUtils(st).hasInvalidNodes());
    }

#endif
}

void CellGraph::GatePortal::addHullVertices (const Vector4& pleq, const Array<Vector3>& verts)
{
    m_portalHulls.getDefault(pleq, PortalHull(m_portalHulls.getAllocator())).append(verts);
}

int CellGraph::assignClusters (int offset)
{
    // find clusters and set cluster id to cells

    UnionFind<int> clusters(getAllocator());

    for (int i = 0; i < getCellCount(); i++)
    {
        const Cell& cell = getCell(i);
        for (int j = 0; j < cell.getPortalCount(); j++)
        {
            if (!cell.getPortal(j).isGate())
                clusters.unionSets(i, cell.getPortal(j).getTarget());
        }
    }

    Hash<int, int> idToIdx(getAllocator());
    int numClusters = 0;

    for (int i = 0; i < getCellCount(); i++)
    {
        Cell& cell = getCell(i);
        cell.clearClusters();
        int id = clusters.findSet(i);
        if (!idToIdx.contains(id))
            idToIdx.insert(id, offset + numClusters++);
        cell.addClusterId(*idToIdx.get(id));
    }

    return numClusters;
}

// \todo Implement / find a proper minimum bounding sphere algorithm
Vector3 CellGraph::PortalHull::getCenter(void) const
{
    // Area weighted average of triangle centers - at least this is somewhat closer.
    Vector3 sum;
    float weightsum = 0;

    Vector3 v0 = m_vertices[0];
    for (int i = 1; i < m_vertices.getSize() - 1; i++)
    {
        Vector3 v1 = m_vertices[i];
        Vector3 v2 = m_vertices[i+1];
        float weight = cross(v1-v0, v2-v0).length(); // 2 * area
        sum += (v0 + v1 + v2) * weight;
        weightsum += weight;
    }

    Vector3 center = sum / (weightsum * 3);

    // Clamp center to AABB to make floating point stuff more robust.

    AABB aabb;
    for (int i = 0; i < m_vertices.getSize(); i++)
        aabb.grow(m_vertices[i]);

    center.x = max2(center.x, aabb.getMin().x);
    center.y = max2(center.y, aabb.getMin().y);
    center.z = max2(center.z, aabb.getMin().z);
    center.x = min2(center.x, aabb.getMax().x);
    center.y = min2(center.y, aabb.getMax().y);
    center.z = min2(center.z, aabb.getMax().z);

    return center;
}

float CellGraph::PortalHull::getMaxRadius(const Vector3& center) const
{
    float r2 = 0;
    for (int i = 0; i < m_vertices.getSize(); i++)
        r2 = max2(r2, (m_vertices[i] - center).lengthSqr());
    return sqrtf(r2);
}

float CellGraph::PortalHull::getMinRadius(const Vector3& center) const
{
    float r2 = FLT_MAX;
    for (int i = 0; i < m_vertices.getSize(); i++)
    {
        Vector3 start = m_vertices[i];
        Vector3 end = m_vertices[(i + 1) % m_vertices.getSize()];
        // squared distance between center and hull edge
        float distanceSqr = cross(center - start, center - end).lengthSqr() / (end - start).lengthSqr();
        r2 = min2(r2, distanceSqr);
    }
    return sqrtf(r2);
}

struct SortableVec3
{
    SortableVec3(void) {}
    SortableVec3(const Vector3& v): v(v) {}

    bool operator< (const SortableVec3& o) const
    {
        if (v.x == o.v.x)
        {
            if (v.y == o.v.y)
            {
                return v.z < o.v.z;
            }
            return v.y < o.v.y;
        }
        return v.x < o.v.x;
    }

    bool operator> (const SortableVec3& o) const
    {
        if (v.x == o.v.x)
        {
            if (v.y == o.v.y)
            {
                return v.z > o.v.z;
            }
            return v.y > o.v.y;
        }
        return v.x > o.v.x;
    }

    Vector3 v;
};

bool CellGraph::GatePortal::simplifyPortalHulls()
{
    if (!m_portalHulls.getNumKeys())
        return true;

    Hash<Vector4, PortalHull>::Iterator iter = m_portalHulls.iterate();
    Hash<Vector4, PortalHull> simplified(m_portalHulls.getAllocator());

    while (m_portalHulls.isValid(iter))
    {
        const Vector4& pleq = m_portalHulls.getKey(iter);
        PortalHull& hull = m_portalHulls.getValue(iter);

        // sort input vertices here to produce consistent hulls
        Array<SortableVec3> sortedInput(m_portalHulls.getAllocator());
        for (int i = 0; i < hull.getVertexCount(); i++)
            sortedInput.pushBack(hull.getVertex(i));
        quickSort(sortedInput.getPtr(), sortedInput.getSize());

        // determine projection axis
        Vector3 axisLen = absv(pleq.xyz());
        int projectionAxis = axisLen.x > axisLen.y ?
            (axisLen.x > axisLen.z ? 0 : 2) : (axisLen.y > axisLen.z ? 1 : 2);
        int axis0 = (projectionAxis + 1) % 3;
        int axis1 = (projectionAxis + 2) % 3;

        // build 2d convex hull

        ConvexHull2D builder(m_portalHulls.getAllocator());
        Hash<Vector2, int> ptHash(m_portalHulls.getAllocator());
        for (int i = 0; i < sortedInput.getSize(); i++)
        {
            Vector2 in;
            in.x = sortedInput[i].v[axis0];
            in.y = sortedInput[i].v[axis1];

            if (ptHash.contains(in))
                continue;
            ptHash.insert(in, i);
            builder.addPoint(in);
        }

        UMBRA_ASSERT(builder.testHullness());

        // gather output

        if (builder.getHull().getSize() >= 3)
        {
            PortalHull* newHull = simplified.insert(pleq, PortalHull(m_portalHulls.getAllocator()));
            for (int i = 0; i < builder.getHull().getSize(); i++)
                newHull->add(sortedInput[*ptHash.get(builder.getHull()[i])].v);
        }

        m_portalHulls.next(iter);
    }
    m_portalHulls = simplified;
    return (m_portalHulls.getNumKeys() > 0);
}

void CellGraph::simplifyPortalHulls()
{
    for (int i = 0; i < getCellCount(); i++)
    {
        CellGraph::Cell& c = getCell(i);
        Array<GatePortal> portals(getAllocator());

        for (int j = 0; j < c.getGatePortalCount(); j++)
        {
            CellGraph::GatePortal& p = c.getGatePortal(j);
            if (p.simplifyPortalHulls())
                portals.pushBack(p);
        }

        if (c.getGatePortalCount() != portals.getSize())
        {
            c.clearGatePortals();
            for (int j = 0; j < portals.getSize(); j++)
                c.addGatePortal(portals[j]);
        }
    }
}

static int findFace(const AABB& a, const AABB& b)
{
    UMBRA_ASSERT(!a.intersectsWithVolume(b));
    UMBRA_ASSERT(a.intersectsWithArea(b));

    for (int i = 0; i < 3; i++)
        if (a.getMax()[i] == b.getMin()[i])
            return (i << 1) | 1;
        else
            if (a.getMin()[i] == b.getMax()[i])
                return (i << 1);

    UMBRA_ASSERT(0);
    return 0;
}

static void joinSubdivisionTrees(Allocator* a, SubdivisionTreeSerialization& out, const SubdivisionTreeSerialization& left, const SubdivisionTreeSerialization& right, int firstRight, int axis, float z)
{
    UMBRA_ASSERT(left.getAABB().getFaceRect(axis<<1) == right.getAABB().getFaceRect(axis<<1));
    UMBRA_ASSERT(left.getAABB().getFaceDist((axis<<1)|1) == right.getAABB().getFaceDist(axis<<1));

#if 0
    // TODO: do this in serialized form!
    SubdivisionTree st(a);

    right.deserialize(st);
    AABB rightAABB = st.getAABB();
    SubdivisionTree::Node* rightRoot = st.getRoot();

    SubdivisionTree::LeafIterator iter;
    for (st.iterate(iter); !iter.end(); iter.next())
        if (iter.node()->getLeaf()->getIndex() >= 0)
            iter.node()->getLeaf()->setIndex(iter.node()->getLeaf()->getIndex() + firstRight);

    left.deserialize(st);
    AABB leftAABB = st.getAABB();
    SubdivisionTree::Node* leftRoot = st.getRoot();

    SubdivisionTree::Node* root;

    float dimDiff = leftAABB.getDimensions()[axis] - rightAABB.getDimensions()[axis];
    if (fabsf(dimDiff) <= 0.f)
    {
        root = st.newMedian();
        root->getMedian()->setAxis(axis);
    }
    else
    {
        root = st.newAxial();
        root->getAxial()->setAxis(axis);
        root->getAxial()->setPos(z);
    }

    root->getInner()->setLeft(leftRoot);
    root->getInner()->setRight(rightRoot);
    st.setRoot(root);

    leftAABB.grow(rightAABB);
    st.setAABB(leftAABB);

    UMBRA_ASSERT(!SubdivisionTreeUtils(st).hasInvalidNodes());

    out.serialize(st);
#else
    UMBRA_UNREF(a);
    SubdivisionTreeSerialization tmp = left;
    out.join(axis, z, firstRight, tmp, right);
#endif
}

void CellGraph::joinRight(const CellGraph& other, bool connectionPortals, float featureSize)
{
    UMBRA_ASSERT(this != &other);
    checkConsistency(BIDI);
    other.checkConsistency(BIDI);

    int face = findFace(getAABB(), other.getAABB());
    UMBRA_ASSERT((face & 1) == 1 || !"Only supports joining from right");
    UMBRA_ASSERT(getAABB().getFaceRect(face) == other.getAABB().getFaceRect(face^1));
    UMBRA_ASSERT(getAABB().getFaceDist(face) == other.getAABB().getFaceDist(face^1));
    float z = getAABB().getMax()[face>>1];

    // Append targets.

    Hash<UINT32, int> objMap(getAllocator());
    for (int i = 0; i < getTargetObjectCount(); i++)
    {
        if (!objMap.contains(getTargetObject(i).getId()))
            objMap.insert(getTargetObject(i).getId(), i);
    }

    for (int i = 0; i < other.getTargetObjectCount(); i++)
    {
        UINT32 id = other.getTargetObject(i).getId();
        if (!objMap.contains(id))
        {
            objMap.insert(id, getTargetObjectCount());
            addTargetObject(other.getTargetObject(i));
        }
    }

    // Append cells.

    int firstCell = getCellCount();
    if (other.getCellCount())
    {
        addCell(other.getCellCount());
        for (int i = 0; i < other.getCellCount(); i++)
        {
            const Cell& src = other.getCell(i);
            Cell& c = getCell(firstCell + i);
            c = src;

            for (int j = 0; j < c.getPortalCount(); j++)
            {
                Portal& p = c.getPortal(j);
                p.setTarget(p.getTarget() + firstCell);
            }

            c.clearObjects();
            Hash<int, AABB>::Iterator objIter = src.m_objects.iterate();
            while (src.m_objects.isValid(objIter))
            {
                int key = src.m_objects.getKey(objIter);
                const AABB& val = src.m_objects.getValue(objIter);
                UINT32 id = other.getTargetObject(key).getId();
                int* newkey = objMap.get(id);
                UMBRA_ASSERT(newkey);
                c.addObject(*newkey, val);
                src.m_objects.next(objIter);
            }
        }
    }

    // Connection portals.

    if (connectionPortals)
    {
        ExternalCellGraph ecg(this);
        ecg.connectTo(&other, 0, featureSize);

        for (int i = 0; i < ecg.getCellCount(); i++)
        {
            const ExternalCellGraph::Cell& c = ecg.getCell(i);

            for (int j = 0; j < c.getPortalCount(); j++)
            {
                const ExternalCellGraph::Portal& ep = c.getPortal(j);

                CellGraph::RectPortal p;
                p.setFace(ep.getFace());
                p.setRect(ep.getRect());
                p.setZ(ep.getZ());
                p.setTarget(firstCell + ep.getTarget());

                getCell(i).addRectPortal(p);

                p.setFace(p.getFace() ^ 1);
                p.setTarget(i);

                getCell(firstCell + ep.getTarget()).addRectPortal(p);
            }
        }
    }

    // Join view trees.

    if (!m_viewTree.isEmpty())
        joinSubdivisionTrees(getAllocator(), m_viewTree, m_viewTree, other.m_viewTree, firstCell, face >> 1, z);

    // Join matching trees.

    for (int f = 0; f < 6; f++)
    {
        if (m_matchingTree[f].isEmpty())
            continue;

        if (f == face)
        {
            SubdivisionTree st(getAllocator());
            other.getMatchingTree(f).deserialize(st);

            SubdivisionTree::LeafIterator iter;
            for (st.iterate(iter); !iter.end(); iter.next())
                if (iter.node()->getLeaf()->getIndex() >= 0)
                    iter.node()->getLeaf()->setIndex(iter.node()->getLeaf()->getIndex() + firstCell);

            getMatchingTree(f).serialize(st);
        }
        else if (f != (face^1))
        {
            joinSubdivisionTrees(getAllocator(), getMatchingTree(f), getMatchingTree(f), other.getMatchingTree(f), firstCell, face >> 1, z);
        }
    }

    m_aabb.grow(other.getAABB());

    m_portalExpand = max2(m_portalExpand, other.m_portalExpand);

    checkConsistency(BIDI);
}


void CellGraph::clone (CellGraph& dst, bool viewTree, bool matchingTrees) const
{
    dst.m_aabb = m_aabb;
    dst.m_cells = m_cells;
    dst.m_targetObjs = m_targetObjs;
    if (viewTree)
        dst.m_viewTree = m_viewTree;
    if (matchingTrees)
    {
        for (int i = 0; i < 6; i++)
            dst.m_matchingTree[i] = m_matchingTree[i];
    }
    dst.m_portalExpand = m_portalExpand;
}

void CellGraph::remapCells(const CellRemap& remap)
{
    UMBRA_ASSERT(remap.getSize() == getCellCount());

    int maxCell = remap.getLastTarget();
    int numCells = maxCell+1;

    // View tree.

    if (!m_viewTree.isEmpty())
    {
        if (m_viewTree.canRemapIndices())
            m_viewTree.remapIndices(remap.getArray());
        else
        {
            SubdivisionTree st(getAllocator());
            m_viewTree.deserialize(st);
            SubdivisionTreeUtils::remapLeafIndices(st, remap.getArray());
            m_viewTree.serialize(st);
        }
    }

    // Matching trees.

    for (int i = 0; i < 6; i++)
    {
        if (m_matchingTree[i].isEmpty())
            continue;

        if (m_matchingTree[i].canRemapIndices())
            m_matchingTree[i].remapIndices(remap.getArray());
        else
        {
            SubdivisionTree st(getAllocator());
            m_matchingTree[i].deserialize(st);
            SubdivisionTreeUtils::remapLeafIndices(st, remap.getArray());
            m_matchingTree[i].serialize(st);
        }
    }

    // Cells.

    for (int i = 0; i < numCells; i++)
    {
        int src = remap.reverseMap(i);
        UMBRA_ASSERT(src >= 0 && i <= src);
        if (i != src)
            m_cells[i] = m_cells[src];

        Cell& c = m_cells[i];
        for (int j = 0; j < c.getPortalCount(); j++)
        {
            Portal& p = c.getPortal(j);
            int mapped = remap.map(p.getTarget());
            if (mapped < 0)
            {
                // TODO: support removing portals here?
                UMBRA_ASSERT(!"Cell remap failure: portal leading to removed cell");
                continue;
            }
            UMBRA_ASSERT(mapped < numCells);
            p.setTarget(mapped);
        }
    }
    m_cells.resize(numCells);
}

void CellGraph::removeNonConnectedCells()
{
    // Union reachable stuff to a single set.

    UnionFind<int> uf(getAllocator());

    int reachableCellIdx = getCellCount();

    for (int i = 0; i < getCellCount(); i++)
        for (int j = 0; j < getCell(i).getPortalCount(); j++)
            uf.unionSets(i, getCell(i).getPortal(j).getTarget());

    {
        SubdivisionTree st(getAllocator());
        m_viewTree.deserialize(st);

        SubdivisionTree::LeafIterator iter;
        for (st.iterate(iter); !iter.end(); iter.next())
            uf.unionSets(reachableCellIdx, iter.leaf()->getIndex());
    }

    for (int i = 0; i < 6; i++)
    {
        SubdivisionTree st(getAllocator());
        m_matchingTree[i].deserialize(st);

        SubdivisionTree::LeafIterator iter;
        for (st.iterate(iter); !iter.end(); iter.next())
            uf.unionSets(reachableCellIdx, iter.leaf()->getIndex());
    }

    // Remap.

    CellRemap remap(getAllocator(), getCellCount());

    int reachableSet = uf.findSet(reachableCellIdx);

    int n = 0;
    for (int i = 0; i < getCellCount(); i++)
        if (uf.findSet(i) == reachableSet)
            remap.set(i, n++);

    remapCells(remap);
}

void CellGraph::optimizeMemoryUsage()
{
    m_cells.shrinkToFit(true);
    m_targetObjs.shrinkToFit(true);

    for (int i = 0; i < m_cells.getSize(); i++)
    {
        Cell& c = m_cells[i];
        c.m_rectPortals.shrinkToFit(true);
        c.m_gatePortals.shrinkToFit(true);
        c.m_clusters.shrinkToFit(true);
    }
}

void CellGraph::computeOutsideness()
{
    for (int i = 0; i < getCellCount(); i++)
        getCell(i).setOutside(true);

    SubdivisionTree st(getAllocator());
    m_viewTree.deserialize(st);

    SubdivisionTree::LeafIterator iter;
    for (st.iterate(iter); !iter.end(); iter.next())
    {
        int idx = iter.leaf()->getIndex();
        if (idx >= 0)
            getCell(idx).setOutside(false);
    }
}

void CellGraph::removeTargetObjectsById(const Set<Umbra::UINT32>& removeSet)
{
    Hash<int, int> objRemap(getAllocator());
    Array<ObjectParams> src = m_targetObjs;
    m_targetObjs.clear();

    for (int i = 0; i < src.getSize(); i++)
    {
        if (removeSet.contains(src[i].getId()))
            continue;
        objRemap.insert(i, m_targetObjs.getSize());
        m_targetObjs.pushBack(src[i]);
    }

    if (src.getSize() == m_targetObjs.getSize())
        return;

    for (int i = 0; i < m_cells.getSize(); i++)
    {
        CellGraph::Cell& c = m_cells[i];
        Hash<int, AABB> oldHash = c.m_objects;
        c.m_objects.clear();
        Hash<int, AABB>::Iterator iter = oldHash.iterate();
        while (oldHash.isValid(iter))
        {
            int oldIdx = oldHash.getKey(iter);
            if (objRemap.contains(oldIdx))
                c.m_objects.insert(*objRemap.get(oldIdx), oldHash.getValue(iter));
            oldHash.next(iter);
        }
    }
}

#endif // UMBRA_EXCLUDE_COMPUTATION
