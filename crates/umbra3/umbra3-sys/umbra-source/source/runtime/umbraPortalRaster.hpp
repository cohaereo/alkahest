#pragma once
#ifndef __UMBRAPORTALRASTER_H
#define __UMBRAPORTALRASTER_H

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
 * \brief   Umbra portal raster buffers & operations
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraTransformer.hpp"
#include "umbraSIMD.hpp"
#include "umbraQueryContext.hpp"
#include "umbraRasterDefs.hpp"

namespace Umbra
{

class DepthBuffer;

/*-------------------------------------------------------------------*//*!
 * \brief   Screen bounding box with UINT8 coordinates.
 *//*-------------------------------------------------------------------*/

struct PackedScreenBounds
{
    PackedScreenBounds (void) : minX(0), minY(0), maxX(0), maxY(0) {}
    PackedScreenBounds (const Vector4i& v) : minX((UINT8)v.i), minY((UINT8)v.j), maxX((UINT8)v.k), maxY((UINT8)v.l) {}

    void set(const Vector4i& v) { minX = (UINT8)v.i; minY = (UINT8)v.j; maxX = (UINT8)v.k; maxY = (UINT8)v.l; }
    Vector4i getVector4i(void) const { return Vector4i(minX, minY, maxX, maxY); }
    int getArea(void) const { return ((int)maxX - (int)minX) * ((int)maxY - (int)minY); }
    bool operator==(const PackedScreenBounds& o) { return *(UINT32*)this == *(UINT32*)&o; }

    UINT8   minX;
    UINT8   minY;
    UINT8   maxX;
    UINT8   maxY;
};

UMBRA_CT_ASSERT(sizeof(PackedScreenBounds) == 4);

/*-------------------------------------------------------------------*//*!
 * \brief   1-bit blocked buffer for cell buffers and portal rasterization.
 *          The buffer operates on two levels: 8x4 rect blocks and 4x4 raster
 *          blocks. The 8x4 rect blocks are used for clearing, filling, and
 *          other miscellaneous operations. The 4x4 raster blocks are used
 *          for rendering. There are also 16x16 tiles, which are used for
 *          early-exiting the rendering of very large portals.
 *
 *          The buffer memory layout is such that the 8x4 rect blocks are
 *          stored in linear block-order (one UINT32 per block). The rect
 *          blocks consist of two raster blocks (UINT16), with the leftmost
 *          pixels stored in the first UINT16, and rightmost pixels in the
 *          second. There are no endianess-sensitive operations for the
 *          rect blocks.
 *//*-------------------------------------------------------------------*/

class BlockRasterBuffer
{
public:
    enum
    {
        // Use a 8x4 block for rectangle ops.
        RectBlockWidthLog2      = 3,    // 8
        RectBlockHeightLog2     = 2,    // 4
        RectBlockWidth          = (1 << RectBlockWidthLog2),
        RectBlockHeight         = (1 << RectBlockHeightLog2),

        // Block size for raster ops (only supports square blocks for now).
        RasterBlockSizeLog2     = UMBRA_RASTER_BLOCK_X_LOG,
        RasterBlockSize         = UMBRA_RASTER_BLOCK_X,
        RasterBlocksTotalPerDim = UMBRA_PORTAL_RASTER_SIZE / RasterBlockSize,
        RasterBlocksTotal       = RasterBlocksTotalPerDim * RasterBlocksTotalPerDim,

        // Tile size for rendering of large portals.
        TileSizeLog2            = 4,    // 16
        TileSize                = (1 << TileSizeLog2),
        RasterBlocksPerTile     = TileSize / RasterBlockSize
    };

    BlockRasterBuffer(void) : m_blocks(NULL) {}
    BlockRasterBuffer(const Vector4i& blockRect, void* bufferPtr)
        : m_blockRect(blockRect), m_blocks((UINT32*)bufferPtr)
    {
        UMBRA_ASSERT(is128Aligned(bufferPtr));
    }
    ~BlockRasterBuffer(void) {}

    void reset(void)
    {
        m_blockRect = PackedScreenBounds();
        m_blocks    = NULL;
    }

    const UINT32* getBufferPtr(void) const { return m_blocks; }
    UINT32* getBufferPtr(void) { return m_blocks; }
    bool isEmpty(void) const { return m_blocks == NULL; }
    Vector4i getBlockRect(void) const { return m_blockRect.getVector4i(); }
    int getNumRectBlocks(void) const { return m_blockRect.getArea(); }

    Vector4i getBounds(void) const
    {
        return Vector4i(
            m_blockRect.minX << RectBlockWidthLog2,
            m_blockRect.minY << RectBlockHeightLog2,
            m_blockRect.maxX << RectBlockWidthLog2,
            m_blockRect.maxY << RectBlockHeightLog2);
    }

