#pragma once
#ifndef UMBRAMEMORYACCESS_HPP
#define UMBRAMEMORYACCESS_HPP

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
 * \brief   Umbra data accessors
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraBitOps.hpp"
#include <string.h>

#if UMBRA_ARCH == UMBRA_SPU
#   include <cell/dma.h>
#   define UMBRA_REMOTE_MEMORY
#endif

#define UMBRA_RESERVED_TAG 0

namespace Umbra
{

#if UMBRA_ARCH == UMBRA_SPU

typedef CellDmaListElement MemListElem;

static UMBRA_INLINE void setMemListElem (MemListElem& elem, UINTPTR remote, size_t bytes)
{
    UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
    UMBRA_ASSERT(bytes <= (16 << 10));
    UMBRA_ASSERT(!(remote & 0xf));

    elem.notify    = 0;
    elem.reserved  = 0;
    elem.size      = bytes;
    elem.eal       = remote;
}

#else

struct MemListElem
{
    MemListElem(void): addr(0), size(0) {}

    const void* addr;
    size_t size;
};

static UMBRA_INLINE void setMemListElem (MemListElem& elem, UINTPTR remote, size_t bytes)
{
    UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
    UMBRA_ASSERT(bytes <= (16 << 10));
    UMBRA_ASSERT(!(remote & 0xf));

    elem.addr = (const void*)remote;
    elem.size = bytes;
}

#endif


class MemoryAccess
{
public:
    static UINT32   tagHead             (void) { return UMBRA_RESERVED_TAG + 1; }
    static UINT32   tagTail             (void) { return 31; }

#if UMBRA_ARCH == UMBRA_SPU
public:

    static UINT32   read32              (const void* src)
    {
        UMBRA_ASSERT(!((UINTPTR)src & 3));
        return cellDmaGetUint32((UINTPTR)src, UMBRA_RESERVED_TAG, 0, 0);
    }

    static void     write32             (const void* dst, UINT32 val)
    {
        UMBRA_ASSERT(!((UINTPTR)dst & 3));
        cellDmaPutUint32(val, (UINTPTR)dst, UMBRA_RESERVED_TAG, 0, 0);
    }

    static void     unalignedRead       (void* dst, const void* src, size_t bytes)
    {
        static UINT8 alignBuf[256];
        UMBRA_ASSERT(bytes + 30 < 256);
        UINTPTR remote = (UINTPTR)src;
        size_t copyBytes = bytes + (remote & 0xF);
        copyBytes = (copyBytes + 0xF) & ~0xF;
        cellDmaGet(alignBuf, remote & ~0xF, copyBytes, UMBRA_RESERVED_TAG, 0, 0);
        cellDmaWaitTagStatusAll(1<<UMBRA_RESERVED_TAG);
        memcpy(dst, &alignBuf[remote & 0xF], bytes);
    }

    template <typename T> static void readElem (T& dst, const T* src)
    {
        UINT32 bytes = sizeof(T);
        static UINT8 alignBuf[256];
        UMBRA_ASSERT(bytes + 30 < 256);
        UINTPTR remote = (UINTPTR)src;
        size_t copyBytes = bytes + (remote & 0xF);
        copyBytes = (copyBytes + 0xF) & ~0xF;
        cellDmaGet(alignBuf, remote & ~0xF, copyBytes, UMBRA_RESERVED_TAG, 0, 0);
        cellDmaWaitTagStatusAll(1<<UMBRA_RESERVED_TAG);
        dst = *((const T*)&alignBuf[remote & 0xF]);
    }

    static void     alignedRead         (void* dst, const void* src, size_t bytes)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        cellDmaGet(dst, (UINTPTR)src, bytes, UMBRA_RESERVED_TAG, 0, 0);
        cellDmaWaitTagStatusAll(1<<UMBRA_RESERVED_TAG);
    }

