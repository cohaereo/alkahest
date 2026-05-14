/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra runtime portal culling
 *
 */

#include "umbraTransformer.hpp"
#include "umbraIntersect.hpp"
#include "umbraPortalTraversal.hpp"
#include "umbraSIMD.hpp"
#include <float.h>

using namespace Umbra;


UMBRA_CT_ASSERT(UMBRA_ALIGNOF(Transformer) >= 16);

namespace Umbra {

// Build a NxM grid of parallel query jobs for target job count n.
static Vector2i getJobGrid (int n)
{
    Vector2i best(1, 1);
    int bestDiff = n * n;

    for (int i = 1; i <= UMBRA_MAX_FRUSTUM_SPLITS; i++)
    for (int j = 2; j >= 0; j--)
    {
        int x = i;
        int y = i - j;
        int t = x * y;
        int delta = t - n;
        if (delta >= 0 && delta < bestDiff)
        {
            best = Vector2i(x, y);
            bestDiff = delta;
            if (delta == 0)
                return best;
        }
    }

    return best;
}

}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_INLINE void Transformer::updateFrustumPlanes (void)
{
    // Build frustum AABB, only for non-predicted camera. Currently we can't
    // test against the frustum AABB with predicted camera because of the error
    // created by camera angle uncertainty.

    if (!m_hasPrediction && hasFarPlane())
    {
        AABB frustumAABB;
        for (int i = 0; i < 8; i++)
            frustumAABB.grow(getFrustumCorner(i));

        m_frustumBoundsMinSIMD = SIMDLoadW1(frustumAABB.getMin());
        m_frustumBoundsMaxSIMD = SIMDLoadW1(frustumAABB.getMax());
    }
    else
    {
        m_frustumBoundsMinSIMD = SIMDLoad(-FLT_MAX);
        m_frustumBoundsMaxSIMD = SIMDLoad(FLT_MAX);
    }

    for (int i = 0; i < m_planeCount; i++)
    {
        m_planeIndices[i] = i;
        m_clipPlanesSIMD[i] = SIMDLoad(&m_frustumPlanes[i].x);
        m_clipPlaneClass[i] = classifyPlane(m_clipPlanesSIMD[i]);
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void Transformer::setScissor (const Vector4i& rect)
{
    m_scissor = rect;

    float xScale = (float)UMBRA_PORTAL_RASTER_SIZE / (rect.k - rect.i);
    float yScale = (float)UMBRA_PORTAL_RASTER_SIZE / (rect.l - rect.j);
    float xOfs = (float)rect.i / UMBRA_PORTAL_RASTER_SIZE;
    float yOfs = (float)rect.j / UMBRA_PORTAL_RASTER_SIZE;

    Matrix4x4 subfrustum = MatrixFactory::subFrustum(m_worldToClip, xOfs, yOfs, xScale, yScale);
    m_frustumPlanes[2] = subfrustum.getRow(3) - subfrustum.getRow(0);
    m_frustumPlanes[3] = subfrustum.getRow(3) + subfrustum.getRow(0);
    m_frustumPlanes[4] = subfrustum.getRow(3) - subfrustum.getRow(1);
    m_frustumPlanes[5] = subfrustum.getRow(3) + subfrustum.getRow(1);

    updateFrustumPlanes();
}


/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void Transformer::init(const ImpCameraTransform& camera, Vector3 prediction, int threadId, int numThreads, int xSplits)
{
    UMBRA_ASSERT(numThreads >= 1 && numThreads <= UMBRA_MAX_FRUSTUM_SPLITS * UMBRA_MAX_FRUSTUM_SPLITS);

    m_threadId = threadId;
    m_numThreads = numThreads;
    m_xSplits = xSplits;

    if (camera.m_separate)
        ((ImpCameraTransform&)camera).update();

    // populate members

    m_hasPrediction     = (prediction.lengthSqr() > 0.f);
    m_predictionSIMD    = SIMDLoadW0(prediction);
    m_depthRange        = camera.m_depthRange;
    m_cameraPos         = camera.m_position;
    m_worldToClip       = camera.m_transform;
    m_flipPortalWinding = (m_worldToClip.det() < 0.0f);
    m_planeCount        = 6 + camera.m_userPlaneCount;
    m_worldToClipTranspose = camera.m_transform;
    m_worldToClipTranspose.transpose();

    // setup near, far and zero planes

    m_frustumPlanes[0] = m_worldToClip.getRow(2);
    m_frustumPlanes[1] = m_worldToClip.getRow(3) - m_worldToClip.getRow(2);

    m_frustumPlanes[0] *= 1.f / m_frustumPlanes[0].xyz().length();

    float farLen = m_frustumPlanes[1].xyz().length();
    m_hasFarPlane = (farLen > 0.f) && (dot(m_frustumPlanes[0].xyz(), m_frustumPlanes[1].xyz()) < 0.f);
    m_isOrtho = (m_worldToClip[3].xyz() == Vector3(0.f, 0.f, 0.f));

    Vector4 zeroPlane = m_frustumPlanes[0];
    zeroPlane.w = -dot(zeroPlane.xyz(), m_cameraPos);
    m_zeroPlane = SIMDLoad(zeroPlane);

    // add user defined planes

    for (int i = 0; i < camera.m_userPlaneCount; i++)
        m_frustumPlanes[6 + i] = camera.m_userPlanes[i];

    // limit to appropriate subfrustum
    Vector2i splits;
    if (xSplits)
    {
        splits.i = min2(xSplits, numThreads);
        splits.j = (numThreads + splits.i - 1) / splits.i;
    }
    else
    {
        splits = getJobGrid(numThreads);
    }

    UMBRA_ASSERT(splits.i > 0 && splits.i <= UMBRA_MAX_FRUSTUM_SPLITS);
    UMBRA_ASSERT(splits.j > 0 && splits.j <= UMBRA_MAX_FRUSTUM_SPLITS);

    int xsize = UMBRA_MAX_FRUSTUM_SPLITS / splits.i;
    int xextra = UMBRA_MAX_FRUSTUM_SPLITS % splits.i;
    int ysize = UMBRA_MAX_FRUSTUM_SPLITS / splits.j;
    int yextra = UMBRA_MAX_FRUSTUM_SPLITS % splits.j;

    int y1 = threadId % splits.j;
    int x1 = threadId / splits.j;
    int x2 = x1 + 1;
    int y2 = y1 + 1;
    // expand next-to-last row to cover missing jobs
    int gridSize = splits.i * splits.j;
    if (threadId + splits.j < gridSize &&
        threadId + splits.j >= numThreads)
        x2++;
    Vector4i scissor = Vector4i(
        (x1 * xsize + min2(xextra, x1)) * UMBRA_SCISSOR_ALIGN,
        (y1 * ysize + min2(yextra, y1)) * UMBRA_SCISSOR_ALIGN,
        (x2 * xsize + min2(xextra, x2)) * UMBRA_SCISSOR_ALIGN,
        (y2 * ysize + min2(yextra, y2)) * UMBRA_SCISSOR_ALIGN);
    setScissor(scissor);
}


/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void Transformer::init(const Vector4* planes, int numPlanes)
{
    m_threadId = 0;
    m_numThreads = 1;

    // populate members

    m_hasPrediction     = false;
    m_predictionSIMD    = SIMDZero();

    m_planeCount        = numPlanes;

    for (int i = 0; i < numPlanes; i++)
    {
        m_frustumPlanes[i] = planes[i];
        m_planeIndices[i] = i;
        m_clipPlanesSIMD[i] = SIMDLoad(&m_frustumPlanes[i].x);
        m_clipPlaneClass[i] = classifyPlane(m_clipPlanesSIMD[i]);
    }

    m_frustumBoundsMinSIMD = SIMDLoad(-FLT_MAX);
    m_frustumBoundsMaxSIMD = SIMDLoad(FLT_MAX);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Vector3 Transformer::getFrustumCorner (int i) const
{
    UMBRA_ASSERT(i >= 0 && i < 8);
    UMBRA_ASSERT(hasFarPlane() || i < 4);

    Vector4 zplane = m_frustumPlanes[i >> 2];
    Vector4 xplane = m_frustumPlanes[2 + ((i >> 1) & 1)];
    Vector4 yplane = m_frustumPlanes[4 + (i & 1)];

    Vector3 nxny = cross(xplane.xyz(), yplane.xyz());
    Vector3 nynz = cross(yplane.xyz(), zplane.xyz());
    Vector3 nznx = cross(zplane.xyz(), xplane.xyz());

    return -(zplane.w * nxny + xplane.w * nynz + yplane.w * nznx) / dot(zplane.xyz(), nxny);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Quad Transformer::getNearPlaneQuad (void) const
{
    Quad q;
    q.a = getFrustumCorner(0); // y = 0, x = 0
    q.b = getFrustumCorner(1); // y = 1, x = 0
    q.c = getFrustumCorner(3); // y = 1, x = 1
    q.d = getFrustumCorner(2); // y = 0, x = 1
    return q;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

SIMDRegister Transformer::classifyPlane (const SIMDRegister& v)
{
    return SIMDCompareGT(v, SIMDZero());
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool Transformer::computeActivePlaneSet (ActivePlaneSet& result, SIMDRegister mn, SIMDRegister mx) const
{
    int          numPlanes = m_planeCount;
    float        dist[MaxTotalClipPlanes]; // compute to array to avoid xbox stalls

    // Evaluate distance to plane for point furthest away from plane.
    for (int ndx = 0; ndx < numPlanes; ndx++)
    {
        SIMDRegister pt = SIMDSelect(mx, mn, m_clipPlaneClass[ndx]);
        SIMDRegister d  = SIMDDot4(pt, m_clipPlanesSIMD[ndx]);
        SIMDStore(d, dist[ndx]);
    }

    // Find such planes that at least one vertex of bounding box is outside of it.
    int dstNdx = 0;
    for (int ndx = 0; ndx < numPlanes; ndx++)
    {
        if (dist[ndx] <= 0.0f)
            result.planeTable[dstNdx++] = (UINT8)ndx;
    }

    result.numPlanes = dstNdx;
    return (dstNdx > 0);
}

bool Transformer::frustumTestBoundsFully (const ActivePlaneSet* planeSet, SIMDRegister mn, SIMDRegister mx) const
{
    // try to find plane for which AABB is fully outside

    SIMDRegister minDist = SIMDOne();

    int numPlanes = m_planeCount;
    const int* planeTable = m_planeIndices;

    if (planeSet)
    {
        numPlanes = planeSet->numPlanes;
        planeTable = planeSet->planeTable;
    }

    while (numPlanes >= 4)
    {
        int planeNdx0 = *planeTable++;
        int planeNdx1 = *planeTable++;
        int planeNdx2 = *planeTable++;
        int planeNdx3 = *planeTable++;

        // use overflow-safe dot4
        SIMDRegister d0 = SIMDDot4Safe(SIMDSelect(mx, mn, m_clipPlaneClass[planeNdx0]), m_clipPlanesSIMD[planeNdx0]);
        SIMDRegister d1 = SIMDDot4Safe(SIMDSelect(mx, mn, m_clipPlaneClass[planeNdx1]), m_clipPlanesSIMD[planeNdx1]);
        SIMDRegister d2 = SIMDDot4Safe(SIMDSelect(mx, mn, m_clipPlaneClass[planeNdx2]), m_clipPlanesSIMD[planeNdx2]);
        SIMDRegister d3 = SIMDDot4Safe(SIMDSelect(mx, mn, m_clipPlaneClass[planeNdx3]), m_clipPlanesSIMD[planeNdx3]);
        minDist = SIMDMin(minDist, SIMDMin(SIMDMin(d0, d1), SIMDMin(d2, d3)));
        numPlanes -= 4;
    }

    switch (numPlanes)
    {
        default:
            UMBRA_ASSERT(false);
            break;

        // \note Each case falls through to the next one!
        case 3: { int planeNdx = *planeTable++; minDist = SIMDMin(minDist, SIMDDot4Safe(SIMDSelect(mx, mn, m_clipPlaneClass[planeNdx]), m_clipPlanesSIMD[planeNdx])); }
        case 2: { int planeNdx = *planeTable++; minDist = SIMDMin(minDist, SIMDDot4Safe(SIMDSelect(mx, mn, m_clipPlaneClass[planeNdx]), m_clipPlanesSIMD[planeNdx])); }
        case 1: { int planeNdx = *planeTable++; minDist = SIMDMin(minDist, SIMDDot4Safe(SIMDSelect(mx, mn, m_clipPlaneClass[planeNdx]), m_clipPlanesSIMD[planeNdx])); }
        case 0:
            break;
    }

    return !SIMDCompareGTTestAny(SIMDZero(), minDist);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/
Vector2i Transformer::transformClipXYToRaster(const Vector2& clipXY, bool roundUp) const
{
    int add = roundUp ? 1 : 0;
    return Vector2i(
        (int)(clipXY.x * UMBRA_HALF_RASTER + UMBRA_HALF_RASTER + add),
        (int)(clipXY.y * UMBRA_HALF_RASTER + UMBRA_HALF_RASTER + add));
}
