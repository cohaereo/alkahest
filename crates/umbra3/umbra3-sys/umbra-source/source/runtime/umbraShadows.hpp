// Copyright (c) 2010-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRASHADOWS_HPP
#define UMBRASHADOWS_HPP

#include "umbraQueryArgs.hpp"
#include "runtime/umbraQuery.hpp"
#include "umbraQueryContext.hpp"
#include "umbraSIMD.hpp"
#include "umbraShadowDefs.hpp"

#undef LEFT
#undef RIGHT
#undef TOP
#undef BOTTOM
#undef NEAR
#undef FAR
#undef near
#undef far
#undef min
#undef max

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
* \brief
*
* \note     Assumes z-range is always [0,1]
*//*-------------------------------------------------------------------*/

class ShadowUtils
{
public:

    enum ClipPlane
    {
        NEAR = 0,
        FAR,
        LEFT,
        RIGHT,
        BOTTOM,
        TOP
    };

    enum
    {
        MaxShadowClipPlanes = 16
    };

public:

    static void     getOrthoProjection      (Matrix4x4&         outMatrix,
                                             const Vector3&     mn,
                                             const Vector3&     mx);
    static void     getOrthoProjection      (Matrix4x4&         outMatrix,
                                             float              mnx,
                                             float              mxx,
                                             float              mny,
                                             float              mxy,
                                             float              mnz,
                                             float              mxz);
    static Vector4  normalizePlaneEquation  (const Vector4&     inPEQ);
    static void     getFrustumVertices      (Vector3            outVertices[8],
                                             const Vector4      planes[6]);
    static void     getClipPlanes           (const Matrix4x4&   inMatrix,
                                             Vector4            outPEQs[6],
                                             bool&              hasFarPlane);
    static Vector3  getCameraDof            (const Matrix4x4&   inWorldToClip);
    static void     getShadowClipPlanes     (const Vector3&     inLightToWorldDir,
                                             Vector4            inFrustumPlanes[6],
                                             Vector4            outPlaneArray[MaxShadowClipPlanes],
                                             int&               outPlaneCount);
    static void     getWorldToLightMatrix   (Matrix4x4&         outWorldToLight,
                                             const Matrix4x4&   inWorldToClip,
                                             const Vector3&     inLightToWorldDir);
    static void     getAABB                 (AABB&              outAABB,
                                             const Matrix4x4&   inBasis,
                                             const Vector3*     inVertexArray,
                                             int                inVertexCount);
    static void     getAABB                 (AABB&              outAABB,
                                             const Matrix4x4&   inBasis,
                                             const AABB&        inAABB);
    static void     getLightSpaceAABB       (AABB&              outAABB,
                                             const Matrix4x4&   inWorldToLight,
                                             const Vector4      inFrustumPlanes[6],
                                             const AABB&        worldBounds);
};

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

class SceneBounds
{
public:

    SceneBounds(
        QueryContext&       ctx,
        const IndexList*    staticObjectList,
        const Vector3*      dynamicObjectArray,
        int                 visibleDynamicObjectCount)
    {
        init(ctx, staticObjectList, dynamicObjectArray, visibleDynamicObjectCount);
    }

    void init(
        QueryContext&       ctx,
        const IndexList*    staticObjectList,
        const Vector3*      dynamicObjectArray,
        int                 dynamicObjectCount)
    {
        UMBRA_UNREF(dynamicObjectCount);

        int visibleStaticObjectCount = 0;
        int visibleDynamicObjectCount = 0;

        if (staticObjectList)
            visibleStaticObjectCount += staticObjectList->getSize();
        visibleDynamicObjectCount += dynamicObjectCount;

        int objectCount = visibleStaticObjectCount + visibleDynamicObjectCount;

        if (objectCount == 0)
            return;

        /* \todo [antti 26.2.2013]: use ArrayIndexedIterator here */

        Vector3 wmn(FLT_MAX, FLT_MAX, FLT_MAX);
        Vector3 wmx(-FLT_MAX, -FLT_MAX, -FLT_MAX);

        const ImpTome* tome = ctx.getTome();
        if (!!tome->getObjectBounds())
        {
            // Compute world space bounds
            ArrayMapper boundMapper(&ctx, tome->getObjectBounds());
            for (int i = 0; i < visibleStaticObjectCount; i++)
            {
                ObjectBounds bounds;
                boundMapper.get(bounds, staticObjectList->getPtr()[i]);

                wmn = min(bounds.mn, wmn);
                wmx = max(bounds.mx, wmx);
            }
        }

        for (int i = 0; i < visibleDynamicObjectCount; i++)
        {
            wmn = min(dynamicObjectArray[2*i], wmn);
            wmx = max(dynamicObjectArray[2*i+1], wmx);
        }

        m_AABB.set(wmn, wmx);
    }


