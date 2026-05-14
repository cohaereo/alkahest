#pragma once
#ifndef __UMBRATRANSFORMER_H
#define __UMBRATRANSFORMER_H

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
 * \brief   Umbra runtime view transformations
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraMatrix.hpp"
#include "runtime/umbraQuery.hpp"
#include "umbraSIMD.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraRasterDefs.hpp"

#define UMBRA_SCISSOR_ALIGN 16
#define UMBRA_MAX_FRUSTUM_SPLITS (UMBRA_PORTAL_RASTER_SIZE / UMBRA_SCISSOR_ALIGN)

namespace Umbra
{

class ImpCameraTransform;
class Portal;
struct PortalBounds;

enum
{
    NumFrustumPlanes    = 6,
    MaxTotalClipPlanes  = NumFrustumPlanes + UMBRA_MAX_USER_CLIP_PLANES
};

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

struct VQuad
{
    SIMDRegister x;
    SIMDRegister y;
    SIMDRegister z;
    SIMDRegister expand;
};

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

struct ClipCodes
{
    SIMDRegister xLeft;
    SIMDRegister xRight;
    SIMDRegister yLeft;
    SIMDRegister yRight;
};

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

struct ActivePlaneSet
{
    enum
    {
        NearPlaneIndex = 0  // \note must be zero so it stays as first plane in ActivePlaneSet
    };

    bool isNearPlaneActive (void) const
    {
        return (numPlanes > 0) && (planeTable[0] == ActivePlaneSet::NearPlaneIndex);
    }

    int     numPlanes;
    int     planeTable[MaxTotalClipPlanes];
};

/*-------------------------------------------------------------------*//*!
 * \brief
 * \todo    make prediction distance a 1st class citizen everywhere!
 *//*-------------------------------------------------------------------*/

class UMBRA_ATTRIBUTE_ALIGNED16(Transformer)
{
public:
    Transformer (void)                       { }

    void init (const ImpCameraTransform& camera, Vector3 prediction, int threadID, int numThreads, int xSplits);

    Transformer (const ImpCameraTransform& camera, float prediction,
        int threadID = 0, int numThreads = 1, int xSplits = 0) { init(camera, Vector3(prediction, prediction, prediction), threadID, numThreads, xSplits); }
    Transformer (const ImpCameraTransform& camera, const Vector3& prediction,
        int threadID = 0, int numThreads = 1, int xSplits = 0) { init(camera, prediction, threadID, numThreads, xSplits); }
    Transformer (const Vector4* planes, int numPlanes) { init(planes, numPlanes); }

    void                        init                        (const Vector4* planes, int numPlanes);


    UMBRA_INLINE void           transformBox                (Vector4i& mnmx, SIMDRegister mn, SIMDRegister mx, bool canNearClip) const;
    UMBRA_INLINE void           transformBox                (Vector4i& mnmx, SIMDRegister mn, SIMDRegister mx, bool canNearClip, float& contribution) const;
    UMBRA_INLINE void           transformBox                (Vector4i& mnmx, SIMDRegister mn, SIMDRegister mx, bool canNearClip,
                                                             const SIMDRegister32& scissorBounds, float& contribution) const;
    UMBRA_INLINE void           transformPortal             (Vector4i& mnmx, VQuad& out, const SIMDRegister& portalMin, const SIMDRegister& portalMax,
                                                             int face, bool canNearClip, const SIMDRegister32& scissorBounds) const;
    Vector2i                    transformClipXYToRaster     (const Vector2& clipXY, bool roundUp) const;

    bool                        getFlipPortalWinding        (void) const { return m_flipPortalWinding; }
    const Matrix4x4&            getWorldToClip              (void) const { return m_worldToClip; }
    const Matrix4x4&            getWorldToClipTranspose     (void) const { return m_worldToClipTranspose; }
    const Vector3&              getCameraPos                (void) const { return m_cameraPos; }
    CameraTransform::DepthRange getDepthRange               (void) const { return (CameraTransform::DepthRange)m_depthRange; }
    SIMDRegister                getPrediction               (void) const { return m_predictionSIMD; }
    bool                        hasPrediction               (void) const { return m_hasPrediction; }
    bool                        hasUserPlanes               (void) const { return m_planeCount > 6; }

