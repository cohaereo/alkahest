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
 * \brief   Umbra runtime portal culling
 *
 */

#include "umbraSIMD.hpp"
#include "umbraPortalRaster.hpp"
#include "umbraDepthBuffer.hpp"
#include "umbraBitOps.hpp"
#include "umbraQueryArgs.hpp"
#include "umbraQueryContext.hpp"
#include <string.h>

#define TRANSIENT_ALLOC_FASTPATH 1

using namespace Umbra;

namespace Umbra
{

static const SIMDRegister s_RasterBlockSize = SIMDLoad(4.f); // BlockRasterBuffer::RasterBlockSize;
static const SIMDRegister s_PortalRasterScale = SIMDLoad(2.0f / (float)UMBRA_PORTAL_RASTER_SIZE);
static const SIMDRegister s_PortalRasterOffset = SIMDLoad(-0.5f*(float)UMBRA_PORTAL_RASTER_SIZE + 0.5f);
static const SIMDRegister s_tileSize = SIMDLoad((float)BlockRasterBuffer::TileSize);
static const SIMDRegister s_MinusHalf = SIMDLoad(-0.5f);
static const SIMDRegister s_Epsilon = SIMDLoad(FLT_EPSILON);

} // namespace Umbra

// \todo [petri] DEBUG DEBUG These stats are only temporary.
/*static int numPortals             = 0;
static int numBigPortals            = 0;
static int numSmallPortals          = 0;
static int numFrontClippingPortals  = 0;
static int numPortalBoundsPixels    = 0;
static int numPortalPixels          = 0;
static int numBlocksProcessed       = 0;
static int numBlocksRasterized      = 0;
static int numEmptyBlocks           = 0;
static int numFullBlocks            = 0;
static int numTilesRasterized       = 0;
static int numTileActivePlanes      = 0;
static int numEmptyTiles            = 0;
static int numFullTiles             = 0;
static int numBuffersCompressed         = 0;
static int numMiniBuffersCompressed     = 0;
static int numLargeBuffersCompressed    = 0;
static int numBlocksCompressed          = 0;
static int numLargeTilesCompressed      = 0;*/

inline int BufferAllocator::getAllocSize (const Vector4i& rect)
{
    int numBlocks = rectangleArea(rect);
    return (numBlocks*sizeof(UINT32) + (BufferAllocator::BlockSize - 1)) >> BufferAllocator::BlockSizeLog2;
}

BufferAllocator::BufferAllocator (void)
{
    memset(m_blockAllocatedMask, 0x00, sizeof(m_blockAllocatedMask));
    m_blockAllocatedMask[NumBitfields - 1] = 0xFFFFFFFF;
    m_nonTransientOffset = 0;
    m_transientOffset = NumTotalBlocks;
}

inline bool BufferAllocator::allocateBuffer (BlockRasterBuffer& buffer, const Vector4i& bounds, bool isTransient)
{
    // \note buffer is left uninitialized!
    UMBRA_ASSERT(BlockRasterBuffer::checkBlockRect(bounds));
    int numAllocBlocks = getAllocSize(bounds);
    UMBRA_ASSERT(numAllocBlocks);

    buffer.m_blockRect.set(bounds);
    buffer.m_blocks = (UINT32*)allocate(numAllocBlocks, isTransient);
    if (buffer.m_blocks == NULL)
    {
        buffer.m_blocks = (UINT32*)m_persistent;
        return false;
    }

    return true;
}

bool BufferAllocator::expandBuffer (BlockRasterBuffer& buffer, const Vector4i& bounds, bool isTransient)
{
    // If old buffer is empty, just allocate a new one.
    if (!buffer.getBufferPtr())
    {
        bool isOk = allocateBuffer(buffer, bounds, isTransient);
        if (isOk)
            RasterOps::clear(buffer);
        return isOk;
    }

    UMBRA_ASSERT(BlockRasterBuffer::checkBlockRect(bounds));

    // Compute expanded buffer area (old buffer must be non-empty!).
    Vector4i oldRect = buffer.getBlockRect();
    Vector4i newRect = rectangleUnion(oldRect, bounds);
    UMBRA_ASSERT(rectangleArea(oldRect) > 0);

    // Early-exit if new area within old buffer area.
    if (newRect == oldRect)
        return true;

    // Save off current buffer and reinit
    BlockRasterBuffer oldBuffer(buffer);
    buffer.m_blockRect.set(newRect);

    // This was an expansion of an OOM buffer
    if (buffer.m_blocks == (UINT32*)m_persistent)
        return false;

    // See if it fits in the current allocation
    int oldAllocSize = getAllocSize(oldRect);
    int newAllocSize = getAllocSize(newRect);

    if (newAllocSize > oldAllocSize)
    {
        buffer.m_blocks = (UINT32*)allocate(newAllocSize, isTransient);
        if (!buffer.m_blocks)
        {
            buffer.m_blocks = (UINT32*)m_persistent;
            release(oldBuffer.getBufferPtr(), oldAllocSize);
            return false;
        }
    }

    RasterOps::expandBlit(buffer, oldBuffer);
    if (buffer.getBufferPtr() != oldBuffer.getBufferPtr())
        release(oldBuffer.getBufferPtr(), oldAllocSize);
    return true;
}

void BufferAllocator::releaseBuffer (BlockRasterBuffer& buffer)
{
    void* bufferPtr = buffer.getBufferPtr();
    if ((bufferPtr != NULL) && (bufferPtr != m_persistent))
    {
        release(bufferPtr, getAllocSize(buffer.getBlockRect()));
    }
}

int BufferAllocator::findFreeRun (int numBlocks)
{
    int offset = 0;
    int len = 0;
    int cap = m_transientOffset >> 5;

    for (int ndx = 0; ndx < cap; ndx++)
    {
        UINT32 mask = m_blockAllocatedMask[ndx];

        if (len >= numBlocks)
            return offset;

        if (!mask)
        {
            // an empty 32-bit run
            // checking for enough space done in next iteration
            len += 32;
            continue;
        }

        if (mask == 0xFFFFFFFF)
        {
            // a completely full 32-bit run
            len = 0;
            offset = (ndx + 1) << 5;
            continue;
        }

        while (mask)
        {
            int zeros = countTrailingZeros(mask);
            len += zeros;
            if (len >= numBlocks)
                return offset;
            offset += len;
            len = 0;
            mask >>= zeros;
            int ones = countTrailingZeros(~mask);
            mask >>= ones;
            offset += ones;
        }

        len = (32 - (offset & 31)) & 31;
    }

    return InvalidOffset;
}

inline void* BufferAllocator::allocate (int numBlocks, bool isTransient)
{
    UMBRA_UNREF(isTransient);

    // number of known-to-be-free blocks
    int numFreeBlocks = (m_transientOffset - m_nonTransientOffset);
    int allocOffset;

    // try to make more space if needed
    while ((numFreeBlocks < numBlocks) && (m_nonTransientOffset > 0) &&
        !testBit(m_blockAllocatedMask, m_nonTransientOffset - 1))
    {
        --m_nonTransientOffset;
        numFreeBlocks++;
    }

    if (TRANSIENT_ALLOC_FASTPATH && isTransient && (numFreeBlocks >= numBlocks))
    {
        m_transientOffset -= numBlocks;
        allocOffset = m_transientOffset;
    }
    else
    {
        if (numFreeBlocks >= numBlocks)
        {
            allocOffset = m_nonTransientOffset;
            m_nonTransientOffset += numBlocks;
        }
        else
        {
            allocOffset = findFreeRun(numBlocks);
            if (allocOffset == InvalidOffset)
                return NULL;
            UMBRA_ASSERT(allocOffset + numBlocks < m_transientOffset);
            m_nonTransientOffset = max2(m_nonTransientOffset, allocOffset + numBlocks);
        }
        UMBRA_ASSERT(!testBitRange(m_blockAllocatedMask, allocOffset, allocOffset + numBlocks));
        setBitRange(m_blockAllocatedMask, allocOffset, allocOffset + numBlocks);
    }

    UMBRA_ASSERT(allocOffset >= 0 && allocOffset + numBlocks <= NumTotalBlocks);
    return &m_blocks[allocOffset << BlockSizeLog2];
}

