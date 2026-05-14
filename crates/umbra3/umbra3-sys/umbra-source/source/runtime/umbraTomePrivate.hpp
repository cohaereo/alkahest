#pragma once
#ifndef __UMBRATOMEPRIVATE_H
#define __UMBRATOMEPRIVATE_H

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
 * \brief   Umbra tome implementation
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraBSPTree.hpp"
#include "umbraVector.hpp"
#include "umbraBitOps.hpp"
#include "umbraAABB.hpp"
#include "umbraMemoryAccess.hpp"
#include "umbraSIMD.hpp" // todo move
#include "umbraPrimitives.hpp"
#include "umbraHash.hpp"
#include "runtime/umbraTome.hpp"

namespace Umbra
{

extern const SIMDRegister g_intScale;

enum TomeVersionHistory
{
    TOME_VERSION_PVSBOOSTER                 = 0x001,
    TOME_VERSION_4                          = 0x004,
    TOME_VERSION_ADDED_CELL_AABBS           = 0x005,
    TOME_VERSION_UMBRA_31                   = 0x006,
    TOME_VERSION_USERPORTAL_CENTERS         = 0x007,
    TOME_VERSION_OBJECT_DISTANCES           = 0x008,
    TOME_VERSION_GROUP_IDS                  = 0x009,
    TOME_VERSION_EXIT_PORTALS               = 0x00a,
    TOME_VERSION_USERPORTAL_AABBS           = 0x00b,
    TOME_VERSION_PORTAL_EXPAND              = 0x00c,
    TOME_VERSION_TILEHIERARCHY              = 0x00d,
    TOME_VERSION_UMBRA_32_BETA1             = 0x00e,
    TOME_VERSION_OVELAPPING_GATES           = 0x00f,
    TOME_VERSION_UMBRA_32                   = 0x010,
    TOME_VERSION_UMBRA_32_SIZEOPT           = 0x011,
    TOME_VERSION_UMBRA_32_FINAL             = 0x012
};

enum TomeCollectionVersionHistory
{
    TOMECOLLECTION_VERSION_INITIAL          = 0x001
};

} // namespace Umbra

#define TOME_MAGIC              0xD600
#define TOME_VERSION            TOME_VERSION_UMBRA_32_FINAL
#define TOME_PVSB_MARKER        0xbaadc0de

#define TOMECOLLECTION_MAGIC    0xE700
#define TOMECOLLECTION_VERSION  TOMECOLLECTION_VERSION_INITIAL

// Data limits

#define UMBRA_MAX_CELLS_PER_TILE            2047
#define UMBRA_COMPUTATION_STRING_LENGTH     128

#define BUILD_PORTAL_LINK(face, outside, user, hierarchy, slot) \
    ((face) << 29 | (outside) << 28 | (user) << 27 | (hierarchy) << 26 | (slot))

#if defined(UMBRA_REMOTE_MEMORY)
#   define IMPLEMENT_GETBASE() void* getBase (void) const { return *((void**)(this + 1)); }
#else
#   define IMPLEMENT_GETBASE() void* getBase (void) const { return (void*)this; }
#endif

namespace Umbra
{

class ImpTile;
class ImpTome;
class QueryContext;

enum PrivateStatistic
{
    PRIVSTAT_PORTAL_GEOMETRY_DATA_SIZE,
    PRIVSTAT_BSP_NODE_DATA_SIZE,
    PRIVSTAT_BSP_PLANE_DATA_SIZE,
    PRIVSTAT_BASE_TILE_COUNT,
    PRIVSTAT_HIERARCHY_TILE_COUNT,
    PRIVSTAT_REGULAR_PORTAL_COUNT,
    PRIVSTAT_GATE_PORTAL_COUNT,
    PRIVSTAT_HIERARCHY_PORTAL_COUNT
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

template <typename T, unsigned int size>
struct AlignedPtrData;

// 32-bit implementation
template <typename T>
struct AlignedPtrData<T, 4>
{
    T*      ptr;
    UINT32  dummy;
};

// 64-bit implementation
template <typename T>
struct AlignedPtrData<T, 8>
{
    T*      ptr;
};

template <typename T>
struct AlignedPtr
{
private:
    AlignedPtrData<T, sizeof(void*)> p;

public:
    
    AlignedPtr() { p.ptr = 0; }
    AlignedPtr& operator= (T* t) { p.ptr = t; return *this; }
    
    operator T* () { return p.ptr; }
    operator const T* () const { return p.ptr; }
    
    T* operator-> () { return p.ptr; }
    bool operator! () { return !p.ptr; }
    
    T* getPtr() { return p.ptr; }
    const T* getPtr() const { return p.ptr; }

