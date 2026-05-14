#pragma once

#include "umbraCellGraph.hpp"
#include "umbraRectGrouper.hpp"

namespace Umbra
{

/* External cell graph is a cell graph to other tiles. */

class ExternalCellGraph
{
public:
    struct Portal
    {
        int getTargetTile() const           { return m_tileIdx; }
        int getTarget() const               { return m_target; }
        void setTarget(int t)               { m_target = t; }
        int getFace() const                 { return m_face; };

        AABB getAABB() const
        {
            int axis = m_face >> 1;
            Vector4 r = m_rect;

            AABB aabb;
            aabb.setMin(axis, m_z);
            aabb.setMax(axis, m_z);
            aabb.setMin((axis+1)%3, r.x);
            aabb.setMax((axis+1)%3, r.z);
            aabb.setMin((axis+2)%3, r.y);
            aabb.setMax((axis+2)%3, r.w);
            return aabb;
        }

        const Vector4&  getRect() const     { return m_rect; }
        float           getZ() const        { return m_z; }

        void setAllocator(Allocator* a) { UMBRA_UNREF(a); }

        int             m_tileIdx;
        int             m_target;
        int             m_face;
        Vector4         m_rect;
        float           m_z;
    };

    struct Cell
    {
        int getPortalCount() const { return m_portals.getSize(); }
        const Portal& getPortal(int i) const { return m_portals[i]; }
        Portal& getPortal(int i) { return m_portals[i]; }
        void addPortal(const Portal& p) { m_portals.pushBack(p); }
        void removeLastPortal() { m_portals.popBack(); }
        void clearPortals() { m_portals.clear(); }
        void setAllocator(Allocator* a) { m_portals.setAllocator(a); }

        Array<Portal>   m_portals;
    };

    ExternalCellGraph(Allocator* a)
        : m_allocator(a), m_cellGraph(NULL), m_cells(a)
    {
    }

    ExternalCellGraph(const CellGraph* cg)
        : m_allocator(cg->getAllocator()), m_cellGraph(NULL), m_cells(cg->getAllocator())
    {
        setCellGraph(cg);
    }

    Allocator* getAllocator(void) const { return m_allocator; }

    void setCellGraph(const CellGraph* cg)
    {
        UMBRA_ASSERT(cg);

        m_cellGraph = cg;
        m_cells = Array<Cell>(m_allocator);
        m_cells.reset(m_cellGraph->getCellCount());
    }

    void connectTo(const CellGraph* neighbor, int tileIdx, float featureSize)
    {
        UMBRA_ASSERT(m_cellGraph);
        UMBRA_ASSERT(neighbor);
        UMBRA_ASSERT(m_cellGraph->getAABB().intersectsWithArea(neighbor->getAABB()));
        UMBRA_ASSERT(!m_cellGraph->getAABB().intersectsWithVolume(neighbor->getAABB()));

        // Determine which face to match.

        int face = -1;
        for (int axis = 0; axis < 3; axis++)
            if (neighbor->getAABB().getMax()[axis] == m_cellGraph->getAABB().getMin()[axis])
            {
                UMBRA_ASSERT(face == -1);
                face = axis << 1;
            }
            else if (neighbor->getAABB().getMin()[axis] == m_cellGraph->getAABB().getMax()[axis])
            {
                UMBRA_ASSERT(face == -1);
                face = (axis << 1) | 1;
            }
        UMBRA_ASSERT(face != -1);

        m_curTileIdx = tileIdx;
        m_curFace = face;
        m_curZ = m_cellGraph->getAABB().getFaceDist(face);
        AABB clamp = m_cellGraph->getAABB();
        clamp.clamp(neighbor->getAABB());
        m_curClamp = clamp.getFaceRect(face);
        m_curFeatureSize = featureSize;

        SubdivisionTree selfST(m_allocator);
        m_cellGraph->getMatchingTree(face).deserialize(selfST);

        SubdivisionTree neighborST(m_allocator);
        neighbor->getMatchingTree(face^1).deserialize(neighborST);

        match(selfST.getRoot(), selfST.getAABB(),
              neighborST.getRoot(), neighborST.getAABB());

        groupPortals(tileIdx, featureSize);

#ifdef UMBRA_DEBUG
        for (int i = 0; i < getCellCount(); i++)
        {
            const Cell& c = getCell(i);
            for (int j = 0; j < c.getPortalCount(); j++)
            {
                const Portal& p = c.getPortal(j);
                if (p.getTargetTile() != tileIdx)
                    continue;

                UMBRA_ASSERT(m_cellGraph->getAABB().contains(p.getAABB()));
                UMBRA_ASSERT(neighbor->getAABB().contains(p.getAABB()));
            }
        }
#endif
    }

