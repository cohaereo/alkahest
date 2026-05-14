// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef __UMBRADEPTHBUFFER_H
#define __UMBRADEPTHBUFFER_H

#include "umbraQueryContext.hpp"
#include "umbraRasterDefs.hpp"

#ifdef UMBRA_REMOTE_MEMORY
#   define SUPPORT_REMOTE_DEPTHBUFFER 1
#else
#   define SUPPORT_REMOTE_DEPTHBUFFER 0
#endif

namespace Umbra {

/*-------------------------------------------------------------------*//*!
 * \brief   Occlusion depth buffer implementation.
 *
 * Used for manipulating DepthBuffer bits stored in ImpOcclusionBuffer
 * instance.
 *
 * Supports remote depth buffer memory (for SPU) when
 * SUPPORT_REMOTE_DEPTHBUFFER is set.
 *//*-------------------------------------------------------------------*/

// Currently only 8 vec4 ops implemented, easy to extend
#define UPDATE_FOREACH_VEC4(op, addr) \
    UMBRA_CT_ASSERT(Vec4PerBlock * N == 8); \
    SIMDRegister* p = (addr); \
    p[0] = op.exec(p[0], 0); \
    p[1] = op.exec(p[1], 1); \
    p[2] = op.exec(p[2], 2); \
    p[3] = op.exec(p[3], 3); \
    p[4] = op.exec(p[4], 4); \
    p[5] = op.exec(p[5], 5); \
    p[6] = op.exec(p[6], 6); \
    p[7] = op.exec(p[7], 7);

class DepthBuffer
{
public:

    enum
    {
        BytesPerPixel       = 4,
        PixelsPerBlock      = (UMBRA_RASTER_BLOCK_X * UMBRA_RASTER_BLOCK_Y),
        Vec4PerBlock        = PixelsPerBlock / 4,
        BlockStride         = (UMBRA_PORTAL_RASTER_SIZE / UMBRA_RASTER_BLOCK_X) * PixelsPerBlock,
        NumRowCaches        = 2,
        BlockRowBytes       = BlockStride * BytesPerPixel
    };

    template<int N>
    class BlockSpan
    {
    private:
        static UMBRA_INLINE SIMDRegister32 getPixelMask4 (int iter)
        {
#if UMBRA_OS == UMBRA_IOS || UMBRA_OS == UMBRA_METRO || defined(__QNXNTO__) || defined(__clang__)
            // ios (and metro) compiler fails constant propagation here
            SIMDRegister32 base = SIMDLoad32(1, 2, 4, 8);
            switch (iter)
            {
            case 0: return base;
            case 1: return SIMDLeftShift32(base, 4);
            case 2: return SIMDLeftShift32(base, 8);
            case 3: return SIMDLeftShift32(base, 12);
            case 4: return SIMDLeftShift32(base, 16);
            case 5: return SIMDLeftShift32(base, 20);
            case 6: return SIMDLeftShift32(base, 24);
            case 7: return SIMDLeftShift32(base, 28);
            default:
	            UMBRA_ASSERT(!"invalid iteration count");
                return base;
            }
#else
            if ((UMBRA_BYTE_ORDER == UMBRA_BIG_ENDIAN) && (N == 2))
            {
                // the 16-bit blocks are in reverse order on big endian
                return SIMDLeftShift32(SIMDLoad32(1, 2, 4, 8), ((iter + 4) & 0x7) * 4);
            }
            else
            {
                return SIMDLeftShift32(SIMDLoad32(1, 2, 4, 8), iter * 4);
            }
#endif
        }

    public:
        BlockSpan(SIMDRegister* addr): addr(addr)
        {}

        UMBRA_INLINE void fill (SIMDRegister z) const
        {
            struct Op
            {
                UMBRA_INLINE Op(SIMDRegister z): z(z) {}

                UMBRA_INLINE SIMDRegister exec(SIMDRegister, int) const
                {
                    return z;
                }

                SIMDRegister z;
            };

            UPDATE_FOREACH_VEC4(Op(z), addr);
        }

        UMBRA_INLINE void max2 (SIMDRegister z) const
        {
            struct Op
            {
                UMBRA_INLINE Op(SIMDRegister z): z(z) {}

                UMBRA_INLINE SIMDRegister exec(SIMDRegister in, int) const
                {
                    return SIMDMax(z, in);
                }

                SIMDRegister z;
            };

            UPDATE_FOREACH_VEC4(Op(z), addr);
        }