inline void BufferAllocator::release (void* ptr, int numBlocks)
{
    UMBRA_ASSERT(ptr != m_persistent);
    int offset = (int)((UINT8*)ptr - &m_blocks[0]) >> BlockSizeLog2;
    UMBRA_ASSERT(offset >= 0 && offset + numBlocks <= NumTotalBlocks);

    if (offset < m_nonTransientOffset)
    {
        clearBitRange(m_blockAllocatedMask, offset, offset + numBlocks);
        // Opportunistically wind non-transient offset back, if this
        // happened to be the last non-transient allocation. Note that
        // the offset may not represent reality, the preceding allocated
        // bits are tested lazily in allocate() when needed.
        if (offset + numBlocks == m_nonTransientOffset)
            m_nonTransientOffset = offset;
    }
}

#if 0 // debug methods for dumping the buffer contents
void BlockRasterBuffer::dump (void)
{
    int minX = m_blockRect.minX << RectBlockWidthLog2;
    int maxX = m_blockRect.maxX << RectBlockWidthLog2;
    int minY = m_blockRect.minY << RectBlockHeightLog2;
    int maxY = m_blockRect.maxY << RectBlockHeightLog2;

    for (int y = 0; y < UMBRA_PORTAL_RASTER_SIZE; y++)
    {
        char buf[256];
        for (int x = 0; x < UMBRA_PORTAL_RASTER_SIZE; x++)
        {
            bool isInside   = (y >= minY && y < maxY && x >= minX && x < maxX);
            bool isEnabled  = isInside && testPixel(x, y);
            buf[x] = isEnabled ? 'X' : isInside ? '.' : ' ';
        }
        buf[UMBRA_PORTAL_RASTER_SIZE] = 0;

#if UMBRA_ARCH == UMBRA_SPU
        spu_printf("    \"%s\" [%d]\n", buf, y);
#elif UMBRA_OS == UMBRA_WINDOWS
        printf("    \"%s\" [%d]\n", buf, y);
#endif
    }
}

void BlockRasterBuffer::dump (const BlockRasterBuffer& src)
{
    int minX = m_blockRect.minX << RectBlockWidthLog2;
    int maxX = m_blockRect.maxX << RectBlockWidthLog2;
    int minY = m_blockRect.minY << RectBlockHeightLog2;
    int maxY = m_blockRect.maxY << RectBlockHeightLog2;

    int srcMinX = src.m_blockRect.minX << RectBlockWidthLog2;
    int srcMaxX = src.m_blockRect.maxX << RectBlockWidthLog2;
    int srcMinY = src.m_blockRect.minY << RectBlockHeightLog2;
    int srcMaxY = src.m_blockRect.maxY << RectBlockHeightLog2;

    for (int y = 0; y < UMBRA_PORTAL_RASTER_SIZE; y++)
    {
        char buf[256];
        for (int x = 0; x < UMBRA_PORTAL_RASTER_SIZE; x++)
        {
            bool isInside   = (y >= minY && y < maxY && x >= minX && x < maxX);
            bool isEnabled  = isInside && testPixel(x, y);
            bool isSrcOn    = (y >= srcMinY && y < srcMaxY && x >= srcMinX && x < srcMaxX) && src.testPixel(x, y);

            char c;
            if      (isEnabled && isSrcOn)  c = 'X';
            else if (isEnabled && !isSrcOn) c = '-';
            else if (!isEnabled && isSrcOn) c = '.';
            else                            c = ' ';
            buf[x] = c;
        }
        buf[UMBRA_PORTAL_RASTER_SIZE] = 0;

#if UMBRA_ARCH == UMBRA_SPU
        spu_printf("    \"%s\" [%d]\n", buf, y);
#elif UMBRA_OS == UMBRA_WINDOWS
        printf("    \"%s\" [%d]\n", buf, y);
#endif
    }
}
#endif

