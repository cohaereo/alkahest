#pragma once
#ifndef __UMBRAQUERYARGS_H
#define __UMBRAQUERYARGS_H

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
 * \brief   Umbra query argument object implementations
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "umbraMatrix.hpp"
#include "runtime/umbraQuery.hpp"
#include "umbraTransformer.hpp"
#include "umbraBitOps.hpp"
#include "umbraAABB.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraShadowDefs.hpp"

#define CHECK_APICLASS_STORAGE(ImpClass, size) \
    UMBRA_CT_ASSERT(sizeof(ImpClass) + UMBRA_ALIGNOF(ImpClass) - 1 <= size)
#define IMPL(Instance) GetApiObjectImplementation(Instance)

#define UMBRA_MAX_GATE_INDICES  16

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief   Api object class boilerplate
 *//*-------------------------------------------------------------------*/

template<typename ApiClass>
struct ApiClassTraits
{
    enum { Declared = 0 };
};

template<class ApiClass>
static inline typename ApiClassTraits<ApiClass>::ImpClass* GetApiObjectImplementation(ApiClass* Instance)
{
    if (!Instance)
        return NULL;
    return (typename ApiClassTraits<ApiClass>::ImpClass*)UMBRA_ALIGN(Instance->m_mem, UMBRA_ALIGNOF(typename ApiClassTraits<ApiClass>::ImpClass));
}

template<class ApiClass>
static inline const typename ApiClassTraits<ApiClass>::ImpClass* GetApiObjectImplementation(const ApiClass* Instance)
{
    if (!Instance)
        return NULL;
    return (const typename ApiClassTraits<ApiClass>::ImpClass*)UMBRA_ALIGN(Instance->m_mem, UMBRA_ALIGNOF(typename ApiClassTraits<ApiClass>::ImpClass));
}

#define DECLARE_API_CLASS(xApiClass, xImpClass) \
    template<> struct ApiClassTraits<xApiClass> { typedef xImpClass ImpClass; enum { Declared = 1 }; };

// Mapping from API classes to implementation classes

DECLARE_API_CLASS(CameraTransform, class ImpCameraTransform)
template <typename ELEM> class UserList;
DECLARE_API_CLASS(IndexList, UserList<int>)
DECLARE_API_CLASS(FloatList, UserList<float>)
DECLARE_API_CLASS(Visibility, class ImpVisibility)
DECLARE_API_CLASS(ObjectDistanceParams, class ImpObjectDistanceParams)
DECLARE_API_CLASS(OcclusionBuffer, class ImpOcclusionBuffer)
DECLARE_API_CLASS(ReceiverMaskBuffer, class ImpReceiverMaskBuffer)
DECLARE_API_CLASS(Path, class ImpPath)
DECLARE_API_CLASS(PortalInfo, class ImpPortalInfo)
DECLARE_API_CLASS(LineSegmentQuery, class ImpLineSegmentQuery)
DECLARE_API_CLASS(Query, class QueryState)
DECLARE_API_CLASS(QueryExt, class QueryState)
DECLARE_API_CLASS(ShadowCullerExt, class ImpShadowCuller)
DECLARE_API_CLASS(TomeCollection, class ImpTomeCollection)

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

template<typename ELEM>
class UserList
{
public:
    UserList(void) : m_buf(NULL), m_capacity(0), m_size(0) {}
    UserList(ELEM* arr, int cap, int sz = 0): m_buf(arr), m_capacity(cap), m_size(sz) {}

    void    clear       (void) { m_size = 0; }
    void    setSize     (int size) { m_size = size; }
    bool    isMaxed     (void) const { return m_size > m_capacity; }
    int     getCapacity (void) const { return m_capacity; }
    int     getSize     (void) const { return min2(m_size, m_capacity); }
    ELEM*   getBuf      (void) const { return m_buf; }
    bool    isRemote    (void) const { return MemoryAccess::isRemoteAddress(m_buf); }