    static void     alignedWrite        (void* dst, const void* src, size_t bytes)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        cellDmaPut(src, (UINTPTR)dst, bytes, UMBRA_RESERVED_TAG, 0, 0);
        cellDmaWaitTagStatusAll(1<<UMBRA_RESERVED_TAG);
    }

    static void     alignedWriteAsync   (void* dst, const void* src, size_t bytes, UINT32 tag)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        cellDmaPut(src, (UINTPTR)dst, bytes, tag, 0, 0);
    }

    static void     alignedLargeWrite   (void* dst, const void* src, size_t bytes)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        cellDmaLargePut(src, (UINTPTR)dst, bytes, UMBRA_RESERVED_TAG, 0, 0);
        cellDmaWaitTagStatusAll(1<<UMBRA_RESERVED_TAG);
    }

    static void     alignedReadAsync    (void* dst, const void* src, size_t bytes, UINT32 tag)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        UMBRA_ASSERT(1 <= tag && tag <= 31);
        cellDmaGet(dst, (UINTPTR)src, bytes, tag, 0, 0);
    }

    static void     alignedReadIndexedAsync (void* dst, MemListElem* list, int numElements, UINT32 tag)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)list  & 0xf));
        UMBRA_ASSERT((numElements * sizeof(MemListElem) < (16 << 10)));
        UMBRA_ASSERT(1 <= tag && tag <= 31);
        cellDmaListGet(dst, 0, list, numElements * sizeof(MemListElem), tag, 0, 0);
    }

    static void     wait                (UINT32 tag)
    {
        UMBRA_ASSERT(1 <= tag && tag <= 31);
        cellDmaWaitTagStatusAll(1<<tag);
    }

    static bool     isRemoteAddress     (const void* addr)
    {
        return ((UINTPTR)addr > 0x40000);
    }

#else

public:

    static UINT32   read32              (const void* src)
    {
        UMBRA_ASSERT(!((UINTPTR)src & 3));
        return *((const UINT32*)src);
    }

    static void     write32             (void* dst, UINT32 val)
    {
        UMBRA_ASSERT(!((UINTPTR)dst & 3));
        *((UINT32*)dst) = val;
    }

    static void     unalignedRead       (void* dst, const void* src, size_t bytes)
    {
        memcpy(dst, src, bytes);
    }

    template <typename T> static void readElem (T& dst, const T* src)
    {
        dst = *src;
    }

    static void     alignedRead         (void* dst, const void* src, size_t bytes)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        memcpy(dst, src, bytes);
    }

    static void     alignedWrite        (void* dst, const void* src, size_t bytes)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        memcpy(dst, src, bytes);
    }

    static void     alignedLargeWrite   (void* dst, const void* src, size_t bytes)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        memcpy(dst, src, bytes);
    }

    static void     alignedReadAsync    (void* dst, const void* src, size_t bytes, UINT32 tag)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        UMBRA_ASSERT(1 <= tag && tag <= 31);
        UMBRA_UNREF(tag);
        memcpy(dst, src, bytes);
    }

    static void     alignedWriteAsync   (void* dst, const void* src, size_t bytes, UINT32 tag)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)src   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)bytes & 0xf));
        UMBRA_ASSERT(bytes <= (16 << 10));
        UMBRA_ASSERT(1 <= tag && tag <= 31);
        UMBRA_UNREF(tag);
        memcpy(dst, src, bytes);
    }

    static void     alignedReadIndexedAsync (void* dst, MemListElem* list, int numElements, UINT32 tag)
    {
        UMBRA_ASSERT(!((UINTPTR)dst   & 0xf));
        UMBRA_ASSERT(!((UINTPTR)list  & 0xf));
        UMBRA_ASSERT(1 <= tag && tag <= 31);
        UMBRA_ASSERT((numElements * sizeof(MemListElem) < (16 << 10)));
        UMBRA_UNREF(tag);

        UINT8* ptr = (UINT8*)dst;
        for (int i = 0; i < numElements; i++)
        {
            memcpy(ptr, list[i].addr, list[i].size);
            ptr += list[i].size;
        }
    }

    static void     wait                (UINT32 tag)
    {
        UMBRA_ASSERT(1 <= tag && tag <= 31);
        UMBRA_UNREF(tag);
    }

    static bool     isRemoteAddress     (const void*)
    {
#ifdef UMBRA_REMOTE_MEMORY
        // for remote memory testing
        return true;
#else
        return false;
#endif
    }

#endif
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Data pointer as offset from parent struct
 *//*-------------------------------------------------------------------*/

class DataPtr
{
public:
    DataPtr(void): m_offset(0) {}
    DataPtr(UINT32 ofs): m_offset(ofs) {}
    DataPtr(void* base, void* alloc) { m_offset = alloc ? (UINT32)((size_t)alloc - (size_t)base) : 0; }