        // masked max op for 32 pixels at a time, mask duplicated across all lanes
        UMBRA_INLINE void bitmaskMax32 (SIMDRegister z, SIMDRegister32 mask) const
        {
            UMBRA_CT_ASSERT(PixelsPerBlock * N == 32);

            struct Op
            {
                UMBRA_INLINE Op(SIMDRegister z, SIMDRegister32 mask): z(z), mask(mask) {}

                UMBRA_INLINE SIMDRegister exec(SIMDRegister in, int n) const
                {
                    SIMDRegister32 pixMask = BlockSpan::getPixelMask4(n);
                    SIMDRegister32 curMaskNeg = SIMDCompareEQ32(SIMDBitwiseAnd32(mask, pixMask), SIMDZero32());
                    return SIMDMax(in, SIMDBitwiseAndNot(z, SIMDBitPatternToFloat(curMaskNeg)));
                }

                SIMDRegister z;
                SIMDRegister32 mask;
            };

            UPDATE_FOREACH_VEC4(Op(z, mask), addr);
        }

        // depth buffer test for 16 pixels at a time
        UMBRA_INLINE UINT32 test16(SIMDRegister z) const
        {
            UMBRA_CT_ASSERT(PixelsPerBlock * N == 16);

            const SIMDRegister* ptr = addr;
            SIMDRegister r1 = SIMDCompareGE(ptr[0], z);
            SIMDRegister r2 = SIMDCompareGE(ptr[1], z);
            SIMDRegister r3 = SIMDCompareGE(ptr[2], z);
            SIMDRegister r4 = SIMDCompareGE(ptr[3], z);

#if UMBRA_OS == UMBRA_XBOX360
            r1 = __vand(r1, BlockSpan::getPixelMask4(0));
            r2 = __vand(r2, BlockSpan::getPixelMask4(1));
            r3 = __vand(r3, BlockSpan::getPixelMask4(2));
            r4 = __vand(r4, BlockSpan::getPixelMask4(3));
            SIMDRegister comb = __vor(__vor(r1, r2), __vor(r3, r4));
            comb = __vor(comb, __vrlimi(SIMDZero(), comb, 0xF, 1));
            comb = __vor(comb, __vrlimi(SIMDZero(), comb, 0xF, 2));
            UINT32 res;
            __stvewx(comb, &res, 0);
            return res;
#else
            // TODO: doesn't need to be sign bit: extract any bit would do
            UINT32 res = (SIMDExtractSignBits(r1) << 0) |
                         (SIMDExtractSignBits(r2) << 4) |
                         (SIMDExtractSignBits(r3) << 8) |
                         (SIMDExtractSignBits(r4) << 12);
#if UMBRA_OS == UMBRA_PS3
            // extracting sign bits on ps3 happens in reverse channel order
            res = ((res >> 1) & 0x55555555) | ((res & 0x55555555) << 1);
            res = ((res >> 2) & 0x33333333) | ((res & 0x33333333) << 2);
#endif
            return res;
#endif
        }

        // find value greater-or-equal than input z for 16 pixels at a time with mask
        UMBRA_INLINE SIMDRegister bitmaskTestAny16(SIMDRegister z, SIMDRegister32 mask) const
        {
            UMBRA_CT_ASSERT(PixelsPerBlock * N == 16);

            struct Op
            {
                UMBRA_INLINE Op(SIMDRegister z, SIMDRegister32 mask): z(z), mask(mask) {}

                UMBRA_INLINE SIMDRegister exec(SIMDRegister in, int n) const
                {
                    SIMDRegister32 pixMask = BlockSpan::getPixelMask4(n);
                    SIMDRegister32 curMaskNeg = SIMDCompareEQ32(SIMDBitwiseAnd32(mask, pixMask), SIMDZero32());
                    SIMDRegister r = SIMDCompareGE(in, z);
                    return SIMDBitwiseAndNot(r, SIMDBitPatternToFloat(curMaskNeg));
                }

                SIMDRegister z;
                SIMDRegister32 mask;
            };

            const Op op(z, mask);
            const SIMDRegister* ptr = addr;
            SIMDRegister ret = op.exec(ptr[0], 0);
            ret = SIMDBitwiseOr(ret, op.exec(ptr[1], 1));
            ret = SIMDBitwiseOr(ret, op.exec(ptr[2], 2));
            return SIMDBitwiseOr(ret, op.exec(ptr[3], 3));
        }

