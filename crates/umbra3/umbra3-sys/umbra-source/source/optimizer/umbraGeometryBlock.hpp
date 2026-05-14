#pragma once

#include "umbraPrivateDefs.hpp"
#include "umbraMath.hpp"
#include "umbraArray.hpp"
#include "umbraAABB.hpp"

namespace Umbra
{

class SceneObject;

struct ViewVolume
{
    template<typename OP> void streamOp (OP& op)
    {
        stream(op, id);
        stream(op, aabb);
        stream(op, backfaceLimit);
        stream(op, cellSplits);
        stream(op, isClusterMarker);
    }

    unsigned int id;
    AABB         aabb;
    float        backfaceLimit;
    int          cellSplits;
    bool         isClusterMarker;
    // \todo [Hannu] add other modifiers
};

struct ObjectParams
{
    /* \todo [antti 27.6.2012]: these need to match SceneObject::Flags */
    enum Flags
    {
        OCCLUDER        = (1<<0),
        TARGET          = (1<<1),
        GATE            = (1<<2),
        VOLUME          = (1<<3),
    };

    bool isOccluder() const { return !!(m_flags & OCCLUDER) && (m_drawDistance[0] == 0.f); }
    bool isTarget() const { return !!(m_flags & TARGET); }
    bool isGate() const { return !!(m_flags & GATE); }
    bool isVolume() const { return !!(m_flags & VOLUME); }
    UINT32 getId() const { return m_id; }
    float getCost() const { return m_cost; }

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, m_id);
        stream(op, m_flags);
        stream(op, m_drawDistance);
        stream(op, m_distanceBound);
        stream(op, m_bounds);
        stream(op, m_cost);
    }

    UINT32   m_id;
    UINT32   m_flags;
    Vector2  m_drawDistance;
    AABB     m_distanceBound;
    AABB     m_bounds;
    float    m_cost;
};

class GeometryBlock
{
public:

    struct Triangle
    {
        template<typename OP> void streamOp (OP& op)
        {
            stream(op, m_vertices);
            stream(op, m_objectIdx);
        }

        Vector3i    m_vertices;
        UINT32      m_objectIdx;
    };

    GeometryBlock(Allocator* a): m_vertices(a), 
        m_triangles(a), m_viewVolumes(a), m_objects(a) {}

    void clear (void)
    {
        m_targetAABB = AABB();
        m_occluderAABB = AABB();
        m_vertices.clear();
        m_triangles.clear();
        m_viewVolumes.clear();
        m_objects.clear();
    }

    void importObject (const SceneObject* so, const Array<Vector3>& transformedVertices);

    const Array<Triangle>& getTriangles() const { return m_triangles; }
    int getTriangleCount() const { return m_triangles.getSize(); }

    const Triangle& getTriangle(int i) const
    {
        UMBRA_ASSERT(i >= 0 && i < m_triangles.getSize());
        return m_triangles[i];
    }

    const ObjectParams& getObject(int i) const
    {
        UMBRA_ASSERT(i >= 0 && i < m_objects.getSize());
        return m_objects[i];
    }

    int getObjectCount() const { return m_objects.getSize(); }

    const ObjectParams& getTriangleObject(int i) const
    {
        return getObject(getTriangle(i).m_objectIdx);
    }

    AABB getTriangleAABB(int i) const
    {
        AABB aabb;
        Vector3 a, b, c;
        getVertices(i, a, b, c);
        aabb.grow(a);
        aabb.grow(b);
        aabb.grow(c);
        return aabb;
    }

    const Array<Vector3>& getVertices() const { return m_vertices; }
    int getVertexCount() const { return m_vertices.getSize(); }

    const Vector3& getVertex(int i) const
    {
        UMBRA_ASSERT(i >= 0 && i < m_vertices.getSize());
        return m_vertices[i];
    }

    void getVertices(int i, Vector3& a, Vector3& b, Vector3& c) const
    {
        const Triangle& tri = getTriangle(i);
        a = getVertex(tri.m_vertices.i);
        b = getVertex(tri.m_vertices.j);
        c = getVertex(tri.m_vertices.k);
    }

    int getViewVolumeCount() const { return m_viewVolumes.getSize(); }
    const ViewVolume& getViewVolume(int i) const { return m_viewVolumes[i]; }
    Array<ViewVolume>& getViewVolumes() { return m_viewVolumes; }

    void setTargetAABB(const AABB& aabb) { m_targetAABB = aabb; }
    const AABB& getTargetAABB(void) const { return m_targetAABB; }

    void setOccluderAABB(const AABB& aabb) { m_occluderAABB = aabb; }
    const AABB& getOccluderAABB(void) const { return m_occluderAABB; }

    bool isEmpty() const
    {
        return m_triangles.getSize() == 0 && m_viewVolumes.getSize() == 0;
    }

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, m_targetAABB);
        stream(op, m_occluderAABB);
        stream(op, m_vertices);
        stream(op, m_triangles);
        stream(op, m_viewVolumes);
        stream(op, m_objects);
    }

private:

    int addObject (const SceneObject* o);

    AABB                m_targetAABB;
    AABB                m_occluderAABB;
    Array<Vector3>      m_vertices;
    Array<Triangle>     m_triangles;
    Array<ViewVolume>   m_viewVolumes;
    Array<ObjectParams> m_objects;
};

}
