// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef __UMBRADEPTHMAP_H
#define __UMBRADEPTHMAP_H

#include "umbraPrivateDefs.hpp"
#include "umbraMemoryAccess.hpp"
#include "umbraTomePrivate.hpp"

namespace Umbra 
{

// Absolute value helper
static UMBRA_INLINE float absolute(float f)
{
#if UMBRA_ARCH == UMBRA_ARM
    return bitPatternFloat(floatBitPattern(f) & (~(0x80000000)));
#else
    return fabsf(f);
#endif
}

// Absolute value with bit pattern conversion
static UMBRA_INLINE UINT32 absolutef2i(float f)
{
    return floatBitPattern(f) & (~(0x80000000));
}

// Generic object for depthmap tests
class DepthmapReader
{
public:
    UMBRA_INLINE DepthmapReader(const ImpTome* tome)
    {
        init(tome);
    }

    UMBRA_INLINE DepthmapReader()
        : m_data(NULL)
        , m_faces(NULL)
        , m_palettes(NULL)
        , m_numFaces(0)
    {
    }

    UMBRA_INLINE void init(const ImpTome* tome)
    {
        m_data     = (const DepthmapData*)tome->getObjectDepthmaps().getAddr(tome->getBase());
        m_faces    = (const UINT32*)tome->getDepthmapFaces().getAddr(tome->getBase());
        m_palettes = (const UINT16*)tome->getDepthmapPalettes().getAddr(tome->getBase());
        m_numFaces = tome->getNumFaces();
    }

    // Lookup by face coordinate
    UMBRA_INLINE        float    lookup                 (const DepthmapData& data, const Vector3i& i) const;
    // Lookup by face coordinate
    UMBRA_INLINE        int      lookupInfinite         (const DepthmapData& data, const Vector3i& i) const;
    // Lookup by direction
    UMBRA_INLINE        float    lookup                 (const DepthmapData& data, const Vector3& v)  const { return lookup(data, DepthmapReader::map(v)); }
    // Lookup by SIMD register
    UMBRA_INLINE        float    lookup                 (const DepthmapData& data, const SIMDRegister& v)  const { return lookup(data, DepthmapReader::map(v)); }

    // Test if position is inside the depthmap
    UMBRA_INLINE        bool     testPosition           (int objectIdx, const Vector3& pos, float offset = 0.f) const;
    // Test if depthmap is visible to direction
    UMBRA_INLINE        bool     testDirection          (int objectIdx, const Vector3& dir, const Vector4& near) const;
    // Test if depthmap is visible to direction
    UMBRA_INLINE        bool     testDirection          (int objectIdx, const Vector3i& mapped, const Vector4& near) const;
    // Test if depthmap is visible to direction
    UMBRA_INLINE        bool     testDirectionInfinite  (int objectIdx, const Vector3& dir) const;
    // Test if depthmap is visible to direction
    UMBRA_INLINE        bool     testDirectionInfinite  (int objectIdx, const Vector3i& mapped) const;
    // Visualize depthmap
                        void     visualize              (const ImpTome* tome, class QueryContext* query, int objectIdx) const;
    
    // integer triple Vector3i -> (pixel x, pixel y, face number)

    // Map face coordinate to index
    static UMBRA_INLINE int      getDwordIdx            (const Vector3i& i) { return UMBRA_BIT_DWORD((i.j * DepthmapData::Resolution + i.i) * DepthmapData::DepthBits); }
    // Map coordinate to shuffled bit index
    static UMBRA_INLINE int      getBitIdx              (int faceIdx, int totalFaces, const Vector3i& i) { return (((i.j * DepthmapData::Resolution + i.i) * totalFaces + faceIdx) * DepthmapData::DepthBits); }
    // Map normalized direction to face coordinate
    static UMBRA_INLINE Vector3i map                    (const Vector3& v);
    // SIMD variant
    static UMBRA_INLINE Vector3i map                    (const SIMDRegister& v);
    // Map normalized direction to face coordinate of given face
    static UMBRA_INLINE Vector3i map                    (const Vector3& v, int face);
    // Map face coordinate to normalized direction
    static UMBRA_INLINE Vector3  map                    (const Vector3i& i);

private:
    friend class DepthmapReaderDirectional;
    
    const DepthmapData*                  m_data;
    const UINT32*                        m_faces;
    const UINT16*                        m_palettes;
    int                                  m_numFaces;
};

// Optimized variant for shadows or orthographic queries.
// Looks up constant direction / pixel.
class DepthmapReaderDirectional
{
public:
    UMBRA_INLINE DepthmapReaderDirectional(const Vector3& dir)
        : m_base(0), m_shadowMapData(0)
    {
        m_mapped = DepthmapReader::map(dir);
    }

