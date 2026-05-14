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
 * \brief   Umbra query arguments
 *
 */

#include "umbraQueryArgs.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraQueryContext.hpp"
#include "umbraIntersect.hpp"
#include "umbraDepthBuffer.hpp"
#include <float.h>

using namespace Umbra;


namespace Umbra
{
static UMBRA_INLINE Vector3 getClipSpaceCoord(int x, int y, float z)
{
    float xc = -1.f + ((float)x / (UMBRA_PORTAL_RASTER_SIZE >> 1));
    float yc = -1.f + ((float)y / (UMBRA_PORTAL_RASTER_SIZE >> 1));
    return Vector3(xc, yc, z);
}
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpCameraTransform::update(void)
{
    if (!m_separate)
        return;
#if UMBRA_ARCH != UMBRA_SPU
    Matrix4x4 view(m_view);
    if (m_mf == MF_COLUMN_MAJOR)
        view.transpose();

    // note: this makes assumptions about view matrix handedness and z direction!
    // note: this only works for perspective projection!
    float m00 = 2.0f * m_frustum.zNear / (m_frustum.right - m_frustum.left);
    float m11 = 2.0f * m_frustum.zNear / (m_frustum.top - m_frustum.bottom);
    float m22 = m_frustum.zFar / (m_frustum.zFar - m_frustum.zNear);
    float m32 = 1.0f;
    float m23 = (-1.0f * m_frustum.zNear * m_frustum.zFar) / (m_frustum.zFar - m_frustum.zNear);
    float m33 = 0.0f;
    float m02 = (m_frustum.left + m_frustum.right) / (m_frustum.right - m_frustum.left);
    float m12 = (m_frustum.bottom + m_frustum.top) / (m_frustum.top - m_frustum.bottom);

    Matrix4x4 proj(
        m00, 0.f, m02, 0.f,
        0.f, m11, m12, 0.f,
        0.f, 0.f, m22, m23,
        0.f, 0.f, m32, m33);

    m_transform = view * proj;
    view.invert();
    m_position = view.getTranslation();
#else
    UMBRA_ASSERT(!"legacy camera format not supported on SPU");
#endif
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

VisibilityResult::VisibilityResult(QueryContext& ctx, const Visibility& params_, const Transformer& transform, bool hasDepth)
: m_ctx(&ctx)
{
    const ImpVisibility* params = IMPL(&params_);
    m_objects = IMPL(params->m_objects);
    m_clusters = IMPL(params->m_clusters);
    m_processedObjectVector = NULL;
    m_visibleObjectVector = NULL;
    m_clusterVector = NULL;
    m_occlusionBuffer = NULL;
    m_inputDepthBuffer = NULL;
    m_remoteOcclusionBuffer = 0;

    if ((params->m_inputBuffer != NULL) &&
        (params->m_inputBuffer == params->m_occlusionBuffer))
    {
        // same buffer as input&output currently not supported
        ctx.setError(Query::ERROR_INVALID_ARGUMENT);
        return;
    }

    if (params->m_objectMask)
    {
#if !defined(UMBRA_REMOTE_MEMORY)
        m_visibleObjectVector = params->m_objectMask;
        memset(m_visibleObjectVector, 0, UMBRA_BITVECTOR_SIZE(m_ctx->getTome()->getNumObjects()));
#else
        // not supported
        ctx.setError(Query::ERROR_INVALID_ARGUMENT);
        return;
#endif
    }

    if (hasObjectVisibility())
    {
        m_processedObjectVector = UMBRA_HEAP_NEW_ARRAY(m_ctx->getAllocator(), UINT32, UMBRA_BITVECTOR_DWORDS(m_ctx->getTome()->getNumObjects()));
        if (params->m_filter)
        {
            const UserList<int>* input = IMPL(params->m_filter);
            memset(m_processedObjectVector, 0xFF, UMBRA_BITVECTOR_SIZE(m_ctx->getTome()->getNumObjects()));

            if (input->isRemote())
            {
                const int BATCH_SIZE = 64;
                int UMBRA_ATTRIBUTE_ALIGNED16(batch0[BATCH_SIZE]);
                int UMBRA_ATTRIBUTE_ALIGNED16(batch1[BATCH_SIZE]);
                int* batches[] = {batch0, batch1};
                int currBatch = 0;
                int* inputBuf = input->getBuf();
                int remaining = input->getSize();
                MemoryAccess::alignedReadAsync(batches[0], inputBuf, sizeof(int) * BATCH_SIZE, UMBRA_RESERVED_TAG);

                while (remaining > 0)
                {
                    int nextBatch = currBatch ^ 1;
                    int currBatchSize = BATCH_SIZE;
                    if (remaining < BATCH_SIZE)
                        currBatchSize = remaining;
                    remaining -= currBatchSize;
                    inputBuf += BATCH_SIZE;

                    MemoryAccess::wait(UMBRA_RESERVED_TAG);
                    MemoryAccess::alignedReadAsync(batches[nextBatch], inputBuf, sizeof(int) * BATCH_SIZE, UMBRA_RESERVED_TAG);

                    for (int i = 0; i < currBatchSize; i++)
                        clearBit(m_processedObjectVector, batches[currBatch][i]);
                    currBatch = nextBatch;
                }
                MemoryAccess::wait(UMBRA_RESERVED_TAG);
            }
            else
            {
                for (int i = 0; i < input->getSize(); i++)
                    clearBit(m_processedObjectVector, input->get(i));
            }
        }
        else
        {
            memset(m_processedObjectVector, 0, UMBRA_BITVECTOR_SIZE(m_ctx->getTome()->getNumObjects()));
        }
        if (m_objects)
            m_objects->clear();

        if (params->m_objectDistances)
        {
            m_objectDistances = UserList<float>(params->m_objectDistances, m_ctx->getTome()->getNumObjects());
        }
        
        if (params->m_objectContributions)
        {
            m_objectContributions = UserList<float>(params->m_objectContributions, m_objects->getCapacity());
        }
    }
    if (m_clusters)
    {
        m_clusterVector = UMBRA_HEAP_NEW_ARRAY(m_ctx->getAllocator(), UINT32, UMBRA_BITVECTOR_DWORDS(m_ctx->getTome()->getNumClusters()));
        memset(m_clusterVector, 0, UMBRA_BITVECTOR_SIZE(m_ctx->getTome()->getNumClusters()));
        m_clusters->clear();
    }

    // Note: the occlusion buffer pointers are remote when UMBRA_REMOTE_MEMORY is defined

    if (params->m_occlusionBuffer)
    {
        ImpOcclusionBuffer* occlusionBuffer = IMPL(params->m_occlusionBuffer);
#ifdef UMBRA_REMOTE_MEMORY
        m_remoteOcclusionBuffer = (UINTPTR)occlusionBuffer;
        m_occlusionBuffer = UMBRA_HEAP_NEW(m_ctx->getAllocator(), ImpOcclusionBuffer);
#else
        m_occlusionBuffer = occlusionBuffer;
#endif
        m_occlusionBuffer->init(transform, hasDepth ? (float*)(((UINT8*)occlusionBuffer) + UMBRA_OCCLUSIONBUFFER_DEPTH_OFFSET) : NULL);
    }

    if (params->m_inputBuffer)
    {
        const ImpOcclusionBuffer* inputBuffer = IMPL(params->m_inputBuffer);
#ifdef UMBRA_REMOTE_MEMORY
        ImpOcclusionBuffer local;
        MemoryAccess::alignedRead(&local, inputBuffer, sizeof(ImpOcclusionBuffer));
        inputBuffer = &local;
#endif
        if (inputBuffer->isValid())
            m_inputDepthBuffer = inputBuffer->getDepthBufferPtr(false);
    }
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

VisibilityResult::~VisibilityResult (void)
{
    if (m_remoteOcclusionBuffer)
    {
        MemoryAccess::alignedWrite((void*)m_remoteOcclusionBuffer, m_occlusionBuffer, sizeof(ImpOcclusionBuffer));
        UMBRA_HEAP_DELETE(m_ctx->getAllocator(), m_occlusionBuffer);
    }

    if (m_clusterVector)
        UMBRA_HEAP_DELETE_ARRAY(m_ctx->getAllocator(), m_clusterVector);
    if (m_processedObjectVector)
        UMBRA_HEAP_DELETE_ARRAY(m_ctx->getAllocator(), m_processedObjectVector);

    if (m_objects && m_objects->isMaxed())
        m_ctx->setError(Query::ERROR_OUT_OF_MEMORY);
    if (m_clusters && m_clusters->isMaxed())
        m_ctx->setError(Query::ERROR_OUT_OF_MEMORY);
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool ImpOcclusionBuffer::isAABBVisible(const Vector3& vmn, const Vector3& vmx, float* contribution) const
{
    SIMDRegister mn = SIMDLoadW1(vmn);
    SIMDRegister mx = SIMDLoadW1(vmx);

    // TODO: get rid of this
    mn = SIMDSub(mn, m_transformer.getPrediction());
    mx = SIMDAdd(mx, m_transformer.getPrediction());

    float dummy;
    if (!contribution)
        contribution = &dummy;
    *contribution = 0.f;

    if (m_transformer.frustumTestBounds(mn, mx))
    {
        if (!m_depthBuffer)
        {
            if (contribution)
            {
                Vector4i UMBRA_ATTRIBUTE_ALIGNED16(mnmx);
                m_transformer.transformBox(mnmx, mn, mx, true, *contribution);
            }
            return true;
        }

        /* \todo [antti 29.11.2011]: more accurate test? */
        float minD = m_transformer.getMinDeviceZ(mn, mx);
        if (minD == 0.f)
        {
            *contribution = 1.f;
            return true;
        }
        minD = min2(minD, 1.f);

        Vector4i UMBRA_ATTRIBUTE_ALIGNED16(mnmx);
        m_transformer.transformBox(mnmx, mn, mx, false, *contribution);

        bool ret = isPixelAARectVisible<false>(Vector2i(mnmx.i, mnmx.j), Vector2i(mnmx.k, mnmx.l), minD);
#ifdef DEV_COMPARE_TO_REF
        bool ref = isPixelAARectVisibleReference<false>(Vector2i(mnmx.i, mnmx.j), Vector2i(mnmx.k, mnmx.l), minD);
        UMBRA_ASSERT(ret == ref);
#endif
        return ret;
    }

    return false;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

bool ImpOcclusionBuffer::isAABBFullyVisible(const Vector3& vmn, const Vector3& vmx) const
{
    SIMDRegister mn = SIMDLoadW1(vmn);
    SIMDRegister mx = SIMDLoadW1(vmx);

    // TODO: get rid of this
    mn = SIMDSub(mn, m_transformer.getPrediction());
    mx = SIMDAdd(mx, m_transformer.getPrediction());

    if (m_transformer.frustumTestBoundsFully(mn, mx))
    {
        if (!m_depthBuffer)
            return true;

        /* \todo [antti 29.11.2011]: more accurate test? */
        float maxD = m_transformer.getMaxDeviceZ(mn, mx);
        maxD = max2(maxD, 0.f);
        maxD = min2(maxD, 1.f);

        Vector4i UMBRA_ATTRIBUTE_ALIGNED16(mnmx);
        m_transformer.transformBox(mnmx, mn, mx, false);

        bool ret = isPixelAARectVisible<true>(Vector2i(mnmx.i, mnmx.j), Vector2i(mnmx.k, mnmx.l), maxD);
#ifdef DEV_COMPARE_TO_REF
        bool ref = isPixelAARectVisibleReference<true>(Vector2i(mnmx.i, mnmx.j), Vector2i(mnmx.k, mnmx.l), maxD);
        UMBRA_ASSERT(ret == ref);
#endif
        return ret;
    }

    return false;
}

bool ImpOcclusionBuffer::isAARectVisible(const Vector2& clipMin, const Vector2& clipMax, float deviceZ) const
{
    if (!m_depthBuffer)
        return true;
    return isPixelAARectVisible<false>(
        m_transformer.transformClipXYToRaster(clipMin, false),
        m_transformer.transformClipXYToRaster(clipMax, true),
        convertUserFloat(deviceZ));
}

bool ImpOcclusionBuffer::isAARectFullyVisible(const Vector2& clipMin, const Vector2& clipMax, float deviceZ) const
{
    if (!m_depthBuffer)
        return true;
    return isPixelAARectVisible<true>(
        m_transformer.transformClipXYToRaster(clipMin, false),
        m_transformer.transformClipXYToRaster(clipMax, true),
        convertUserFloat(deviceZ));
}

template<bool FULLY_VISIBLE>
bool ImpOcclusionBuffer::isPixelAARectVisibleReference(const Vector2i& rasterMin, const Vector2i& rasterMax, float depth) const
{
    UMBRA_ASSERT(m_depthBuffer);

    if (rasterMax.i <= 0 || rasterMax.j <= 0 ||
        rasterMin.i >= UMBRA_PORTAL_RASTER_SIZE || rasterMin.j >= UMBRA_PORTAL_RASTER_SIZE)
        return false;

    // Clamp to raster surface.
    Vector2i mn = Vector2i(
        max2(0, rasterMin.i), 
        max2(0, rasterMin.j));
    Vector2i mx = Vector2i(
        min2(UMBRA_PORTAL_RASTER_SIZE, rasterMax.i), 
        min2(UMBRA_PORTAL_RASTER_SIZE, rasterMax.j));

    for (int y = mn.j; y < mx.j; y++)
    for (int x = mn.i; x < mx.i; x++)
    {
        bool visible = testDepth(depth, x, y);
        if (FULLY_VISIBLE != visible)
            return !FULLY_VISIBLE;
    }

    return FULLY_VISIBLE;
}

template<bool FULLY_VISIBLE>
bool ImpOcclusionBuffer::isPixelAARectVisible(const Vector2i& rasterMin, const Vector2i& rasterMax, float depth) const
{
    UMBRA_ASSERT(m_depthBuffer);

    // Clamp to raster surface.
    int x0 = max2(0, rasterMin.i);
    int x1 = min2(UMBRA_PORTAL_RASTER_SIZE, rasterMax.i);
    int y0 = max2(0, rasterMin.j);
    int y1 = min2(UMBRA_PORTAL_RASTER_SIZE, rasterMax.j);

    // Early exit empty rectangles.
    if (x0 >= x1 || y0 >= y1)
        return false;

    // Compute block coverage.
    int blockXMin = (x0 >> BlockSizeLog2);
    int blockXMax = (x1 + BlockSize-1) >> BlockSizeLog2;
    int blockYMin = (y0 >> BlockSizeLog2);
    int blockYMax = (y1 + BlockSize-1) >> BlockSizeLog2;

    static const UINT32 s_x0Masks[4] = { 0xFFFF, 0xEEEE, 0xCCCC, 0x8888 };
    static const UINT32 s_x1Masks[4] = { 0xFFFF, 0x1111, 0x3333, 0x7777 };
    static const UINT32 s_y0Masks[4] = { 0xFFFF, 0xFFF0, 0xFF00, 0xF000 };
    static const UINT32 s_y1Masks[4] = { 0xFFFF, 0x000F, 0x00FF, 0x0FFF };

    // Compute masks for X/Y extents.
    SIMDRegister32 xMask0 = SIMDLoad32(s_x0Masks[x0 & (BlockSize-1)]);
    SIMDRegister32 xMask1 = SIMDLoad32(s_x1Masks[x1 & (BlockSize-1)]);
    SIMDRegister32 yMask0 = SIMDLoad32(s_y0Masks[y0 & (BlockSize-1)]);
    SIMDRegister32 yMask1 = SIMDLoad32(s_y1Masks[y1 & (BlockSize-1)]);

    DepthBuffer buf(NULL);
    buf.setBuffer(getDepthBufferPtr(true));

    DepthBuffer::BlockIterator<1, true, false, false> iter =
        buf.iterateBlocksLocal<1, true, false>(Vector4i(blockXMin, blockYMin, blockXMax, blockYMax));
    int yBlocks = blockYMax - blockYMin - 1;
    int xBlocks = blockXMax - blockXMin - 1;

    if (!yBlocks)
        yMask0 = SIMDBitwiseAnd32(yMask0, yMask1);
    if (!xBlocks)
        xMask0 = SIMDBitwiseAnd32(xMask0, xMask1);

    SIMDRegister z4 = SIMDLoad(depth);
    SIMDRegister32 rowMask = yMask0;
    SIMDRegister32 blockMask = SIMDBitwiseAnd32(yMask0, xMask0);
    SIMDRegister ret = SIMDZero();

    for (; !iter.end(); iter.next())
    {
        SIMDRegister32 nextMask = rowMask;
        if (!iter.leftInRow())
        {
            // read ret value once per row
            if (SIMDBitwiseOrTestAny(ret, SIMDZero()))
                return !FULLY_VISIBLE;
            // last block in row, apply x1
            blockMask = SIMDBitwiseAnd32(blockMask, xMask1);
            // find rowmask for next row
            rowMask = (--yBlocks == 0) ? yMask1 : SIMDLoad32(0xFFFFFFFF);
            nextMask = SIMDBitwiseAnd32(rowMask, xMask0);
        }

        if (FULLY_VISIBLE)
            ret = SIMDBitwiseOr(ret, iter.blocks().bitmaskTestAll16(z4, blockMask));
        else
            ret = SIMDBitwiseOr(ret, iter.blocks().bitmaskTestAny16(z4, blockMask));
        blockMask = nextMask;
    }

    if (SIMDBitwiseOrTestAny(ret, SIMDZero()))
        return !FULLY_VISIBLE;
    return FULLY_VISIBLE;
}

class Linearizer
{
public:
    Linearizer(float near, float far, bool hasFar, bool isOrthographic)
        : m_hasFar(hasFar)
        , m_isOrthographic(isOrthographic)
    {
        m_near        = SIMDLoad(near);
        m_far         = SIMDLoad(far);
        m_farnear2    = SIMDMultiply(SIMDMultiply(SIMDLoad(2.f), m_near), m_far);
    }

    inline SIMDRegister linearize(SIMDRegister values)
    {
        if (m_isOrthographic || !m_hasFar)
            return values;

        SIMDRegister nearValue = SIMDMultiply(m_near, values);
        SIMDRegister farValue  = SIMDMultiply(m_far, values);
        return SIMDMultiply(m_farnear2, SIMDReciprocalAccurate(SIMDSub(SIMDAdd(m_far, nearValue), farValue)));
    }
private:

    SIMDRegister m_near;
    SIMDRegister m_far;
    SIMDRegister m_farnear2;
    bool         m_hasFar;
    bool         m_isOrthographic;
};

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer::ErrorCode ImpOcclusionBuffer::dumpDebugBuffer (void* out, const OcclusionBuffer::BufferDesc& desc) const
{
    int bpp = (int)UMBRA_FORMAT_BPP(desc.format);

    if (!out)
        return OcclusionBuffer::ERROR_INVALID_POINTER;

    if (desc.width != UMBRA_PORTAL_RASTER_SIZE ||
        desc.height != UMBRA_PORTAL_RASTER_SIZE)
        return OcclusionBuffer::ERROR_INVALID_DIMENSIONS;

    if (desc.stride < ((desc.width * bpp + 7) / 8))
        return OcclusionBuffer::ERROR_INVALID_STRIDE;

    switch (desc.format)
    {
    case OcclusionBuffer::FORMAT_HISTOGRAM_8BPP:
        return dump8bpp((Umbra::UINT8*)out, desc);
    case OcclusionBuffer::FORMAT_NDC_FLOAT:
        return dumpFloat((float*)out, desc);
    default:
        return OcclusionBuffer::ERROR_INVALID_FORMAT;
    };
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer::ErrorCode ImpOcclusionBuffer::dumpFloat (float* out, const OcclusionBuffer::BufferDesc& desc) const
{
    UMBRA_CT_ASSERT(BlockSize == 4);

    const float* srcBuffer = reinterpret_cast<const float*>(m_depthBuffer);
    if (!srcBuffer)
        return OcclusionBuffer::ERROR_INVALID_POINTER;

    // The buffer is internally stored in blocks of BlockSize x BlockSize (4x4) pixels,
    // so that each 16 consecutive floats in memory form a 4x4 pixel block. Instead of 
    // simply memcpying the buffer to out pointer, we need to unswizzle the pixels to make
    // a linear output buffer.
    //
    // Storage index for each depth value:
    // 
    // 0  1  2  3  | 16 17 18 19 | 32 33 ...
    // 4  5  6  7  | 20 21 22 23 | 
    // 8  9  10 11 | 24 25 26 27 | 
    // 12 13 14 15 | 28 29 30 31 | 

    SIMDRegister scale = SIMDOne();
    SIMDRegister bias  = SIMDZero();
    if (m_transformer.getDepthRange() == CameraTransform::DEPTHRANGE_MINUS_ONE_TO_ONE)
    {
        // Internally umbra stores [0, 1] depth values.
        // Scale output values to [-1, 1] to match user's setup if required.
        scale = SIMDLoad(2.f);
        bias  = SIMDLoad(-1.f);
    }

    Umbra::UINT8* outPtr = (Umbra::UINT8*)out;
    for (int y = 0; y < UMBRA_PORTAL_RASTER_SIZE; y += BlockSize)
    {
        // Pointer to start of block of rows
        const float* srcBlockRow = srcBuffer;

        // Iterate current block row's 4 pixel rows
        for (int blockY = 0; blockY < BlockSize; blockY++)
        {
            // Pointer next input pixels within row of blocks
            const float* srcRow = srcBlockRow;
            // Points to output row start
            float* outRow = (float*)outPtr;

            if (is128Aligned(outRow))
            {
                for (int x = 0; x < UMBRA_PORTAL_RASTER_SIZE / 4; x += BlockSize)
                {
#define UMBRA_BUFFER_READ(var) \
                    SIMDRegister r##var = SIMDLoadAligned((float*)srcRow); \
                    r##var = SIMDMultiplyAdd(r##var, scale, bias); \
                    srcRow += PixelsPerBlock; // Advance input one block, getting the next pixels on current row
#define UMBRA_BUFFER_WRITE(var) \
                    SIMDStoreAligned(r##var, outRow); \
                    outRow += BlockSize;      // Advance output linearly

                    UMBRA_BUFFER_READ(p1);
                    UMBRA_BUFFER_READ(p2);
                    UMBRA_BUFFER_READ(p3);
                    UMBRA_BUFFER_READ(p4);

                    UMBRA_BUFFER_WRITE(p1);
                    UMBRA_BUFFER_WRITE(p2);
                    UMBRA_BUFFER_WRITE(p3);
                    UMBRA_BUFFER_WRITE(p4);

#undef  UMBRA_BUFFER_READ
#undef  UMBRA_BUFFER_WRITE
                 }
            } else
            {
                for (int x = 0; x < UMBRA_PORTAL_RASTER_SIZE / 4; x += BlockSize)
                {   
#define UMBRA_BUFFER_READ(var) \
                    SIMDRegister r##var = SIMDLoadAligned((float*)srcRow); \
                    r##var = SIMDMultiplyAdd(r##var, scale, bias); \
                    srcRow += PixelsPerBlock; // Advance input one block, getting the next pixels on current row
#define UMBRA_BUFFER_WRITE(var) \
                    SIMDStore(r##var, outRow); \
                    outRow += BlockSize;      // Advance output linearly

                    UMBRA_BUFFER_READ(p1);
                    UMBRA_BUFFER_READ(p2);
                    UMBRA_BUFFER_READ(p3);
                    UMBRA_BUFFER_READ(p4);

                    UMBRA_BUFFER_WRITE(p1);
                    UMBRA_BUFFER_WRITE(p2);
                    UMBRA_BUFFER_WRITE(p3);
                    UMBRA_BUFFER_WRITE(p4);

#undef  UMBRA_BUFFER_READ
#undef  UMBRA_BUFFER_WRITE
                }
            }

            // Next row inside this block starts after four pixels
            srcBlockRow += BlockSize;
            // Advance output pointer linearly line-by-line
            outPtr += desc.stride;
        }

        // Advance one row of blocks, typically 128x4 pixels or 32x1 blocks.
        srcBuffer += BlockBufferStride;
    }

    return OcclusionBuffer::ERROR_OK;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer::ErrorCode ImpOcclusionBuffer::dump8bpp (Umbra::UINT8* out, const OcclusionBuffer::BufferDesc& desc) const
{
    const float* srcBuffer = reinterpret_cast<const float*>(m_depthBuffer);
    if (!srcBuffer)
        return OcclusionBuffer::ERROR_INVALID_POINTER;

    const int histogramSize = 512;
    int histogram[histogramSize];
    memset(histogram, 0, sizeof(int) * histogramSize);

    bool hasFar = m_transformer.hasFarPlane();
    bool isOrthographic = m_transformer.isOrtho();

    const Vector4& nearPleq = m_transformer.getNearPlane();
    float near = -dot(nearPleq, m_transformer.getCameraPos()) / nearPleq.xyz().length();

    float far = 0.f;
    if (hasFar)
    {
        const Vector4& farPleq = m_transformer.getFrustumPlanes()[1];
        far = dot(farPleq, m_transformer.getCameraPos()) / farPleq.xyz().length();
        if (far < near)
            swap2(near, far);
    }

    SIMDRegister minInputSIMD = SIMDOne();
    SIMDRegister maxInputSIMD = SIMDZero();

    for (int i = 0; i < UMBRA_PORTAL_RASTER_SIZE * UMBRA_PORTAL_RASTER_SIZE; i += 4)
    {
        UMBRA_ASSERT(srcBuffer[i]   >= 0.f && srcBuffer[i]   <= 1.f);
        UMBRA_ASSERT(srcBuffer[i+1] >= 0.f && srcBuffer[i+1] <= 1.f);
        UMBRA_ASSERT(srcBuffer[i+2] >= 0.f && srcBuffer[i+2] <= 1.f);
        UMBRA_ASSERT(srcBuffer[i+3] >= 0.f && srcBuffer[i+3] <= 1.f);

        SIMDRegister z = SIMDLoadAligned((float*)srcBuffer+i);
        minInputSIMD = SIMDMin(minInputSIMD, z);
        maxInputSIMD = SIMDMax(maxInputSIMD, SIMDSelect(z, SIMDZero(),  SIMDCompareEQ(z, SIMDOne())));
    }

    Vector4 UMBRA_ATTRIBUTE_ALIGNED16(temp);
    SIMDStoreAligned(minInputSIMD, (float*)&temp);

    if (temp == Vector4(1.f))
    {
        memset(out, 0xff, UMBRA_PORTAL_RASTER_SIZE * UMBRA_PORTAL_RASTER_SIZE);
        return OcclusionBuffer::ERROR_OK;
    }

    Linearizer linearizer(near, far, hasFar, isOrthographic);

    minInputSIMD = linearizer.linearize(minInputSIMD);
    maxInputSIMD = linearizer.linearize(maxInputSIMD);

    SIMDStoreAligned(minInputSIMD, (float*)&temp);
    float minInput = min2(temp.x, min2(temp.y, min2(temp.z, temp.w)));

    SIMDStoreAligned(maxInputSIMD, (float*)&temp);
    float maxInput = max2(temp.x, max2(temp.y, max2(temp.z, temp.w)));

    minInputSIMD = SIMDLoad(minInput);
    maxInputSIMD = SIMDLoad(maxInput);

    SIMDRegister scale = SIMDZero();
    SIMDRegister bias  = SIMDZero();

    if (minInput == maxInput)
    {
        scale = SIMDOne();
        bias  = SIMDLoad(-minInput);
    } else
    if (minInput < maxInput)
    {
        scale = SIMDLoad(1.0f / (maxInput - minInput));
        scale = SIMDMultiply(scale, SIMDLoad((float)(histogramSize-1)));
        bias  = SIMDMultiply(SIMDLoad(-minInput), scale);
    }

    Vector4i UMBRA_ATTRIBUTE_ALIGNED16(values);
    int minValue = INT_MAX;
    int samples = 0;

    for (int i = 0; i < UMBRA_PORTAL_RASTER_SIZE * UMBRA_PORTAL_RASTER_SIZE; i += 4)
    {
        SIMDRegister z = SIMDLoadAligned((float*)srcBuffer+i);
        z = linearizer.linearize(z);
        z = SIMDMultiplyAdd(z, scale, bias);
        SIMDStoreAligned32(SIMDFloatToInt(z), (int*)&values);

        for (int j = 0; j < 4; j++)
        {
            if (values[j] < 0 || values[j] >= histogramSize)
                continue;
            UMBRA_ASSERT(values[j] >= 0 && values[j] < histogramSize);
            histogram[values[j]]++;
            minValue = min2(minValue, values[j]);
            samples++;
        }
    }

    UINT8 colors[histogramSize];
    if (samples)
    {
        int sum = 0;
        float denominator = (float)(samples - histogram[minValue]);
        if (denominator == 0)
            denominator = 1.f;

        for (int i = 0; i < histogramSize; i++)
        {
            if (!histogram[i])
            {
                colors[i] = 0;
                continue;
            }
            sum += histogram[i];
            float c = (float)(sum - histogram[minValue]) / denominator;
            UMBRA_ASSERT(c >= 0.f && c <= 1.f);
            colors[i] = (UINT8)(c * 254.f);
            UMBRA_ASSERT(colors[i] <= 254);
        }
    }

    for (int blockY = 0; blockY < UMBRA_PORTAL_RASTER_SIZE >> BlockSizeLog2; blockY++)
    {
        const float* srcPtr = srcBuffer;
        for (int blockX = 0; blockX < UMBRA_PORTAL_RASTER_SIZE >> BlockSizeLog2; blockX++)
        {
            for (int y = 0; y < BlockSize; y++)
            {
                UINT8* dstPtr = &out[y * desc.stride + blockX * BlockSize];

                SIMDRegister z = SIMDLoadAligned((float*)srcPtr);
                z = SIMDMax(SIMDZero(), z);
                z = linearizer.linearize(z);
                z = SIMDMultiplyAdd(z, scale, bias);
                SIMDStoreAligned32(SIMDFloatToInt(z), (int*)&values);

                for (int j = 0; j < 4; j++)
                {
                    if (values[j] < 0 || values[j] >= histogramSize)
                        (*dstPtr++) = 255;
                    else
                    {
                        UMBRA_ASSERT(values[j] >= 0 && values[j] < histogramSize);
                        (*dstPtr++) = colors[values[j]];
                    }
                }

                srcPtr += 4;
            }
        }

        out       += BlockSize * desc.stride;
        srcBuffer += BlockBufferStride;
    }

    return OcclusionBuffer::ERROR_OK;
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpOcclusionBuffer::visualizeHull(QueryContext* ctx) const
{
    if (!m_depthBuffer)
        return;

    Matrix4x4 clipToWorld = m_transformer.getWorldToClip();
    clipToWorld.invert();

    // max horizontal span of equal pixels
    const int maxSpan = 16;

    // render depthbuffer visualization
    for (int y = 0; y < UMBRA_PORTAL_RASTER_SIZE-1; y++)
    for (int x = 0; x < UMBRA_PORTAL_RASTER_SIZE-1; x++)
    {
        int x2 = x;
        int y2 = y + 1;
        float depth = readDepth(x, y);

        // figure out a horizontal span of equal pixels to reduce amount of quads needed
        int span = 0;
        if (readDepth(x, y2) == depth)
        {
            int end = min2(x + maxSpan, UMBRA_PORTAL_RASTER_SIZE - 1);
            while (x2 + 1 < end &&
                readDepth(x2 + 1, y) == depth &&
                readDepth(x2 + 1, y2) == depth)
            {
                x2++;
                span++;
            }
        }

        // no span found, use 1x1
        if (span == 0)
            x2 = x + 1;

        Vector3 nearVert[4];
        Vector3 farVert[4];

        for (int i = 0; i < 4; i++)
        {
            int x_ = ((i >> 1) == (i & 1)) ? x : x2;
            int y_ = (i & 2) ? y2 : y;
            float depth = readDepth(x_, y_);
            nearVert[i] = clipToWorld.transformDivByW(getClipSpaceCoord(x_, y_, 0.f));
            farVert[i] = clipToWorld.transformDivByW(getClipSpaceCoord(x_, y_, depth));
        }

        // try to color differently facing sides a bit differently (todo)
        Vector3 a = (farVert[2] - farVert[0]).normalize();
        float d = dot(a, Vector3(1,1,1).normalize());

        ctx->addQueryDebugQuad(
            farVert[0], farVert[1], farVert[2], farVert[3],
            Vector4(0.f, 0.5f + 0.5f * d, 0.f, 0.5f));

        // handle frustum sides as special cases

        if (x == 0)
            ctx->addQueryDebugQuad(
                    nearVert[0], nearVert[3], farVert[3], farVert[0],
                    Vector4(0.f, 0.5f, 0.f, 0.5f));
        if (x2 == UMBRA_PORTAL_RASTER_SIZE-1)
            ctx->addQueryDebugQuad(
                    nearVert[1], nearVert[2], farVert[2], farVert[1],
                    Vector4(0.f, 0.5f, 0.f, 0.5f));
        if (y == 0)
            ctx->addQueryDebugQuad(
                    nearVert[0], nearVert[1], farVert[1], farVert[0],
                    Vector4(0.f, 1.0f, 0.f, 0.5f));
        if (y2 == UMBRA_PORTAL_RASTER_SIZE-1)
            ctx->addQueryDebugQuad(
                    nearVert[3], nearVert[2], farVert[2], farVert[3],
                    Vector4(0.f, 1.0f, 0.f, 0.5f));

        x = x2 - 1;
    }
}

/*----------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*----------------------------------------------------------------------*/

void ImpOcclusionBuffer::combine (const ImpOcclusionBuffer& other)
{
    /* \todo [antti 28.5.2013]: verify that transforms match */
    Vector4i rect = other.m_transformer.getScissor();
    
    // Scissor rect is always block-aligned
    UMBRA_ASSERT((rect.i & (BlockSize - 1)) == 0);
    UMBRA_ASSERT((rect.j & (BlockSize - 1)) == 0);
    UMBRA_ASSERT((rect.k & (BlockSize - 1)) == 0);
    UMBRA_ASSERT((rect.l & (BlockSize - 1)) == 0);

    Vector4i scissor = m_transformer.getScissor();
    scissor.i = min2(scissor.i, rect.i);
    scissor.j = min2(scissor.j, rect.j);
    scissor.k = max2(scissor.k, rect.k);
    scissor.l = max2(scissor.l, rect.l);
    m_transformer.setScissor(scissor);

    if (m_depthBuffer && other.m_depthBuffer)
    {
        // todo: doesn't currently work for remote memory
        DepthBuffer self(NULL), input(NULL);
        self.setBuffer(getDepthBufferPtr(true));
        input.setBuffer(other.getDepthBufferPtr(true));

        Vector4i blockRect(rect.i >> BlockSizeLog2, rect.j >> BlockSizeLog2,
            rect.k >> BlockSizeLog2, rect.l >> BlockSizeLog2);
        DepthBuffer::BlockIterator<2, true, true> iter1 = self.iterateBlocks<2, true, true>(blockRect);
        DepthBuffer::BlockIterator<2, true, false> iter2 = input.iterateBlocks<2, true, false>(blockRect);

        while (!iter1.end())
        {
            iter1.blocks().combineMax(iter2.blocks());
            iter1.next();
            iter2.next();
        }
    }
}
