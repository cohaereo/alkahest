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
 * \brief   Potentially visible set solver
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraAABB.hpp"
#include "umbraVector.hpp"
#include "umbraMatrix.hpp"
#include "umbraTomePrivate.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   Unpacked depth map.
 *//*-------------------------------------------------------------------*/
struct RawDepthmap
{
    RawDepthmap() { memset(this, 0, sizeof(RawDepthmap)); }
    Vector3 center;
    UINT32 inf[DepthmapData::FaceCount][UMBRA_BITVECTOR_DWORDS(DepthmapData::Resolution * DepthmapData::Resolution)];
    float  depthmap[DepthmapData::FaceCount][DepthmapData::Resolution][DepthmapData::Resolution];
};

/*-------------------------------------------------------------------*//*!
 * \brief   Updates a RawDepthmap for given AABB. 
 *          Can be called incrementally.
 *//*-------------------------------------------------------------------*/
class DepthmapSolver : public Base
{
public:

    DepthmapSolver(Allocator* a, const ImpTome* tome, const AABB& worldAABB);
    ~DepthmapSolver(void);

    // Solve cubemap for an AABB and update RawDepthmap.
    void solve                      (RawDepthmap& raw, const AABB& objAABB, int objectIdx, float targetInflation);
    
    // Partition AABB to suitable parts for a solve. 
    void partitionComputationUnits  (const AABB& aabb, Array<AABB>& split);

private:
    struct FaceData
    {
        class OcclusionBuffer*  buffer;
        Matrix4x4               inverse;
        Vector3                 pos;
        Vector3                 frustumCorners[8];
    };

    void                drawDepthmapQuad        (RawDepthmap& raw, const Vector3* v, int dstFace, bool updateValue, bool updateInf, bool accurate);
    void                drawDepthmapQuadSplit   (RawDepthmap& raw, int srcFace, const Vector3* v, int dstFace, bool inf);

    Vector4             m_pixelPlanes[6][DepthmapData::Resolution][DepthmapData::Resolution][4];
    class QueryExt*     m_query;
    class QueryContext* m_queryContext;
    UINT8*              m_workMem;
    FaceData            m_faceDatas[6];
    const ImpTome*      m_tome;
    float*              m_floatBuffer;
    AABB                m_worldAABB;
};

} // namespace Umbra