#ifdef UMBRA_SIMD_CODE
/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_FORCE_INLINE static void simdFill (BlockRasterBuffer& buf, SIMDRegister val)
{
    float* ptr = (float*)buf.getBufferPtr();
    int blocks = buf.getNumRectBlocks();
    int count = (blocks + 3) >> 2;
    while (count--)
    {
        SIMDStoreAligned(val, ptr);
        ptr += 4;
    }
}
#endif

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void RasterOps::clear(BlockRasterBuffer& buf)
{
#if defined(UMBRA_SIMD_CODE)
    simdFill(buf, SIMDZero());
#else
    fill(buf, 0);
#endif
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void RasterOps::fillOnes(BlockRasterBuffer& buf)
{
#if defined(UMBRA_SIMD_CODE)
    simdFill(buf, SIMDMaskXYZW());
#else
    fill(buf, 0xFF);
#endif
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void RasterOps::fill(BlockRasterBuffer& buf, int c)
{
    memset(buf.getBufferPtr(), c, buf.getNumRectBlocks()*sizeof(UINT32));
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void RasterOps::blit(BlockRasterBuffer& dst, const BlockRasterBuffer& src)
{
    UMBRA_ASSERT(rectangleContains(dst.getBlockRect(), src.getBlockRect()));
    UMBRA_ASSERT(src.getNumRectBlocks() > 0);

    int blockXMin = src.getBlockRect().i;
    int blockXMax = src.getBlockRect().k;
    int blockYMin = src.getBlockRect().j;
    int blockYMax = src.getBlockRect().l;
    const UINT32* srcPtr = src.getBufferPtr();

    for (int blockY = blockYMin; blockY < blockYMax; blockY++)
    {
        UINT32* dstPtr = dst.getRectBlockPtr(blockXMin, blockY);
        int count = (blockXMax - blockXMin);

        do
        {
            *dstPtr++ = *srcPtr++;
        } while (--count);
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool RasterOps::blitOr(BlockRasterBuffer& dst, const BlockRasterBuffer& src)
{
    UMBRA_ASSERT(rectangleContains(dst.getBlockRect(), src.getBlockRect()));
    UMBRA_ASSERT(src.getNumRectBlocks() > 0);

    int blockXMin = src.getBlockRect().i;
    int blockXMax = src.getBlockRect().k;
    int blockYMin = src.getBlockRect().j;
    int blockYMax = src.getBlockRect().l;
    const UINT32* srcPtr = src.getBufferPtr();
    UINT32 changed = 0;

    for (int blockY = blockYMin; blockY < blockYMax; blockY++)
    {
        UINT32* dstPtr = dst.getRectBlockPtr(blockXMin, blockY);
        int count = (blockXMax - blockXMin);

        do
        {
            UINT32 dst = *dstPtr;
            UINT32 val = dst | *srcPtr++;
            changed |= dst ^ val;
            *dstPtr++ = val;
        } while (--count);
    }
    return changed != 0;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Fills dst with src where they overlap, 0 elsewhere.
 *          Supports in-place operation where src and dst point to same
 *          memory.
 *//*-------------------------------------------------------------------*/

void RasterOps::expandBlit(BlockRasterBuffer& dst, const BlockRasterBuffer& src)
{
    UMBRA_ASSERT(rectangleContains(dst.getBlockRect(), src.getBlockRect()));
    UMBRA_ASSERT(src.getNumRectBlocks() > 0);
    Vector4i dstRect = dst.getBlockRect();
    Vector4i srcRect = src.getBlockRect();
    int dstWidth = dstRect.k - dstRect.i;
    int dstHeight = dstRect.l - dstRect.j;
    int srcWidth = srcRect.k - srcRect.i;
    int srcHeight = srcRect.l - srcRect.j;
    int srcOffsetY = srcRect.j - dstRect.j;

    // fill tail
    UINT32* tail = dst.getBufferPtr() + dstWidth * (srcOffsetY + srcHeight);
    UINT32* ptr = dst.getBufferPtr() + (dstWidth * dstHeight) - 1;
    while (ptr >= tail)
        *ptr-- = 0;

    // selected optimized paths
    if (srcWidth == dstWidth)
    {
        if (srcOffsetY == 0 && (dst.getBufferPtr() == src.getBufferPtr()))
            return;
        int numBlocks = srcWidth * srcHeight;
        const UINT32* srcPtr = src.getBufferPtr() + numBlocks - 1;
        while (numBlocks--)
            *ptr-- = *srcPtr--;
    }
    else
    {
        // process overlapping scanlines, must be done in reverse order!
        int fillBlocksEnd = dstRect.k - srcRect.k;
        int fillBlocksStart = srcRect.i - dstRect.i;
        int copyBlocks = srcWidth;
        int numRows = srcHeight;
        UMBRA_ASSERT(fillBlocksStart + copyBlocks + fillBlocksEnd == dstWidth);
        const UINT32* srcPtr = src.getBufferPtr() + (srcHeight * srcWidth) - 1;
        while (numRows--)
        {
            for (int i = 0; i < fillBlocksEnd; i++)
                *ptr-- = 0;
            for (int i = 0; i < copyBlocks; i++)
                *ptr-- = *srcPtr--;
            for (int i = 0; i < fillBlocksStart; i++)
                *ptr-- = 0;
        }
    }

    // fill head
    UINT32* head = dst.getBufferPtr();
    while (ptr >= head)
        *ptr-- = 0;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool RasterOps::testRectAnyReference (const BlockRasterBuffer& buf, const Vector4i& mnmx, DepthBuffer* input, float z)
{
    Vector4i bounds = rectangleIntersection(buf.getBounds(), mnmx);
    int x0 = bounds.i;
    int x1 = bounds.k;
    int y0 = bounds.j;
    int y1 = bounds.l;

    // Early exit empty rectangles.
    if (x0 >= x1 || y0 >= y1)
        return false;
    
    for (int y = y0; y < y1; y++)
        for (int x = x0; x < x1; x++)
        {
            if (buf.testPixel(x, y))
            {
                if (!input)
                    return true;
                if (((float*)input->getBuffer())[ImpOcclusionBuffer::getPixelOffset(x, y)] >= z)
                    return true;
            }
        }
    return false;
}

/*-------------------------------------------------------------------*//*!
 * \brief Return true if any pixels are set inside the block.
 *//*-------------------------------------------------------------------*/

bool RasterOps::testRectAny(const BlockRasterBuffer& buf, const Vector4i& mnmx)
{
    UMBRA_ASSERT(rectangleArea(mnmx) > 0);
    UMBRA_ASSERT(rectangleContains(buf.getBounds(), mnmx));

    int x0 = mnmx.i;
    int x1 = mnmx.k;
    int y0 = mnmx.j;
    int y1 = mnmx.l;

    // Compute block coverage.
    int blockXMin = (x0 >> BlockRasterBuffer::RectBlockWidthLog2);
    int blockXMax = (x1 + BlockRasterBuffer::RectBlockWidth-1) >> BlockRasterBuffer::RectBlockWidthLog2;
    int blockYMin = (y0 >> BlockRasterBuffer::RectBlockHeightLog2);
    int blockYMax = (y1 + BlockRasterBuffer::RectBlockHeight-1) >> BlockRasterBuffer::RectBlockHeightLog2;

#if UMBRA_BYTE_ORDER == UMBRA_BIG_ENDIAN
    static const UINT32 s_x0Masks[8] = { 0xFFFFFFFF, 0xEEEEFFFF, 0xCCCCFFFF, 0x8888FFFF,
                                         0x0000FFFF, 0x0000EEEE, 0x0000CCCC, 0x00008888 };
    static const UINT32 s_x1Masks[8] = { 0xFFFFFFFF, 0x11110000, 0x33330000, 0x77770000,
                                         0xFFFF0000, 0xFFFF1111, 0xFFFF3333, 0xFFFF7777 };
#else
    static const UINT32 s_x0Masks[8] = { 0xFFFFFFFF, 0xFFFFEEEE, 0xFFFFCCCC, 0xFFFF8888,
                                         0xFFFF0000, 0xEEEE0000, 0xCCCC0000, 0x88880000 };
    static const UINT32 s_x1Masks[8] = { 0xFFFFFFFF, 0x00001111, 0x00003333, 0x00007777,
                                         0x0000FFFF, 0x1111FFFF, 0x3333FFFF, 0x7777FFFF };
#endif

    static const UINT32 s_y0Masks[4] = { 0xFFFFFFFF, 0xFFF0FFF0, 0xFF00FF00, 0xF000F000 };
    static const UINT32 s_y1Masks[4] = { 0xFFFFFFFF, 0x000F000F, 0x00FF00FF, 0x0FFF0FFF };

    // Compute masks for X/Y extents.
    UINT32 xMask0 = s_x0Masks[x0 & (BlockRasterBuffer::RectBlockWidth-1)];
    UINT32 xMask1 = s_x1Masks[x1 & (BlockRasterBuffer::RectBlockWidth-1)];
    UINT32 yMask0 = s_y0Masks[y0 & (BlockRasterBuffer::RectBlockHeight-1)];
    UINT32 yMask1 = s_y1Masks[y1 & (BlockRasterBuffer::RectBlockHeight-1)];

    const UINT32* ptr = buf.getRectBlockPtr(blockXMin, blockYMin);
    int rowLen = blockXMax - blockXMin;
    int numRows = blockYMax - blockYMin;
    int skip = buf.getRectBlockStride() - rowLen;
    UINT32 rowMask = yMask0;
    UINT32 diff = 0x0;

    while (--numRows && (diff == 0))
    {
        UINT32 mask = rowMask & xMask0;
        int counter = rowLen;
        while (--counter)
        {
            diff |= (*ptr++ & mask);
            mask = rowMask;
        }
        mask &= xMask1;
        diff |= (*ptr++ & mask);
        ptr += skip;
        rowMask = 0xFFFFFFFF;
    }

    // last row
    rowMask &= yMask1;
    UINT32 mask = rowMask & xMask0;
    int counter = rowLen;
    while (--counter)
    {
        diff |= (*ptr++ & mask);
        mask = rowMask;
    }
    mask &= xMask1;
    diff |= (*ptr++ & mask);
    return (diff != 0);
}

bool RasterOps::testRectAny(const BlockRasterBuffer& buf, const Vector4i& mnmx, DepthBuffer* input, float z)
{
    UMBRA_ASSERT(input);
    UMBRA_ASSERT(rectangleArea(mnmx) > 0);
    UMBRA_ASSERT(rectangleContains(buf.getBounds(), mnmx));

    // Clamp to buffer bounds.
    int x0 = mnmx.i;
    int x1 = mnmx.k;
    int y0 = mnmx.j;
    int y1 = mnmx.l;

    static const UINT32 s_x0Masks[4] = { 0xFFFF, 0xEEEE, 0xCCCC, 0x8888 };
    static const UINT32 s_x1Masks[4] = { 0xFFFF, 0x1111, 0x3333, 0x7777 };
    static const UINT32 s_y0Masks[4] = { 0xFFFF, 0xFFF0, 0xFF00, 0xF000 };
    static const UINT32 s_y1Masks[4] = { 0xFFFF, 0x000F, 0x00FF, 0x0FFF };

    // Compute raster block coverage.
    int blockXMin = (x0 >> BlockRasterBuffer::RasterBlockSizeLog2);
    int blockXMax = (x1 + BlockRasterBuffer::RasterBlockSize-1) >> BlockRasterBuffer::RasterBlockSizeLog2;
    int blockYMin = (y0 >> BlockRasterBuffer::RasterBlockSizeLog2);
    int blockYMax = (y1 + BlockRasterBuffer::RasterBlockSize-1) >> BlockRasterBuffer::RasterBlockSizeLog2;

    // Compute masks for X/Y extents.
    UINT32 xMask0 = s_x0Masks[x0 & (BlockRasterBuffer::RasterBlockSize-1)];
    UINT32 xMask1 = s_x1Masks[x1 & (BlockRasterBuffer::RasterBlockSize-1)];
    UINT32 yMask0 = s_y0Masks[y0 & (BlockRasterBuffer::RasterBlockSize-1)];
    UINT32 yMask1 = s_y1Masks[y1 & (BlockRasterBuffer::RasterBlockSize-1)];

    DepthBuffer::BlockIterator<1, true, false> iter =
        input->iterateBlocks<1, true, false>(Vector4i(blockXMin, blockYMin, blockXMax, blockYMax));
    SIMDRegister z4 = SIMDLoad(z);

    for (int blockY = blockYMin; blockY < blockYMax; blockY++)
    {
        const UINT16*   ptr     = buf.getRasterBlockPtr(blockXMin, blockY);
        int             diff    = 0;
        UINT32          rowMask = 0xFFFFu;
        if (blockY == blockYMin)    rowMask &= yMask0;
        if (blockY == blockYMax-1)  rowMask &= yMask1;

        UINT32  mask    = rowMask & xMask0;
        int     count   = blockXMax - blockXMin;
        while (count--)
        {
            mask &= xMask1 | (-count >> 31); // apply xMask1 only on last block
            UINT32 val = *ptr++ & mask;
            val &= iter.blocks().test16(z4);
            diff |= val;
            mask = rowMask;
            iter.next();
        }

        if (diff != 0)
            return true;
    }

    return false;
}

/*-------------------------------------------------------------------*//*!
 * \brief Update depth buffer to given value according to current state.
 *//*-------------------------------------------------------------------*/

void RasterOps::updateDepthBuffer(const BlockRasterBuffer& buf, DepthBuffer& depthBuffer, float z)
{
#if !defined(UMBRA_SIMD_CODE) || ((UMBRA_OS == UMBRA_PS3) && (UMBRA_ARCH == UMBRA_PPC))
    // with slow SIMD implementations just use scalar version
    return updateDepthBufferReference(buf, depthBuffer, z);
#else
    Vector4i blockRect = buf.getBlockRect();
    int rectBlocks = rectangleArea(blockRect);
    UMBRA_ASSERT(rectBlocks);
    bool canFill = (z >= depthBuffer.updateMaxZ(z));
    const Umbra::UINT32* blockPtr = buf.getBufferPtr();
    // translate to raster blocks
    blockRect.i <<= 1;
    blockRect.k <<= 1;
    SIMDRegister z4 = SIMDLoad(z);

    DepthBuffer::BlockIterator<2, true, true> iter = depthBuffer.iterateBlocks<2, true, true>(blockRect);

    if (UMBRA_OPT_LARGE_FOOTPRINT)
    {
        // Main loop, unrolled by 4. Handles 8 raster blocks per iteration.
        UMBRA_ASSERT(is128Aligned(blockPtr));
        SIMDRegister32 mask = SIMDLoadAligned32((const int*)blockPtr);

        int notAllZeros = 1;
        int notAllOnes = 1;

#ifdef UMBRA_SIMD_AVX
        if (!_mm_testnzc_si128(mask, SIMDLoad32(0xFFFFFFFF)))
        {
            if (*blockPtr)
                notAllOnes = 0;
            else
                notAllZeros = 0;
        }
#else
        notAllZeros = SIMDNotZero32(mask);
        notAllOnes = SIMDNotZero32(SIMDBitwiseAndNot32(SIMDLoad32(0xFFFFFFFF), mask));
#endif

        while (rectBlocks >= 4)
        {
            blockPtr += 4;
            SIMDRegister32 nextMask = SIMDLoadAligned32((const int*)blockPtr);
            int allOnes = !notAllOnes;
            int allZeros = !notAllZeros;

#ifdef UMBRA_SIMD_AVX
            notAllOnes = 1;
            notAllZeros = 1;
            if (!_mm_testnzc_si128(nextMask, SIMDLoad32(0xFFFFFFFF)))
            {
                if (*blockPtr)
                    notAllOnes = 0;
                else
                    notAllZeros = 0;
            }
#else
            notAllZeros = SIMDNotZero32(nextMask);
            notAllOnes = SIMDNotZero32(SIMDBitwiseAndNot32(SIMDLoad32(0xFFFFFFFF), nextMask));
#endif

            // Special paths for completely full and empty 32x4 strips

            if (allOnes)
            {
                if (canFill)
                {
                    iter.blocks().fill(z4); iter.next();
                    iter.blocks().fill(z4); iter.next();
                    iter.blocks().fill(z4); iter.next();
                    iter.blocks().fill(z4); iter.next();
                }
                else
                {
                    iter.blocks().max2(z4); iter.next();
                    iter.blocks().max2(z4); iter.next();
                    iter.blocks().max2(z4); iter.next();
                    iter.blocks().max2(z4); iter.next();
                }
            }
            else if (allZeros)
            {
                iter.skip(8);
            }
            else
            {
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 0)); iter.next();
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 1)); iter.next();
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 2)); iter.next();
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 3)); iter.next();
            }

            mask = nextMask;
            rectBlocks -= 4;
        }

        // remainder blocks
        if (rectBlocks)
        {
            switch (rectBlocks)
            {
            case 3:
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 0)); iter.next();
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 1)); iter.next();
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 2)); iter.next();
                break;
            case 2:
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 0)); iter.next();
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 1)); iter.next();
                break;
            case 1:
                iter.blocks().bitmaskMax32(z4, SIMDReplicate32(mask, 0)); iter.next();
                break;
            }
        }
    }
    else
    {
        // Code size optimized version
        while (rectBlocks--)
        {
            SIMDRegister32 mask = SIMDLoad32(*blockPtr++);
            iter.blocks().bitmaskMax32(z4, mask);
            iter.next();
        }
    }
    UMBRA_ASSERT(iter.end());