    UMBRA_INLINE float          getMinDeviceZ               (SIMDRegister mn, SIMDRegister mx) const;
    UMBRA_INLINE float          getMaxDeviceZ               (SIMDRegister mn, SIMDRegister mx) const;

    bool                        computeActivePlaneSet       (ActivePlaneSet& result, SIMDRegister mn, SIMDRegister mx) const;
    bool                        frustumTestBounds           (SIMDRegister mn, SIMDRegister mx) const { return frustumTestBounds(NULL, mn, mx); }
    UMBRA_INLINE bool           frustumTestBounds           (const ActivePlaneSet* planeSet, SIMDRegister mn, SIMDRegister mx) const;
    bool                        frustumTestBoundsFully      (SIMDRegister mn, SIMDRegister mx) const { return frustumTestBoundsFully(NULL, mn, mx); }
    bool                        frustumTestBoundsFully      (const ActivePlaneSet* planeSet, SIMDRegister mn, SIMDRegister mx) const;
    UMBRA_INLINE bool           frustumTestBoundsZeroPlane  (SIMDRegister mn, SIMDRegister mx) const;
    bool                        VFTest                      (const Vector4& mn, const Vector4& mx) const { return frustumTestBounds(SIMDLoad(mn), SIMDLoad(mx)); }
    bool                        VFTestFully                 (const Vector4& mn, const Vector4& mx) const { return frustumTestBoundsFully(SIMDLoad(mn), SIMDLoad(mx)); }

    const Vector4*              getFrustumPlanes            (void) const { return m_frustumPlanes; }
    Vector3                     getFrustumCorner            (int i) const;
    Quad                        getNearPlaneQuad            (void) const;
    Vector4                     getNearPlane                (void) const { return m_frustumPlanes[0]; }
    bool                        isOrtho                     (void) const { return m_isOrtho; }
    bool                        hasFarPlane                 (void) const { return m_hasFarPlane; }
    const Vector4i&             getScissor                  (void) const { return m_scissor; }
    SIMDRegister32              getScissorSIMD              (void) const { return SIMDLoadAligned32(&m_scissor.i); }
    void                        setScissor                  (const Vector4i& scissor);

    int                         getThreadId                 (void) const { return m_threadId; }
    int                         getNumThreads               (void) const { return m_numThreads; }
    int                         getXSplits				    (void) const { return m_xSplits; }

private:

    SIMDRegister                        classifyPlane       (const SIMDRegister& v);
    void                                updateFrustumPlanes (void);