    const AABB& getAABB(void) const
    {
        return m_AABB;
    }

private:

    AABB    m_AABB;
};

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

class ReceiverMask
{
public:

    enum
    {
        BufferSize = UMBRA_RECEIVER_MASK_BUFFER_SIZE,
        // one value reserved for -inf
        MaxDepth = 65534
    };

public:

    ReceiverMask (void)
    {
        clear(0);

        m_halfBufferSize = SIMDLoad(BufferSize/2);
        m_bufferSize     = SIMDLoad(BufferSize);
        m_maxZ           = SIMDLoad((float)MaxDepth);
    }

    bool addAABB(const Vector3& mn, const Vector3& mx)
    {
        int left, top, right, bottom, far;
        if (!getClampedRect(left, top, right, bottom, far, mn, mx))
            return false;

        int loopCount = right - left;
        int remainder = loopCount & 3;
        int limit     = right - remainder;

        for (int y = bottom; y < top; y++)
        {
            for (int x = left; x < limit; x+=4)
            {
                UINT16* writePtr      = &m_mask[BufferSize*y+x];
                const UINT16* readPtr = writePtr;

                UINT16 z0 = *readPtr++;
                UINT16 z1 = *readPtr++;
                UINT16 z2 = *readPtr++;
                UINT16 z3 = *readPtr++;

                *writePtr++ = max2(z0, (UINT16)far);
                *writePtr++ = max2(z1, (UINT16)far);
                *writePtr++ = max2(z2, (UINT16)far);
                *writePtr++ = max2(z3, (UINT16)far);
            }

            int x = limit;
            while (x < right)
            {
                UINT16 old = m_mask[BufferSize*y+x];
                m_mask[BufferSize*y+x] = max2(old, (UINT16)far);
                x++;
            }
        }
        return true;
    }

    float getFloatDepth(int x, int y)
    {
        UINT16 intDepth = m_mask[BufferSize*y+x];
        return float(intDepth)/65535.0f;
    }

    UMBRA_FORCE_INLINE bool testAABB_SIMD(const SIMDRegister& rmn, const SIMDRegister& rmx) const
    {
        // Compute screen space rectangle

        SIMDRegister minMaxXY = SIMDShuffle_A0A1B0B1(rmn, rmx);
        minMaxXY = SIMDMultiplyAdd(minMaxXY, m_halfBufferSize, SIMDAdd(m_halfBufferSize, SIMDLoadXXYY(0.f, 1.f)));
        SIMDRegister32 minMaxXY32 = SIMDFloatToInt(SIMDMin(m_bufferSize, SIMDMax(SIMDZero(), minMaxXY)));

        // Compute min z

        SIMDRegister n = SIMDMultiply(SIMDReplicate(rmn, 2), m_maxZ);
        SIMDRegister32 n32 = SIMDFloatToInt(SIMDMin(m_maxZ, SIMDMax(SIMDZero(), n)));

        Vector4i UMBRA_ATTRIBUTE_ALIGNED16(rect);
        Vector4i UMBRA_ATTRIBUTE_ALIGNED16(depth);
        SIMDStoreAligned32(minMaxXY32, &rect.i);
        // TODO: replace with scalar store
        SIMDStoreAligned32(n32, &depth.i);

        for (int y = rect[1]; y < rect[3]; y++)
        for (int x = rect[0]; x < rect[2]; x++)
        {
            if (depth[0] < m_mask[BufferSize*y+x])
                return true;
        }

        return false;
    }