#endif
}

/*-------------------------------------------------------------------*//*!
 * \brief Update depth buffer to given value according to current state.
 *//*-------------------------------------------------------------------*/

void RasterOps::updateDepthBufferReference(const BlockRasterBuffer& buf, DepthBuffer& depthBuffer, float z)
{
    if (!buf.getNumRectBlocks())
        return;

    Vector4i blockRect = buf.getBlockRect();
    blockRect.i *= 2;
    blockRect.k *= 2;
    int blockXMin = blockRect.i;
    int blockXMax = blockRect.k;
    int blockYMin = blockRect.j;
    int blockYMax = blockRect.l;

    depthBuffer.updateMaxZ(z);
    DepthBuffer::BlockIterator<1, true, true> iter = depthBuffer.iterateBlocks<1, true, true>(blockRect);

    for (int blockY = blockYMin; blockY < blockYMax; blockY++)
    for (int blockX = blockXMin; blockX < blockXMax; blockX++)
    {
        UINT32 blockMask = *buf.getRasterBlockPtr(blockX, blockY);
        float* ptr = iter.blocks().getPtr();
        for (int pix = 0; pix < 16; pix++)
        {
            if ((blockMask & (1 << pix)) == 0)
                continue;
            ptr[pix] = max2(ptr[pix], z);
        }
        iter.next();
    }
    UMBRA_ASSERT(iter.end());
}

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief Compute a 16-bit sign mask of 4x4 floating point values.
 *//*-------------------------------------------------------------------*/