    UMBRA_INLINE bool pushBack (const ELEM& elem);
    UMBRA_INLINE ELEM get (int pos) const;

    UMBRA_INLINE void updateRemote(UserList<ELEM>*) const;

private:
    ELEM*   m_buf;
    int     m_capacity;
    int     m_size;

#if !defined(_WIN64) && !defined(__LP64__)
    // ensure 16 byte size
    int     m_padding;
#endif
};

// PS3 DMA alignment
UMBRA_CT_ASSERT((sizeof(UserList<int>) & 0xf) == 0);

template<typename ELEM> UMBRA_INLINE bool UserList<ELEM>::pushBack(const ELEM& elem)
{
    bool ok = (m_size < m_capacity);
    if (ok)
    {
        if (isRemote())
            MemoryAccess::alignedWrite(&m_buf[m_size], &elem, sizeof(ELEM));
        else
            m_buf[m_size] = elem;
    }
    m_size++;
    return ok;
}

template<> UMBRA_INLINE bool UserList<int>::pushBack(const int& elem)
{
    bool ok = (m_size < m_capacity);
    if (ok)
    {
        if (isRemote())
            MemoryAccess::write32(&m_buf[m_size], elem);
        else
            m_buf[m_size] = elem;
    }
    m_size++;
    return ok;
}

template<> UMBRA_INLINE bool UserList<float>::pushBack(const float& elem)
{
    bool ok = (m_size < m_capacity);
    if (ok)
    {
        if (isRemote())
            MemoryAccess::write32(&m_buf[m_size], (const UINT32&)elem);
        else
            m_buf[m_size] = elem;
    }
    m_size++;
    return ok;
}

template<> UMBRA_INLINE int UserList<int>::get(int pos) const
{
    UMBRA_ASSERT(pos >= 0 && pos < m_size);
    if (isRemote())
        return MemoryAccess::read32(&m_buf[pos]);
    return m_buf[pos];
}