    const UINT16* getPtr(void) const
    {
        return m_mask;
    }

    void clear(int value)
    {
        memset(m_mask, value, BufferSize*BufferSize*sizeof(UINT16));
    }

private:

    bool getClampedRect(int& left, int& top, int& right, int& bottom, int& far, const Vector3& mn, const Vector3& mx) const
    {
        // Scalar code

        left = int((mn.x+1.0f)*0.5f*(float)BufferSize);
        left = max2(left, 0);

        bottom = int((mn.y+1.0f)*0.5f*(float)BufferSize);
        bottom = max2(bottom, 0);

        right  = int((mx.x+1.0f)*0.5f*(float)BufferSize)+1;
        right = min2(right, (int)BufferSize);

        top  = int((mx.y+1.0f)*0.5f*(float)BufferSize)+1;
        top = min2(top, (int)BufferSize);

        if (left >= right || bottom >= top || mx.z < 0.f)
            return false;

        // We increase far values by one and then do LT operation in test.
        // This is to reserve value 0 for meaning negative infinity (no receiver)
        far = int(MaxDepth*mx.z)+1;
        far = min2(far, MaxDepth+1);

        return true;
    }

    SIMDRegister  m_halfBufferSize;
    SIMDRegister  m_bufferSize;
    SIMDRegister  m_maxZ;
    UINT16        m_mask[BufferSize*BufferSize];
};

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

class DownsampledDepthBuffer
{
public:

    enum
    {
        Size = UMBRA_RECEIVER_MASK_DOWNSAMPLED_DEPTH_BUFFER_SIZE
    };

    DownsampledDepthBuffer (const Matrix4x4& worldToClip, const Matrix4x4& clipToWorld, const Vector3& cameraPos, const ImpOcclusionBuffer* dyn)
    {
        UMBRA_ASSERT(UMBRA_PORTAL_RASTER_SIZE % Size == 0);
        UMBRA_ASSERT(dyn && dyn->getDepthBufferPtr(false));

        UMBRA_UNREF(cameraPos);
        UMBRA_UNREF(clipToWorld);
        UMBRA_UNREF(worldToClip);

        int s = UMBRA_PORTAL_RASTER_SIZE / Size;

        for (int j = 0; j < Size; j++)
        for (int i = 0; i < Size; i++)
        {
            float currentMax = 0;

            for (int y = s*j; y < s*(j+1); y++)
            for (int x = s*i; x < s*(i+1); x++)
            {
                float z = dyn->readDepth(x, y);
                if (z != ImpOcclusionBuffer::getMaxDepth() && z > currentMax)
                    currentMax = z;
            };

            m_buffer[j*Size+i] = currentMax;
        }
    }

    UMBRA_FORCE_INLINE float getDepth(int i, int j) const
    {
        return m_buffer[j*Size+i];
    }

private:

    float           m_buffer[Size*Size];
};

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

class ReceiverMaskCuller

{
public:

    enum
    {
        FrustumSplits = UMBRA_RECEIVER_MASK_FRUSTUM_SPLIT_COUNT
    };

public:

    void init (const Vector3& cameraPos, const Matrix4x4& worldToClip, const Matrix4x4& clipToWorld, const Matrix4x4& worldToLightClip, const ImpOcclusionBuffer* occBuffer)
    {
        DownsampledDepthBuffer depthBuffer(worldToClip, clipToWorld, cameraPos, occBuffer);
        m_worldToLightClip = worldToLightClip;
        m_worldToLightClipTranspose = worldToLightClip;
        m_worldToLightClipTranspose.transpose();

        float A        = 2.0f / float(DownsampledDepthBuffer::Size);
        float B        = -1.0f;
        float offset   = A*0.5f;

        Vector3 nearBase = clipToWorld.transformDivByW(Vector3(-1.f, -1.f, 0.f));
        Vector3 nearX    = clipToWorld.transformDivByW(Vector3(1.f, -1.f, 0.f)) - nearBase;
        Vector3 nearY    = clipToWorld.transformDivByW(Vector3(-1.f, 1.f, 0.f)) - nearBase;
        nearX *= (1.f / float(DownsampledDepthBuffer::Size));
        nearY *= (1.f / float(DownsampledDepthBuffer::Size));

        Vector3 mn, mx;
        Vector3 y = nearBase;

        for (int j = 0; j < DownsampledDepthBuffer::Size; j++)
        {
            Vector3 cur = y;
            y += nearY;

            for (int i = 0; i < DownsampledDepthBuffer::Size; i++)
            {
                float cx    = A*float(i+0.5f) + B;
                float cy    = A*float(j+0.5f) + B;
                float depth = depthBuffer.getDepth(i,j);

                if (!depth)
                {
                    cur += nearX;
                    continue;
                }

                Vector3 nearQuad[4] =
                {
                    cur,
                    cur + nearY,
                    cur + nearX + nearY,
                    cur + nearX,
                };

                Vector3 farVector[4] =
                {
                    clipToWorld.transformDivByW(Vector3(cx-offset, cy-offset, depth)),
                    clipToWorld.transformDivByW(Vector3(cx-offset, cy+offset, depth)),
                    clipToWorld.transformDivByW(Vector3(cx+offset, cy+offset, depth)),
                    clipToWorld.transformDivByW(Vector3(cx+offset, cy-offset, depth)),
                };

#if 0
                query->addQueryDebugLine(nearQuad[0], nearQuad[1], Vector4(1,1,1,1));
                query->addQueryDebugLine(nearQuad[1], nearQuad[2], Vector4(1,1,1,1));
                query->addQueryDebugLine(nearQuad[2], nearQuad[3], Vector4(1,1,1,1));
                query->addQueryDebugLine(nearQuad[3], nearQuad[0], Vector4(1,1,1,1));

                query->addQueryDebugLine(farVector[0], farVector[1], Vector4(1,1,1,1));
                query->addQueryDebugLine(farVector[1], farVector[2], Vector4(1,1,1,1));
                query->addQueryDebugLine(farVector[2], farVector[3], Vector4(1,1,1,1));
                query->addQueryDebugLine(farVector[3], farVector[0], Vector4(1,1,1,1));

                query->addQueryDebugLine(nearQuad[0], farVector[0], Vector4(1,1,1,1));
                query->addQueryDebugLine(nearQuad[1], farVector[1], Vector4(1,1,1,1));
                query->addQueryDebugLine(nearQuad[2], farVector[2], Vector4(1,1,1,1));
                query->addQueryDebugLine(nearQuad[3], farVector[3], Vector4(1,1,1,1));
#endif

                farVector[0] -= nearQuad[0];
                farVector[1] -= nearQuad[1];
                farVector[2] -= nearQuad[2];
                farVector[3] -= nearQuad[3];

#if 0
                if (flags & QueryExt::SHADOWQUERY_DEBUG)
                {
                    Vector3 dir = normalize(quad[0]+quad[1]+quad[2]+quad[3]);
                    Vector3 center = depth / dot(dir, eye) * dir + cameraPos;

                    //Vector3 dir = normalize(0.25f*(quad[0]+quad[1]+quad[2]+quad[3])-cameraPos);
                    //Vector3 center = -depth * dir + cameraPos;

                    query->addQueryDebugLine(cameraPos, cameraPos + depth*quad[0], Vector4(1,0,0,1));
                    query->addQueryDebugLine(cameraPos, cameraPos + depth*quad[1], Vector4(1,0,0,1));
                    query->addQueryDebugLine(cameraPos, cameraPos + depth*quad[2], Vector4(1,0,0,1));
                    query->addQueryDebugLine(cameraPos, cameraPos + depth*quad[3], Vector4(1,0,0,1));
                }
#endif

                for (int n = 0; n < FrustumSplits; n++)
                {
                    float f;
                    Vector3 v;

                    mn = Vector3(FLT_MAX, FLT_MAX, FLT_MAX);
                    mx = Vector3(-FLT_MAX, -FLT_MAX, -FLT_MAX);

                    for (int k = 0; k < 4; k++)
                    {
                        f = n/float(FrustumSplits);
                        v = worldToLightClip.transformProjectToXYZ(f * farVector[k] + nearQuad[k]);
                        mn = min(v, mn);
                        mx = max(v, mx);

                        f = (n + 1.0f) / float(FrustumSplits);
                        v = worldToLightClip.transformProjectToXYZ(f * farVector[k] + nearQuad[k]);
                        mn = min(v, mn);
                        mx = max(v, mx);
                    }

#if !defined(UMBRA_RECEIVER_MASK_EXTRA_DEBUG_VISUALIZATION)
                    m_receiverMask.addAABB(mn, mx);
#else
                    if (m_receiverMask.addAABB(mn,mx))
                    {
                        if (flags & QueryExt::SHADOWQUERY_DEBUG)
                        {
                            AABB aabb(mn,mx);

                            Vector3 worldSpacePoints[8];
                            for (int i = 0; i < 8; i++)
                                worldSpacePoints[i] = lightClipToWorld.transformDivByW(aabb.getCorner((AABB::Corner)i));

                            Vector4 clr(0,1,0,1);
                            for (int i = 0; i < 4; i++)
                            {
                                //query->addQueryDebugQuad(worldSpacePoints[0], worldSpacePoints[1], worldSpacePoints[3], worldSpacePoints[2], clr);
                                query->addQueryDebugQuad(worldSpacePoints[4], worldSpacePoints[5], worldSpacePoints[7], worldSpacePoints[6], clr);
                            }
                            query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[1], clr);
                            query->addQueryDebugLine(worldSpacePoints[2], worldSpacePoints[3], clr);
                            query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[2], clr);
                            query->addQueryDebugLine(worldSpacePoints[1], worldSpacePoints[3], clr);

                            query->addQueryDebugLine(worldSpacePoints[4], worldSpacePoints[5], clr);
                            query->addQueryDebugLine(worldSpacePoints[6], worldSpacePoints[7], clr);
                            query->addQueryDebugLine(worldSpacePoints[4], worldSpacePoints[6], clr);
                            query->addQueryDebugLine(worldSpacePoints[5], worldSpacePoints[7], clr);

                            query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[4], clr);
                            query->addQueryDebugLine(worldSpacePoints[1], worldSpacePoints[5], clr);
                            query->addQueryDebugLine(worldSpacePoints[2], worldSpacePoints[6], clr);
                            query->addQueryDebugLine(worldSpacePoints[3], worldSpacePoints[7], clr);
                        }
                    }
#endif
                }

