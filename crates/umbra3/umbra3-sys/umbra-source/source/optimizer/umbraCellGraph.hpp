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
 * \brief   Cells and portals graph
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVoxelTree.hpp"
#include "umbraSerializer.hpp"
#include "umbraHash.hpp"
#include "umbraSort.hpp"
#include "umbraGeometryBlock.hpp"
#include "umbraSet.hpp"
#include "umbraSubdivisionTree.hpp"
#include "umbraPrimitives.hpp"

namespace Umbra
{

class CellRemap;

class CellGraph
{
public:
    struct PortalHull
    {
        PortalHull(Allocator* a = NULL): m_vertices(a) {}

        Vector3 getCenter() const;
        float getMaxRadius(const Vector3& center) const;
        float getMinRadius(const Vector3& center) const;

        void add (const Vector3& vert) { m_vertices.pushBack(vert); }
        void append (const Array<Vector3>& verts) { m_vertices.append(verts); }
        void reset (void) { m_vertices.clear(); }

        int getVertexCount() const { return m_vertices.getSize(); }
        const Vector3& getVertex(int idx) const { return m_vertices[idx]; }
        const Array<Vector3>& getVertices(void) const { return m_vertices; }

        void setAllocator(Allocator* heap) { m_vertices.setAllocator(heap); }
        bool operator== (const PortalHull& o) { return m_vertices == o.m_vertices; }

        template<typename OP> void streamOp (OP& op)
        {
            stream(op, m_vertices);
        }

    private:
        Array<Vector3> m_vertices;
    };

    struct RectPortal;
    struct GatePortal;

    struct Portal
    {
        Portal()
        {
            m_target = 0;
            m_face = 6; // invalid
        }

        int     getTarget() const { return m_target; }
        void    setTarget(int idx) { m_target = idx; }

        bool    isGate() const { UMBRA_ASSERT(m_face != 6); return m_face == 7; }

        const RectPortal& getRectPortal() const { UMBRA_ASSERT(!isGate()); return (const RectPortal&)*this; }
        RectPortal& getRectPortal() { UMBRA_ASSERT(!isGate()); return (RectPortal&)*this; }
        const GatePortal& getGatePortal() const { UMBRA_ASSERT(isGate()); return (const GatePortal&)*this; }
        GatePortal& getGatePortal() { UMBRA_ASSERT(isGate()); return (GatePortal&)*this; }

    protected:
        unsigned int       m_target    : 29;
        unsigned int       m_face      : 3;    // 7 == gate
    };

    struct RectPortal : public Portal
    {
        RectPortal()
        {
            m_z = 0.f;
        }

        int     getFace() const { UMBRA_ASSERT(!isGate()); return m_face; }
        int     getAxis() const { UMBRA_ASSERT(!isGate()); return m_face >> 1; }
        void    setFace(int f) { UMBRA_ASSERT(f >= 0 && f < 6); m_face = f; }

        float   getZ() const { return m_z; }
        void    setZ(float z) { m_z = z; }

        const Vector4& getRect() const { UMBRA_ASSERT(!isGate()); return m_rect; }
        void    setRect(const Vector4& r) { UMBRA_ASSERT(!isGate()); m_rect = r; }

        AABB getAABB() const
        {
            int axis = getAxis();
            Vector4 r = getRect();

            AABB aabb;
            aabb.setMin(axis, getZ());
            aabb.setMax(axis, getZ());
            aabb.setMin((axis+1)%3, r.x);
            aabb.setMax((axis+1)%3, r.z);
            aabb.setMin((axis+2)%3, r.y);
            aabb.setMax((axis+2)%3, r.w);
            return aabb;
        }

        template<typename OP> void streamOp (OP& op)
        {
            UINT32 packed = (m_target << 3) | m_face;
            stream(op, packed);
            m_target = packed >> 3;
            m_face = packed & 0x7;

            UMBRA_ASSERT(m_face != 6);

            stream(op, m_rect);
            stream(op, m_z);
        }

    private:
        Vector4 m_rect;
        float   m_z;
    };

    struct GatePortal : public Portal
    {
        void setAllocator (Allocator* heap)
        {
            m_gateIDs.setAllocator(heap);
            m_portalHulls.setAllocator(heap);
        }

        const Hash<Vector4, PortalHull>& getPortalHulls (void) const { return m_portalHulls; }
        void    addHullVertices (const Vector4& pleq, const Array<Vector3>& verts);
        bool    simplifyPortalHulls (void);

        const Set<int>& getGateIDs() const { UMBRA_ASSERT(isGate()); return m_gateIDs; }
        void    setGates(const Set<int>& gateIDs) { m_face = 7; m_gateIDs = gateIDs; UMBRA_ASSERT(m_gateIDs.getSize() > 0); }