    UMBRA_INLINE DepthmapReaderDirectional(const Vector3i& mapped)
        : m_base(0), m_shadowMapData(0)
    {
        m_mapped = mapped;
    }

    UMBRA_INLINE void init(const ImpTome* tome)
    {
        bool hasShadowmaps = tome->hasObjectShadowmaps();
        UMBRA_ASSERT(tome->hasObjectDepthmaps() || hasShadowmaps);
        
        if (hasShadowmaps)
        {
            const int res2 = DepthmapData::Resolution * DepthmapData::Resolution;
            m_base = m_mapped.k * tome->getNumObjects() * res2 + (m_mapped.i + m_mapped.j * DepthmapData::Resolution) * tome->getNumObjects();
            m_shadowMapData = (const UINT32*)tome->getDepthmapFaces().getAddrNoCheck(tome->getBase());
        } else
        {
            m_reader.init(tome);
            m_base = DepthmapReader::getBitIdx(0, m_reader.m_numFaces, m_mapped);
        }
    }

    UMBRA_INLINE bool test(int objectIdx)
    {
        if (m_shadowMapData)
        {
            int bitIdx = m_base + objectIdx;
#if defined(UMBRA_REMOTE_MEMORY)
            UINT32 dword;
            MemoryAccess::readElem(dword, &m_shadowMapData[UMBRA_BIT_DWORD(bitIdx)]);
            return !!(dword & UMBRA_BIT_MASK(bitIdx));
#else
            return testBit(m_shadowMapData, bitIdx);
#endif
        } else
        {
#if defined(UMBRA_REMOTE_MEMORY)
            const DepthmapData& remoteData = m_reader.m_data[objectIdx];
            DepthmapData data;
            MemoryAccess::alignedRead(&data, &remoteData, sizeof(DepthmapData));

            const UINT16* palette = m_reader.m_palettes + data.faces[m_mapped.k].paletteOffset;
            int   bitIdx = m_base + data.faces[m_mapped.k].faceIdx * DepthmapData::DepthBits;

            UINT32 dword;
            MemoryAccess::readElem(dword, &m_reader.m_faces[UMBRA_BIT_DWORD(bitIdx)]);
            UINT8 nibble = (UINT8)((dword >> (bitIdx & 0x1f)) & 0xf);
            UINT16 paletteEntry;
            MemoryAccess::readElem(paletteEntry, &palette[nibble^1]);
            return paletteEntry == pattern;
#else
            const DepthmapData& data                   = m_reader.m_data[objectIdx];
            const UINT16* palette                      = m_reader.m_palettes + data.faces[m_mapped.k].paletteOffset;
            int bitIdx                                 = m_base + data.faces[m_mapped.k].faceIdx * DepthmapData::DepthBits;
            UINT8 nibble                               = (UINT8)((m_reader.m_faces[UMBRA_BIT_DWORD(bitIdx)] >> (bitIdx & 0x1f)) & 0xf);
#if UMBRA_BYTE_ORDER == UMBRA_LITTLE_ENDIAN
            return palette[nibble] == pattern;
#else
            return palette[nibble^1] == pattern;
#endif
#endif // UMBRA_REMOTE_MEMORY
        }
    }

private:
    static const UINT32 pattern = 0x7F7F; // floatBitPattern(FLT_MAX) >> 16;
    DepthmapReader m_reader;
    Vector3i       m_mapped;
    int            m_base;
    const UINT32*  m_shadowMapData;
    int            m_numObjects;
   
};

// Depthmap coordinate -> relative world space coordinate
Vector3 DepthmapReader::map(const Vector3i& i)
{
    int axis = getFaceAxis(i.k);
    
    int axisX = (1 << axis)  & 3;
    int axisY = (1 << axisX) & 3;

    Vector3 v;
    v[axis]  = (float)getFaceDirectionSign(i.k);
    v[axisX] = ((float)i.i / (float)DepthmapData::Resolution) * 2.f - 1.f;
    v[axisY] = ((float)i.j / (float)DepthmapData::Resolution) * 2.f - 1.f;

    return v;
}

// relative world space coordinate -> depthmap coodinate
Vector3i DepthmapReader::map(const SIMDRegister& simdV)
{
    SIMDRegister simdAbs = SIMDAbs(simdV);

    Vector4i UMBRA_ATTRIBUTE_ALIGNED(16, absInt);
    Vector4  UMBRA_ATTRIBUTE_ALIGNED(16, inv);
    Vector4  UMBRA_ATTRIBUTE_ALIGNED(16, v);
    SIMDStoreAligned32(SIMDFloatToBitPattern(simdAbs), (int*)&absInt);
    SIMDStoreAligned(simdV, (float*)&v);
    SIMDStoreAligned(SIMDReciprocal(simdAbs), (float*)&inv);

    Vector3i i;
    int axis = (absInt.i >= absInt.j) ? ((absInt.i >= absInt.k) ? 0 : 2) : ((absInt.j >= absInt.k) ? 1 : 2);
    int axisX = (1 << axis)  & 3;
    int axisY = (1 << axisX) & 3;

    int dir  = (((UINT32&)(v[axis])) >> 31) ^ 1;
    int face = buildFace(axis, dir);

    i.i = (int)((v[axisX] * inv[axis] + 1.f) * 0.5f * (float)DepthmapData::Resolution);
    i.j = (int)((v[axisY] * inv[axis] + 1.f) * 0.5f * (float)DepthmapData::Resolution);
    i.k = face;
    return i;
}

// relative world space coordinate -> depthmap coodinate
Vector3i DepthmapReader::map(const Vector3& v)
{
    Vector3i absInt(
        absolutef2i(v.x),
        absolutef2i(v.y),
        absolutef2i(v.z));
    
    int axis = (absInt.i >= absInt.j) ? ((absInt.i >= absInt.k) ? 0 : 2) : ((absInt.j >= absInt.k) ? 1 : 2);
    int dir  = (((UINT32&)(v[axis])) >> 31) ^ 1;
    int face = buildFace(axis, dir);
    
    int axisX = (1 << axis)  & 3;

    if (absInt[axis] == 0)
        return Vector3i(0,0,0);

    int axisY = (1 << axisX) & 3;

    const float halfRes = 0.5f * (float)DepthmapData::Resolution;
#if UMBRA_ARCH == UMBRA_ARM
    float mul = halfRes / bitPatternFloat(absInt[axis]);
#else
    float mul = halfRes / absolute(v[axis]);
#endif
    Vector3i i;
    i.k = face;
    i.i = (int)(v[axisX] * mul + halfRes);
    i.j = (int)(v[axisY] * mul + halfRes);

    return i;
}

// relative world space coordinate, face number -> depthmap coodinate
Vector3i DepthmapReader::map(const Vector3& v, int face)
{
    const float halfRes = 0.5f * (float)DepthmapData::Resolution;

    int axis = getFaceAxis(face);

    int axisX = (1 << axis)  & 3;

    float value = absolute(v[axis]);

    int axisY = (1 << axisX) & 3;

    float mul   = halfRes / value;
    Vector3i i;
    i.i = (int)(v[axisX] * mul + halfRes);
    i.j = (int)(v[axisY] * mul + halfRes);
    i.k = face;

    return i;
}

UMBRA_CT_ASSERT(DepthmapData::DepthBits == 4);

#if defined(UMBRA_REMOTE_MEMORY)
// SPU variants

// Lookup pixel value
float DepthmapReader::lookup(const DepthmapData& data, const Vector3i& i) const
{
    const UINT16* palette = m_palettes + data.faces[i.k].paletteOffset;
    int   bitIdx = DepthmapReader::getBitIdx(data.faces[i.k].faceIdx, m_numFaces, i);

    UINT32 dword;
    MemoryAccess::readElem(dword, &m_faces[UMBRA_BIT_DWORD(bitIdx)]);
    UINT8 nibble = (UINT8)((dword >> (bitIdx & 0x1f)) & 0xf);
    UINT16 paletteEntry;
    MemoryAccess::readElem(paletteEntry, &palette[nibble^1]);
    return bitPatternFloat(paletteEntry << 16);
}

// Test for infinite value
int DepthmapReader::lookupInfinite(const DepthmapData& data, const Vector3i& i) const
{
    const UINT16* palette = m_palettes + data.faces[i.k].paletteOffset;
    int   bitIdx = DepthmapReader::getBitIdx(data.faces[i.k].faceIdx, m_numFaces, i);

    UINT32 dword;
    MemoryAccess::readElem(dword, &m_faces[UMBRA_BIT_DWORD(bitIdx)]);
    UINT8 nibble = (UINT8)((dword >> (bitIdx & 0x1f)) & 0xf);
    UINT16 paletteEntry;
    MemoryAccess::readElem(paletteEntry, &palette[nibble^1]);
    return paletteEntry == floatBitPattern(FLT_MAX) >> 16;
}

#else

// Lookup pixel value
float DepthmapReader::lookup(const DepthmapData& data, const Vector3i& i) const
{
    // expects 4-bit data
 
    const UINT16* palette = m_palettes + data.faces[i.k].paletteOffset;
    int   bitIdx = DepthmapReader::getBitIdx(data.faces[i.k].faceIdx, m_numFaces, i);
    UINT8 nibble = (UINT8)((m_faces[UMBRA_BIT_DWORD(bitIdx)] >> (bitIdx & 0x1f)) & 0xf);
#if UMBRA_BYTE_ORDER == UMBRA_LITTLE_ENDIAN
    return bitPatternFloat(palette[nibble] << 16);
#else
    return bitPatternFloat(palette[nibble ^ 1] << 16);
#endif
}

// Test for infinite value
int DepthmapReader::lookupInfinite(const DepthmapData& data, const Vector3i& i) const
{
    const UINT16* palette = m_palettes + data.faces[i.k].paletteOffset;
    int   bitIdx = DepthmapReader::getBitIdx(data.faces[i.k].faceIdx, m_numFaces, i);
    UINT8 nibble = (UINT8)((m_faces[UMBRA_BIT_DWORD(bitIdx)] >> (bitIdx & 0x1f)) & 0xf);
#if UMBRA_BYTE_ORDER == UMBRA_LITTLE_ENDIAN
    return palette[nibble] == floatBitPattern(FLT_MAX) >> 16;
#else
    return palette[nibble^1] == floatBitPattern(FLT_MAX) >> 16;
#endif
}

#endif

// Test if position is visible from given object
bool DepthmapReader::testPosition(int objectIdx, const Vector3& pos, float offset) const
{
#if defined(UMBRA_REMOTE_MEMORY)
    DepthmapData data;
    MemoryAccess::alignedRead(&data, &m_data[objectIdx], sizeof(DepthmapData));
#else
    const DepthmapData& data = m_data[objectIdx];
#endif

    Vector3  dir = pos - data.reference;

    Vector3i mapped = DepthmapReader::map(dir);
    float value = lookup(data, mapped);

    float ref = absolute(dir[getFaceAxis(mapped.k)]);
    return ref < value + offset;
}

// Test if given direction is visible from given object
bool DepthmapReader::testDirection(int objectIdx, const Vector3& dir, const Vector4& near) const
{
#if defined(UMBRA_REMOTE_MEMORY)
    DepthmapData data;
    MemoryAccess::alignedRead(&data, &m_data[objectIdx], sizeof(DepthmapData));
#else
    const DepthmapData& data = m_data[objectIdx];
#endif
    Vector3i mapped = DepthmapReader::map(dir);
    float value = lookup(data, mapped);

    // Depthmap distance from near plane along axis. 
    float ref = dot(near, data.reference) * near[getFaceAxis(mapped.k)];
    // Depth must reach near plane.
    return absolute(ref) < value;
}

// Test if given mapped pixel is visible for given object
bool DepthmapReader::testDirection(int objectIdx, const Vector3i& mapped, const Vector4& near) const
{
#if defined(UMBRA_REMOTE_MEMORY)
    DepthmapData data;
    MemoryAccess::alignedRead(&data, &m_data[objectIdx], sizeof(DepthmapData));
#else
    const DepthmapData& data = m_data[objectIdx];
#endif
    float value = lookup(data, mapped);
    
    // Depthmap distance from near plane along axis
    float ref = dot(near, data.reference) * near[getFaceAxis(mapped.k)];
    // Depth must reach near plane.
    return absolute(ref) < value;
}

// Test if given direction is infinitely visible
bool DepthmapReader::testDirectionInfinite(int objectIdx, const Vector3& dir) const
{
#if defined(UMBRA_REMOTE_MEMORY)
    DepthmapData data;
    MemoryAccess::alignedRead(&data, &m_data[objectIdx], sizeof(DepthmapData));
#else
    const DepthmapData& data = m_data[objectIdx];
#endif
    Vector3i mapped = DepthmapReader::map(dir);
    return !!lookupInfinite(data, mapped);
}

// Test if pixel direction is infinitely visible
bool DepthmapReader::testDirectionInfinite(int objectIdx, const Vector3i& mapped) const
{
#if defined(UMBRA_REMOTE_MEMORY)
    DepthmapData data;
    MemoryAccess::alignedRead(&data, &m_data[objectIdx], sizeof(DepthmapData));
#else
    const DepthmapData& data = m_data[objectIdx];
#endif
    return !!lookupInfinite(data, mapped);
}

}

#endif