                cur += nearX;
            }
        }
    }

    void addAABB(const Vector3& inmn, const Vector3& inmx)
    {
        Vector3 mn, mx;
        computeLightSpaceAABB(mn, mx, inmn, inmx);

        m_receiverMask.addAABB(mn, mx);
    }

    void computeLightSpaceAABB(Vector3& mn, Vector3& mx, const Vector3& inmn, const Vector3& inmx) const
    {
        mn = mx = Vector3(0,0,0);

        for (int i = 0; i < 3; i++)
        {
            Vector4 r = m_worldToLightClip.getRow(i);

            mn[i] += min2(r.x*inmn.x, r.x*inmx.x);
            mn[i] += min2(r.y*inmn.y, r.y*inmx.y);
            mn[i] += min2(r.z*inmn.z, r.z*inmx.z);
            mn[i] += r.w;

            mx[i] += max2(r.x*inmn.x, r.x*inmx.x);
            mx[i] += max2(r.y*inmn.y, r.y*inmx.y);
            mx[i] += max2(r.z*inmn.z, r.z*inmx.z);
            mx[i] += r.w;
        }
    }

    UMBRA_FORCE_INLINE void computeLightSpaceAABB_SIMD(SIMDRegister& mn, SIMDRegister& mx, const SIMDRegister& v0, const SIMDRegister& v1) const
    {
        SIMDRegister c0 = SIMDLoadAligned(m_worldToLightClipTranspose[0]);
        SIMDRegister c1 = SIMDLoadAligned(m_worldToLightClipTranspose[1]);
        SIMDRegister c2 = SIMDLoadAligned(m_worldToLightClipTranspose[2]);
        SIMDRegister c3 = SIMDLoadAligned(m_worldToLightClipTranspose[3]);

        SIMDRegister v0x = SIMDMultiply(c0, SIMDReplicate(v0, 0));
        SIMDRegister v0y = SIMDMultiply(c1, SIMDReplicate(v0, 1));
        SIMDRegister v0z = SIMDMultiply(c2, SIMDReplicate(v0, 2));
        SIMDRegister v1x = SIMDMultiply(c0, SIMDReplicate(v1, 0));
        SIMDRegister v1y = SIMDMultiply(c1, SIMDReplicate(v1, 1));
        SIMDRegister v1z = SIMDMultiply(c2, SIMDReplicate(v1, 2));

        SIMDRegister xMask = SIMDCompareGT(v0x, v1x);
        SIMDRegister yMask = SIMDCompareGT(v0y, v1y);
        SIMDRegister zMask = SIMDCompareGT(v0z, v1z);

        mn = SIMDAdd(c3, SIMDSelect(v0x, v1x, xMask));
        mn = SIMDAdd(mn, SIMDSelect(v0y, v1y, yMask));
        mn = SIMDAdd(mn, SIMDSelect(v0z, v1z, zMask));
        mx = SIMDAdd(c3, SIMDSelect(v1x, v0x, xMask));
        mx = SIMDAdd(mx, SIMDSelect(v1y, v0y, yMask));
        mx = SIMDAdd(mx, SIMDSelect(v1z, v0z, zMask));
    }

    UMBRA_FORCE_INLINE bool isAABBVisible_SIMD(const SIMDRegister& inmn, const SIMDRegister& inmx) const
    {
        SIMDRegister mn, mx;
        computeLightSpaceAABB_SIMD(mn, mx, inmn, inmx);
        return m_receiverMask.testAABB_SIMD(mn, mx);
    }

    void debugDraw(QueryContext* query)
    {
        for (int y = 0; y < ReceiverMask::BufferSize; y++)
        for (int x = 0; x < ReceiverMask::BufferSize; x++)
        {
            float x0 = float(x)/float(ReceiverMask::BufferSize)*2.0f-1.0f;
            float y0 = float(y)/float(ReceiverMask::BufferSize)*2.0f-1.0f;
            float x1 = float(x+1)/float(ReceiverMask::BufferSize)*2.0f-1.0f;
            float y1 = float(y+1)/float(ReceiverMask::BufferSize)*2.0f-1.0f;
            float z = m_receiverMask.getFloatDepth(x,y);

            Vector3 mn(x0,y0,z);
            Vector3 mx(x1,y1,z);

            AABB aabb(mn,mx);

            Vector3 worldSpacePoints[8];
            for (int i = 0; i < 8; i++)
                worldSpacePoints[i] = m_lightClipToWorld.transformDivByW(aabb.getCorner((AABB::Corner)i));

            Vector4 clr(1,0,0,1);
            for (int i = 0; i < 4; i++)
            {
                //query->addQueryDebugQuad(worldSpacePoints[0], worldSpacePoints[1], worldSpacePoints[3], worldSpacePoints[2], clr);
                query->addQueryDebugQuad(worldSpacePoints[4], worldSpacePoints[5], worldSpacePoints[7], worldSpacePoints[6], clr);
            }
            query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[1], clr);
            query->addQueryDebugLine(worldSpacePoints[2], worldSpacePoints[3], clr);
            query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[2], clr);
            query->addQueryDebugLine(worldSpacePoints[1], worldSpacePoints[3], clr);

            query->addQueryDebugLine(worldSpacePoints[4], worldSpacePoints[5], clr);
            query->addQueryDebugLine(worldSpacePoints[6], worldSpacePoints[7], clr);
            query->addQueryDebugLine(worldSpacePoints[4], worldSpacePoints[6], clr);
            query->addQueryDebugLine(worldSpacePoints[5], worldSpacePoints[7], clr);

            query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[4], clr);
            query->addQueryDebugLine(worldSpacePoints[1], worldSpacePoints[5], clr);
            query->addQueryDebugLine(worldSpacePoints[2], worldSpacePoints[6], clr);
            query->addQueryDebugLine(worldSpacePoints[3], worldSpacePoints[7], clr);
        }
    }

    void debugDrawAABB(QueryContext* query, const Vector3& inmn, const Vector3& inmx)
    {
        Vector3 mn, mx;
        computeLightSpaceAABB(mn, mx, inmn, inmx);
        AABB aabb(mn,mx);

        Vector3 worldSpacePoints[8];
        for (int i = 0; i < 8; i++)
            worldSpacePoints[i] = m_lightClipToWorld.transformDivByW(aabb.getCorner((AABB::Corner)i));

        Vector4 clr(0,0,1,1);
        for (int i = 0; i < 4; i++)
        {
            query->addQueryDebugQuad(worldSpacePoints[4], worldSpacePoints[5], worldSpacePoints[7], worldSpacePoints[6], clr);
        }
        query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[1], clr);
        query->addQueryDebugLine(worldSpacePoints[2], worldSpacePoints[3], clr);
        query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[2], clr);
        query->addQueryDebugLine(worldSpacePoints[1], worldSpacePoints[3], clr);

        query->addQueryDebugLine(worldSpacePoints[4], worldSpacePoints[5], clr);
        query->addQueryDebugLine(worldSpacePoints[6], worldSpacePoints[7], clr);
        query->addQueryDebugLine(worldSpacePoints[4], worldSpacePoints[6], clr);
        query->addQueryDebugLine(worldSpacePoints[5], worldSpacePoints[7], clr);

        query->addQueryDebugLine(worldSpacePoints[0], worldSpacePoints[4], clr);
        query->addQueryDebugLine(worldSpacePoints[1], worldSpacePoints[5], clr);
        query->addQueryDebugLine(worldSpacePoints[2], worldSpacePoints[6], clr);
        query->addQueryDebugLine(worldSpacePoints[3], worldSpacePoints[7], clr);
    }

    const UINT16* getReceiverMaskBufferPtr(void) const
    {
        return (const UINT16*) m_receiverMask.getPtr();
    }

    const Matrix4x4& getWorldToLightClip(void) const
    {
        return m_worldToLightClip;
    }