    bool         operator!      (void) const              { return m_offset == 0; }
    UINT32       getOffset      (void) const              { return m_offset; }
    void*        getAddr        (void* base) const        { return !m_offset ? NULL : (UINT8*)base + m_offset; }
    const void*  getAddr        (const void* base) const  { return !m_offset ? NULL : (const UINT8*)base + m_offset; }
    const void*  getAddrNoCheck (const void* base) const  { UMBRA_ASSERT(m_offset); return (const UINT8*)base + m_offset; }
private:
    UINT32      m_offset;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Wrapper for holding data array info
 *//*-------------------------------------------------------------------*/

class DataArray
{
public:
    DataArray(void): m_base(NULL), m_ofs(), m_elemSize(0), m_count(0) {}
    DataArray(const void* base, DataPtr ofs, int elemSize, int count):
      m_base(base), m_ofs(ofs), m_elemSize(elemSize), m_count(count) {}

    bool operator! (void) const { return !m_ofs; }

    template <class T> void getElem (T& elem, int idx) const
    {
        UMBRA_ASSERT(m_base && !!m_ofs);
        UMBRA_ASSERT(sizeof(T) == m_elemSize);
        UMBRA_ASSERT(idx >= 0 && (idx < m_count || m_count == -1));
        const void* addr = m_ofs.getAddrNoCheck(m_base);
        MemoryAccess::readElem(elem, (const T*)addr + idx);
    }

    template <class T> void getElems (T* elem, int idx, int num) const
    {
        UMBRA_ASSERT(m_base && !!m_ofs);
        UMBRA_ASSERT(sizeof(T) == m_elemSize);
        UMBRA_ASSERT(idx >= 0 && (idx + num <= m_count || m_count == -1));
        const void* addr = m_ofs.getAddrNoCheck(m_base);
        MemoryAccess::unalignedRead(elem, (const T*)addr + idx, sizeof(T) * num);
    }

    void getElem (int& elem, int idx) const
    {
        UMBRA_ASSERT(m_base && !!m_ofs);
        UMBRA_ASSERT(sizeof(int) == m_elemSize);
        UMBRA_ASSERT(idx >= 0 && (idx < m_count || m_count == -1));
        const void* addr = m_ofs.getAddrNoCheck(m_base);
        elem = MemoryAccess::read32((const int*)addr + idx);
    }

    int getSizeInBytes() const
    {
        return (!!*this) ? m_count * m_elemSize : 0;
    }

    int getCount (void) const
    {
        return m_count;
    }

    DataArray slice (int start, int count) const
    {
        UMBRA_ASSERT(m_count == -1 || start + count <= m_count);
        return DataArray(m_base, DataPtr(m_ofs.getOffset() + (start * m_elemSize)),
            m_elemSize, count);
    }

    const void* m_base;
    DataPtr     m_ofs;
    int         m_elemSize;
    int         m_count;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class BitDataArray
{
public:
    BitDataArray(void) : m_array(), m_bitOffset(0) {}
    BitDataArray(const DataArray& arr, int bitOffset, int bitLen = -1)
        : m_array(arr), m_bitOffset(bitOffset), m_bitLen(bitLen)
    {
        UMBRA_ASSERT(arr.m_elemSize == sizeof(UINT32));
    }

    bool operator! (void) const { return !m_array; }

    UINT32 getElem (int bitIdx, int width) const
    {
        UMBRA_ASSERT(width < 32);
        UINT32 data[2];
        int idx = m_bitOffset + bitIdx;
        UMBRA_ASSERT(m_bitLen == -1 || idx + width < m_bitLen);
        m_array.getElems(data, UMBRA_BIT_DWORD(idx), 2);
        return unpackElem(data, UMBRA_BIT_IDX(idx), width);
    }

    UINT32 getElem32 (int bitIdx) const
    {
        UINT32 data[2];
        int idx = m_bitOffset + bitIdx;
        UMBRA_ASSERT(m_bitLen == -1 || idx + 32 < m_bitLen);
        m_array.getElems(data, UMBRA_BIT_DWORD(idx), 2);
        return unpackElem32(data, UMBRA_BIT_IDX(idx));
    }

    DataArray   m_array;
    UINT32      m_bitOffset;
    int         m_bitLen;
};

}   // namespace Umbra

#endif // UMBRAMEMORYACCESS_HPP