// \note On the PS3 SPU, the negative bits are extracted in the opposite order to other platforms.
//       We circumvent the issue by computing the pixels in the reverse order here.
// \todo [petri] This is probably not the cleanest way to handle the issue. Where does the flip actually occur?

#if UMBRA_OS == UMBRA_PS3
static const SIMDRegister s_scanPixelOffsets = SIMDLoad(3.0f, 2.0f, 1.0f, 0.0f);
#else
static const SIMDRegister s_scanPixelOffsets = SIMDLoad(0.0f, 1.0f, 2.0f, 3.0f);
#endif

template <int NumActivePlanes>
static UMBRA_FORCE_INLINE Umbra::UINT32 rasterizePortalBlock (const SIMDRegister& blockC, const SIMDRegister& cxStep, const SIMDRegister& cyStep)
{
    // Is outside masks.
    SIMDRegister scan0Mask;
    SIMDRegister scan1Mask;
    SIMDRegister scan2Mask;
    SIMDRegister scan3Mask;

    // Apply plane tests to each scanline outside mask.
#define COMPUTE_PLANE_MASKS(PLANE_NDX)                                                  \
    do                                                                                      \
    {                                                                                       \
        SIMDRegister scanC      = SIMDReplicate(blockC, PLANE_NDX);                         \
        SIMDRegister scanCYStep = SIMDReplicate(cyStep, PLANE_NDX);                         \
        SIMDRegister cxOffsets  = SIMDReplicate(cxStep, PLANE_NDX);                         \
        SIMDRegister scan0C     = SIMDMultiplyAdd(cxOffsets, s_scanPixelOffsets, scanC);    \
        SIMDRegister scan1C     = SIMDAdd(scan0C, scanCYStep);                              \
        SIMDRegister scan2C     = SIMDAdd(scan1C, scanCYStep);                              \
        SIMDRegister scan3C     = SIMDAdd(scan2C, scanCYStep);                              \
        if (PLANE_NDX == 0)                                                                 \
        {                                                                                   \
            scan0Mask = scan0C;                                                             \
            scan1Mask = scan1C;                                                             \
            scan2Mask = scan2C;                                                             \
            scan3Mask = scan3C;                                                             \
        }                                                                                   \
        else                                                                                \
        {                                                                                   \
            scan0Mask = SIMDBitwiseAnd(scan0Mask, scan0C);                                  \
            scan1Mask = SIMDBitwiseAnd(scan1Mask, scan1C);                                  \
            scan2Mask = SIMDBitwiseAnd(scan2Mask, scan2C);                                  \
            scan3Mask = SIMDBitwiseAnd(scan3Mask, scan3C);                                  \
        }                                                                                   \
    } while (0)

    COMPUTE_PLANE_MASKS(0);
    if (NumActivePlanes >= 2) COMPUTE_PLANE_MASKS(1);
    if (NumActivePlanes >= 3) COMPUTE_PLANE_MASKS(2);
    if (NumActivePlanes >= 4) COMPUTE_PLANE_MASKS(3);

#undef COMPUTE_PLANE_MASKS

    return SIMDExtract16Signs(scan0Mask, scan1Mask, scan2Mask, scan3Mask);
}

#ifdef UMBRA_SIMD_NEON

template<> UMBRA_FORCE_INLINE
Umbra::UINT32 rasterizePortalBlock<4>(
    const SIMDRegister& blockC,
    const SIMDRegister& cxStep,
    const SIMDRegister& cyStep)
{
    SIMDRegister cyStep2 = SIMDAdd(cyStep, cyStep);
    SIMDRegister cyStep3 = SIMDAdd(cyStep2, cyStep);

    SIMDRegister p0 = blockC;
    SIMDRegister p1 = SIMDAdd(blockC, cxStep);
    SIMDRegister p2 = SIMDAdd(p1, cxStep);
    SIMDRegister p3 = SIMDAdd(p2, cxStep);

    SIMDRegister p0_3 = SIMDAdd(p0, cyStep3);
    SIMDRegister p1_3 = SIMDAdd(p1, cyStep3);
    SIMDRegister p2_3 = SIMDAdd(p2, cyStep3);
    SIMDRegister p3_3 = SIMDAdd(p3, cyStep3);

    SIMDRegister p0_2 = SIMDAdd(p0, cyStep2);
    SIMDRegister p1_2 = SIMDAdd(p1, cyStep2);
    SIMDRegister p2_2 = SIMDAdd(p2, cyStep2);
    SIMDRegister p3_2 = SIMDAdd(p3, cyStep2);

    SIMDRegister p0_1 = SIMDAdd(p0, cyStep);
    SIMDRegister p1_1 = SIMDAdd(p1, cyStep);
    SIMDRegister p2_1 = SIMDAdd(p2, cyStep);
    SIMDRegister p3_1 = SIMDAdd(p3, cyStep);

    uint32x4_t combined = vshrq_n_u32(vreinterpretq_u32_f32(p3_3), 16);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p2_3), 17);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p1_3), 18);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p0_3), 19);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p3_2), 20);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p2_2), 21);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p1_2), 22);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p0_2), 23);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p3_1), 24);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p2_1), 25);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p1_1), 26);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p0_1), 27);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p3), 28);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p2), 29);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p1), 30);
    combined = vsriq_n_u32(combined, vreinterpretq_u32_f32(p0), 31);

    uint32x2_t narrowed = vand_u32(vget_high_u32(combined), vget_low_u32(combined));
    UINT32 ret = vget_lane_u32(narrowed, 0);
    ret &= vget_lane_u32(narrowed, 1);
    return ret;
}

#endif

class RasterizerImpl
{
private:
    BlockRasterBuffer& buf;
    const BlockRasterBuffer& mask;
    Vector4i bounds;
    SIMDRegister cBase;
    SIMDRegister cxStep;
    SIMDRegister cyStep;

public:
    UMBRA_FORCE_INLINE RasterizerImpl(
        BlockRasterBuffer& buf,
        const BlockRasterBuffer& mask,
        const Vector4i& bounds)
        : buf(buf), mask(mask), bounds(bounds)
    {
    }