    Matrix4x4                           UMBRA_ATTRIBUTE_ALIGNED16(m_worldToClip);
    Matrix4x4                           UMBRA_ATTRIBUTE_ALIGNED16(m_worldToClipTranspose);
    Vector4                             m_frustumPlanes[MaxTotalClipPlanes];
    SIMDRegister                        m_clipPlanesSIMD[MaxTotalClipPlanes];
    SIMDRegister                        m_clipPlaneClass[MaxTotalClipPlanes];
    Vector3                             m_cameraPos;
    INT32                               m_planeCount;
    UINT32                              m_depthRange;
    bool                                m_flipPortalWinding;
    SIMDRegister                        m_predictionSIMD;
    bool                                m_hasPrediction;
    int                                 m_planeIndices[MaxTotalClipPlanes];
    SIMDRegister                        m_frustumBoundsMinSIMD;
    SIMDRegister                        m_frustumBoundsMaxSIMD;
    SIMDRegister                        m_zeroPlane;
    bool                                m_hasFarPlane;
    bool                                m_isOrtho;
    Vector4i                            UMBRA_ATTRIBUTE_ALIGNED16(m_scissor);
    int                                 m_threadId;
    int                                 m_numThreads;
    int                                 m_xSplits;
};


static UMBRA_INLINE void getClipCodes(
    SIMDRegister tx,
    SIMDRegister ty,
    SIMDRegister tw,
    ClipCodes& codes)
{
    SIMDRegister mw = SIMDNegate(tw);
    codes.xLeft  = SIMDCompareGT(tx, mw);  // x > -w
    codes.xRight = SIMDCompareGT(tw, tx);  // x < w
    codes.yLeft  = SIMDCompareGT(ty, mw);  // y > -w
    codes.yRight = SIMDCompareGT(tw, ty);  // y < w
}

static UMBRA_INLINE SIMDRegister combineMin(
    SIMDRegister x,
    SIMDRegister y)
{
    // returns in form (minx, miny, garbage, garbage)

    SIMDRegister x0y0x1y1 = SIMDMergeLow(x, y);
    SIMDRegister x2y2x3y3 = SIMDMergeHigh(x, y);
    SIMDRegister x02y02x13y13 = SIMDMin(x0y0x1y1, x2y2x3y3);
    return SIMDMin(x02y02x13y13, SIMDHighToLow(x02y02x13y13));
}

static UMBRA_INLINE SIMDRegister combineMax(
    SIMDRegister x,
    SIMDRegister y)
{
    // returns in form (maxx, maxy, garbage, garbage)

    SIMDRegister x0y0x1y1 = SIMDMergeLow(x, y);
    SIMDRegister x2y2x3y3 = SIMDMergeHigh(x, y);
    SIMDRegister x02y02x13y13 = SIMDMax(x0y0x1y1, x2y2x3y3);
    return SIMDMax(x02y02x13y13, SIMDHighToLow(x02y02x13y13));
}

#ifdef UMBRA_SIMD_NEON

static UMBRA_INLINE SIMDRegister combineMinMax(
    SIMDRegister x,
    SIMDRegister y)
{
    float32x2_t x01 = vget_low_f32(x);
    float32x2_t x23 = vget_high_f32(x);
    float32x2_t y01 = vget_low_f32(y);
    float32x2_t y23 = vget_high_f32(y);

    float32x2_t min_x01_x23 = vpmin_f32(x01, x23);
    float32x2_t min_y01_y23 = vpmin_f32(y01, y23);
    float32x2_t max_x01_x23 = vpmax_f32(x01, x23);
    float32x2_t max_y01_y23 = vpmax_f32(y01, y23);

    float32x2_t min_xy = vpmin_f32(min_x01_x23, min_y01_y23);
    float32x2_t max_xy = vpmax_f32(max_x01_x23, max_y01_y23);

    return vcombine_f32(min_xy, max_xy);
}

static UMBRA_INLINE SIMDRegister combineMinMax(
    SIMDRegister x1,
    SIMDRegister x2,
    SIMDRegister y1,
    SIMDRegister y2)
{
    float32x4_t min_x = vminq_f32(x1, x2);
    float32x4_t max_x = vmaxq_f32(x1, x2);
    float32x4_t min_y = vminq_f32(y1, y2);
    float32x4_t max_y = vmaxq_f32(y1, y2);

    float32x2_t min_x01_x23 = vpmin_f32(vget_low_f32(min_x), vget_high_f32(min_x));
    float32x2_t min_y01_y23 = vpmin_f32(vget_low_f32(min_y), vget_high_f32(min_y));
    float32x2_t max_x01_x23 = vpmax_f32(vget_low_f32(max_x), vget_high_f32(max_x));
    float32x2_t max_y01_y23 = vpmax_f32(vget_low_f32(max_y), vget_high_f32(max_y));

    float32x2_t min_xy = vpmin_f32(min_x01_x23, min_y01_y23);
    float32x2_t max_xy = vpmax_f32(max_x01_x23, max_y01_y23);

    return vcombine_f32(min_xy, max_xy);
}

#else

static UMBRA_INLINE SIMDRegister combineMinMax(
    SIMDRegister x,
    SIMDRegister y)
{
    SIMDRegister mn = combineMin(x, y);
    SIMDRegister mx = combineMax(x, y);
    return SIMDShuffle_A0A1B0B1(mn, mx);
}

static UMBRA_INLINE SIMDRegister combineMinMax(
    SIMDRegister x1,
    SIMDRegister x2,
    const SIMDRegister& y1,
    const SIMDRegister& y2)
{
    SIMDRegister mn = combineMin(SIMDMin(x1, x2), SIMDMin(y1, y2));
    SIMDRegister mx = combineMax(SIMDMax(x1, x2), SIMDMax(y1, y2));
    return SIMDShuffle_A0A1B0B1(mn, mx);
}

#endif


// Overflow-safe dot4 operation
#if UMBRA_OS == UMBRA_XBOX360
UMBRA_FORCE_INLINE SIMDRegister SIMDDot4Safe(const SIMDRegister& a, const SIMDRegister& b)
{
    // XBOX360's dot product produces nan on overflow instead of inf, making
    // isAABBVisible with MAX_FLT AABB fail. I think this is the fastest
    // overflow-safe version (12 extra cycles)?
    return SIMDDot4(SIMDMultiply(a, b), SIMDOne());
}
#else
#define SIMDDot4Safe(a,b) SIMDDot4(a,b)
#endif

bool Transformer::frustumTestBounds (const ActivePlaneSet* planeSet, SIMDRegister mn, SIMDRegister mx) const
{
    int numPlanes = m_planeCount;
    const int* planeTable = m_planeIndices;

    if (planeSet)
    {
        numPlanes = planeSet->numPlanes;
        planeTable = planeSet->planeTable;
    }

    if (!UMBRA_OPT_AVOID_BRANCHES && !numPlanes)
        return true;

    // test frustum AABB against aabb

    SIMDRegister outside = SIMDBitwiseOr(SIMDCompareGT(mn, m_frustumBoundsMaxSIMD),
                                         SIMDCompareGT(m_frustumBoundsMinSIMD, mx));

    // try to find plane for which AABB is fully outside

    SIMDRegister minDist = SIMDZero();

    while (numPlanes >= 4)
    {
        int planeNdx0 = *planeTable++;
        int planeNdx1 = *planeTable++;
        int planeNdx2 = *planeTable++;
        int planeNdx3 = *planeTable++;

#if defined(UMBRA_SIMD_AVX)
        SIMDRegister d0 = SIMDMultiply(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx0]), m_clipPlanesSIMD[planeNdx0]);
        SIMDRegister d1 = SIMDMultiply(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx1]), m_clipPlanesSIMD[planeNdx1]);
        SIMDRegister d2 = SIMDMultiply(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx2]), m_clipPlanesSIMD[planeNdx2]);
        SIMDRegister d3 = SIMDMultiply(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx3]), m_clipPlanesSIMD[planeNdx3]);
        minDist = SIMDMin(minDist, _mm_hadd_ps(_mm_hadd_ps(d0, d1), _mm_hadd_ps(d2, d3)));