        template<typename OP> void streamOp (OP& op)
        {
            UINT32 packed = (m_target << 3) | m_face;
            stream(op, packed);
            m_target = packed >> 3;
            m_face = packed & 0x7;

            UMBRA_ASSERT(m_face != 6);

            stream(op, m_gateIDs);
            stream(op, m_portalHulls);
        }

    private:
        Set<int>                  m_gateIDs;
        Hash<Vector4, PortalHull> m_portalHulls;
    };

    class Cell
    {
    public:
        Cell(void)
        : m_outside(0), m_reachable(0), m_forceReachable(0), m_aabb() {}

        void setAllocator (Allocator* heap)
        {
            m_objects.setAllocator(heap);
            m_rectPortals.setAllocator(heap);
            m_gatePortals.setAllocator(heap);
            m_clusters.setAllocator(heap);
        }

        void addObject      (int idx, const AABB& aabb);
        void removeObject   (int idx);
        int  getObjectCount (void) const { return m_objects.getNumKeys(); }
        void getObjects     (Array<int>& objs) const { m_objects.getKeyArray(objs); }
        void getObjectBounds(Array<AABB>& bounds) const { m_objects.getValueArray(bounds); }
        bool hasObject      (int idx) const { return m_objects.contains(idx); }
        void clearObjects   (void) { m_objects.clear(); }

        bool isOutside      (void) const { return m_outside ? true : false; }
        void setOutside     (bool b)     { m_outside = b ? 1 : 0; }
        bool isReachable    (void) const { return m_reachable ? true : false; }
        void setReachable   (bool b)     { m_reachable = b ? 1 : 0; }
        bool isForceReachable (void) const { return m_forceReachable ? true : false; }
        void setForceReachable(bool b)     { m_forceReachable = b ? 1 : 0; }

        void    addRectPortal(const RectPortal& p) { m_rectPortals.pushBack(p); }
        void    addGatePortal(const GatePortal& p) { m_gatePortals.pushBack(p); }

        int     getRectPortalCount() const { return m_rectPortals.getSize(); }
        int     getGatePortalCount() const { return m_gatePortals.getSize(); }
        RectPortal& getRectPortal(int i)   { return m_rectPortals[i]; }
        const RectPortal& getRectPortal(int i) const { return m_rectPortals[i]; }
        GatePortal& getGatePortal(int i)   { return m_gatePortals[i]; }
        const GatePortal& getGatePortal(int i) const { return m_gatePortals[i]; }
        void    removeLastRectPortal()      { m_rectPortals.popBack(); }
        void    clearPortals()          { m_rectPortals.clear(); m_gatePortals.clear(); }
        void    clearRectPortals()          { m_rectPortals.clear(); }
        void    clearGatePortals()          { m_gatePortals.clear(); }

        // Obsolete portal accessing interface.

        int           getPortalCount() const { return m_rectPortals.getSize() + m_gatePortals.getSize(); }
        const Portal& getPortal(int i) const { if (i < m_rectPortals.getSize()) return m_rectPortals[i]; else return m_gatePortals[i-m_rectPortals.getSize()]; }
        Portal& getPortal(int i) { if (i < m_rectPortals.getSize()) return m_rectPortals[i]; else return m_gatePortals[i-m_rectPortals.getSize()]; }

        void    clearClusters(void)     { m_clusters.clear(); }
        void    addClusterId(int id)    { m_clusters.pushBack(id); }
        const Array<int>& getClusters(void) const { return m_clusters; }
        Array<int>& getClusters(void)   { return m_clusters; }

        void        setAABB(const AABB& aabb) { m_aabb = aabb; }
        const AABB& getAABB(void) const       { return m_aabb; }
        void        growAABB(const AABB& aabb) { m_aabb.grow(aabb); }

        template<typename OP> void streamOp (OP& op)
        {
            UINT32 packed = (m_forceReachable << 3) | (m_reachable << 1) | m_outside;
            stream(op, packed);
            m_outside        = packed & 1;
            m_reachable      = (packed >> 1) & 1;
            m_forceReachable = (packed >> 3) & 1;
            stream(op, m_aabb);
            stream(op, m_objects);
            stream(op, m_rectPortals);
            stream(op, m_gatePortals);
            stream(op, m_clusters);
        }

    private: // \todo [Hannu] make private
        unsigned int      m_outside        : 1;
        unsigned int      m_reachable      : 1;
        unsigned int      m_forceReachable : 1;
        unsigned int      m_padding        : 28;
        AABB              m_aabb;
        Hash<int, AABB>   m_objects;
        Array<RectPortal> m_rectPortals;
        Array<GatePortal> m_gatePortals;
        Array<int>        m_clusters;