        // find value smaller than input z for 16 pixels at a time with mask
        UMBRA_INLINE SIMDRegister bitmaskTestAll16(SIMDRegister z, SIMDRegister32 mask) const
        {
            UMBRA_CT_ASSERT(PixelsPerBlock * N == 16);

            struct Op
            {
                UMBRA_INLINE Op(SIMDRegister z, SIMDRegister32 mask): z(z), mask(mask) {}

                UMBRA_INLINE SIMDRegister exec(SIMDRegister in, int n) const
                {
                    SIMDRegister32 pixMask = BlockSpan::getPixelMask4(n);
                    SIMDRegister32 curMaskNeg = SIMDCompareEQ32(SIMDBitwiseAnd32(mask, pixMask), SIMDZero32());
                    SIMDRegister r = SIMDCompareGT(z, in);
                    return SIMDBitwiseAndNot(r, SIMDBitPatternToFloat(curMaskNeg));
                }

                SIMDRegister z;
                SIMDRegister32 mask;
            };

            const Op op(z, mask);
            const SIMDRegister* ptr = addr;
            SIMDRegister ret = op.exec(ptr[0], 0);
            ret = SIMDBitwiseOr(ret, op.exec(ptr[1], 1));
            ret = SIMDBitwiseOr(ret, op.exec(ptr[2], 2));
            return SIMDBitwiseOr(ret, op.exec(ptr[3], 3));
        }


        UMBRA_INLINE void combineMin (const BlockSpan<N>& other) const
        {
            SIMDRegister* dst = addr;
            SIMDRegister* src = other.addr;

            UMBRA_CT_ASSERT(PixelsPerBlock * N == 32);

            dst[0] = SIMDMin(dst[0], src[0]);
            dst[1] = SIMDMin(dst[1], src[1]);
            dst[2] = SIMDMin(dst[2], src[2]);
            dst[3] = SIMDMin(dst[3], src[3]);
            dst[4] = SIMDMin(dst[4], src[4]);
            dst[5] = SIMDMin(dst[5], src[5]);
            dst[6] = SIMDMin(dst[6], src[6]);
            dst[7] = SIMDMin(dst[7], src[7]);
        }

        UMBRA_INLINE void combineMax (const BlockSpan<N>& other) const
        {
            SIMDRegister* dst = addr;
            SIMDRegister* src = other.addr;

            UMBRA_CT_ASSERT(PixelsPerBlock * N == 32);

            dst[0] = SIMDMax(dst[0], src[0]);
            dst[1] = SIMDMax(dst[1], src[1]);
            dst[2] = SIMDMax(dst[2], src[2]);
            dst[3] = SIMDMax(dst[3], src[3]);
            dst[4] = SIMDMax(dst[4], src[4]);
            dst[5] = SIMDMax(dst[5], src[5]);
            dst[6] = SIMDMax(dst[6], src[6]);
            dst[7] = SIMDMax(dst[7], src[7]);
        }

        float* getPtr (void) const
        {
            return (float*)addr;
        }

    private:
        SIMDRegister* addr;
    };

    DepthBuffer(QueryContext* query):
        m_query(query),
        m_buffer(NULL),
        m_maxZ(1.f)
    {
        m_rowCache[0].local = NULL;

        if (supportsRemoteBuffer())
        {
            size_t rowBytes = BlockStride * BytesPerPixel;
            UINT8* caches = (UINT8*)UMBRA_HEAP_ALLOC(query->getAllocator(), rowBytes*NumRowCaches);
            for (int i = 0; i < NumRowCaches; i++)
            {
                m_rowCache[i].syncTag.init(query->getTagManager());
                m_rowCache[i].local = caches;
                caches += rowBytes;
            }
        }
    }

    ~DepthBuffer (void)
    {
        if (supportsRemoteBuffer())
        {
            UMBRA_HEAP_FREE(m_query->getAllocator(), m_rowCache[0].local);
            for (int i = NumRowCaches - 1; i >= 0; i--)
            {
                MemoryAccess::wait(m_rowCache[i].syncTag.getValue());
                m_rowCache[i].syncTag.deinit();
            }
        }
    }

    void setBuffer (void* buf)
    {
        UMBRA_ASSERT(supportsRemoteBuffer() || !MemoryAccess::isRemoteAddress(buf));
        m_buffer = buf;
        m_readOnly = false;
    }

    void setBuffer (const void* buf)
    {
        UMBRA_ASSERT(supportsRemoteBuffer() || !MemoryAccess::isRemoteAddress(buf));
        m_buffer = (void*)buf;
        m_readOnly = true;
        // TODO: enforce read only usage
    }

    void* getBuffer (void) const
    {
        UMBRA_ASSERT(!isRemote());
        return m_buffer;
    }