private:

    Matrix4x4    UMBRA_ATTRIBUTE_ALIGNED16(m_worldToLightClip);
    Matrix4x4    UMBRA_ATTRIBUTE_ALIGNED16(m_worldToLightClipTranspose);
    Matrix4x4    m_lightClipToWorld;
    ReceiverMask m_receiverMask;
};

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

class SinglePlaneCuller
{
public:

    void init(const Vector4& planeEquation)
    {
        m_planeEquation = SIMDLoad(planeEquation);
        m_selectMask    = SIMDCompareGT(m_planeEquation, SIMDZero());
    }

    bool isVisible(const Vector4& mn, const Vector4& mx) const
    {
        SIMDRegister rmn = SIMDLoad(mn);
        SIMDRegister rmx = SIMDLoad(mx);
        return isVisible(rmn, rmx);
    }

    bool isVisible(SIMDRegister mn, SIMDRegister mx) const
    {
        return !SIMDCompareGTTestAny(SIMDZero(), SIMDDot4(m_planeEquation, SIMDSelect(mn, mx, m_selectMask)));
    }

private:

    SIMDRegister m_selectMask;
    SIMDRegister m_planeEquation;
};

class ImpShadowCuller
{
public:
    static const int MAX_CASCADES = 8;

    ImpShadowCuller (void) : m_hasMask(false), m_hasCustomFarPlane(false), m_hasVisibleStaticObjs(false), m_numCascades(0) {}