template<typename ELEM> UMBRA_INLINE void UserList<ELEM>::updateRemote(UserList<ELEM>* dst) const
{
#ifdef UMBRA_REMOTE_MEMORY
    int size = getSize();
    MemoryAccess::write32(&dst->m_size, (UINT32&)size);
#endif
    UMBRA_UNREF(dst);
}

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ImpPortalInfo
{
public:
    ImpPortalInfo(void): m_tome(NULL), m_targetCluster(-1), m_numGateIndices(0), 
        m_planeEq(0, 0, 0, 0), m_minRadius(0), m_maxRadius(0),  m_numVertices(0), 
        m_portalGeometryOfs(-1), m_isUser(false) {}

    const ImpTome*  m_tome;

    int     m_targetCluster;
    int     m_numGateIndices;
    Vector3 m_aabbMin;
    Vector3 m_aabbMax;
    Vector3 m_center;
    Vector4 m_planeEq;
    float   m_minRadius;
    float   m_maxRadius;
    int     m_numVertices;
    int     m_portalGeometryOfs;
    int     m_gateIndices[UMBRA_MAX_GATE_INDICES];
    bool    m_isUser;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ImpPath : public UserList<Path::Node>
{
public:
    ImpPath(Path::Node* arr, int cap): UserList<Path::Node>(arr, cap) {}

    void reset (void)
    {
        clear();
    }

    bool pushNode (const Vector3& coord, int portalIndex, float dist)
    {
        Path::Node UMBRA_ATTRIBUTE_ALIGNED16(n);
        n.coord = coord;
        n.portalIndex = portalIndex;
        n.distanceFromStart = dist;
        n.reserved1 = 0;
        n.reserved2 = 0;
        n.reserved3 = 0;
        return pushBack(n);
    }
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ImpCameraTransform
{
public:
    ImpCameraTransform(void):
      m_transform(),
      m_userPlaneCount(0),
      m_depthRange(CameraTransform::DEPTHRANGE_ZERO_TO_ONE),
      m_view(),
      m_frustum(),
      m_mf(MF_COLUMN_MAJOR),
      m_separate(false) {}

    void update (void);

    Matrix4x4                   m_transform;
    Vector4                     m_userPlanes[UMBRA_MAX_USER_CLIP_PLANES];
    Vector3                     m_position;
    int                         m_userPlaneCount;
    CameraTransform::DepthRange m_depthRange;

    // deprecated
    Matrix4x4                   m_view;
    Frustum                     m_frustum;
    MatrixFormat                m_mf;
    bool                        m_separate;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

#define UMBRA_OCCLUSIONBUFFER_DEPTH_OFFSET UMBRA_ALIGN(sizeof(ImpOcclusionBuffer), 16)
#define UMBRA_OCCLUSIONBUFFER_DEPTH_SIZE (sizeof(float) * UMBRA_PORTAL_RASTER_SIZE * UMBRA_PORTAL_RASTER_SIZE)

/* All depth values in OcclusionBuffer manipulation functions must be in NDC space clamped to range [0,1] */
class UMBRA_ATTRIBUTE_ALIGNED16(ImpOcclusionBuffer)
{
public:
    enum
    {
        BlockSizeLog2       = 2, // 4x4 blocks
        BlockSize           = (1 << BlockSizeLog2),
        PixelsPerBlock      = (BlockSize * BlockSize),
        BlockBufferStride   = (UMBRA_PORTAL_RASTER_SIZE / BlockSize) * (BlockSize * BlockSize)  // in depth values (float)
    };

    ImpOcclusionBuffer (void): m_transformer(), m_depthBuffer(NULL), m_isValid(false) {}

    ImpOcclusionBuffer& operator= (const ImpOcclusionBuffer& o)
    {
        m_isValid = o.m_isValid;
        m_transformer = o.m_transformer;
        m_depthBuffer = NULL;
        if (o.m_depthBuffer)
        {
            m_depthBuffer = (float*)((UINT8*)this + UMBRA_OCCLUSIONBUFFER_DEPTH_OFFSET);
            memcpy(m_depthBuffer, o.m_depthBuffer, UMBRA_OCCLUSIONBUFFER_DEPTH_SIZE);
        }
        return *this;
    }

    void init (const Transformer& transformer, void* depth)
    {
        m_isValid = true;
        m_transformer = transformer;
        m_depthBuffer = depth;
    }

    UMBRA_INLINE bool isValid (void) const { return m_isValid; }

    static UMBRA_FORCE_INLINE float getMaxDepth()
    {
        return 1.f;
    }

    UMBRA_INLINE void* getDepthBufferPtr (bool forceLocal) const
    {
        if (forceLocal)
            return ((UINT8*)this + UMBRA_OCCLUSIONBUFFER_DEPTH_OFFSET);
        return m_depthBuffer;
    }

    UMBRA_INLINE void writeDepth (float depth, int x, int y)
    {
        UMBRA_ASSERT(isDepthInputValid(depth));
        int ofs = getPixelOffset(x, y);
        reinterpret_cast<float*>(m_depthBuffer)[ofs] = depth;
    }

    UMBRA_INLINE float readDepth (int x, int y) const
    {
        int ofs = getPixelOffset(x, y);
        return reinterpret_cast<const float*>(m_depthBuffer)[ofs];
    }

    static UMBRA_INLINE int getPixelOffset(int x, int y)
    {
        UMBRA_ASSERT(x >= 0 && x < UMBRA_PORTAL_RASTER_SIZE);
        UMBRA_ASSERT(y >= 0 && y < UMBRA_PORTAL_RASTER_SIZE);
        int ofs = (y >> BlockSizeLog2) * BlockBufferStride + (x >> BlockSizeLog2) * PixelsPerBlock;
        ofs += (y & 3) * BlockSize + (x & 3);
        return ofs;
    }

    bool                        isAABBVisible            (const Vector3& mn, const Vector3& mx, float* contribution) const;
    bool                        isAABBFullyVisible       (const Vector3& mn, const Vector3& mx) const;
    bool                        isAARectVisible          (const Vector2& clipMin, const Vector2& clipMax, float clipZ) const;
    bool                        isAARectFullyVisible     (const Vector2& clipMin, const Vector2& clipMax, float clipZ) const;

    OcclusionBuffer::ErrorCode  dumpDebugBuffer          (void* out, const OcclusionBuffer::BufferDesc& desc) const;

    void                        visualizeHull            (QueryContext* q) const;
    Transformer*                getTransformer           (void) { return &m_transformer; }
    void                        setTransformer           (const Transformer& transformer) { m_transformer = transformer; }
    const Transformer*          getTransformer           (void) const { return &m_transformer; }
    void                        combine                  (const ImpOcclusionBuffer& other);

private:

    OcclusionBuffer::ErrorCode  dump8bpp                 (Umbra::UINT8* out, const OcclusionBuffer::BufferDesc& desc) const;
    OcclusionBuffer::ErrorCode  dumpFloat                (float* out, const OcclusionBuffer::BufferDesc& desc) const;

    static UMBRA_FORCE_INLINE bool isDepthInputValid(float f)
    {
        return f >= 0.f && f <= 1.f;
    }

    UMBRA_INLINE float convertUserFloat(float f) const
    {
        if (m_transformer.getDepthRange() == CameraTransform::DEPTHRANGE_MINUS_ONE_TO_ONE)
            f = (f / 2.f) + 0.5f;
        UMBRA_ASSERT(isDepthInputValid(f));
        return f;
    }

    UMBRA_INLINE bool testDepth(float depth, int x, int y) const
    {
        int ofs = getPixelOffset(x, y);
        return floatBitPattern(depth) <= reinterpret_cast<const UINT32*>(m_depthBuffer)[ofs];
    }

    template<bool FULLY_VISIBLE>
    bool isPixelAARectVisible (const Vector2i& rasterMin, const Vector2i& rasterMax, float depth) const;

    template<bool FULLY_VISIBLE>
    bool isPixelAARectVisibleReference (const Vector2i& rasterMin, const Vector2i& rasterMax, float depth) const;

    Transformer m_transformer;
    void*       m_depthBuffer;
    bool        m_isValid;
};

// aligned size required for remote copying
UMBRA_CT_ASSERT((sizeof(ImpOcclusionBuffer) & 0xF) == 0);

// this represents the occlusion buffer layout in memory, for size assertion
// note: it is important for this to by POD
struct ImpOcclusionBufferMem
{
    UINT8 UMBRA_ALIGNED(16) imp[sizeof(ImpOcclusionBuffer)];
    UINT8 UMBRA_ALIGNED(16) mem[UMBRA_OCCLUSIONBUFFER_DEPTH_SIZE];
};

UMBRA_CT_ASSERT(offsetof(ImpOcclusionBufferMem, mem) == UMBRA_OCCLUSIONBUFFER_DEPTH_OFFSET);

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ImpReceiverMaskBuffer
{
public:

    void init(const UINT16* buffer, const Matrix4x4& worldToLightClip)
    {
        const int byteSize = UMBRA_RECEIVER_MASK_BUFFER_SIZE*UMBRA_RECEIVER_MASK_BUFFER_SIZE*sizeof(UINT16);
        memcpy(m_depthBuffer, buffer, byteSize);
        m_cameraTransform.set(worldToLightClip, Vector3(0,0,0), CameraTransform::DEPTHRANGE_ZERO_TO_ONE, MF_ROW_MAJOR);
    }

    int getWidth(void) const
    {
        return UMBRA_RECEIVER_MASK_BUFFER_SIZE;
    }

    int getHeight(void) const
    {
        return UMBRA_RECEIVER_MASK_BUFFER_SIZE;
    }

    float getDepth(int x, int y) const
    {
        UINT16 intDepth = m_depthBuffer[UMBRA_RECEIVER_MASK_BUFFER_SIZE*y+x];
        return float(intDepth)/65535.0f;
    }

    const CameraTransform& getCameraTransform(void) const
    {
        return m_cameraTransform;
    }

private:

    UINT16          m_depthBuffer[UMBRA_RECEIVER_MASK_BUFFER_SIZE*UMBRA_RECEIVER_MASK_BUFFER_SIZE];
    CameraTransform m_cameraTransform;
};


/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ImpVisibility
{
public:
    ImpVisibility(void): m_objects(NULL), m_clusters(NULL),
        m_occlusionBuffer(NULL), m_filter(NULL), m_inputBuffer(NULL),
        m_objectDistances(NULL), m_objectMask(NULL), m_objectContributions(NULL) {}

    IndexList*           m_objects;
    IndexList*           m_clusters;
    OcclusionBuffer*     m_occlusionBuffer;
    const IndexList*     m_filter;
    const OcclusionBuffer* m_inputBuffer;
    float*               m_objectDistances;
    uint32_t*            m_objectMask;
    float*               m_objectContributions;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ImpObjectDistanceParams
{
public:
    ImpObjectDistanceParams(void): m_reference(), m_scale(0.f), m_hasReference(false), m_minContribution(0.f) {}

    static float getEffectiveScale (const ImpObjectDistanceParams* imp)
    {
        if (!imp || (imp->m_scale <= 0.f))
            return 1.f;
        return min2(1.f, imp->m_scale);
    }

    static float getEffectiveScaleSqr (const ImpObjectDistanceParams* imp)
    {
        float scale = getEffectiveScale(imp);
        return scale * scale;
    }

    static Vector3 getEffectiveReference (const ImpObjectDistanceParams* imp, const Vector3& def)
    {
        if (!imp || !imp->m_hasReference)
            return def;
        return imp->m_reference;
    }
    
    static float getEffectiveMinContribution (const ImpObjectDistanceParams* imp)
    {
        if (!imp)
            return 0.f;
        return imp->m_minContribution;
    }

    Vector3              m_reference;
    float                m_scale;
    bool                 m_hasReference;
    float                m_minContribution;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class VisibilityResult
{
public:

    VisibilityResult(QueryContext& ctx, const Visibility& params, const Transformer& transform, bool hasDepth);
    ~VisibilityResult(void);

    QueryContext*           m_ctx;
    UserList<int>*          m_objects;
    UserList<int>*          m_clusters;
    UINT32*                 m_processedObjectVector;
    UINT32*                 m_visibleObjectVector;
    UINT32*                 m_clusterVector;
    ImpOcclusionBuffer*     m_occlusionBuffer;
    const void*             m_inputDepthBuffer;
    UserList<float>         m_objectDistances;
    UserList<float>         m_objectContributions;

    bool hasObjectVisibility() const { return m_objects || m_visibleObjectVector; }

private:
    UINTPTR                 m_remoteOcclusionBuffer;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class ImpLineSegmentQuery
{
public:
    ImpLineSegmentQuery()
    {
        q.m_result = LineSegmentQuery::RESULT_NO_INTERSECTION;
        q.m_objectSet = NULL;
    }

    ImpLineSegmentQuery(const Vector3& a, const Vector3& b, IndexList* list)
    {
        q.m_start = a;
        q.m_end = b;
        q.m_result = LineSegmentQuery::RESULT_NO_INTERSECTION;
        q.m_objectSet = list;
    }

    struct Data
    {
        Vector3                         m_start;
        Vector3                         m_end;

        LineSegmentQuery::ResultCode    m_result;
        IndexList*                      m_objectSet;
    } q;

    UINT8 padding[UMBRA_LINESEGMENTQUERY_SIZE - sizeof(Data)];

};

#if UMBRA_OS == UMBRA_PS3
// SPU DMA limitation
UMBRA_CT_ASSERT(32 * sizeof(LineSegmentQuery) < 16 * 1024);
#endif

}

#endif