#elif defined(UMBRA_SIMD_NEON)
        float32x2_t d0 = SIMDDot4_Partial(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx0]), m_clipPlanesSIMD[planeNdx0]);
        float32x2_t d1 = SIMDDot4_Partial(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx1]), m_clipPlanesSIMD[planeNdx1]);
        float32x2_t d2 = SIMDDot4_Partial(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx2]), m_clipPlanesSIMD[planeNdx2]);
        float32x2_t d3 = SIMDDot4_Partial(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx3]), m_clipPlanesSIMD[planeNdx3]);
        float32x2_t d0d1 = vpadd_f32(d0, d1);
        float32x2_t d2d3 = vpadd_f32(d2, d3);
        minDist = SIMDMin(minDist, vcombine_f32(d0d1, d2d3));
#else
        // use overflow-safe dot4
        SIMDRegister d0 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx0]), m_clipPlanesSIMD[planeNdx0]);
        SIMDRegister d1 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx1]), m_clipPlanesSIMD[planeNdx1]);
        SIMDRegister d2 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx2]), m_clipPlanesSIMD[planeNdx2]);
        SIMDRegister d3 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx3]), m_clipPlanesSIMD[planeNdx3]);
        minDist = SIMDMin(minDist, SIMDMin(SIMDMin(d0, d1), SIMDMin(d2, d3)));
#endif
        numPlanes -= 4;
    }

    switch (numPlanes)
    {
        default:
            UMBRA_ASSERT(false);
            break;

        // \note Each case falls through to the next one!
        case 3: { int planeNdx = *planeTable++; minDist = SIMDMin(minDist, SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx]), m_clipPlanesSIMD[planeNdx])); }
        case 2: { int planeNdx = *planeTable++; minDist = SIMDMin(minDist, SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx]), m_clipPlanesSIMD[planeNdx])); }
        case 1: { int planeNdx = *planeTable++; minDist = SIMDMin(minDist, SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[planeNdx]), m_clipPlanesSIMD[planeNdx])); }
        case 0:
            break;
    }

    return !SIMDBitwiseOrTestAny(outside, SIMDCompareGT(SIMDZero(), minDist));
}