    void groupPortals(int targetTileIdx, float featureSize)
    {
        for (int i = 0; i < getCellCount(); i++)
        {
            Cell& c = getCell(i);

            Array<Portal> newPortals(getAllocator());

            Set<int> usedTargetCells(getAllocator());

            for (int j = 0; j < c.getPortalCount(); j++)
            {
                Portal& p = c.getPortal(j);

                if (p.getTargetTile() != targetTileIdx)
                {
                    newPortals.pushBack(p);
                    continue;
                }

                if (usedTargetCells.contains(p.getTarget()))
                    continue;
                usedTargetCells.insert(p.getTarget());

                RectGrouper rg(getAllocator());

                if (featureSize > 0.f)
                {
                    rg.setStrategy(RectGrouper::FOUR_QUADRANTS);
                    rg.setThreshold(featureSize * featureSize);
                }
                else
                    rg.setStrategy(RectGrouper::COMBINE_ALL);

                for (int k = 0; k < c.getPortalCount(); k++)
                {
                    const Portal& p2 = c.getPortal(k);

                    if (p2.getTargetTile() == targetTileIdx && p2.getTarget() == p.getTarget())
                        rg.addRect(p2.getRect());
                }

                rg.execute();

                for (int i = 0; i < rg.getResult().getSize(); i++)
                {
                    Portal np = p;
                    np.m_rect = rg.getResult()[i];
                    newPortals.pushBack(np);
                }
            }

            c.m_portals = newPortals;
        }
    }

    void connectBorder(int face)
    {
        m_curTileIdx = -1;
        m_curFace    = face;
        m_curZ       = m_cellGraph->getAABB().getFaceDist(face);
        m_curClamp   = m_cellGraph->getAABB().getFaceRect(face);
        m_curFeatureSize = 0.f;

        SubdivisionTree st(m_allocator);
        m_cellGraph->getMatchingTree(face).deserialize(st);

        matchBorder(st.getRoot(), st.getAABB());
    }

    void remapCells (const CellRemap& remap, const Array<CellRemap>& remaps)
    {
        int numCells = remap.getLastTarget() + 1;
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
                if (p.getTargetTile() < 0)
                    continue;
                const CellRemap& tgtRemap = remaps[p.getTargetTile()];
                int mapped = tgtRemap.map(p.getTarget());
                if (mapped < 0)
                {
                    // TODO: support removing portals here?
                    UMBRA_ASSERT(!"Cell remap failure: portal leading to removed cell");
                    continue;
                }
                p.setTarget(mapped);
            }
        }
        m_cells.resize(numCells);
    }

    int getCellCount(void) const { return m_cells.getSize(); }
    const Cell& getCell(int i) const { return m_cells[i]; }
    Cell& getCell(int i) { return m_cells[i]; }

    Cell& addCell() { m_cells.resize(m_cells.getSize()+1); return m_cells[m_cells.getSize()-1]; }

    void optimizeMemoryUsage()
    {
        for (int i = 0; i < m_cells.getSize(); i++)
            m_cells[i].m_portals.shrinkToFit(true);
    }