    static Vector4i boundsToBlockRect(const Vector4i& bounds)
    {
        UMBRA_ASSERT(checkBounds(bounds));
        return Vector4i(
            bounds.i >> RectBlockWidthLog2,
            bounds.j >> RectBlockHeightLog2,
            (bounds.k + RectBlockWidth-1) >> RectBlockWidthLog2,
            (bounds.l + RectBlockHeight-1) >> RectBlockHeightLog2);
    }

    static Vector4i boundsToRasterRect(const Vector4i& bounds)
    {
        UMBRA_ASSERT(checkBounds(bounds));
        return Vector4i(
            bounds.i >> RasterBlockSizeLog2,
            bounds.j >> RasterBlockSizeLog2,
            (bounds.k + RasterBlockSize-1) >> RasterBlockSizeLog2,
            (bounds.l + RasterBlockSize-1) >> RasterBlockSizeLog2);
    }

    void setPixel (int x, int y)
    {
        UMBRA_ASSERT(x >= m_blockRect.minX*RectBlockWidth  && x < m_blockRect.maxX*RectBlockWidth);
        UMBRA_ASSERT(y >= m_blockRect.minY*RectBlockHeight && y < m_blockRect.maxY*RectBlockHeight);
        int shift = ((y & 3) << 2) + (x & 3);
        *getRasterBlockPtr(x>>RasterBlockSizeLog2, y>>RasterBlockSizeLog2) |= (1 << shift);
    }

    bool testPixel (int x, int y) const
    {
        UMBRA_ASSERT(x >= m_blockRect.minX*RectBlockWidth  && x < m_blockRect.maxX*RectBlockWidth);
        UMBRA_ASSERT(y >= m_blockRect.minY*RectBlockHeight && y < m_blockRect.maxY*RectBlockHeight);
        int shift = ((y & 3) << 2) + (x & 3);
        return (*getRasterBlockPtr(x>>RasterBlockSizeLog2, y>>RasterBlockSizeLog2) & (1 << shift)) != 0;
    }

    UMBRA_FORCE_INLINE UINT32*  getRectBlockPtr         (int blockX, int blockY) const
    {
        UMBRA_ASSERT(blockX >= m_blockRect.minX && blockX < m_blockRect.maxX);
        UMBRA_ASSERT(blockY >= m_blockRect.minY && blockY < m_blockRect.maxY);
        int     rectBlockStride = getRectBlockStride();
        int     localBlockY     = blockY - m_blockRect.minY;
        int     localBlockX     = blockX - m_blockRect.minX;
        return &m_blocks[localBlockY*rectBlockStride + localBlockX];
    }
    UMBRA_FORCE_INLINE UINT16*  getRasterBlockPtr       (int blockX, int blockY) const
    {
        UMBRA_ASSERT(blockX >= 2*m_blockRect.minX && blockX < 2*m_blockRect.maxX);
        UMBRA_ASSERT(blockY >= m_blockRect.minY && blockY < m_blockRect.maxY);
        UINT16* ptr                 = (UINT16*)m_blocks;
        int     rasterBlockStride   = getRasterBlockStride();
        int     localBlockY         = blockY - m_blockRect.minY;
        int     localBlockX         = blockX - 2*m_blockRect.minX; // multiplied by two to convert rect blocks to raster blocks
        return &ptr[localBlockY*rasterBlockStride + localBlockX];
    }

    int getRectBlockStride      (void) const { return (m_blockRect.maxX - m_blockRect.minX); }
    int getRasterBlockStride    (void) const { return getRectBlockStride() * 2; /* two 4x4 raster blocks for each 8x4 rect block */ }

private:

    static UMBRA_INLINE bool checkBounds (const Vector4i& bounds)
    {
        return (bounds.k > bounds.i) && (bounds.l > bounds.j) &&
            (bounds.i >= 0) && (bounds.j >= 0) &&
            (bounds.k <= UMBRA_PORTAL_RASTER_SIZE) &&
            (bounds.l <= UMBRA_PORTAL_RASTER_SIZE);
    }

    static UMBRA_INLINE bool checkBlockRect (const Vector4i& blockRect)
    {
        return (blockRect.k > blockRect.i) && (blockRect.l > blockRect.j) &&
            (blockRect.i >= 0) && (blockRect.j >= 0) &&
            (blockRect.k <= (UMBRA_PORTAL_RASTER_SIZE >> RectBlockWidthLog2)) &&
            (blockRect.l <= (UMBRA_PORTAL_RASTER_SIZE >> RectBlockHeightLog2));
    }


    // Member variables.
    PackedScreenBounds  m_blockRect;
    UINT32*             m_blocks;

