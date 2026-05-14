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
 * \brief   AABB
 *
 */

#include "umbraAABB.hpp"

namespace Umbra
{

void AABB::validateBounds (void)
{
//  if (m_min[0] == m_max[0]) m_max[0] = fixFloat(m_max[0]);
//  if (m_min[1] == m_max[1]) m_max[1] = fixFloat(m_max[1]);
//  if (m_min[2] == m_max[2]) m_max[2] = fixFloat(m_max[2]);
}

// Sides are facing as: 0= -x, 1= -y, 2= -z, 3= +x, 4= +y, 5= +z
void AABB::getPlaneEquations(Vector4 pleqs[6]) const
{
    for (int i = 0; i < 6; i++)
        pleqs[i] = getPlaneEq(i);
}

Vector4 AABB::getPlaneEq(int face) const
{
    int axis = getFaceAxis(face);
    int dir = getFaceDirection(face);
    Vector3 v = dir ? getMax() : getMin();
    float pos = v[axis];
    Vector4 pleq;
    pleq[axis] = dir ? -1.f : 1.f;
    pleq.w = dir ? pos : -pos;
    return pleq;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief   Creates a 4-vertex quad from a bounding box side
 * \note    Winding is counter-clockwise (viewed from outside of the box)
 * \note    Sides are facing as: 0= -x, 1= -y, 2= -z, 3= +x, 4= +y, 5= +z
 *//*----------------------------------------------------------------------*/
void AABB::getSideQuad(int side, Vector3 quad[4]) const
{
    UMBRA_ASSERT(side >= 0 && side < 6);

    static const Corner cornerList[6][4] = {
        { MINX_MINY_MINZ, MINX_MINY_MAXZ, MINX_MAXY_MAXZ, MINX_MAXY_MINZ },
        { MINX_MINY_MINZ, MAXX_MINY_MINZ, MAXX_MINY_MAXZ, MINX_MINY_MAXZ },
        { MINX_MINY_MINZ, MINX_MAXY_MINZ, MAXX_MAXY_MINZ, MAXX_MINY_MINZ },
        { MAXX_MINY_MINZ, MAXX_MAXY_MINZ, MAXX_MAXY_MAXZ, MAXX_MINY_MAXZ },
        { MINX_MAXY_MINZ, MINX_MAXY_MAXZ, MAXX_MAXY_MAXZ, MAXX_MAXY_MINZ },
        { MINX_MINY_MAXZ, MAXX_MINY_MAXZ, MAXX_MAXY_MAXZ, MINX_MAXY_MAXZ }
    };

    for (int i=0; i < 4; i++)
        quad[i] = getCorner(cornerList[side][i]);
}

// Sides are facing as: 0= -x, 1= -y, 2= -z, 3= +x, 4= +y, 5= +z
void AABB::flattenToSide(int side)
{
    UMBRA_ASSERT(side >= 0 && side < 6);

    if (side < 3)
        m_max[side] = m_min[side];
    else
        m_min[side-3] = m_max[side-3];
}

//------------------------------------------------------------------------
} // namespace Umbra