    UMBRA_FORCE_INLINE void edgeSetup (const VQuad& quad, const AxisNormals& normals)
    {
        // Setup plane equations from homogeneous clip coordinates.
        SIMDRegister clipX      = quad.x;
        SIMDRegister clipY      = quad.y;
        SIMDRegister clipW      = quad.z;
        SIMDRegister clipXRot   = SIMDShuffle<1,2,3,0>(clipX);
        SIMDRegister clipYRot   = SIMDShuffle<1,2,3,0>(clipY);
        SIMDRegister clipWRot   = SIMDShuffle<1,2,3,0>(clipW);
        SIMDRegister expX       = SIMDReplicate(quad.expand, 0);
        SIMDRegister expY       = SIMDReplicate(quad.expand, 1);
        SIMDRegister expW       = SIMDReplicate(quad.expand, 3);

        // constant coefficients for portal side planes
        SIMDRegister planeConst =
            SIMDMultiplyAdd(SIMDLoadAligned(normals.x), clipX,
            SIMDMultiplyAdd(SIMDLoadAligned(normals.y), clipY,
            SIMDMultiply(SIMDLoadAligned(normals.z), clipW)));

        // constant coefficient sign dictates expansion direction
        SIMDRegister expMask = SIMDCompareGT(planeConst, SIMDZero());
        expX = SIMDBitwiseAnd(expX, expMask);
        expY = SIMDBitwiseAnd(expY, expMask);
        expW = SIMDBitwiseAnd(expW, expMask);

        // expand edges
        clipX = SIMDAdd(clipX, expX);
        clipXRot = SIMDAdd(clipXRot, expX);
        clipY = SIMDAdd(clipY, expY);
        clipYRot = SIMDAdd(clipYRot, expY);
        clipW = SIMDAdd(clipW, expW);
        clipWRot = SIMDAdd(clipWRot, expW);

        // Calculate plane eqs
        cxStep = SIMDSub(SIMDMultiply(clipY, clipWRot), SIMDMultiply(clipW, clipYRot));
        cyStep = SIMDSub(SIMDMultiply(clipW, clipXRot), SIMDMultiply(clipX, clipWRot));
        cBase = SIMDSub(SIMDMultiply(clipX, clipYRot), SIMDMultiply(clipY, clipXRot));

        // Transform plane equations from [-1.0, +1.0] range to [0, rasterSize].
        cxStep = SIMDMultiply(cxStep, s_PortalRasterScale);
        cyStep = SIMDMultiply(cyStep, s_PortalRasterScale);
        cBase = SIMDMultiplyAdd(s_PortalRasterOffset, cxStep, cBase);
        cBase = SIMDMultiplyAdd(s_PortalRasterOffset, cyStep, cBase);

        // Expand edges to cover all touched pixels.
        SIMDRegister expand = SIMDAdd(SIMDAbs(cxStep), SIMDAbs(cyStep));
        expand = SIMDMax(expand, s_Epsilon); // fix degenerate cases where camera is on quad edge
        cBase = SIMDMultiplyAdd(s_MinusHalf, expand, cBase);
    }

    UMBRA_FORCE_INLINE bool rasterize(void)
    {
        Vector4i blockRect = BlockRasterBuffer::boundsToRasterRect(bounds);
        UINT32 ret;

        int xBlocks = blockRect.k - blockRect.i;
        int yBlocks = blockRect.l - blockRect.j;
        int numBlocks = xBlocks * yBlocks;
        if (UMBRA_OPT_LARGE_FOOTPRINT && (numBlocks <= 10 || xBlocks == 1 || yBlocks == 1))
        {
            ret = processRasterBlocksSimple(blockRect.i, blockRect.j, blockRect.k, blockRect.l);
        }
        else if (BlockRasterBuffer::RasterBlocksTotal < 100 || numBlocks <= 40)
        {
            ret = processRasterBlocks(blockRect.i, blockRect.j, blockRect.k, blockRect.l);
        }
        else
        {
            ret = processTileBlocks(blockRect.i, blockRect.j, blockRect.k, blockRect.l);
        }

        return (ret != 0);
    }

private:
    RasterizerImpl(const RasterizerImpl&);
    RasterizerImpl& operator=(const RasterizerImpl&);

    struct BlockIterator
    {
        UMBRA_FORCE_INLINE BlockIterator(
            int x0, int y0, int x1, int y1,
            SIMDRegister cornerC, SIMDRegister blockXStep, SIMDRegister blockYStep,
            BlockRasterBuffer& dst, const BlockRasterBuffer& mask)
        {
            scanlineC = cornerC;
            currentC = cornerC;
            xStep = blockXStep;
            yStep = blockYStep;
            rowLen = x1 - x0;
            // TODO: see if this can be optimized
            dstPtr = dst.getRasterBlockPtr(x0, y0);
            maskPtr = mask.getRasterBlockPtr(x0, y0);
            dstSkip = dst.getRasterBlockStride() - rowLen + 1;
            maskSkip = mask.getRasterBlockStride() - rowLen + 1;
            count = rowLen * (y1 - y0);
            leftInRow = --rowLen;
        }

        UMBRA_FORCE_INLINE void next()
        {
            if (!leftInRow)
            {
                leftInRow = rowLen;
                scanlineC = SIMDAdd(scanlineC, yStep);
                currentC = scanlineC;
                dstPtr += dstSkip;
                maskPtr += maskSkip;
            }
            else
            {
                --leftInRow;
                currentC = SIMDAdd(currentC, xStep);
                dstPtr++;
                maskPtr++;
            }
        }

        UINT16* dstPtr;
        const UINT16* maskPtr;
        SIMDRegister currentC;
        SIMDRegister scanlineC;
        SIMDRegister xStep;
        SIMDRegister yStep;
        int rowLen;
        int leftInRow;
        int dstSkip;
        int maskSkip;
        int count;
    };

    // TODO: combine logic with BlockIterator
    struct TileIterator
    {
        UMBRA_FORCE_INLINE TileIterator(
            int x0, int y0, int x1, int y1,
            SIMDRegister cBase, SIMDRegister cxStep, SIMDRegister cyStep)
        {
            int blockToTile = BlockRasterBuffer::TileSizeLog2 - BlockRasterBuffer::RasterBlockSizeLog2;
            int tileXMin = x0 >> blockToTile;
            int tileXMax = (x1 + BlockRasterBuffer::RasterBlocksPerTile-1) >> blockToTile;
            int tileYMin = y0 >> blockToTile;
            int tileYMax = (y1 + BlockRasterBuffer::RasterBlocksPerTile-1) >> blockToTile;
            SIMDRegister xOfs = SIMDIntToFloat(SIMDLoad32(tileXMin));
            SIMDRegister yOfs = SIMDIntToFloat(SIMDLoad32(tileYMin));
            xStep = SIMDMultiply(cxStep, s_tileSize);
            yStep = SIMDMultiply(cyStep, s_tileSize);
            blockXStep = SIMDMultiply(cxStep, s_RasterBlockSize);
            blockYStep = SIMDMultiply(cyStep, s_RasterBlockSize);
            int xBlockOfs = x0 & (BlockRasterBuffer::RasterBlocksPerTile - 1);
            int yBlockOfs = y0 & (BlockRasterBuffer::RasterBlocksPerTile - 1);
            blockXStartOfs = SIMDMultiply(blockXStep, SIMDLoad((float)xBlockOfs));
            blockYStartOfs = SIMDMultiply(blockYStep, SIMDLoad((float)yBlockOfs));
            scanlineC = SIMDMultiplyAdd(yStep, yOfs, SIMDMultiplyAdd(xStep, xOfs, cBase));
            currentC = scanlineC;
            rowStart = tileXMin;
            rowEnd = tileXMax - 1;
            x = rowStart;
            y = tileYMin;
            count = (tileXMax - tileXMin) * (tileYMax - tileYMin);
            blockOfs = SIMDAdd(blockXStartOfs, blockYStartOfs);
        }

        UMBRA_FORCE_INLINE void next()
        {
            if (x == rowEnd)
            {
                scanlineC = SIMDAdd(scanlineC, yStep);
                currentC = scanlineC;
                blockOfs = blockXStartOfs;
                blockYStartOfs = SIMDZero();
                x = rowStart;
                y++;
            }
            else
            {
                currentC = SIMDAdd(currentC, xStep);
                blockOfs = blockYStartOfs;
                x++;
            }
        }

        SIMDRegister currentC;
        SIMDRegister scanlineC;
        SIMDRegister blockOfs;
        SIMDRegister xStep;
        SIMDRegister yStep;
        SIMDRegister blockXStep;
        SIMDRegister blockYStep;
        SIMDRegister blockXStartOfs;
        SIMDRegister blockYStartOfs;
        int rowStart;
        int rowEnd;
        int x;
        int y;
        int count;
    };