bool Transformer::frustumTestBoundsZeroPlane (SIMDRegister mn, SIMDRegister mx) const
{
    // try to find plane for which AABB is fully outside
    // use overflow-safe dot4
    SIMDRegister minDist = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[0]), m_zeroPlane);
    SIMDRegister d1 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[1]), m_clipPlanesSIMD[1]);
    minDist = SIMDMin(minDist, d1);
    SIMDRegister d2 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[2]), m_clipPlanesSIMD[2]);
    minDist = SIMDMin(minDist, d2);
    SIMDRegister d3 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[3]), m_clipPlanesSIMD[3]);
    minDist = SIMDMin(minDist, d3);
    SIMDRegister d4 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[4]), m_clipPlanesSIMD[4]);
    minDist = SIMDMin(minDist, d4);
    SIMDRegister d5 = SIMDDot4Safe(SIMDSelect(mn, mx, m_clipPlaneClass[5]), m_clipPlanesSIMD[5]);
    minDist = SIMDMin(minDist, d5);

    return !SIMDCompareGTTestAny(SIMDZero(), minDist);
}

static UMBRA_INLINE SIMDRegister getClipCodes(
    const SIMDRegister& xyFlipped,
    const SIMDRegister& negw)
{
    return SIMDCompareGT(xyFlipped, negw);
}

static UMBRA_INLINE void processClipVertex(
    const SIMDRegister& xy,
    const SIMDRegister& w,
    SIMDRegister& minmaxXY,
    SIMDRegister& minmaxW)
{
    // update min and max
    SIMDRegister updateMask = SIMDCompareGT(SIMDMultiply(xy, minmaxW), SIMDMultiply(minmaxXY, w));
    minmaxXY = SIMDSelect(xy, minmaxXY, updateMask);
    minmaxW = SIMDSelect(w, minmaxW, updateMask);
}

static UMBRA_INLINE SIMDRegister SIMDReplicateMinMax(
    const SIMDRegister& x)
{
#ifdef UMBRA_SIMD_NEON
    float32x2_t xy = vget_low_f32(x);
    return vcombine_f32(xy, vneg_f32(xy));
#else
    return SIMDShuffle_A0A1B0B1(x, SIMDNegate(x));
#endif
}