private:
    void match(const SubdivisionTree::Node* anode, const AABB& arect, const SubdivisionTree::Node* bnode, const AABB& brect)
    {
        if (!arect.intersectsWithArea(brect)) // TODO: optimize
            return;

        if (anode->isLeaf() && bnode->isLeaf())
        {
            int avc = anode->getLeaf()->getIndex();
            int bvc = bnode->getLeaf()->getIndex();

            if (avc < 0 || bvc < 0)
                return;

            Array<Portal>& portals = m_cells[avc].m_portals;

            AABB aabb = arect;
            aabb.clamp(brect);
            UMBRA_ASSERT(aabb.isOK());

            Vector4 r = aabb.getFaceRect(m_curFace);

            r = rectIntersection(r, m_curClamp);
            UMBRA_ASSERT(rectArea(r) > 0.f);

            bool found = false;

            if (m_curFeatureSize == 0.f)
                for (int i = 0; i < portals.getSize(); i++)
                {
                    Portal& p = portals[i];
                    if (p.m_face == m_curFace && p.m_target == bvc && p.m_tileIdx == m_curTileIdx)
                    {
                        found = true;
                        p.m_rect = rectUnion(p.m_rect, r);
                        break;
                    }
                }

            if (!found)
            {
                Portal p;
                p.m_tileIdx = m_curTileIdx;
                p.m_target = bvc;
                p.m_face = m_curFace;
                p.m_rect = r;
                p.m_z = m_curZ;
                portals.pushBack(p);
            }

            return;
        }

        bool explodeA;
        if (!anode->isLeaf() && !bnode->isLeaf())
            explodeA = arect.getSurfaceArea() >= brect.getSurfaceArea();
        else
            explodeA = anode->isLeaf() ? false : true;

        AABB left, right;

        if (explodeA)
        {
            SubdivisionTreeUtils::splitBounds(anode, arect, left, right);
            match(anode->getInner()->getLeft(), left, bnode, brect);
            match(anode->getInner()->getRight(), right, bnode, brect);
        }
        else
        {
            SubdivisionTreeUtils::splitBounds(bnode, brect, left, right);
            match(anode, arect, bnode->getInner()->getLeft(), left);
            match(anode, arect, bnode->getInner()->getRight(), right);
        }
    }

    void matchBorder(const SubdivisionTree::Node* node, const AABB& rect)
    {
        UMBRA_ASSERT(rect.isOK());

        if (node->isLeaf())
        {
            int vc = node->getLeaf()->getIndex();
            if (vc < 0)
                return;

            Array<Portal>& portals = m_cells[vc].m_portals;
            Vector4 r = rect.getFaceRect(m_curFace);

            r = rectIntersection(r, m_curClamp);
            UMBRA_ASSERT(rectArea(r) > 0.f);

            bool found = false;

            for (int i = 0; i < portals.getSize(); i++)
            {
                Portal& p = portals[i];
                if (p.m_face == m_curFace && p.m_target == -1 && p.m_tileIdx == -1)
                {
                    found = true;
                    p.m_rect = rectUnion(p.m_rect, r);
                    break;
                }
            }

            if (!found)
            {
                Portal p;
                p.m_tileIdx = -1;
                p.m_target = -1;
                p.m_face = m_curFace;
                p.m_rect = r;
                p.m_z = m_curZ;
                portals.pushBack(p);
            }

            return;
        }

        AABB left, right;

        SubdivisionTreeUtils::splitBounds(node, rect, left, right);
        matchBorder(node->getInner()->getLeft(), left);
        matchBorder(node->getInner()->getRight(), right);
    }

    Allocator*          m_allocator;

    const CellGraph*    m_cellGraph;
    Array<Cell>         m_cells;

    int                 m_curTileIdx;
    int                 m_curFace;
    float               m_curZ;
    Vector4             m_curClamp;
    float               m_curFeatureSize;
};

static inline void copyHeap (ExternalCellGraph::Cell* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

static inline void copyHeap (ExternalCellGraph::Portal* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

}