    UMBRA_FORCE_INLINE BlockIterator iterateBlocks(int x0, int y0, int x1, int y1,
        SIMDRegister xStep, SIMDRegister yStep, SIMDRegister cornerC)
    {
        return BlockIterator(x0, y0, x1, y1, cornerC, xStep, yStep, buf, mask);
    }

    UMBRA_FORCE_INLINE BlockIterator iterateBlocks(int x0, int y0, int x1, int y1)
    {
        SIMDRegister xOfs = SIMDIntToFloat(SIMDLoad32(x0));
        SIMDRegister yOfs = SIMDIntToFloat(SIMDLoad32(y0));
        SIMDRegister blockXStep = SIMDMultiply(cxStep, s_RasterBlockSize);
        SIMDRegister blockYStep = SIMDMultiply(cyStep, s_RasterBlockSize);
        SIMDRegister cornerC = SIMDMultiplyAdd(blockYStep, yOfs, SIMDMultiplyAdd(blockXStep, xOfs, cBase));
        return iterateBlocks(x0, y0, x1, y1, blockXStep, blockYStep, cornerC);
    }

    UMBRA_FORCE_INLINE TileIterator iterateTiles(int x0, int y0, int x1, int y1)
    {
        return TileIterator(x0, y0, x1, y1, cBase, cxStep, cyStep);
    }

    UMBRA_FORCE_INLINE UINT32 updateBlock(UINT16* dst, UINT32 mask, UINT32 portal)
    {
        UINT32 cur = *dst;
        UINT32 res = cur | (mask & portal);
        *dst = (UINT16)res;
        return res ^ cur;
    }

    UMBRA_FORCE_INLINE void computeNearFarOffsets(SIMDRegister& near_offset, SIMDRegister& far_offset, const SIMDRegister& blockXStep, const SIMDRegister& blockYStep)
    {
        // Compute C offsets for vertices which nearest/farthest from each plane.
        // These are used for early-exit testing for inside/outside all planes test.
        SIMDRegister blockX = SIMDSub(blockXStep, cxStep);
        SIMDRegister blockY = SIMDSub(blockYStep, cyStep);
        SIMDRegister xNeg = SIMDCompareGT(SIMDZero(), cxStep);
        SIMDRegister yNeg = SIMDCompareGT(SIMDZero(), cyStep);
        near_offset = SIMDAdd(SIMDBitwiseAnd(blockX, xNeg), SIMDBitwiseAnd(blockY, yNeg));
        far_offset = SIMDAdd(SIMDBitwiseAndNot(blockX, xNeg), SIMDBitwiseAndNot(blockY, yNeg));
    }

    // Simplest possible rasterblock processing, no stall avoidance or early exits.
    Umbra::UINT32 processRasterBlocksReference(int numBlocks, int blockXMin, int blockYMin, int blockXMax, int blockYMax)
    {
        BlockIterator iter = iterateBlocks(blockXMin, blockYMin, blockXMax, blockYMax);
        UINT32 changedMask = 0;

        int counter = numBlocks;
        while (counter--)
        {
            changedMask |= updateBlock(iter.dstPtr, *iter.maskPtr,
                rasterizePortalBlock<4>(iter.currentC, cxStep, cyStep));
            iter.next();
        }

        return changedMask;
    }

    // Simple rasterblock processing, postpones reading portal mask for block until next
    // iteration to avoid stalls from reading back SIMD register contents.
    UMBRA_FORCE_INLINE Umbra::UINT32 processRasterBlocksSimple(BlockIterator& iter)
    {
        UINT32 changedMask = 0;
        UINT32 portalMask = rasterizePortalBlock<4>(iter.currentC, cxStep, cyStep);

        iter.count--;
        while (iter.count--)
        {
            UINT32 portal = portalMask;
            UINT32 mask = *iter.maskPtr;
            UINT16* dstPtr = iter.dstPtr;

            // proceed one block
            iter.next();

            // rasterize current
            portalMask = rasterizePortalBlock<4>(iter.currentC, cxStep, cyStep);

            // apply results
            changedMask |= updateBlock(dstPtr, mask, portal);
        }

        // loop footer
        changedMask |= updateBlock(iter.dstPtr, *iter.maskPtr, portalMask);
        return changedMask;
    }

    UMBRA_FORCE_INLINE UINT32 processRasterBlocksSimple(int blockXMin, int blockYMin, int blockXMax, int blockYMax)
    {
        BlockIterator iter = iterateBlocks(blockXMin, blockYMin, blockXMax, blockYMax);
        return processRasterBlocksSimple(iter);
    }

    // Fully-featured rasterblock processing. Avoids SIMD register read stalls, and does early exits for
    // completely inside and outside blocks and for 0 src mask blocks.
    UMBRA_FORCE_INLINE UINT32 processRasterBlocks(BlockIterator& iter, const SIMDRegister& nearCornerOfs, const SIMDRegister& farCornerOfs)
    {
        UINT32 changedMask = 0;
        UINT32 srcMask = 0;
        // Dirty trick to avoid having to branch inside loop: we write the first UINT16 twice
        UINT16* dstPtr = iter.dstPtr;
        UINT32 portalMask = 0;
        int nearMask;
        int farMask;

        SIMDWriteNegativeMask2(nearMask, farMask,
            SIMDAdd(iter.currentC, nearCornerOfs), SIMDAdd(iter.currentC, farCornerOfs));

        while (iter.count--)
        {
            UINT32 prevSrc = srcMask;
            UINT16* prevDst = dstPtr;
            UINT32 prevPortal = portalMask;
            srcMask = *iter.maskPtr;
            dstPtr = iter.dstPtr;

            bool isFullyOutside = (nearMask != FullNegativeMask);
            bool isFullyInside = (farMask == FullNegativeMask);
            SIMDRegister blockC = iter.currentC;

            // get negative masks for next block
            iter.next();
            SIMDWriteNegativeMask2(nearMask, farMask,
                SIMDAdd(iter.currentC, nearCornerOfs), SIMDAdd(iter.currentC, farCornerOfs));

            // retrieve portalMask for current block
            if (isFullyOutside || !srcMask)
                portalMask = 0;
            else if (isFullyInside)
                portalMask = 0xFFFF;
            else
                portalMask = rasterizePortalBlock<4>(blockC, cxStep, cyStep);

            changedMask |= updateBlock(prevDst, prevSrc, prevPortal);
        }

        // loop footer
        changedMask |= updateBlock(dstPtr, srcMask, portalMask);
        return changedMask;
    }

    UMBRA_FORCE_INLINE UINT32 processRasterBlocks(int blockXMin, int blockYMin, int blockXMax, int blockYMax)
    {
        SIMDRegister near_offset, far_offset;
        BlockIterator iter = iterateBlocks(blockXMin, blockYMin, blockXMax, blockYMax);
        computeNearFarOffsets(near_offset, far_offset, iter.xStep, iter.yStep);
        return processRasterBlocks(iter, near_offset, far_offset);
    }

    // Process tiles, reference version
    UINT32 processTileBlocksReference(int blockXMin, int blockYMin, int blockXMax, int blockYMax)
    {
        TileIterator iter = iterateTiles(blockXMin, blockYMin, blockXMax, blockYMax);
        UINT32 changedMask = 0;
        int counter = iter.count;
        while (counter--)
        {
            int xOfs = iter.x << (BlockRasterBuffer::TileSizeLog2 - BlockRasterBuffer::RasterBlockSizeLog2);
            int yOfs = iter.y << (BlockRasterBuffer::TileSizeLog2 - BlockRasterBuffer::RasterBlockSizeLog2);
            int x0 = max2(blockXMin, xOfs);
            int y0 = max2(blockYMin, yOfs);
            int x1 = min2(blockXMax, xOfs + BlockRasterBuffer::RasterBlocksPerTile);
            int y1 = min2(blockYMax, yOfs + BlockRasterBuffer::RasterBlocksPerTile);
            changedMask |= processRasterBlocks(x0, y0, x1, y1);
            iter.next();
        }

        return changedMask;
    }