        friend class CellGraph;
    };

    CellGraph (Allocator* a = NULL);
    Allocator* getAllocator(void) const { return m_cells.getAllocator(); }

    void        setAABB(const AABB& aabb) { m_aabb = aabb; }
    const AABB& getAABB(void) const { return m_aabb; }

    SubdivisionTreeSerialization& getViewTree() { return m_viewTree; }
    const SubdivisionTreeSerialization& getViewTree() const { return m_viewTree; }

    const SubdivisionTreeSerialization& getMatchingTree(int i) const { UMBRA_ASSERT(i >= 0 && i < 6); return m_matchingTree[i]; }
    SubdivisionTreeSerialization&       getMatchingTree(int i) { UMBRA_ASSERT(i >= 0 && i < 6); return m_matchingTree[i]; }

    Cell&   addCell(int num = 1) { int cur = m_cells.getSize(); m_cells.resize(m_cells.getSize() + num); return m_cells[cur]; }
    int     getCellCount() const { return m_cells.getSize(); }
    Cell&   getCell(int i) { return m_cells[i]; }
    const Cell& getCell(int i) const { return m_cells[i]; }

    int     assignClusters (int offset);

    int     getTargetObjectCount(void) const { return m_targetObjs.getSize(); }
    void    addTargetObject(const ObjectParams& obj) { m_targetObjs.pushBack(obj); }
    const ObjectParams& getTargetObject(int idx) const { return m_targetObjs[idx]; }
    void    removeTargetObjectsById(const Set<UINT32>& id);

    float getPortalExpand() const { return m_portalExpand; }
    void setPortalExpand(float f) { m_portalExpand = f; }

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, m_aabb);
        stream(op, m_viewTree);
        for (int i = 0; i < (int)UMBRA_ARRAY_SIZE(m_matchingTree); i++)
            stream(op, m_matchingTree[i]);
        stream(op, m_cells);
        stream(op, m_targetObjs);
        stream(op, m_portalExpand);
    }

    enum Property
    {
        RAW  = 1,
        BIDI = 2
    };

    void checkConsistency(UINT32 flags) const;

    void simplifyPortalHulls();

    void remapCells(const CellRemap& remap);

    void joinRight(const CellGraph& other, bool connectionPortals, float featureSize);

    void clone (CellGraph& dst, bool viewTree, bool matchingTrees) const;

    void removeNonConnectedCells();

    void optimizeMemoryUsage();

    void computeOutsideness();

private:

    AABB                         m_aabb;
    SubdivisionTreeSerialization m_viewTree;
    SubdivisionTreeSerialization m_matchingTree[6];
    Array<Cell>                  m_cells;
    Array<ObjectParams>          m_targetObjs;
    float                        m_portalExpand;
};

static inline void copyHeap (CellGraph::PortalHull* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

static inline void copyHeap (CellGraph::GatePortal* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

static inline void copyHeap (CellGraph::Cell* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

class CellRemap
{
public:

    CellRemap(void) {}

    CellRemap(Allocator* a, int s) 
        : m_mapping(a), m_reverse(a)
    { 
        reset(s);
    }

    void setAllocator(Allocator* a)
    {
        m_mapping.setAllocator(a);
        m_reverse.setAllocator(a);
    }

    void reset (int size)
    {
        m_mapping.reset(size);
        m_reverse.reset(size);
        m_last = -1;
        for (int i = 0; i < size; i++)
        {
            m_mapping[i] = -1;
            m_reverse[i] = -1;
        }
    }

    void set(int idx, int target)
    {
        UMBRA_ASSERT(target >= -1);
        m_mapping[idx] = target;
        if (target >= 0)
        {
            UMBRA_ASSERT(target > m_last || !"Only monotonically increasing mapping supported");
            UMBRA_ASSERT(m_reverse[target] == -1 || !"Mapping must be injective");
            m_reverse[target] = idx;
        }
        m_last = max2(m_last, target);
    }

    int map(int idx) const
    {
        return m_mapping[idx];
    }

    int reverseMap(int idx) const
    {
        return m_reverse[idx];
    }

    int getSize() const
    {
        return m_mapping.getSize();
    }

    int getLastTarget() const
    {
        return m_last;
    }

    const Array<int>& getArray(void) const { return m_mapping; }

private:
    Array<int> m_mapping;
    Array<int> m_reverse;
    int m_last;
};

static inline void copyHeap (CellRemap* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}


} // namespace Umbra
