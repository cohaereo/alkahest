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
 * \brief   Computation tile data containers
 *
 */

#include "umbraComputationTile.hpp"
#include "umbraFPUControl.hpp"
#include "umbraCRCStream.hpp"

using namespace Umbra;



/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

AABB ImpTileInput::getAABB (void) const
{
    return AABB(Vector3((float)m_aabbMin.i, (float)m_aabbMin.j, (float)m_aabbMin.k)*m_unitSize,
                Vector3((float)m_aabbMax.i, (float)m_aabbMax.j, (float)m_aabbMax.k)*m_unitSize);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

const char* ImpTileInput::getHash (void) const
{
    if (!m_hash.length())
    {
        HashGenerator hasher(getAllocator());
        if (serialize(hasher))            
            m_hash = hasher.getHashValue();
    }
    return m_hash.length() ? m_hash.toCharPtr() : NULL;
}
/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/


bool ImpTileInput::serialize(OutputStream& out) const
{
    bool ret = true;
    CRCOutputStream out2(out);
    Serializer serializer(&out2);
    stream(serializer, (ImpTileInput&)*this);
    ret = serializer.isOk();
    if (!out2.flush())
        ret = false;
    return ret;
}


/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool ImpTileInputSet::get (ImpTileInput** out, int idx)
{
    UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY

    if (idx < 0 || idx >= m_grid.getNumNodes())
        return false;

    ImpTileInput* tile = *out;

    if (!tile)
    {
        tile = UMBRA_NEW(ImpTileInput, getCtx());
        if (!tile)
            return false; // TODO: error
    }

    tile->m_aabbMin = m_grid.getIntMin(idx);
    tile->m_aabbMax = m_grid.getIntMax(idx);
    tile->m_cellGeneratorParams = m_grid.getCellGeneratorParams(idx);
    tile->m_unitSize = m_grid.getUnitSize();
    tile->m_geometry.clear();
    m_grid.fillBlock(tile->m_geometry, idx);
    tile->m_computationString = m_grid.getComputationString();

    *out = tile;
    return true;
}


#endif // UMBRA_EXCLUDE_COMPUTATION