    T** getPtrAddr() { return &p.ptr; }
    T* const * getPtrAddr() const { return &p.ptr; }

};

UMBRA_CT_ASSERT(sizeof(AlignedPtr<int>) == 8);

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

UMBRA_FORCE_INLINE float fixedLerp (int factor, float mn, float mx)
{
    float f = (float)factor * (1.0f / 65535.0f);
    return (1.0f-f)*mn + f*mx;
}

UMBRA_FORCE_INLINE float lerp (float f, float mn, float mx)
{
    return (1.0f-f)*mn + f*mx;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Tome::Status checkStatus (const ImpTome* imp, bool checkCRC32);

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct SerializedTreeData
{
    UINT32      m_nodeCount_mapWidth;
    DataPtr     m_treeData;
    DataPtr     m_map;
    UINT32      m_numSplitValues;
    DataPtr     m_splitValues;

    int getNodeCount(void) const { return m_nodeCount_mapWidth >> 5; }
    int getMapWidth(void) const { return m_nodeCount_mapWidth & 0x1F; }

    void setNodeCount (int nodeCount)
    {
        UMBRA_ASSERT((UINT32)nodeCount <= (0xFFFFFFFF >> 5));
        m_nodeCount_mapWidth = nodeCount << 5 | getMapWidth();
    }

    void setMapWidth (int mapWidth)
    {
        UMBRA_ASSERT(mapWidth <= 0x1F);
        m_nodeCount_mapWidth = (getNodeCount() << 5) | mapWidth;
    }

    DataArray   getTreeData    (const void* base) const { return DataArray(base, m_treeData, sizeof(UINT32), KDTree::getDataDwords(getNodeCount())); }
    DataArray   getSplitValues (const void* base) const { return DataArray(base, m_splitValues, sizeof(float), m_numSplitValues); }
    DataArray   getTreeMap     (const void* base) const { return DataArray(base, m_map, sizeof(UINT32), -1); }
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class DepthmapData
{
public:
    
    enum
    {
        Resolution      = 16,
        FaceCount       = 6,
        DepthBits       = 4,
        PaletteEntries  = (1 << DepthBits),
        InvalidIdx      = 0xffffffff
    };

    class DepthmapFace
    {
    public:
        UINT32 pixels[UMBRA_BITVECTOR_DWORDS(DepthmapData::Resolution * DepthmapData::Resolution * DepthmapData::DepthBits)];
        bool operator==(const DepthmapFace& other) { return !memcmp(this, &other, sizeof(DepthmapFace)); }
    };

    class DepthmapPalette
    {
    public:
        UINT16 palette[DepthmapData::PaletteEntries];
        bool operator==(const DepthmapPalette& other) { return !memcmp(this, &other, sizeof(DepthmapPalette)); }
    };

    UMBRA_INLINE void    clear  (void) { memset(this, 0, sizeof(DepthmapData)); }

    struct 
    {
        UINT32 faceIdx;
        UINT32 paletteOffset;
    } faces[6];

    Vector3          reference;
    UINT32           unused;
};

template <> inline unsigned int getHashValue (const DepthmapData::DepthmapFace& f)
{
    uint32 a = 0xa9a26f44, b = 0xdb71a632, c = 0xba687907;
    shuffleInts(a, b, c, (UINT32*)f.pixels, sizeof(f.pixels) / sizeof(int));
    return a ^ b ^ c;
}

template <> inline unsigned int getHashValue (const DepthmapData::DepthmapPalette& p)
{
    uint32 a = 0xa9a26f44, b = 0xdb71a632, c = 0xba687907;
    shuffleInts(a, b, c, (UINT32*)((void*)p.palette), sizeof(p.palette) / sizeof(int));
    return a ^ b ^ c;
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct PackedAABB
{
    UINT32 mnx_mny;
    UINT32 mnz_mxx;
    UINT32 mxy_mxz;

    inline UINT16 getMnx(void) const { return mnx_mny >> 16; }
    inline UINT16 getMny(void) const { return mnx_mny & 0xffff; }
    inline UINT16 getMnz(void) const { return mnz_mxx >> 16; }
    inline UINT16 getMxx(void) const { return mnz_mxx & 0xffff; }
    inline UINT16 getMxy(void) const { return mxy_mxz >> 16; }
    inline UINT16 getMxz(void) const { return mxy_mxz & 0xffff; }

    void pack (const AABB& ref, const AABB& bounds)
    {
        Vector3 mn, mx;

        if (bounds.isOK())
        {
            mn = bounds.getMin() - ref.getMin();
            mx = bounds.getMax() - ref.getMin();
        }

        Vector3 dim = ref.getDimensions();
        for (int i = 0; i < 3; ++i)
        {
            mn[i] /= dim[i];
            mx[i] /= dim[i];
        }

        mnx_mny = ((UINT32)floorf(mn.x * 65535.f) << 16) | (UINT16)floorf(mn.y * 65535.f);
        mnz_mxx = ((UINT32)floorf(mn.z * 65535.f) << 16) | (UINT16)ceilf(mx.x * 65535.f);
        mxy_mxz = ((UINT32)ceilf(mx.y * 65535.f) << 16)  | (UINT16)ceilf(mx.z * 65535.f);
    }

    void unpack (const AABB& ref, Vector3& mn, Vector3& mx) const
    {
        SIMDRegister32 mnSIMDInt = SIMDLoad32(getMnx(), getMny(), getMnz(), 0);
        SIMDRegister32 mxSIMDInt = SIMDLoad32(getMxx(), getMxy(), getMxz(), 0);

        SIMDRegister mnSIMD = SIMDMultiply(SIMDIntToFloat(mnSIMDInt), g_intScale);
        SIMDRegister mxSIMD = SIMDMultiply(SIMDIntToFloat(mxSIMDInt), g_intScale);

        float UMBRA_ATTRIBUTE_ALIGNED16(mnf[4]);
        float UMBRA_ATTRIBUTE_ALIGNED16(mxf[4]);

        SIMDStoreAligned(mnSIMD, mnf);
        SIMDStoreAligned(mxSIMD, mxf);

        for (int i = 0; i < 3; ++i)
        {
            mn[i] = lerp(mnf[i], ref.getMin()[i], ref.getMax()[i]);
            mx[i] = lerp(mxf[i], ref.getMin()[i], ref.getMax()[i]);
        }
    }
};
UMBRA_CT_ASSERT(sizeof(PackedAABB) == 12);

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct TempBspNode
{
public:
    TempBspNode(void) : m_planeIdxAndLeafBits(0), m_frontAndBack(0) { }

    void set(int planeIdx, bool isFrontLeaf, int front, bool isBackLeaf, int back)
    {
        UMBRA_ASSERT((front & 0xffff0000) == 0);
        UMBRA_ASSERT((back & 0xffff0000) == 0);

        m_planeIdxAndLeafBits = planeIdx;
        if (isFrontLeaf)
            m_planeIdxAndLeafBits |= (1u<<31);
        if (isBackLeaf)
            m_planeIdxAndLeafBits |= (1u<<30);
        m_frontAndBack = front << 16 | back;
    }

    int getFront(void) { return m_frontAndBack >> 16; }
    int getBack(void) { return m_frontAndBack & 0xffff; }

    int getPlaneIndex(void) { return m_planeIdxAndLeafBits & ~( (1<<31) | (1<<30) ); }
    bool isFrontLeaf()  { return (m_planeIdxAndLeafBits & (1u<<31)) ? true : false;  }
    bool isBackLeaf()   { return (m_planeIdxAndLeafBits & (1u<<30)) ? true : false;  }

private:
    UINT32  m_planeIdxAndLeafBits;
    UINT32  m_frontAndBack;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Data for an axis aligned portal
 *//*-------------------------------------------------------------------*/

class Portal
{
public:
    Portal(void): link(0), idx_z(0), xmn_xmx(0), ymn_ymx(0) {}

    Face            			getFace          (void) const { return (Face)(link >> 29); }
    bool            			isOutside        (void) const { return ((link >> 28) & 0x1) ? true : false; }
    bool            			isHierarchy      (void) const { return ((link >> 26) & 0x1) ? true : false; }
    bool            			hasTarget        (void) const { return !isOutside(); }
    bool            			isUser           (void) const { return ((link >> 27) & 0x1) ? true : false; }
    int             			getTargetTileIdx (void) const { UMBRA_ASSERT(hasTarget()); return link & 0x3FFFFFF; }
    int             			getTargetCluster (void) const { return getTargetTileIdx(); }
    int             			getTargetIndex   (void) const
    {
        UMBRA_ASSERT(hasTarget());
        int idx = idx_z >> 16;
        UMBRA_ASSERT(idx < UMBRA_MAX_CELLS_PER_TILE);
        return idx;
    }
    int             			getUserObjOfs    (void) const { UMBRA_ASSERT(isUser()); return xmn_xmx >> 12; }
    int             			getUserObjCount  (void) const { UMBRA_ASSERT(isUser()); return xmn_xmx & 0xFFF; }
    int             			getGeometryOfs   (void) const { UMBRA_ASSERT(isUser()); return ymn_ymx >> 12; }
    int             			getVertexCount   (void) const { UMBRA_ASSERT(isUser()); return ymn_ymx & 0xFFF; }
	float						getZ		     (const Vector3& pmn, const Vector3& pmx) const;
    void            			getQuad          (const Vector3& pmn, const Vector3& pmx, float portalExpand, Vector3& x0y0, Vector3& x0y1, Vector3& x1y1, Vector3& x1y0) const;
    UMBRA_FORCE_INLINE void     getQuad          (SIMDRegister& mn, SIMDRegister& mx, const SIMDRegister& expand, const SIMDRegister& scale, const SIMDRegister& bias) const;

    template<class VectorType>
    UMBRA_FORCE_INLINE void     getIntMinMax     (VectorType& mn, VectorType& mx) const;

    template<class VectorType>
	UMBRA_FORCE_INLINE void		getMinMax        (const Vector3& pmn, const Vector3& pmx, float portalExpand, VectorType& mn, VectorType& mx) const;

    static int getMaxSlotIdx() { return 0x3FFFFFF; }

	UINT32      link;       // 31-29: face, 28: outside (exit portal), 27: user, 25: hierarchy, 25-0: target tile or cluster index
    UINT32      idx_z;      // 31-16: target local index, 15-0: depth value (0.16 fixed)
    UINT32      xmn_xmx;    // if (!user) 31-16: x min (0.16 fixed), 15-0: x max (0.16 fixed); else 31-0: user object index
    UINT32      ymn_ymx;    // if (!user) 31-16: y min (0.16 fixed), 15-0: y max (0.16 fixed)
                            // else 31-12: 1st vertex offset, 11-0: vertex count:
                            // For cluster portals: AABB min, AABB max
                            // For cell portals: center, plane eq(x, y, z), (plane eq(w), radius), triangles; triangle count = vertex count - 5
};

void Portal::getQuad(SIMDRegister& mn, SIMDRegister& mx, const SIMDRegister& expand, const SIMDRegister& scale, const SIMDRegister& ofs) const
{
    int index_z = getFaceAxis(getFace());
    int index_x = (1 << index_z) & 3;
    int index_y = (1 << index_x) & 3;
    int UMBRA_ATTRIBUTE_ALIGNED16(minCornerArr)[4];
    int UMBRA_ATTRIBUTE_ALIGNED16(maxCornerArr)[4];

    // extract portal corner points
    minCornerArr[index_x] = xmn_xmx >> 16;
    minCornerArr[index_y] = ymn_ymx >> 16;
    minCornerArr[index_z] = idx_z & 0xFFFF;
    minCornerArr[3] = 0;
    maxCornerArr[index_x] = xmn_xmx & 0xFFFF;
    maxCornerArr[index_y] = ymn_ymx & 0xFFFF;
    maxCornerArr[index_z] = idx_z & 0xFFFF;
    maxCornerArr[3] = 0;

    SIMDRegister32 imn = SIMDLoadAligned32(minCornerArr);
    SIMDRegister32 imx = SIMDLoadAligned32(maxCornerArr);

    // expand portal
    SIMDRegister ofsMx = SIMDAdd(ofs, expand);
    SIMDRegister ofsMn = SIMDSub(ofs, expand);

    // transform to global coordinate system
    mn = SIMDMultiplyAdd(SIMDIntToFloat(imn), scale, ofsMn);
    mx = SIMDMultiplyAdd(SIMDIntToFloat(imx), scale, ofsMx);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Data for a portal graph cell node
 * \note    Leaf tile cells have their cluster index directly stored
 *          in clusterIndex, and clusterCount is always 0. Hierarchy
 *          tiles store a (index, count) slice of the tome-global
 *          cluster list.
 *//*-------------------------------------------------------------------*/

class CellNode
{
public:

    void setObjects             (int idx, int numObjects) { objIdx = idx; objCount = numObjects; }
    void setClusters            (int cIdx, int numClusters) { clusterIndex = cIdx; clusterCount = numClusters; }
    void setPortalIdxAndCount   (int idx, int count) { portalIdx = idx; portalCount = count; }
    void setBounds              (const PackedAABB& aabb) { bounds = aabb; }

    int getPortalIndex  (void) const { return portalIdx; }
    int getPortalCount  (void) const { return portalCount; }
    int getObjectIndex  (void) const { return objIdx; }
    int getObjectCount  (void) const { return objCount; }
    int getLastPortal   (void) const { return getPortalIndex() + getPortalCount(); }
    int getLastObject   (void) const { return getObjectIndex() + getObjectCount(); }
    int getClusterIndex (void) const { return clusterIndex; }
    int getClusterCount (void) const { return clusterCount; }
    int getLastCluster  (void) const { return getClusterIndex() + getClusterCount(); }
    const PackedAABB& getBounds (void) const { return bounds; }

private:
    UINT32      portalIdx;
    UINT32      portalCount;
    UINT32      objIdx;
    UINT32      objCount;
    UINT32      clusterIndex;
    UINT32      clusterCount;
    PackedAABB  bounds;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ExtCellNode
{
public:
    void setPortalIdxAndCount   (int idx, int count) { portalIdx = idx; portalCount = count; }
    int getPortalIndex  (void) const { return portalIdx; }
    int getPortalCount  (void) const { return portalCount; }

    ExtCellNode() : portalIdx(0), portalCount(0)
    {}

private:
    UINT32      portalIdx;
    UINT32      portalCount;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Data for a connectivity graph node
 *//*-------------------------------------------------------------------*/

class ClusterNode
{
public:
    void setPortalIdxAndCount   (int idx, int count) { portalIdx = idx; portalCount = count; }
    void setBounds (const PackedAABB& aabb) { bounds = aabb; }

    int getPortalIndex  (void) const { return portalIdx; }
    int getPortalCount  (void) const { return portalCount; }
    int getLastPortal   (void) const { return getPortalIndex() + getPortalCount(); }
    const PackedAABB& getBounds (void) const { return bounds; }

private:
    UINT32      portalIdx;
    UINT32      portalCount;
    PackedAABB  bounds;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ExtClusterNode
{
public:
    ExtClusterNode() : portalIdx(0), portalCount(0) {}
    void setPortalIdxAndCount   (int idx, int count) { portalIdx = idx; portalCount = count; }

    int getPortalIndex  (void) const { return portalIdx; }
    int getPortalCount  (void) const { return portalCount; }
    int getLastPortal   (void) const { return getPortalIndex() + getPortalCount(); }

private:
    UINT32      portalIdx;
    UINT32      portalCount;
};
/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class UMBRA_ATTRIBUTE_ALIGNED16(ObjectDistance)
{
public:
    ObjectDistance(void): nearLimit(0.f), farLimit(FLT_MAX) {}

    Vector3     boundMin;
    float       nearLimit;
    Vector3     boundMax;
    float       farLimit;
};

struct ObjectBounds
{
    Vector3 mn;
    Vector3 mx;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

struct LeafTileMatchData
{
    int         m_matchTreeOfsAndCount;

    DataPtr     m_cellLodMapping;       // Hierarchy cell mapping (leaf tiles only)
    int         m_cellLodBitWidth;
    int         m_cellLodElemWidth;

    DataArray   getCellLodMap  (const void* base) const { return DataArray(base, m_cellLodMapping, sizeof(UINT32), -1); }

    void        setMatchTreeOfsAndCount (int ofs, int count)
    {
        UMBRA_ASSERT(count <= 6);
        m_matchTreeOfsAndCount = (ofs << 3) | count;
    }

    int         getMatchTreeOfs (void) const
    {
        return m_matchTreeOfsAndCount >> 3;
    }

    int         getMatchTreeCount (void) const
    {
        return m_matchTreeOfsAndCount & 0x7;
    }
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Tome implementation
 *//*-------------------------------------------------------------------*/

class ImpTome
{
public:
    enum Flags
    {
        TOMEFLAG_DEPTHMAPS          = (1 << 0),
        TOMEFLAG_SHADOW_DEPTHMAPS   = (1 << 1)
    };

    UINT32                      getVersionMagic          (void) const { return m_versionMagic; }
    UINT32                      getSize                  (void) const { return m_size; }
    UINT32                      getTomeVersion           (void) const { return m_versionMagic & 0xFFFF; }
    float                       getLodBaseDistance       (void) const { return m_lodBaseDistance; }
    UINT32                      getFlags                 (void) const { return m_flags; }
    UINT32                      getCRC32                 (void) const { return m_crc32; }
    UINT32                      computeCRC32             (void) const;

    // Tile tree
    const Vector3&              getTreeMin               (void) const { return m_treeMin; }
    const Vector3&              getTreeMax               (void) const { return m_treeMax; }
    AABB                        getAABB                  (void) const { return AABB(m_treeMin, m_treeMax); }
    const SerializedTreeData&   getTileTree        (void) const { return m_tileTree; }
    int                         getTreeNodeCount         (void) const { return m_tileTree.getNodeCount(); }
    DataArray                   getTreeData              (void) const { return DataArray(getBase(), m_tileTree.m_treeData, sizeof(UINT32), KDTree::getDataDwords(m_tileTree.getNodeCount())); }
    DataArray                   getTreeSplits            (void) const { return DataArray(getBase(), m_tileTree.m_splitValues, sizeof(float), m_tileTree.getNodeCount()); }
    const char*                 getComputationString     (void) const { return m_computationString; } // max UMBRA_COMPUTATION_STRING_LENGTH characters, might not be null-terminated

    // Objects
    int                         getNumObjects            (void) const { return m_numObjects; }
    DataArray                   getObjectBounds          (void) const { return DataArray(getBase(), m_objBounds, sizeof(ObjectBounds), m_numObjects); }
    inline DataArray            getObjectDistances       (void) const;
    bool                        containsGroups           (void) const { return getUserIDStarts() != 0; }
    const int*                  getUserIDStarts          (void) const { return (const int*)m_userIDStarts.getAddr(this); }
    const UINT32*               getUserIDs               (void) const { return (const UINT32*)m_userIDs.getAddr(this); }
    int                         getUserIDCount           (int g) const { return getUserIDStarts()[g+1] - getUserIDStarts()[g]; }
    const UINT32*               getUserIDs               (int g) const { return &getUserIDs()[getUserIDStarts()[g]]; }

    // Gates
    int                         getNumGates              (void) const { return m_numGates; }
    DataArray                   getGateVertices          (void) const { return DataArray(getBase(), m_gateVertices, sizeof(Vector3), m_numGateVertices); }
    DataArray                   getGateIndices           (void) const { return DataArray(getBase(), m_gateIndices, sizeof(int), -1); }
    const UINT32*               getGateIndexMap          (void) const { return (const UINT32*)m_gateIndexMap.getAddr(this); }
    inline void                 getGateBounds            (const Portal& portal, const Vector3& portalExpand, Vector3& mn, Vector3& mx) const;
    inline Vector3              getGateCenter            (const Portal& portal) const;

    // Clusters
    int                         getNumClusters           (void) const { return m_numClusters; }
    inline int                  getNumClusterPortals     (void) const;
    DataArray                   getClusterNodes          (void) const { return DataArray(getBase(), m_clusters, sizeof(ClusterNode), getNumClusters()); }
    DataArray                   getClusterPortals        (void) const { return DataArray(getBase(), m_clusterPortals, sizeof(Portal), getNumClusterPortals()); }
    inline DataArray            getClusterPortals        (const ClusterNode& node) const;
    inline Vector3              getClusterPortalCenter   (const Portal& portal) const;
    inline void                 getClusterBounds         (Vector3& mn, Vector3& mx, int idx) const;

    // Lists
    DataArray                   getObjectLists           (void) const { return DataArray(getBase(), m_objectLists, sizeof(UINT32), UMBRA_BITVECTOR_DWORDS(m_objectListSize * getObjectListTotalWidth())); }
    int                         getObjectListElemWidth   (void) const { return (int)((m_listWidths)      & ((1 << 5) - 1)); }
    int                         getObjectListCountWidth  (void) const { return (int)((m_listWidths >> 5) & ((1 << 5) - 1)); }
    int                         getObjectListTotalWidth  (void) const { return getObjectListElemWidth() + getObjectListCountWidth(); }
    int                         getClusterListElemWidth  (void) const { return (int)((m_listWidths >> 10) & ((1 << 5) - 1)); }
    int                         getClusterListCountWidth (void) const { return (int)((m_listWidths >> 15) & ((1 << 5) - 1)); }
    int                         getClusterListTotalWidth (void) const { return getClusterListElemWidth() + getClusterListCountWidth(); }
    DataArray                   getClusterLists          (void) const { return DataArray(getBase(), m_clusterLists, sizeof(UINT32), UMBRA_BITVECTOR_DWORDS(m_clusterListSize * getClusterListTotalWidth())); }

    // Global cells
    int                         getNumCells              (void) const { if (!getCellStarts()) return 0; int i; getCellStarts().getElem(i, m_numTiles); return i; }
    DataArray                   getCellStarts            (void) const { return DataArray(getBase(), m_cellStarts, sizeof(int), m_numTiles + 1); }
    int                         getCellStart             (int slot) const { int i; getCellStarts().getElem(i, slot); return i; }

    // Per-leaf data
    int                         getNumLeafTiles          (void) const { return m_numLeafTiles; }
    int                         getNumTiles              (void) const { return m_numTiles; }
    int                         getTileArraySize         (void) const { return m_tileTree.getNodeCount(); }
    DataArray                   getTileLodLevels         (void) const { return DataArray(getBase(), m_tileLodLevels, sizeof(float), getTileArraySize()); }

    inline int                  getBitsPerSlotPath       (void) const;
    inline DataArray            getTilePaths             (void) const;
    DataArray                   getTileOffsets           (bool tilesArePointers) const { return DataArray(getBase(), m_tiles, tilesArePointers ? sizeof(AlignedPtr<const ImpTile>) : sizeof(DataPtr), m_numTiles); }
    void*                       getTileBase              (void) const { return getBase(); }
    inline const ImpTile*       getTile                  (int idx, bool tilesArePointers) const;

    DataArray                   getMatchingData          (void) const { return DataArray(getBase(), m_tileMatchingData, sizeof(LeafTileMatchData), getNumLeafTiles()); }
    DataArray                   getMatchingTrees         (void) const { return DataArray(getBase(), m_matchingTrees,    sizeof(SerializedTreeData), getNumMatchingTrees()); }
    int                         getNumMatchingTrees      (void) const { return m_numMatchingTrees; }

    // TomeCollection API
    int                         getTomeCount             (void) const { return m_numTomes; }
    DataArray                   getClusterStarts         (void) const { return DataArray(getBase(), m_tomeClusterStarts, sizeof(int), m_numTomes + 1); }
    DataArray                   getClusterPortalStarts   (void) const { return DataArray(getBase(), m_tomeClusterPortalStarts, sizeof(int), m_numTomes + 1); }

    bool                        hasObjectDepthmaps       (void) const { return !!(m_flags & TOMEFLAG_DEPTHMAPS); }
    bool                        hasObjectShadowmaps      (void) const { return !!(m_flags & TOMEFLAG_SHADOW_DEPTHMAPS); }
    DataPtr                     getObjectDepthmaps       (void) const { return m_objectDepthmaps; }
    DataPtr                     getDepthmapFaces         (void) const { return m_depthmapFaces; }
    DataPtr                     getDepthmapPalettes      (void) const { return m_depthmapPalettes; }
    int                         getNumFaces              (void) const { return m_numFaces; }

    static UINT32               getPrivateStatistic      (const ImpTome* t, PrivateStatistic s);

    ImpTome(void) {}

    IMPLEMENT_GETBASE()

private:

    UINT32              m_versionMagic;
    UINT32              m_crc32;
    UINT32              m_size;
    float               m_lodBaseDistance;
    UINT32              m_flags;

    Vector3             m_treeMin;
    Vector3             m_treeMax;
    SerializedTreeData  m_tileTree;

    int                 m_numObjects;
    DataPtr             m_objBounds;
    DataPtr             m_objDistances;
    DataPtr             m_userIDStarts;
    DataPtr             m_userIDs;

    UINT32              m_listWidths;
    DataPtr             m_objectLists;
    int                 m_objectListSize;
    DataPtr             m_clusterLists;
    int                 m_clusterListSize;

    int                 m_numGates;
    DataPtr             m_gateIndexMap;
    DataPtr             m_gateVertices;
    int                 m_numGateVertices;
    DataPtr             m_gateIndices;

    int                 m_numClusters;
    DataPtr             m_clusters;
    DataPtr             m_clusterPortals;
    DataPtr             m_cellStarts;

    int                 m_numLeafTiles;
    int                 m_numTiles;
    int                 m_bitsPerSlotPath;
    DataPtr             m_slotPaths;
    DataPtr             m_tileLodLevels;
    DataPtr             m_tiles;                // ImpTile
    DataPtr             m_tileMatchingData;     // LeafTileMatchData
    DataPtr             m_matchingTrees;
    int                 m_numMatchingTrees;

    // only with TomeCollection
    int                 m_numTomes;
    DataPtr             m_tomeClusterStarts;        // tome-local to global cluster index mapping
    DataPtr             m_tomeClusterPortalStarts;  // tome-local to global cluster portal index mapping

    char                m_computationString[UMBRA_COMPUTATION_STRING_LENGTH];

    DataPtr             m_objectDepthmaps;
    DataPtr             m_depthmapFaces;
    DataPtr             m_depthmapPalettes;
    int                 m_numFaces;

    int                 m_pad[1];

    friend class TomeWriter;
    friend class RuntimeTomeGenerator;
};

DataArray ImpTome::getObjectDistances(void) const
{
    return DataArray(getBase(), m_objDistances, sizeof(ObjectDistance), m_numObjects);
}

int ImpTome::getNumClusterPortals(void) const
{
    if (!!m_tomeClusterStarts)
    {
        int count = 0;
        getClusterPortalStarts().getElem(count, m_numTomes);
        return count;
    } else
    {
        ClusterNode last;
        getClusterNodes().getElem(last, getNumClusters() - 1);
        return last.getLastPortal();
    }
}

DataArray ImpTome::getClusterPortals(const ClusterNode& node) const
{
    return DataArray(getBase(), DataPtr(m_clusterPortals.getOffset() + node.getPortalIndex() * sizeof(Portal)), sizeof(Portal), node.getPortalCount());
}

void ImpTome::getGateBounds(const Portal& portal, const Vector3& portalExpand, Vector3& mn, Vector3& mx) const
{
    UMBRA_ASSERT(portal.isUser());
    DataArray vertices = getGateVertices();
    Vector3 bounds[2];
    vertices.getElems(bounds, portal.getGeometryOfs(), 2);
    mn = bounds[0] - portalExpand;
    mx = bounds[1] + portalExpand;
}

Vector3 ImpTome::getGateCenter(const Portal& portal) const
{
    UMBRA_ASSERT(portal.isUser());
    DataArray vertices = getGateVertices();
    Vector3 center;
    vertices.getElem(center, portal.getGeometryOfs());
    return center;
}

Vector3 ImpTome::getClusterPortalCenter (const Portal& portal) const
{
    if (!portal.isUser())
    {
        Vector3 mn, mx;
        portal.getMinMax(m_treeMin, m_treeMax, 0.f, mn, mx);
        return (mn + mx) * 0.5f;
    }
    return getGateCenter(portal);
}

DataArray ImpTome::getTilePaths     (void) const
{
    return DataArray(getBase(), m_slotPaths,
        sizeof(UINT32), UMBRA_BITVECTOR_DWORDS(m_numTiles * getBitsPerSlotPath()));
}

int ImpTome::getBitsPerSlotPath (void) const
{
    return m_bitsPerSlotPath;
}

void ImpTome::getClusterBounds (Vector3& mn, Vector3& mx, int idx) const
{
    ClusterNode node;
    getClusterNodes().getElem(node, idx);
    node.getBounds().unpack(AABB(getTreeMin(), getTreeMax()), mn, mx);
}

const ImpTile* ImpTome::getTile (int idx, bool tilesArePointers) const 
{ 
    if (tilesArePointers)
    {
        AlignedPtr<const ImpTile> tile;
        tile = (const ImpTile*)NULL;
        getTileOffsets(true).getElem(tile, idx); 
        return tile;
    }
        
    DataPtr ptr; 
    getTileOffsets(false).getElem(ptr, idx); 
    return (const ImpTile*)ptr.getAddr(getBase());
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class TomeContext
{
public:
    AlignedPtr<const ImpTome> m_tome;

    DataArray   getObjMap(const void* base) const { return DataArray(base, m_objGlobalIdx, sizeof(int), -1); }
    DataArray   getTileMap(const void* base) const { return DataArray(base, m_tileGlobalIdx, sizeof(int), -1); }
    DataArray   getGateMap(const void* base) const { return DataArray(base, m_gateGlobalIdx, sizeof(int), -1); }

    bool hasExtClusters (void) const
    {
        return !!m_extClusters;
    }

    DataArray getExtClusters(const void* base, int num) const
    {
        return DataArray(base, m_extClusters, sizeof(ExtClusterNode), num);
    }

    DataArray getExtPortals(const void* base, const ExtClusterNode& node) const
    {
        return DataArray(base, DataPtr(m_extPortals.getOffset() + node.getPortalIndex() * sizeof(Portal)),
            sizeof(Portal), node.getPortalCount());
    }

    DataArray getExtPortals(const void* base) const
    {
        return DataArray(base, m_extPortals, sizeof(Portal), -1);
    }

private:
    DataPtr     m_objGlobalIdx;     // (int) object index mapping from local to global indices
    DataPtr     m_tileGlobalIdx;    // (int) local tile index to global index
    DataPtr     m_gateGlobalIdx;    // (int) local gate index to global index
    DataPtr     m_extClusters;      // (ExtClusterNode)
    DataPtr     m_extPortals;       // (Portal)
    int         m_pad[1];

    friend class RuntimeTomeGenerator;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ExtTile
{
public:

    int getTomeIdx (void) const
    {
        return m_tomeIdx;
    }

    UINT32 getExitPortalMask (void) const
    {
        return m_exitPortalMask;
    }

    int getLocalSlot (void) const
    {
        return m_localSlot;
    }

    bool hasExtCells (void) const
    {
        return !!m_extCells;
    }

    DataArray getExtCells(const void* base, int num) const
    {
        return DataArray(base, m_extCells, sizeof(ExtCellNode), num);
    }

    DataArray getExtPortals(const void* base, const ExtCellNode& node) const
    {
        return DataArray(base, DataPtr(m_extPortals.getOffset() + node.getPortalIndex() * sizeof(Portal)),
            sizeof(Portal), node.getPortalCount());
    }

private:
    int                 m_tomeIdx;
    UINT32              m_exitPortalMask;
    DataPtr             m_extCells;     // ExtCellNode
    DataPtr             m_extPortals;   // Portal
    int                 m_localSlot;
    int                 m_pad[3];

    friend class RuntimeTomeGenerator;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Umbra tile data chunk implementation
 *//*-------------------------------------------------------------------*/

class ImpTile
{
public:
    enum Flags
    {
        TILEFLAG_ISLEAF       = (1 << 0),
        TILEFLAG_ISEMPTY      = (1 << 1)
    };

    UINT32                  getSize             (void) const { return ((UINT32)m_sizeAndFlags) >> 8; }
    UINT32                  getFlags            (void) const { return ((UINT32)m_sizeAndFlags) & 0xFF; }
    bool                    isLeaf              (void) const { return (getFlags() & TILEFLAG_ISLEAF) != 0; }
    float                   getPortalExpand     (void) const { return m_portalExpand; }

    // view tree
    const Vector3&          getTreeMin          (void) const { return m_treeMin; }
    const Vector3&          getTreeMax          (void) const { return m_treeMax; }
    AABB                    getAABB             (void) const { return AABB(m_treeMin, m_treeMax); }
    int                     getTreeNodeCount    (void) const { return m_viewTree.getNodeCount(); }
    DataArray               getTreeData         (void) const { return m_viewTree.getTreeData(getBase()); }
    DataArray               getTreeSplits       (void) const { return m_viewTree.getSplitValues(getBase()); }
    SerializedTreeData      getViewTree         (void) const { return m_viewTree; }
    int                     getMappingWidth     (void) const { return m_viewTree.getMapWidth(); }

    // cell graph
    int                     getNumCells         (void) const { return ((UINT32)m_numCellsAndClusters) & 0xFFFF; }
    DataArray               getCellNodes        (void) const { return DataArray(getBase(), m_cells, sizeof(CellNode), getNumCells()); }
    inline DataArray        getPortals          (const CellNode& node) const;
    UINT32                  getNodeData         (int nodeIdx) const;
    int                     getCellIndex        (int nodeIdx, const Vector3& pos) const;
    UMBRA_FORCE_INLINE void getCellBounds       (Vector3& mn, Vector3& mx, int cellIdx) const;

    // clusters
    int                     getNumClusters      (void) const { return ((UINT32)m_numCellsAndClusters) >> 16; }
    inline int              getClusterIndex     (int cellIdx) const;

    // Gates
    DataArray               getBSPTriangles     (void) const { return DataArray(getBase(), m_bsp, sizeof(Umbra::TempBspNode), -1); }
    int                     getNumBSPNodes      (void) const { return m_numBspNodes; }
    DataArray               getPlanes           (void) const { return DataArray(getBase(), m_planes, sizeof(Umbra::Vector4), -1); }
    int                     getNumPlanes        (void) const { return m_numPlanes; }

    // TomeAPI

    static UINT32           getViewTreeDataSize      (const ImpTile* t);
    static UINT32           getAccurateFindDataSize  (const ImpTile* t);
    static UINT32           getPortalDataSize        (const ImpTile* t);

                            ImpTile             (void) : m_sizeAndFlags(0) {}

protected:
    IMPLEMENT_GETBASE()

private:

    BitDataArray            getCellMap          (void) const { return BitDataArray(DataArray(getBase(), m_viewTree.m_map, sizeof(UINT32), -1), 0); }

    // View tree, spatial cell mapping
    Vector3                 m_treeMin;
    Vector3                 m_treeMax;
    SerializedTreeData      m_viewTree;

    // Misc
    int                     m_sizeAndFlags;
    float                   m_portalExpand;

    // cell graphs
    int                     m_numCellsAndClusters;  /* number of cells and clusters in this tile */
    DataPtr                 m_cells;                /* cells, 0 for no portal graph */
    DataPtr                 m_portals;              /* portals, 0 for no portal graph*/

    // Gates
    DataPtr                 m_bsp;                  /* BspTree::Node entries for exact cell locating */
    int                     m_numBspNodes;
    DataPtr                 m_planes;               /* User portal triangle lists */
    int                     m_numPlanes;

    friend class TomeWriter;
    friend class LegacyTome;
    friend class TomeConverter7;
    friend class RuntimeTomeGenerator;
};

DataArray ImpTile::getPortals(const CellNode& node) const
{
    return DataArray(getBase(), DataPtr(m_portals.getOffset() + node.getPortalIndex() * sizeof(Portal)), sizeof(Portal), node.getPortalCount());
}

int ImpTile::getClusterIndex(int cellIdx) const
{
    UMBRA_ASSERT(isLeaf());
    if (!getNumClusters())
        return -1;
    CellNode cell;
    getCellNodes().getElem(cell, cellIdx);
    UMBRA_ASSERT(cell.getClusterCount() == 0);
    return cell.getClusterIndex();
}

void ImpTile::getCellBounds(Vector3& mn, Vector3& mx, int cellIdx) const
{
    CellNode cell;
    getCellNodes().getElem(cell, cellIdx);
    cell.getBounds().unpack(AABB(getTreeMin(), getTreeMax()), mn, mx);
}

class ImpTomeCollectionSerialized
{
public:

    UINT32              getVersionMagic          (void) const { return m_versionMagic; }
    UINT32              getSize                  (void) const { return m_size; }
    UINT32              getMagic                 (void) const { return (m_versionMagic >> 16) & 0xFFFF; }
    UINT32              getVersion               (void) const { return m_versionMagic & 0xFFFF; }
    UINT32              getDataSize              (void) const { return m_dataSize; }
    DataPtr             getData                  (void) const { return m_data; }            // Relative to this object
    DataPtr             getExtTiles              (void) const { return m_extTiles; }        // Relative to data
    DataPtr             getContexts              (void) const { return m_contexts; }        // Relative to data
    int                 getNumContexts           (void) const { return m_numContexts; }

private:

    UINT32              m_versionMagic;
    UINT32              m_size;
    UINT32              m_dataSize;
    DataPtr             m_data;         // Relative to this object
    DataPtr             m_extTiles;     // Relative to m_data
    DataPtr             m_contexts;     // Relative to m_data
    int                 m_numContexts;
    int                 m_pad[1];

    friend class ImpTomeCollection;
};

class Tome;
class Allocator;
Tome* allocTome (size_t size, Allocator* a);
/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

template<class VectorType>
UMBRA_FORCE_INLINE void Portal::getIntMinMax (VectorType& mn, VectorType& mx) const
{
    /*
    UMBRA_ASSERT(!isUser());
    int axis = getFaceAxis(getFace());
    mn[axis] = mx[axis] = idx_z & 0xFFFF;
    int axisX = (axis + 1) % 3;
    mn[axisX] = xmn_xmx >> 16;
    mx[axisX] = xmn_xmx & 0xFFFF;
    int axisY = (axis + 2) % 3;
    mn[axisY] = ymn_ymx >> 16;
    mx[axisY] = ymn_ymx & 0xFFFF; */
    
#if UMBRA_OS == UMBRA_XBOX360 || UMBRA_OS == UMBRA_PS3
    UMBRA_ASSERT(!isUser());
    int axis = getFaceAxis(getFace());
    mn[axis] = mx[axis] = idx_z & 0xFFFF;
    int axisX = (axis + 1) - (3 & UMBRA_SIGN_EXTEND(axis - 1));
    mn[axisX] = xmn_xmx >> 16;
    mx[axisX] = xmn_xmx & 0xFFFF;
    int axisY = (axis + 2) - (3 & UMBRA_SIGN_EXTEND(axis));
    mn[axisY] = ymn_ymx >> 16;
    mx[axisY] = ymn_ymx & 0xFFFF;
#else
    UMBRA_ASSERT(!isUser());
    int axis = getFaceAxis(getFace());
    mn[axis] = mx[axis] = idx_z & 0xFFFF;
    int axisX = (1 << axis) & 3;
    mn[axisX] = xmn_xmx >> 16;
    mx[axisX] = xmn_xmx & 0xFFFF;
    int axisY = (1 << axisX) & 3;
    mn[axisY] = ymn_ymx >> 16;
    mx[axisY] = ymn_ymx & 0xFFFF;
#endif
    
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

template<class VectorType>
UMBRA_FORCE_INLINE void Portal::getMinMax (const Vector3& pmn, const Vector3& pmx, float portalExpand, VectorType& mn, VectorType& mx) const
{
    UMBRA_ASSERT(!isUser());
    int axis = getFaceAxis(getFace());

    SIMDRegister a = SIMDMultiply(SIMDIntToFloat(SIMDLoad32(idx_z & 0xFFFF)), g_intScale);
    SIMDRegister b = SIMDMultiply(SIMDIntToFloat(SIMDLoad32(xmn_xmx >> 16, xmn_xmx & 0xFFFF, ymn_ymx >> 16, ymn_ymx & 0xFFFF)), g_intScale);

    float idx_z2;
    Vector4 UMBRA_ATTRIBUTE_ALIGNED16(portal);

    SIMDStore(a, idx_z2);
#if UMBRA_OS == UMBRA_XBOX360
    SIMDStore(b, portal);
#else
    SIMDStoreAligned(b, &portal.x);
#endif

    mn[axis] = mx[axis] = lerp(idx_z2, pmn[axis], pmx[axis]);
    axis = (axis + 1) % 3;
    mn[axis] = lerp(portal.x, pmn[axis], pmx[axis]) - portalExpand;
    mx[axis] = lerp(portal.y, pmn[axis], pmx[axis]) + portalExpand;
    axis = (axis + 1) % 3;
    mn[axis] = lerp(portal.z, pmn[axis], pmx[axis]) - portalExpand;
    mx[axis] = lerp(portal.w, pmn[axis], pmx[axis]) + portalExpand;
/*
    UMBRA_ASSERT(mn.x >= pmn.x && mx.x <= pmx.x && mx.x >= mn.x);
    UMBRA_ASSERT(mn.y >= pmn.y && mx.y <= pmx.y && mx.y >= mn.y);
    UMBRA_ASSERT(mn.z >= pmn.z && mx.z <= pmx.z && mx.z >= mn.z);
    */
}

}

#endif