    QueryExt::ErrorCode getBuffer (ImpReceiverMaskBuffer* dst) const
    {
        dst->init(m_receiverMaskCuller.getReceiverMaskBufferPtr(), m_receiverMaskCuller.getWorldToLightClip());
        return Query::ERROR_OK;
    }

    void initReceiverMask (const Vector3& cameraPos, const Matrix4x4& worldToClip, const Matrix4x4& clipToWorld, const Matrix4x4& worldToLightClip, const ImpOcclusionBuffer* occBuffer)
    {
        getReceiverMaskCuller().init(cameraPos, worldToClip, clipToWorld, worldToLightClip, occBuffer);
        m_hasMask = true;
    }

    bool initCascades (const CameraTransform** cascades, int numCascades)
    {
        UMBRA_ASSERT(numCascades >= 0 && numCascades < MAX_CASCADES);
        m_numCascades = numCascades;
        for (int i = 0; i < numCascades; i++)
        {
            if (!cascades[i])
                return false;
            m_cascades[i].init(*IMPL(cascades[i]), Vector3(), 0, 1, 0);
        }
        return true;
    }

    void                setFlags (UINT32 flags) { m_flags = flags; }
    UINT32              getFlags (void) const   { return m_flags; }

    int                 getNumCascades()  const { return m_numCascades; }
    const Transformer&  getCascade(int i) const { UMBRA_ASSERT(i >= 0 && i < m_numCascades); return m_cascades[i]; }