    // Process tiles, (somewhat) optimized
    // Further optimization opportunities:
    // - process fully covered tiles separately from partials
    // - make use of plane inside mask in per-tile processing, for example special processing
    //   for single edge tiles
    UMBRA_FORCE_INLINE UINT32 processTileBlocks(int blockXMin, int blockYMin, int blockXMax, int blockYMax)
    {
        TileIterator iter = iterateTiles(blockXMin, blockYMin, blockXMax, blockYMax);

        SIMDRegister nearCornerOfs, farCornerOfs;
        SIMDRegister nearCornerOfsBlock, farCornerOfsBlock;
        computeNearFarOffsets(nearCornerOfs, farCornerOfs, iter.xStep, iter.yStep);
        computeNearFarOffsets(nearCornerOfsBlock, farCornerOfsBlock, iter.blockXStep, iter.blockYStep);

        int nearMask, farMask;
        SIMDWriteNegativeMask2(nearMask, farMask,
            SIMDAdd(iter.currentC, nearCornerOfs), SIMDAdd(iter.currentC, farCornerOfs));

        UINT32 changedMask = 0;
        while (iter.count--)
        {
            bool isFullyOutside = (nearMask != FullNegativeMask);
            bool isFullyInside = (farMask == FullNegativeMask);
            int xOfs = iter.x << (BlockRasterBuffer::TileSizeLog2 - BlockRasterBuffer::RasterBlockSizeLog2);
            int yOfs = iter.y << (BlockRasterBuffer::TileSizeLog2 - BlockRasterBuffer::RasterBlockSizeLog2);
            SIMDRegister blockC = SIMDAdd(iter.currentC, iter.blockOfs);

            iter.next();
            SIMDWriteNegativeMask2(nearMask, farMask,
                SIMDAdd(iter.currentC, nearCornerOfs), SIMDAdd(iter.currentC, farCornerOfs));

            if (isFullyOutside)
                continue;

            int x0 = max2(blockXMin, xOfs);
            int y0 = max2(blockYMin, yOfs);
            int x1 = min2(blockXMax, xOfs + BlockRasterBuffer::RasterBlocksPerTile);
            int y1 = min2(blockYMax, yOfs + BlockRasterBuffer::RasterBlocksPerTile);

            if (isFullyInside)
            {
                // Note: it is possible for a tile to be fully inside but not fully
                // covered by the block rect, due to clamping to mask. Mask is always
                // known to be rectblock-aligned, so we can do filling with rect
                // blocks instead of 16 bits at a time.
                x0 = x0 >> 1;
                x1 = (x1 + 1) >> 1;
                int rowLen = x1 - x0;
                int numRows = y1 - y0;
                UINT32* dstPtr = buf.getRectBlockPtr(x0, y0);
                const UINT32* maskPtr = mask.getRectBlockPtr(x0, y0);
                int dstSkip = buf.getRectBlockStride() - rowLen;
                int maskSkip = mask.getRectBlockStride() - rowLen;
                while (numRows--)
                {
                    int x = rowLen;
                    while (x--)
                    {
                        UINT32 dst = *dstPtr;
                        UINT32 res = dst | *maskPtr++;
                        changedMask |= (res ^ dst);
                        *dstPtr++ = res;
                    }
                    dstPtr += dstSkip;
                    maskPtr += maskSkip;
                }
            }
            else
            {
                BlockIterator blockIter = iterateBlocks(x0, y0, x1, y1, iter.blockXStep, iter.blockYStep, blockC);
                changedMask |= processRasterBlocks(blockIter, nearCornerOfsBlock, farCornerOfsBlock);
            }
        }

        return changedMask;
    }

};

} // namespace Umbra

bool RasterOps::rasterizePortal (BlockRasterBuffer& buf, const Vector4i& mnmx, const VQuad& quad, const AxisNormals& n, const BlockRasterBuffer& src)
{
    // empty rectangles already culled before this
    UMBRA_ASSERT(mnmx.i < mnmx.k && mnmx.j < mnmx.l);
    // Validate that rect is clamped to buf and mask
    UMBRA_ASSERT(rectangleContains(buf.getBounds(), mnmx));
    UMBRA_ASSERT(rectangleContains(src.getBounds(), mnmx));

    RasterizerImpl impl(buf, src, mnmx);
    impl.edgeSetup(quad, n);
    return impl.rasterize();
}

bool RasterOps::rasterizeQuad (BlockRasterBuffer& buf, const Vector4i& quad, const BlockRasterBuffer& src)
{
    // empty rectangles already culled before this
    UMBRA_ASSERT(quad.i < quad.k && quad.j < quad.l);
    // Validate that rect is clamped to buf and mask
    UMBRA_ASSERT(rectangleContains(buf.getBounds(), quad));
    UMBRA_ASSERT(rectangleContains(src.getBounds(), quad));

    int x0 = quad.i;
    int y0 = quad.j;
    int x1 = quad.k;
    int y1 = quad.l;

    static const UINT32 s_x0Masks[4] = { 0xFFFF, 0xEEEE, 0xCCCC, 0x8888 };
    static const UINT32 s_x1Masks[4] = { 0xFFFF, 0x1111, 0x3333, 0x7777 };
    static const UINT32 s_y0Masks[4] = { 0xFFFF, 0xFFF0, 0xFF00, 0xF000 };
    static const UINT32 s_y1Masks[4] = { 0xFFFF, 0x000F, 0x00FF, 0x0FFF };

    // Get area of affected 4x4 blocks in buffer.
    Vector4i blockRect = BlockRasterBuffer::boundsToRasterRect(quad);
    int blockXMin = blockRect.i;
    int blockXMax = blockRect.k;
    int blockYMin = blockRect.j;
    int blockYMax = blockRect.l;

    // Compute masks for X/Y extents.
    UINT32 xMask0 = s_x0Masks[x0 & (BlockRasterBuffer::RasterBlockSize-1)];
    UINT32 xMask1 = s_x1Masks[x1 & (BlockRasterBuffer::RasterBlockSize-1)];
    UINT32 yMask0 = s_y0Masks[y0 & (BlockRasterBuffer::RasterBlockSize-1)];
    UINT32 yMask1 = s_y1Masks[y1 & (BlockRasterBuffer::RasterBlockSize-1)];
    UINT32 changedMask = 0x0;

    for (int blockY = blockYMin; blockY < blockYMax; blockY++)
    {
        const UINT16*   srcPtr  = src.getRasterBlockPtr(blockXMin, blockY);
        UINT16*         dstPtr  = buf.getRasterBlockPtr(blockXMin, blockY);
        UINT32          rowMask = 0xFFFFu;
        if (blockY == blockYMin)    rowMask &= yMask0;
        if (blockY == blockYMax-1)  rowMask &= yMask1;

        UINT32  mask    = rowMask & xMask0;
        int     count   = blockXMax - blockXMin;
        while (count--)
        {
            mask &= xMask1 | (-count >> 31); // apply xMask1 only on last block
            UINT32 dst = *dstPtr;
            UINT32 src = *srcPtr++;
            UINT32 res = dst | (src & mask);
            changedMask |= (dst ^ res);
            *dstPtr++ = (UINT16)res;
            mask = rowMask;
        }
    }

    return (changedMask != 0);
}

void RasterOps::debugStats(void)
{
}