void Transformer::transformPortal(
    Vector4i& mnmx, VQuad& out, const SIMDRegister& portalMin, const SIMDRegister& portalMax,
    int face, bool canNearClip, const SIMDRegister32& scissorBounds) const
{
    SIMDRegister transform0 = SIMDLoadAligned(m_worldToClipTranspose[0]);
    SIMDRegister transform1 = SIMDLoadAligned(m_worldToClipTranspose[1]);
    SIMDRegister transform2 = SIMDLoadAligned(m_worldToClipTranspose[2]);
    SIMDRegister transform3 = SIMDLoadAligned(m_worldToClipTranspose[3]);

    // transform min corner

    SIMDRegister q0 = SIMDMultiplyAdd(transform0, SIMDReplicate(portalMin, 0),
                      SIMDMultiplyAdd(transform1, SIMDReplicate(portalMin, 1),
                      SIMDMultiplyAdd(transform2, SIMDReplicate(portalMin, 2),
                                      transform3)));

    // transform base vectors

    SIMDRegister diag = SIMDSub(portalMax, portalMin);
    SIMDRegister baseX = SIMDMultiply(transform0, SIMDReplicate(diag, 0));
    SIMDRegister baseY = SIMDMultiply(transform1, SIMDReplicate(diag, 1));
    SIMDRegister baseZ = SIMDMultiply(transform2, SIMDReplicate(diag, 2));

    // shuffle base vectors, TODO: without conditionals?

    SIMDRegister dir, right, top;
    switch (getFaceAxis(face))
    {
    case 0: dir = baseX; right = baseY; top = baseZ; break;
    case 1: dir = baseY; right = baseZ; top = baseX; break;
    default: dir = baseZ; right = baseX; top = baseY; break;
    }

    if (getFaceDirection(face) != getFlipPortalWinding())
        swap2(top, right);

    // reconstruct min z-plane quad vertices

    SIMDRegister q1 = SIMDAdd(q0, right);
    SIMDRegister q2 = SIMDAdd(q1, top);
    SIMDRegister q3 = SIMDAdd(q0, top);

    SIMDRegister qx1, qy1, qz1, qw1;
    SIMDTranspose(qx1, qy1, qz1, qw1, q0, q1, q2, q3);

    out.x = qx1;
    out.y = qy1;
    out.z = qw1;
    out.expand = dir;

    SIMDRegister qx2 = SIMDAdd(qx1, SIMDReplicate(dir, 0));
    SIMDRegister qy2 = SIMDAdd(qy1, SIMDReplicate(dir, 1));
    SIMDRegister qz2 = SIMDAdd(qz1, SIMDReplicate(dir, 2));
    SIMDRegister qw2 = SIMDAdd(qw1, SIMDReplicate(dir, 3));

    SIMDRegister oneperwA = SIMDReciprocal(qw1);
    SIMDRegister oneperwB = SIMDReciprocal(qw2);
    SIMDRegister cxA = SIMDMultiply(qx1, oneperwA);
    SIMDRegister cyA = SIMDMultiply(qy1, oneperwA);
    SIMDRegister cxB = SIMDMultiply(qx2, oneperwB);
    SIMDRegister cyB = SIMDMultiply(qy2, oneperwB);

    // near and far clip
    int nearFarMask;
    SIMDWriteAnyMask(nearFarMask, SIMDBitwiseAnd(SIMDCompareGE(SIMDMax(qw1, qw2), SIMDZero()),
        SIMDBitwiseOr(SIMDCompareGE(qw1, qz1), SIMDCompareGE(qw2, qz2))));
    if (!nearFarMask)
    {
        mnmx = Vector4i();
        return;
    }

    SIMDRegister clipmnmx;
    if (!canNearClip || !SIMDCompareGTTestAny(SIMDZero(), SIMDMin(qw1, qw2)))
    {
        clipmnmx = combineMinMax(cxA, cxB, cyA, cyB);
    }
    else
    {
        ClipCodes codesA, codesB;
        getClipCodes(qx1, qy1, qw1, codesA);
        getClipCodes(qx2, qy2, qw2, codesB);
        SIMDRegister xCodeA = SIMDBitwiseAnd(codesA.xLeft, codesA.xRight);
        SIMDRegister yCodeA = SIMDBitwiseAnd(codesA.yLeft, codesA.yRight);
        SIMDRegister xCodeB = SIMDBitwiseAnd(codesB.xLeft, codesB.xRight);
        SIMDRegister yCodeB = SIMDBitwiseAnd(codesB.yLeft, codesB.yRight);
        SIMDRegister xmin = SIMDMin(
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cxA, xCodeA), codesA.xLeft),
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cxB, xCodeB), codesB.xLeft));
        SIMDRegister xmax = SIMDMax(
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cxA, xCodeA), codesA.xRight),
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cxB, xCodeB), codesB.xRight));
        SIMDRegister ymin = SIMDMin(
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cyA, yCodeA), codesA.yLeft),
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cyB, yCodeB), codesB.yLeft));
        SIMDRegister ymax = SIMDMax(
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cyA, yCodeA), codesA.yRight),
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cyB, yCodeB), codesB.yRight));
        clipmnmx = SIMDShuffle_A0A1B0B1(combineMin(xmin, ymin), combineMax(xmax, ymax));
    }

    // Transform to screen space and clamp to scissor
    SIMDRegister halfRaster = SIMDLoad(UMBRA_HALF_RASTER);
    SIMDRegister translate = SIMDLoadXXYY(UMBRA_HALF_RASTER, UMBRA_HALF_RASTER + 1.f - FLT_EPSILON * UMBRA_HALF_RASTER);
    SIMDRegister screen = SIMDMultiplyAdd(clipmnmx, halfRaster, translate);
    SIMDRegister32 iMinMax = SIMDClamp32(SIMDFloatToInt(screen), scissorBounds);

    // Store screen-space bounds.
    UMBRA_ASSERT(is128Aligned(&mnmx));
    SIMDStoreAligned32(iMinMax, (int*)&mnmx);
}