    ReceiverMaskCuller& getReceiverMaskCuller   (void) { return m_receiverMaskCuller; }
    SinglePlaneCuller&  getSinglePlaneCuller    (void) { return m_singlePlaneCuller; }
    Transformer&        getPlaneCuller          (void) { return m_planeCuller; }
    const ReceiverMaskCuller& getReceiverMaskCuller   (void) const { return m_receiverMaskCuller; }
    const SinglePlaneCuller&  getSinglePlaneCuller    (void) const { return m_singlePlaneCuller; }
    const Transformer&        getPlaneCuller          (void) const { return m_planeCuller; }
    
    void setLightDir (const Vector3& dir) { m_lightDir = dir; }
    const Vector3& getLightDir (void) const { return m_lightDir; }

    void setCameraPos (const Vector3& pos) { m_cameraPos = pos; }
    Vector3 getCameraPos (void) const { return m_cameraPos; }

    void setCustomFarPlane (bool hasCustom) { m_hasCustomFarPlane = hasCustom; }
    bool hasCustomFarPlane (void) const     { return m_hasCustomFarPlane; }

    bool setVisibleStaticObjs (int numObjs)
    {
        m_hasVisibleStaticObjs = (numObjs <= getMaxVisibleStaticObjs());
        return m_hasVisibleStaticObjs;
    }

    UINT32* getVisibleStaticObjs (void)
    {
        return (m_hasVisibleStaticObjs ? m_visibleStaticObjs : NULL);
    }
    
    const UINT32* getVisibleStaticObjs (void) const
    {
        return const_cast<ImpShadowCuller*>(this)->getVisibleStaticObjs();
    }

    UMBRA_FORCE_INLINE bool isAABBActivePlanes (SIMDRegister mn, SIMDRegister mx, ActivePlaneSet* activePlaneSet = NULL) const
    {
        return m_planeCuller.frustumTestBounds(activePlaneSet, mn, mx);
    }

    UMBRA_FORCE_INLINE bool isAABBActiveMask (SIMDRegister mn, SIMDRegister mx) const
    {
        if (m_hasMask && !m_receiverMaskCuller.isAABBVisible_SIMD(mn, mx))
            return false;

        return true;
    }

    UMBRA_FORCE_INLINE bool isAABBActive (SIMDRegister mn, SIMDRegister mx, ActivePlaneSet* activePlaneSet = NULL) const
    {
        return isAABBActivePlanes(mn, mx, activePlaneSet) && 
               isAABBActiveMask(mn, mx);
    }

private:
    int getMaxVisibleStaticObjs (void)
    {
        // max bitvector length
        return ((((UINTPTR)this) + UMBRA_SHADOW_CULLER_SIZE - (UINTPTR)m_visibleStaticObjs) &~3) * 8;
    }

    ReceiverMaskCuller  m_receiverMaskCuller;
    SinglePlaneCuller   m_singlePlaneCuller;
    Transformer         m_planeCuller;
    Vector3             m_cameraPos;
    Vector3             m_lightDir;

    // \todo hash for visible dynamic objects?
    bool                m_hasMask;
    bool                m_hasCustomFarPlane;
    bool                m_hasVisibleStaticObjs;
    UINT32              m_visibleStaticObjs[1];

    UINT32              m_flags;
    int                 m_numCascades;
    Transformer         m_cascades[MAX_CASCADES];

};

} // namespace Umbra

#endif /// UMBRASHADOWS_HPP
