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

#include "umbraGeometryBlock.hpp"
#include "umbraBitMath.hpp"
#include "umbraIntersectExact.hpp"
#include "umbraHash.hpp"
#include "optimizer/umbraScene.hpp"
#include "umbraImpScene.hpp"

using namespace Umbra;

int GeometryBlock::addObject(const SceneObject* so)
{
    ObjectParams o;
    o.m_id = so->getID();
    o.m_flags = so->getFlags();
    o.m_drawDistance = so->getDrawDistance();
    Vector3 boundMn, boundMx;
    so->getDistanceBound(boundMn, boundMx);
    o.m_distanceBound = AABB(boundMn, boundMx);
    o.m_bounds = ImpScene::getImplementation(const_cast<SceneObject*>(so))->getAABB();
    o.m_cost = ImpScene::getImplementation(so->getModel())->getCost();
    m_objects.pushBack(o);
    return m_objects.getSize() - 1;
}

void GeometryBlock::importObject(const SceneObject* so, const Array<Vector3>& transformedVertices)
{
    UMBRA_ASSERT(m_targetAABB.isOK());
    UMBRA_ASSERT(m_occluderAABB.isOK());

    int objIdx = addObject(so);
    const SceneModel* model = so->getModel();
    const Vector3i* srcTris = model->getTriangles();
    const AABB& bounds = (so->getFlags() & (SceneObject::OCCLUDER | SceneObject::GATE)) ? m_occluderAABB : m_targetAABB;
    bool swapTris = (so->getTriangleWinding() == WINDING_CW);
    /* \todo [antti 27.6.2012]: get rid of duplication */
    bool duplicate = ((so->getTriangleWinding() == WINDING_TWO_SIDED) && (so->getFlags() & (SceneObject::OCCLUDER | SceneObject::GATE)));
    /* \todo [antti 27.6.2012]: could hash across object boundaries as well */
    Hash<Vector3, int> vertexMap(m_vertices.getAllocator());

    bool includeOutsideAABBTriangles = !!(so->getFlags() & SceneObject::VOLUME);

    /* \todo [antti 27.6.2012]: reserve space in arrays */

    for (int t = 0; t < model->getTriangleCount(); t++)
    {
        const Vector3i& srcTri = srcTris[t];
        Vector3 verts[3];
        verts[0] = transformedVertices[srcTri.i];
        verts[1] = transformedVertices[srcTri.j];
        verts[2] = transformedVertices[srcTri.k];

        if (includeOutsideAABBTriangles || intersectAABBTriangle(bounds, verts[0], verts[1], verts[2]))
        {
            Triangle tri;
            tri.m_objectIdx = objIdx;
            for (int i = 0; i < 3; i++)
            {
                int* vertIdx = vertexMap.get(verts[i]);
                if (!vertIdx)
                {
                    vertIdx = vertexMap.insert(verts[i], m_vertices.getSize());
                    m_vertices.pushBack(verts[i]);
                }
                tri.m_vertices[i] = *vertIdx;
            }
            if (swapTris)
                swap2(tri.m_vertices[0], tri.m_vertices[2]);
            m_triangles.pushBack(tri);
            if (duplicate)
            {
                swap2(tri.m_vertices[0], tri.m_vertices[2]);
                m_triangles.pushBack(tri);
            }
        }
    }
}

#endif // UMBRA_EXCLUDE_COMPUTATION