void Transformer::transformBox(
    Vector4i& mnmx,
    SIMDRegister mn,
    SIMDRegister mx,
    bool canNearClip,
    float& contribution) const
{
    return transformBox(mnmx, mn, mx, canNearClip, getScissorSIMD(), contribution);
}


void Transformer::transformBox(
    Vector4i& mnmx,
    SIMDRegister mn,
    SIMDRegister mx,
    bool canNearClip) const
{
    float contribution;
    return transformBox(mnmx, mn, mx, canNearClip, getScissorSIMD(), contribution);
}

void Transformer::transformBox(
    Vector4i&           mnmx,
    SIMDRegister        boxMin,
    SIMDRegister        boxMax,
    bool                canNearClip,
    const SIMDRegister32& scissorBounds,
    float&              contribution) const
{
    UMBRA_UNREF(canNearClip);

    // clip-space base vectors

    SIMDRegister sdiag = SIMDSub(boxMax, boxMin);
    SIMDRegister si = SIMDMultiply(SIMDLoadAligned(m_worldToClipTranspose[0]), SIMDReplicate(sdiag, 0));
    SIMDRegister sj = SIMDMultiply(SIMDLoadAligned(m_worldToClipTranspose[1]), SIMDReplicate(sdiag, 1));
    SIMDRegister sk = SIMDMultiply(SIMDLoadAligned(m_worldToClipTranspose[2]), SIMDReplicate(sdiag, 2));

    // transform one corner

    SIMDRegister s0 = SIMDMultiplyAdd(SIMDLoadAligned(m_worldToClipTranspose[0]), SIMDReplicate(boxMin, 0),
                      SIMDMultiplyAdd(SIMDLoadAligned(m_worldToClipTranspose[1]), SIMDReplicate(boxMin, 1),
                      SIMDMultiplyAdd(SIMDLoadAligned(m_worldToClipTranspose[2]), SIMDReplicate(boxMin, 2),
                                      SIMDLoadAligned(m_worldToClipTranspose[3]))));

    // reconstruct transformed corner vertices

    SIMDRegister s1 = SIMDAdd(s0, si);
    SIMDRegister s2 = SIMDAdd(s1, sj);
    SIMDRegister s3 = SIMDAdd(s0, sj);

    SIMDRegister txA, tyA, tzA, twA;
    SIMDTranspose(txA, tyA, tzA, twA, s0, s1, s2, s3);

    SIMDRegister txB = SIMDAdd(txA, SIMDReplicate(sk, 0));
    SIMDRegister tyB = SIMDAdd(tyA, SIMDReplicate(sk, 1));
    SIMDRegister twB = SIMDAdd(twA, SIMDReplicate(sk, 3));

    // perspective divide

    SIMDRegister oneperwA = SIMDReciprocal(twA);
    SIMDRegister oneperwB = SIMDReciprocal(twB);
    SIMDRegister cxA = SIMDMultiply(txA, oneperwA);
    SIMDRegister cyA = SIMDMultiply(tyA, oneperwA);
    SIMDRegister cxB = SIMDMultiply(txB, oneperwB);
    SIMDRegister cyB = SIMDMultiply(tyB, oneperwB);

    SIMDRegister clipmnmx;

    if (!UMBRA_OPT_AVOID_BRANCHES && (!canNearClip || !SIMDCompareGTTestAny(SIMDZero(), SIMDMin(twA, twB))))
    {
        // w > 0 fast path
        UMBRA_ASSERT(!SIMDCompareGTTestAny(SIMDZero(), SIMDMin(twA, twB)));
        clipmnmx = combineMinMax(cxA, cxB, cyA, cyB);
    }
    else
    {
        // slow path that does blinn's minmax scanning and supports w < 0
        ClipCodes codesA, codesB;
        getClipCodes(txA, tyA, twA, codesA);
        getClipCodes(txB, tyB, twB, codesB);
        SIMDRegister xCodeA = SIMDBitwiseAnd(codesA.xLeft, codesA.xRight);
        SIMDRegister yCodeA = SIMDBitwiseAnd(codesA.yLeft, codesA.yRight);
        SIMDRegister xCodeB = SIMDBitwiseAnd(codesB.xLeft, codesB.xRight);
        SIMDRegister yCodeB = SIMDBitwiseAnd(codesB.yLeft, codesB.yRight);
        SIMDRegister xmin = SIMDMin(
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cxA, xCodeA), codesA.xLeft),
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cxB, xCodeB), codesB.xLeft));
        SIMDRegister xmax = SIMDMax(
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cxA, xCodeA), codesA.xRight),
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cxB, xCodeB), codesB.xRight));
        SIMDRegister ymin = SIMDMin(
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cyA, yCodeA), codesA.yLeft),
            SIMDSelect(SIMDMinusOne(), SIMDSelect(SIMDOne(), cyB, yCodeB), codesB.yLeft));
        SIMDRegister ymax = SIMDMax(
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cyA, yCodeA), codesA.yRight),
            SIMDSelect(SIMDOne(), SIMDSelect(SIMDMinusOne(), cyB, yCodeB), codesB.yRight));
        SIMDRegister mn = combineMin(xmin, ymin);
        SIMDRegister mx = combineMax(xmax, ymax);
        clipmnmx = SIMDShuffle_A0A1B0B1(mn, mx);
    }

    // Clamp coordinates to [-1,1] to get correct contribution value
    SIMDRegister clamped = SIMDMax(SIMDMin(clipmnmx, SIMDOne()), SIMDMinusOne());
    // compute dimensions on screen (max - min)
    SIMDRegister diff = SIMDSub(SIMDHighToLow(clamped), clamped);
    // compute area from dimensions (width * height)
    diff = SIMDMultiply(diff, SIMDReplicate(diff, 1));
    // store to float
    SIMDStore(diff, contribution);
    // divide by four to get relative value, since original coordinates are in [-1,1], and the area [0, 2]
    contribution *= 0.25f;

    // Transform to screen space and clamp to scissor
    SIMDRegister halfRaster = SIMDLoad(UMBRA_HALF_RASTER);
    SIMDRegister translate = SIMDLoadXXYY(UMBRA_HALF_RASTER, UMBRA_HALF_RASTER + 1.f - FLT_EPSILON * UMBRA_HALF_RASTER);
    SIMDRegister screen = SIMDMultiplyAdd(clipmnmx, halfRaster, translate);
    SIMDRegister32 iMinMax = SIMDClamp32(SIMDFloatToInt(screen), scissorBounds);

    // Store screen-space bounds.
    UMBRA_ASSERT(is128Aligned(&mnmx));
    SIMDStoreAligned32(iMinMax, (int*)&mnmx);
}