    void clear (void)
    {
        UMBRA_ASSERT(m_buffer);
        int blockRows = UMBRA_PORTAL_RASTER_SIZE / UMBRA_RASTER_BLOCK_Y;
        for (BlockRowIterator<false, true> iter = iterateBlockRows<false, true>(0, blockRows); !iter.end(); iter.next())
        {
            memset(iter.blockRow(), 0x0, BlockRowBytes);
        }
        m_maxZ = 0.f;
    }

    bool compare (DepthBuffer& other)
    {
        UMBRA_ASSERT(m_buffer);
        int blockRows = UMBRA_PORTAL_RASTER_SIZE / UMBRA_RASTER_BLOCK_Y;
        BlockRowIterator<true, false> iter1 = iterateBlockRows<true, false>(0, blockRows);
        BlockRowIterator<true, false> iter2 = other.iterateBlockRows<true, false>(0, blockRows);
        while (!iter1.end())
        {
            if (memcmp(iter1.blockRow(), iter2.blockRow(), BlockRowBytes) != 0)
                return false;
            iter1.next();
            iter2.next();
        }
        return true;
    }

    UMBRA_INLINE float updateMaxZ (float z)
    {
        float prev = m_maxZ; m_maxZ = max2(m_maxZ, z); return prev;
    }

private:

    struct CachedBlockRow
    {
        void* local;
        Tag syncTag;
    };

    template<bool READ, bool WRITE, bool SUPPORT_REMOTE = true>
    class BlockRowIterator
    {
    public:
        UMBRA_INLINE void next (void)
        {
            if (isRemote())
            {
                UMBRA_ASSERT(m_curCacheIndex >= 0);
                if (WRITE)
                {
                    // write out current cached row
                    CachedBlockRow& writeCache = m_buf->m_rowCache[m_curCacheIndex];
                    MemoryAccess::alignedWriteAsync((void*)m_curRemoteAddr, writeCache.local, BlockRowBytes, writeCache.syncTag.getValue());
                }
                // flip caches and advance
                m_curCacheIndex = (m_curCacheIndex + 1) % NumRowCaches;
                m_curRemoteAddr += BlockRowBytes;
                if (READ)
                {
                    // issue read for next block row
                    CachedBlockRow& readCache = m_buf->m_rowCache[(m_curCacheIndex + 1) % NumRowCaches];
                    MemoryAccess::wait(readCache.syncTag.getValue());
                    MemoryAccess::alignedReadAsync(readCache.local, (const void*)(m_curRemoteAddr + BlockRowBytes), BlockRowBytes, readCache.syncTag.getValue());
                }
            }
            else
            {
                m_curRemoteAddr += BlockRowBytes;
            }
        }

        UMBRA_INLINE void* blockRow (void) const
        {
            if (isRemote())
            {
                CachedBlockRow& cur = m_buf->m_rowCache[m_curCacheIndex];
                MemoryAccess::wait(cur.syncTag.getValue());
                return cur.local;
            }
            else
            {
                return (void*)m_curRemoteAddr;
            }
        }

        UMBRA_INLINE bool end (void) const
        {
            return m_curRemoteAddr >= m_endRemoteAddr;
        }

        UMBRA_INLINE BlockRowIterator(void) {}

        UMBRA_INLINE BlockRowIterator(const BlockRowIterator& o) { *this = o; }

        UMBRA_INLINE BlockRowIterator(DepthBuffer* buf, int start, int len): m_buf(buf)
        {
            m_curCacheIndex = 0;
            m_curRemoteAddr = (UINTPTR)m_buf->m_buffer + start * BlockRowBytes;
            m_endRemoteAddr = m_curRemoteAddr + len * BlockRowBytes;
            if (READ && isRemote())
            {
                CachedBlockRow& read0 = m_buf->m_rowCache[0];
                CachedBlockRow& read1 = m_buf->m_rowCache[1];
                MemoryAccess::alignedReadAsync(read0.local, (const void*)m_curRemoteAddr, BlockRowBytes, read0.syncTag.getValue());
                MemoryAccess::alignedReadAsync(read1.local, (const void*)(m_curRemoteAddr + BlockRowBytes), BlockRowBytes, read1.syncTag.getValue());
            }
        }

        UMBRA_INLINE BlockRowIterator& operator= (const BlockRowIterator& o)
        {
            m_buf = o.m_buf;
            m_curCacheIndex = o.m_curCacheIndex;
            m_curRemoteAddr = o.m_curRemoteAddr;
            m_endRemoteAddr = o.m_endRemoteAddr;
            return *this;
        }