    friend class BufferAllocator;
};

class BufferAllocator
{
public:
    enum
    {
        InvalidOffset = -1,
        BlockSizeLog2 = 5,                    // 32 bytes
        BlockSize = (1 << BlockSizeLog2),
        RasterBlocksPerAllocBlock = (BlockSize / (BlockRasterBuffer::RasterBlockSize * BlockRasterBuffer::RasterBlockSize)),
        NumTotalBlocks = (BlockRasterBuffer::RasterBlocksTotal / RasterBlocksPerAllocBlock) * 8, // 8 times full raster
        NumBitfields = (NumTotalBlocks / (sizeof(UINT32) * 8)) + 1
    };

                        BufferAllocator     (void);

    bool                allocateBuffer      (BlockRasterBuffer& buffer, const Vector4i& blockBounds, bool isTransient);
    bool                expandBuffer        (BlockRasterBuffer& buffer, const Vector4i& blockBounds, bool isTransient);
    void                releaseBuffer       (BlockRasterBuffer& buffer);
    void                setPersistent       (BlockRasterBuffer& buffer) { m_persistent = buffer.getBufferPtr(); }
    bool                isPersistent        (const BlockRasterBuffer& buffer) { return m_persistent == buffer.getBufferPtr(); }
    size_t              dataSize            (void) const { return sizeof(m_blocks); }
    void                freeTransients      (void) { m_transientOffset = NumTotalBlocks; }

private:
                        BufferAllocator     (const BufferAllocator&);   // not allowed!
    BufferAllocator&    operator=           (const BufferAllocator&);   // not allowed!

    void*               allocate            (int numBlocks, bool isTransient);
    void                release             (void* blocks, int numBlocks);
    int                 findFreeRun         (int numBlocks);
    static int          getAllocSize        (const Vector4i& rect);

    // Member variables.
    UINT32              m_blockAllocatedMask[NumBitfields];
    int                 m_nonTransientOffset;
    int                 m_transientOffset;
    UINT8               UMBRA_ATTRIBUTE_ALIGNED16(m_blocks)[NumTotalBlocks * BlockSize];
    void*               m_persistent;
};

struct UMBRA_ATTRIBUTE_ALIGNED16(AxisNormals)
{
    Vector4 x;
    Vector4 y;
    Vector4 z;
};

// TODO: separate into own translation unit
class RasterOps
{
public:
    static void clear                       (BlockRasterBuffer& buf);
    static void fillOnes                    (BlockRasterBuffer& buf);
    static void fill                        (BlockRasterBuffer& buf, int c);
    static void blit                        (BlockRasterBuffer& dst, const BlockRasterBuffer& src);
    static bool blitOr                      (BlockRasterBuffer& dst, const BlockRasterBuffer& src);
    static void expandBlit                  (BlockRasterBuffer& dst, const BlockRasterBuffer& src);
    static bool rasterizePortal             (BlockRasterBuffer& buf, const Vector4i& mnmx, const VQuad& quad, const AxisNormals& normals, const BlockRasterBuffer& mask);
    static bool rasterizeQuad               (BlockRasterBuffer& buf, const Vector4i& quad, const BlockRasterBuffer& mask);
    static bool testRectAny                 (const BlockRasterBuffer& buf, const Vector4i& mnmx);
    static bool testRectAny                 (const BlockRasterBuffer& buf, const Vector4i& mnmx, DepthBuffer* input, float z);
    static bool testRectAnyReference        (const BlockRasterBuffer& buf, const Vector4i& mnmx, DepthBuffer* input, float z);
    static void updateDepthBuffer           (const BlockRasterBuffer& buf, DepthBuffer& depthBuffer, float z);
    static void updateDepthBufferReference  (const BlockRasterBuffer& buf, DepthBuffer& depthBuffer, float z);
    static void debugStats                  (void);
};

static UMBRA_FORCE_INLINE Vector4i rectangleIntersection (const Vector4i& a, const Vector4i& b)
{
    return Vector4i(
        max2(a.i, b.i),
        max2(a.j, b.j),
        min2(a.k, b.k),
        min2(a.l, b.l));
}

static UMBRA_FORCE_INLINE Vector4i rectangleUnion (const Vector4i& a, const Vector4i& b)
{
    return Vector4i(
        min2(a.i, b.i),
        min2(a.j, b.j),
        max2(a.k, b.k),
        max2(a.l, b.l));
}

static UMBRA_FORCE_INLINE bool rectangleContains (const Vector4i& a, const Vector4i& b)
{
    return (a.i <= b.i) && (a.j <= b.j) && (a.k >= b.k) && (a.l >= b.l);
}

static UMBRA_FORCE_INLINE int rectangleArea (const Vector4i& a)
{
    return (a.k - a.i) * (a.l - a.j);
}


} // namespace Umbra

#endif