float Transformer::getMaxDeviceZ(SIMDRegister wmn, SIMDRegister wmx) const
{
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, out);
    SIMDRegister fv = SIMDSelect(wmn, wmx, m_clipPlaneClass[0]);
    SIMDRegister fv_z = SIMDDot4(SIMDLoadAligned(m_worldToClip[2]), fv);
    SIMDRegister fv_w = SIMDDot4(SIMDLoadAligned(m_worldToClip[3]), fv);
    SIMDRegister zwzw = SIMDMergeLow(fv_z, fv_w);
    SIMDStoreAligned(zwzw, &out.x);
    if (out[0] <= 0.f)
        return 0.f;
    UMBRA_ASSERT(out[1] > 0.f);
    return out[0] / out[1];
}

float Transformer::getMinDeviceZ(SIMDRegister wmn, SIMDRegister wmx) const
{
    Vector4 UMBRA_ATTRIBUTE_ALIGNED(16, out);
    SIMDRegister nv = SIMDSelect(wmx, wmn, m_clipPlaneClass[0]);
    SIMDRegister nv_z = SIMDDot4(SIMDLoadAligned(m_worldToClip[2]), nv);
    SIMDRegister nv_w = SIMDDot4(SIMDLoadAligned(m_worldToClip[3]), nv);
    SIMDRegister zwzw = SIMDMergeLow(nv_z, nv_w);
    SIMDStoreAligned(zwzw, &out.x);
    if (out[0] <= 0.f)
        return 0.f;
    UMBRA_ASSERT(out[1] > 0.f);
    return out[0] / out[1];
}

}

#endif