    private:

		UMBRA_INLINE bool isRemote (void) const
		{
			return SUPPORT_REMOTE && m_buf->isRemote();
		}

        DepthBuffer* m_buf;
        int m_curCacheIndex;
        UINTPTR m_curRemoteAddr;
        UINTPTR m_endRemoteAddr;

        friend class DepthBuffer;
    };

public:

    // Iterates over blocks in N block intervals
    template<int N, bool READ, bool WRITE, bool SUPPORT_REMOTE = true>
    class BlockIterator
    {
    public:
        UMBRA_INLINE void next (void)
        {
            UMBRA_ASSERT(!end());
            if (!m_leftInRow)
            {
                m_rowIter.next();
                if (!m_rowIter.end())
                {
                    m_cur = (UINTPTR)m_rowIter.blockRow() + m_rowOfs;
                    m_leftInRow = m_rowLenMinusOne;
                }
            }
            else
            {
                m_cur += PixelsPerBlock * BytesPerPixel * N;
                m_leftInRow--;
            }
        }

        UMBRA_INLINE void skip (int n)
        {
            // TODO: optimize
            while (n)
            {
                next();
                n -= N;
            }
        }

       UMBRA_INLINE const BlockSpan<N> blocks (void) const
        {
            return BlockSpan<N>((SIMDRegister*)m_cur);
        }

        UMBRA_INLINE bool end (void) const
        {
            return !m_leftInRow && m_rowIter.end();
        }

        UMBRA_INLINE BlockIterator(const BlockIterator& o) { *this = o; }

        UMBRA_INLINE BlockIterator& operator= (const BlockIterator& o)
        {
            m_rowIter = o.m_rowIter;
            m_rowOfs = o.m_rowOfs;
            m_rowLenMinusOne = o.m_rowLenMinusOne;
            m_cur = o.m_cur;
            m_leftInRow = o.m_leftInRow;
            return *this;
        }

        UMBRA_INLINE int leftInRow(void) const
        {
            return m_leftInRow;
        }

    private:

        UMBRA_INLINE BlockIterator(void) {}

        UMBRA_INLINE BlockIterator(DepthBuffer* buf, const Vector4i& blockRect)
        {
            UMBRA_ASSERT(blockRect.i % N == 0);
            UMBRA_ASSERT(blockRect.k % N == 0);
            UMBRA_ASSERT(blockRect.k > blockRect.i && blockRect.l > blockRect.j);
            m_rowIter = BlockRowIterator<READ, WRITE, SUPPORT_REMOTE>(buf, blockRect.j, blockRect.l - blockRect.j);
            m_rowOfs = blockRect.i * PixelsPerBlock * BytesPerPixel;
            m_rowLenMinusOne = (blockRect.k - blockRect.i) / N - 1;
            m_leftInRow = m_rowLenMinusOne;
            m_cur = (UINTPTR)m_rowIter.blockRow() + m_rowOfs;
        }

        BlockRowIterator<READ, WRITE, SUPPORT_REMOTE> m_rowIter;
        int m_rowOfs;
        int m_rowLenMinusOne;
        UINTPTR m_cur;
        int m_leftInRow;

        friend class DepthBuffer;
    };

    template<int N, bool READ, bool WRITE>
    UMBRA_INLINE BlockIterator<N, READ, WRITE> iterateBlocks(const Vector4i& blockRect)
    {
        return BlockIterator<N, READ, WRITE>(this, blockRect);
    }

    template<int N, bool READ, bool WRITE>
    UMBRA_INLINE BlockIterator<N, READ, WRITE, false> iterateBlocksLocal(const Vector4i& blockRect)
    {
        return BlockIterator<N, READ, WRITE, false>(this, blockRect);
    }

private:

    template<bool READ, bool WRITE>
    UMBRA_INLINE BlockRowIterator<READ, WRITE> iterateBlockRows(int start, int len)
    {
        return BlockRowIterator<READ, WRITE>(this, start, len);
    }

    UMBRA_INLINE bool supportsRemoteBuffer (void) const
    {
        return SUPPORT_REMOTE_DEPTHBUFFER && (m_query != NULL);
    }

    UMBRA_INLINE bool isRemote (void) const
    {
        return supportsRemoteBuffer() && MemoryAccess::isRemoteAddress(m_buffer);
    }

    QueryContext*   m_query;
    void*           m_buffer;
    float           m_maxZ;
    bool            m_readOnly;
    CachedBlockRow  m_rowCache[NumRowCaches];
};


}

#endif
