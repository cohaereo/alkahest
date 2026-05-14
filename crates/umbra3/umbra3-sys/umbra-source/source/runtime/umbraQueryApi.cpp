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
 * \brief   Umbra runtime
 *
 */

#include "umbraQueryContext.hpp"
#include "umbraQueryArgs.hpp"
#include "umbraTransformer.hpp"
#include "umbraPortalRayTracer.hpp"
#include "umbraConnectivity.hpp"
#include "umbraPortalCull.hpp"
#include "runtime/umbraQuery.hpp"
#include "umbraQueryWrapper.hpp"
#include "umbraShadows.hpp"
#include "umbraTomeCollection.hpp"
#include "umbraCubemap.hpp"
#if UMBRA_COMPILER == UMBRA_MSC
#include <new.h>
#else
#include <new>
#endif

#include "umbraSIMD.hpp"

#define HAS_WRAPPER (UMBRA_OS == UMBRA_PS3 && UMBRA_ARCH == UMBRA_PPC)

using namespace Umbra;

CHECK_APICLASS_STORAGE(ImpCameraTransform, UMBRA_CAMERA_TRANSFORM_SIZE);
CHECK_APICLASS_STORAGE(ImpOcclusionBufferMem, UMBRA_OCCLUSION_BUFFER_SIZE);
CHECK_APICLASS_STORAGE(ImpVisibility, UMBRA_VISIBILITY_SIZE);
CHECK_APICLASS_STORAGE(UserList<int>, UMBRA_INDEX_LIST_SIZE);
CHECK_APICLASS_STORAGE(ImpPath, UMBRA_PATH_SIZE);
CHECK_APICLASS_STORAGE(ImpPortalInfo, UMBRA_PORTALINFO_SIZE);
CHECK_APICLASS_STORAGE(ImpReceiverMaskBuffer, UMBRA_RECEIVER_MASK_BUFFER_BYTE_SIZE);
CHECK_APICLASS_STORAGE(ImpObjectDistanceParams, UMBRA_OBJECTDISTANCEPARAMS_SIZE);
CHECK_APICLASS_STORAGE(ImpShadowCuller, UMBRA_SHADOW_CULLER_SIZE);

#define APICLASS_COMMON(ApiClass) \
    ApiClass::ApiClass(const ApiClass& rhs) { *this = rhs; } \
    ApiClass& ApiClass::operator=(const ApiClass& rhs) { *IMPL(this) = *IMPL(&rhs); return *this; }

/////////////////////////////////////////////////////////////////////////////
//// Frustum ////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

Umbra::Frustum::Frustum(float fovY, float aspect, float inNear, float inFar) :
    left    (0.0f),
    right   (0.0f),
    top     (0.0f),
    bottom  (0.0f),
    zNear   (0.0f),
    zFar    (0.0f),
    type    (Frustum::PERSPECTIVE)
{
    if ((inNear > 0.0f) && (inFar > inNear) && (fovY > 0.0f) && (fovY < 3.14159252f) && (aspect > 0.0))
    {
        top     = inNear * tanf(0.5f*fovY);
        bottom  = -top;
        right   = aspect*top;
        left    = -right;
        zNear   = inNear;
        zFar    = inFar;
    }
}

/////////////////////////////////////////////////////////////////////////////
//// GateStateVector ////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

GateStateVector::GateStateVector(void) :
    m_data(NULL)
{}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

GateStateVector::GateStateVector(void* arr, size_t size, bool clear) : m_data(arr)
{
    if (clear)
        memset(m_data, 0xff, size); // open all portals by default
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void* GateStateVector::getPtr() const
{
    return m_data;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 * \todo error checking
 *//*----------------------------------------------------------------------*/

void GateStateVector::setState(int idx, bool open)
{
    UMBRA_ASSERT(idx >= 0);
    if (open)
        setBit((UINT32*)m_data, idx);
    else
        clearBit((UINT32*)m_data, idx);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

bool GateStateVector::getState(int idx) const
{
    UMBRA_ASSERT(idx >= 0);
    return (testBit((const UINT32*)m_data, idx) != 0);
}

/////////////////////////////////////////////////////////////////////////////
//// CameraTransform ////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(CameraTransform)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

CameraTransform::CameraTransform(void)
{
    new (IMPL(this)) ImpCameraTransform();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

CameraTransform::CameraTransform(
    const Matrix4x4& combined,
    const Vector3& pos,
    DepthRange dr,
    MatrixFormat mf)
{
    new (IMPL(this)) ImpCameraTransform();
    setMatrixFormat(mf);
    set(combined, pos, dr, mf);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void CameraTransform::set (const Matrix4x4& combined, const Vector3& position, DepthRange range, MatrixFormat mf)
{
    ImpCameraTransform* imp = IMPL(this);

    imp->m_transform = combined;
    if (mf == MF_COLUMN_MAJOR)
        imp->m_transform.transpose();
    if (range == DEPTHRANGE_MINUS_ONE_TO_ONE)
    {
        // adjust z range from -w,w to 0,w
        for (int i = 0; i < 4; i++)
            imp->m_transform[2][i] = 0.5f * (imp->m_transform[2][i] + imp->m_transform[3][i]);
    }
    imp->m_position     = position;
    imp->m_separate     = false;
    imp->m_depthRange   = range;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void CameraTransform::get(Matrix4x4& outWorldToClip, DepthRange inRange, MatrixFormat inMatrixFormat) const
{
    const ImpCameraTransform* imp = IMPL(this);

    if (imp->m_separate)
        ((ImpCameraTransform*)imp)->update();

    outWorldToClip = imp->m_transform;

    if (inRange == DEPTHRANGE_MINUS_ONE_TO_ONE)
    {
        Matrix4x4 outWorldToClip = imp->m_transform;
        Vector4 r(0,0,2,-1);
        for (int i = 0; i < 4; i++)
            outWorldToClip[2][i] = dot(r, outWorldToClip.getColumn(i));
    }

    if (inMatrixFormat == MF_COLUMN_MAJOR)
        outWorldToClip.transpose();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void CameraTransform::setUserClipPlanes(const Vector4* planes, int planeCount)
{
    UMBRA_ASSERT(planeCount >= 0);
    UMBRA_ASSERT(planeCount <= UMBRA_MAX_USER_CLIP_PLANES);
    UMBRA_ASSERT(planes || planeCount == 0);

    ImpCameraTransform* imp = IMPL(this);

    for (int i = 0; i < planeCount; i++)
        imp->m_userPlanes[i] = planes[i];
    imp->m_userPlaneCount = planeCount;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void CameraTransform::getUserClipPlanes(Vector4 outClipPlanes[UMBRA_MAX_USER_CLIP_PLANES], int& outPlaneCount)
{
    ImpCameraTransform* imp = IMPL(this);

    outPlaneCount = imp->m_userPlaneCount;
    for (int i = 0; i < outPlaneCount; i++)
        outClipPlanes[i] = imp->m_userPlanes[i];
}

// all of the below is deprecated

CameraTransform::CameraTransform(
    const Matrix4x4& cameraToWorld,
    const Frustum& f,
    MatrixFormat mf)
{
    new (IMPL(this)) ImpCameraTransform();
    setMatrixFormat(mf);
    setFrustum(f);
    setCameraToWorld(cameraToWorld);
}

MatrixFormat CameraTransform::getMatrixFormat(void) const
{
    const ImpCameraTransform* imp = IMPL(this);
    return imp->m_mf;
}

void CameraTransform::setMatrixFormat(MatrixFormat mf)
{
    ImpCameraTransform* imp = IMPL(this);
    imp->m_mf = mf;
}

void CameraTransform::setCameraToWorld (const Matrix4x4& matrix)
{
    ImpCameraTransform* imp = IMPL(this);
    imp->m_view = matrix;
    imp->m_view.invert(); // does this work for column major?
    imp->m_separate = true;
}

void CameraTransform::getCameraToWorld (Matrix4x4& matrix) const
{
    const ImpCameraTransform* imp = IMPL(this);
    matrix = imp->m_view;
    matrix.invert();
}

void CameraTransform::setWorldToCamera (const Matrix4x4& matrix)
{
    ImpCameraTransform* imp = IMPL(this);
    imp->m_view = matrix;
    imp->m_separate = true;
}

void CameraTransform::getWorldToCamera (Matrix4x4& matrix) const
{
    const ImpCameraTransform* imp = IMPL(this);
    matrix = imp->m_view;
}

void CameraTransform::setFrustum (const Frustum& frustum)
{
    ImpCameraTransform* imp = IMPL(this);
    imp->m_frustum = frustum;
    imp->m_separate = true;
}

void CameraTransform::setFrustum (float fovY, float aspect, float zNear, float zFar)
{
    Frustum f;
    float a = (float)tanf(0.5f * fovY * 3.14159265358f / 180.0f) * zNear;
    f.left = -a*aspect;
    f.right = a*aspect;
    f.bottom = -a;
    f.top = a;
    f.zNear = zNear;
    f.zFar = zFar;
    f.type = Frustum::PERSPECTIVE;
    setFrustum(f);
}

void CameraTransform::getFrustum (Frustum& frustum) const
{
    const ImpCameraTransform* imp = IMPL(this);
    frustum = imp->m_frustum;
}

/////////////////////////////////////////////////////////////////////////////
//// IndexList //////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(IndexList)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

IndexList::IndexList(void)
{
    new (IMPL(this)) UserList<int>(NULL, 0, 0);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

IndexList::IndexList(int* arr, int capacity, int size)
{
    new (IMPL(this)) UserList<int>(arr, capacity, size);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int* IndexList::getPtr (void) const
{
    return IMPL(this)->getBuf();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void IndexList::setPtr (int* arr)
{
    UserList<int>* list = IMPL(this);
    *list = UserList<int>(arr, list->getCapacity(), list->getSize());
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int IndexList::getCapacity(void) const
{
    return IMPL(this)->getCapacity();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void IndexList::setCapacity(int capacity)
{
    UserList<int>* list = IMPL(this);
    *list = UserList<int>(list->getBuf(), capacity, list->getSize());
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int IndexList::getSize(void) const
{
    return IMPL(this)->getSize();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void IndexList::setSize(int size)
{
    UserList<int>* list = IMPL(this);
    *list = UserList<int>(list->getBuf(), list->getCapacity(), size);
}

/////////////////////////////////////////////////////////////////////////////
//// FloatList //////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(FloatList)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

FloatList::FloatList(void)
{
    new (IMPL(this)) UserList<float>(NULL, 0, 0);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

FloatList::FloatList(float* arr, int capacity, int size)
{
    new (IMPL(this)) UserList<float>(arr, capacity, size);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

float* FloatList::getPtr (void) const
{
    return IMPL(this)->getBuf();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void FloatList::setPtr (float* arr)
{
    UserList<float>* list = IMPL(this);
    *list = UserList<float>(arr, list->getCapacity(), list->getSize());
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int FloatList::getCapacity(void) const
{
    return IMPL(this)->getCapacity();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void FloatList::setCapacity(int capacity)
{
    UserList<float>* list = IMPL(this);
    *list = UserList<float>(list->getBuf(), capacity, list->getSize());
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int FloatList::getSize(void) const
{
    return IMPL(this)->getSize();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void FloatList::setSize(int size)
{
    UserList<float>* list = IMPL(this);
    *list = UserList<float>(list->getBuf(), list->getCapacity(), size);
}

/////////////////////////////////////////////////////////////////////////////
//// Visibility /////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(Visibility)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Visibility::Visibility(void)
{
    new (IMPL(this)) ImpVisibility();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Visibility::Visibility(IndexList* objects, OcclusionBuffer* buffer)
{
    ImpVisibility* imp = new (IMPL(this)) ImpVisibility();
    imp->m_objects = objects;
    imp->m_occlusionBuffer = buffer;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setOutputObjects (IndexList* objects)
{
    IMPL(this)->m_objects = objects;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

IndexList* Visibility::getOutputObjects (void) const
{
    return IMPL(this)->m_objects;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setOutputClusters (IndexList* clusters)
{
    IMPL(this)->m_clusters = clusters;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

IndexList* Visibility::getOutputClusters (void) const
{
    return IMPL(this)->m_clusters;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setOutputBuffer (OcclusionBuffer* buffer)
{
    IMPL(this)->m_occlusionBuffer = buffer;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer* Visibility::getOutputBuffer (void) const
{
    return IMPL(this)->m_occlusionBuffer;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setInputBuffer (const OcclusionBuffer* buffer)
{
    IMPL(this)->m_inputBuffer = buffer;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

const OcclusionBuffer* Visibility::getInputBuffer (void) const
{
    return IMPL(this)->m_inputBuffer;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setInputObjects (const IndexList* objectMask)
{
    IMPL(this)->m_filter = objectMask;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

const IndexList* Visibility::getInputObjects (void) const
{
    return IMPL(this)->m_filter;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setOutputObjectDistances (float* distances)
{
    IMPL(this)->m_objectDistances = distances;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

float* Visibility::getOutputObjectDistances (void) const
{
    return IMPL(this)->m_objectDistances;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setOutputObjectMask(uint32_t* mask)
{
    IMPL(this)->m_objectMask = mask;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Visibility::setOutputContributions (float* contributions)
{
    IMPL(this)->m_objectContributions = contributions;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

uint32_t* Visibility::getOutputObjectMask(void) const
{
    return IMPL(this)->m_objectMask;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

float* Visibility::getOutputContributions (void) const
{
    return IMPL(this)->m_objectContributions;
}

/////////////////////////////////////////////////////////////////////////////
//// ObjectDistanceParams ///////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(ObjectDistanceParams)

    
ObjectDistanceParams::ObjectDistanceParams(void)
{
    new (IMPL(this)) ImpObjectDistanceParams();
}

ObjectDistanceParams::ObjectDistanceParams(const Vector3* referencePt, float distanceScale)
{
    new (IMPL(this)) ImpObjectDistanceParams();
    setReferencePoint(referencePt);
    setDistanceScale(distanceScale);
}

void ObjectDistanceParams::setReferencePoint (const Vector3* referencePt)
{
    ImpObjectDistanceParams* imp = IMPL(this);
    imp->m_hasReference = (referencePt != NULL);
    if (referencePt)
        imp->m_reference = *referencePt;
}

bool ObjectDistanceParams::getReferencePoint (Vector3& referencePtOut) const
{
    const ImpObjectDistanceParams* imp = IMPL(this);
    if (imp->m_hasReference)
        referencePtOut = imp->m_reference;
    return imp->m_hasReference;
}

void ObjectDistanceParams::setDistanceScale (float scale)
{
    ImpObjectDistanceParams* imp = IMPL(this);
    imp->m_scale = scale;
}

float ObjectDistanceParams::getDistanceScale (void) const
{
    const ImpObjectDistanceParams* imp = IMPL(this);
    return imp->m_scale;
}

void ObjectDistanceParams::setMinRelativeContribution (float contribution)
{    
    IMPL(this)->m_minContribution = contribution;
}

float ObjectDistanceParams::getMinRelativeContribution (void) const
{
    return IMPL(this)->m_minContribution;
}

/////////////////////////////////////////////////////////////////////////////
//// OcclusionBuffer ////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(OcclusionBuffer)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer::OcclusionBuffer(void)
{
    new (IMPL(this)) ImpOcclusionBuffer();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer::VisibilityTestResult OcclusionBuffer::testAABBVisibility(const Vector3& mn, const Vector3& mx, Umbra::UINT32 flags, float* contribution) const
{
    const ImpOcclusionBuffer* imp = IMPL(this);
    if (!imp->isValid())
        return (flags & OcclusionBuffer::TEST_FULL_VISIBILITY) ? FULLY_VISIBLE : VISIBLE;

    UINT32 fpState = SIMDSaveState();

    bool visible = imp->isAABBVisible(mn, mx, contribution);

    VisibilityTestResult ret = visible ? VISIBLE : OCCLUDED;
    if (visible && (flags & OcclusionBuffer::TEST_FULL_VISIBILITY))
    {
        if (imp->isAABBFullyVisible(mn, mx))
            ret = FULLY_VISIBLE;
    }

    SIMDRestoreState(fpState);
    return ret;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer::VisibilityTestResult OcclusionBuffer::testAARectVisibility(const Vector2& mn, const Vector2& mx, float z, Umbra::UINT32 flags) const
{
    const ImpOcclusionBuffer* imp = IMPL(this);
    if (!imp->isValid())
        return (flags & OcclusionBuffer::TEST_FULL_VISIBILITY) ? FULLY_VISIBLE : VISIBLE;
    UINT32 fpState = SIMDSaveState();
    VisibilityTestResult ret = imp->isAARectVisible(mn, mx, z) ? VISIBLE : OCCLUDED;
    if (ret == VISIBLE && (flags & OcclusionBuffer::TEST_FULL_VISIBILITY))
        ret = imp->isAARectFullyVisible(mn, mx, z) ? FULLY_VISIBLE : VISIBLE;
    SIMDRestoreState(fpState);
    return ret;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

OcclusionBuffer::ErrorCode OcclusionBuffer::getBuffer(void* data, const BufferDesc* desc) const
{
    const ImpOcclusionBuffer* imp = IMPL(this);
    if (!data)
        return ERROR_INVALID_POINTER;
    if (!imp->isValid())
        return ERROR_EMPTY_BUFFER;

    BufferDesc defaultDesc;
    // Accept format 0, the former default
    if (!desc || desc->format == 0)
    {
        getBufferDesc(defaultDesc);
        desc = &defaultDesc;
    }
    return imp->dumpDebugBuffer(data, *desc);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int OcclusionBuffer::getWidth (void) const
{
    return UMBRA_PORTAL_RASTER_SIZE;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int OcclusionBuffer::getHeight (void) const
{
    return UMBRA_PORTAL_RASTER_SIZE;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void OcclusionBuffer::combine(const OcclusionBuffer& other_)
{
    ImpOcclusionBuffer* imp = IMPL(this);
    const ImpOcclusionBuffer* other = IMPL(&other_);
    if (!other->isValid())
        return;
    if (!imp->isValid())
        *imp = *other;
    else
        imp->combine(*other);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void OcclusionBuffer::clear(void)
{
    ImpOcclusionBuffer* imp = IMPL(this);
    *imp = ImpOcclusionBuffer();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void OcclusionBuffer::setCameraTransform(const CameraTransform& src)
{
    ImpOcclusionBuffer* imp = IMPL(this);
    Transformer* old = imp->getTransformer();
    float prediction;
    SIMDStore(old->getPrediction(), prediction);
    Transformer transformer(*IMPL(&src), prediction, old->getThreadId(), old->getNumThreads(), old->getXSplits());
    imp->setTransformer(transformer);
}

/////////////////////////////////////////////////////////////////////////////
//// ReceiverMaskBuffer /////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(ReceiverMaskBuffer)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

ReceiverMaskBuffer::ReceiverMaskBuffer(void)
{
    memset(m_mem, 0, UMBRA_RECEIVER_MASK_BUFFER_BYTE_SIZE);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

const CameraTransform& ReceiverMaskBuffer::getCameraTransform(void) const
{
    return IMPL(this)->getCameraTransform();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

float ReceiverMaskBuffer::getDepth(int x, int y) const
{
    return IMPL(this)->getDepth(x, y);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int ReceiverMaskBuffer::getWidth(void) const
{
   return IMPL(this)->getWidth();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int ReceiverMaskBuffer::getHeight(void) const
{
   return IMPL(this)->getHeight();
}

/////////////////////////////////////////////////////////////////////////////
//// Path ///////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(Path)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Path::Path(void)
{
    new (IMPL(this)) ImpPath(NULL, 0);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Path::Path(Node* arr, int capacity)
{
    new (IMPL(this)) ImpPath(arr, capacity);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

float Path::getLength(void) const
{
    if (!getNumNodes())
        return -1.f;
    return getNodes()[getNumNodes() - 1].distanceFromStart;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int Path::getNumNodes(void) const
{
    return IMPL(this)->getSize();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int Path::getCapacity(void) const
{
    return IMPL(this)->getCapacity();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Path::Node* Path::getNodes(void) const
{
    return IMPL(this)->getBuf();
}

/////////////////////////////////////////////////////////////////////////////
//// PortalInfo /////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

APICLASS_COMMON(PortalInfo)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

PortalInfo::PortalInfo(void)
{
    new (IMPL(this)) ImpPortalInfo();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void PortalInfo::getCenter (Vector3& center) const
{
    center = IMPL(this)->m_center;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int PortalInfo::getGateIndex(int i) const
{
    const ImpPortalInfo* imp = IMPL(this);
    UMBRA_ASSERT(i >= 0 && i < imp->m_numGateIndices);
    return imp->m_gateIndices[i];
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int PortalInfo::getNumGateIndices(void) const
{
    return IMPL(this)->m_numGateIndices;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int PortalInfo::getTargetCluster (void) const
{
    return IMPL(this)->m_targetCluster;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void PortalInfo::getAABB(Vector3& mn, Vector3& mx) const
{
    const ImpPortalInfo* imp = IMPL(this);

    if (imp->m_isUser)
    {
        DataArray vertices = imp->m_tome->getGateVertices();
        AABB aabb;
        for (int i = 0; i < imp->m_numVertices; i++)
        {
            Vector3 v;
            vertices.getElem(v, imp->m_portalGeometryOfs + i);
            aabb.grow(v);
        }
        mn = aabb.getMin();
        mx = aabb.getMax();
    }
    else
    {
        mn = imp->m_aabbMin;
        mx = imp->m_aabbMax;
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int PortalInfo::getNumTriangles(void) const
{
    const ImpPortalInfo* imp = IMPL(this);
    if (imp->m_isUser)
        return imp->m_numVertices - 2;
    else
        return 0;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void PortalInfo::getTriangle(int idx, Vector3& a, Vector3& b, Vector3& c) const
{
    const ImpPortalInfo* imp = IMPL(this);
    UMBRA_ASSERT(imp->m_isUser && idx < imp->m_numVertices - 2);

    DataArray vertices = imp->m_tome->getGateVertices();
    vertices.getElem(a, imp->m_portalGeometryOfs);
    vertices.getElem(b, imp->m_portalGeometryOfs + 1 + idx);
    vertices.getElem(c, imp->m_portalGeometryOfs + 2 + idx);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

int PortalInfo::getNumHullVertices(void) const
{
    return IMPL(this)->m_numVertices;
}

/*----------------------------------------------------------------------*//*!
 * \brief Get vertices of the convex hull that forms the user portal
 *//*----------------------------------------------------------------------*/

void PortalInfo::getHullVertex(int idx, Vector3& coord) const
{
    const ImpPortalInfo* imp = IMPL(this);
    UMBRA_ASSERT(imp->m_isUser);
    UMBRA_ASSERT(idx < imp->m_numVertices);
    imp->m_tome->getGateVertices().getElem(coord, imp->m_portalGeometryOfs + idx);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void PortalInfo::getBoundingCircles(Vector3& center, float& minRadius, float& maxRadius, Vector4 &planeEq) const
{
    const ImpPortalInfo* imp = IMPL(this);
    UMBRA_ASSERT(imp->m_isUser);
    center = imp->m_center;
    minRadius = imp->m_minRadius;
    maxRadius = imp->m_maxRadius;
    planeEq = imp->m_planeEq;
}

/////////////////////////////////////////////////////////////////////////////
//// Query //////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

static bool validateThreadId (int id, int n, int width)
{
    // supported number of jobs
    if (n <= 0 || n > UMBRA_MAX_FRUSTUM_SPLITS * UMBRA_MAX_FRUSTUM_SPLITS)
        return false;
    // id must be within range
    if (id < 0 || id >= n)
        return false;
    // width must be in range
    if (width < 0 || width > UMBRA_MAX_FRUSTUM_SPLITS)
        return false;
    if (width)
    {
        // with explicit width, numJobs must be exactly a multiple
        if ((n % width) != 0)
            return false;
        if ((n / width) > UMBRA_MAX_FRUSTUM_SPLITS)
            return false;
    }

    return true;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::Query(void)
{
    init((const Tome*)NULL);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::Query(const Tome* tome)
{
    init(tome);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::Query(const TomeCollection* tomes)
{
    init(tomes);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::Query(const Query& rhs)
{
    *IMPL(this) = *IMPL(&rhs);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::~Query(void)
{
    deinit();
}

static inline QueryState* initQueryState(Umbra::UINT8* mem, size_t size)
{
    Umbra::UINT8* statePtr = (Umbra::UINT8*)IMPL((Query*)mem);
    UMBRA_ASSERT(statePtr != NULL);
    Umbra::UINT8* workPtr = statePtr + sizeof(QueryState);
    workPtr = (Umbra::UINT8*)UMBRA_ALIGN(workPtr, 16);
    size_t workSize = (mem + size) - workPtr;
    UMBRA_ASSERT(workPtr + workSize == mem + size);
    return new (statePtr) QueryState(workPtr, workSize);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::init(const Tome* tome)
{
    QueryState* imp = initQueryState(m_mem, sizeof(m_mem));
    imp->setQueryData((const ImpTome*)tome, NULL);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::init(const TomeCollection* tomes)
{
    if (!tomes)
        return;
    QueryState* imp = initQueryState(m_mem, sizeof(m_mem));
    imp->setQueryData(NULL, (const ImpTomeCollection*)tomes);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::deinit(void)
{
    // nothing to do
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query& Query::operator= (const Query& rhs)
{
    const QueryState* o = IMPL(&rhs);
    if (o->getCollection())
        init((const TomeCollection*)o->getCollection());
    else
        init((const Tome*)o->getRootTome());
    // TODO: copy rest of state over?
    return *this;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::setDebugRenderer (DebugRenderer* debug)
{
    IMPL(this)->setDebugRenderer(debug);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::setGateStates (const Umbra::GateStateVector* portals)
{
    IMPL(this)->setGateStates((const UINT32*)portals->getPtr());
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::setGatePathCosts (const float* portals, bool additive)
{
    IMPL(this)->setGateCosts(portals, additive);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::setThreadId (Umbra::UINT32 threadId)
{
    IMPL(this)->setSpuUsage((Query::SpuUsage)(threadId + 1));
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void Query::setSpuUsage (Query::SpuUsage usage)
{
    IMPL(this)->setSpuUsage(usage);
}

void Query::deinitSpu(void)
{
    if (HAS_WRAPPER)
    {
        QueryWrapper::deinit();
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode Query::queryFrustumVisibility(Umbra::UINT32 flags,
    const Visibility& params, const CameraTransform& src, float distance,
    const ObjectDistanceParams* objDist, int threadId, int numThreads)
{
    QueryContext ctx(IMPL(this), flags);
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;
    if (!validateThreadId(threadId, numThreads, 0))
        return Query::ERROR_INVALID_ARGUMENT;
    
    if (!HAS_WRAPPER || (ctx.getState()->getSpuUsage() == Query::SPU_USAGE_NONE))
    {
        const ImpCameraTransform* camera = IMPL(&src);
        Transformer transform(*camera, distance);

#if UMBRA_ARCH != UMBRA_SPU
        // spu binary size optimization
        if (flags & DEBUGFLAG_VIEW_FRUSTUM)
            ctx.visualizeFrustum(transform);
#endif

        VisibilityResult vis(ctx, params, transform, false);

        float minRelativeContribution = ImpObjectDistanceParams::getEffectiveMinContribution(IMPL(objDist));
        bool computeContributions = minRelativeContribution > 0.f || vis.m_objectContributions.getBuf();
        SIMDRegister32 scissor = transform.getScissorSIMD();

        if (vis.hasObjectVisibility())
        {
            SIMDRegister lodRef = SIMDLoadW1(ImpObjectDistanceParams::getEffectiveReference(IMPL(objDist), transform.getCameraPos()));
            SIMDRegister lodScaleSIMDSqr = SIMDLoad(ImpObjectDistanceParams::getEffectiveScaleSqr(IMPL(objDist)));

        	bool    isOrtho = transform.isOrtho();
            Vector3 cam     = transform.getCameraPos();
            Vector3 fwd     = -transform.getNearPlane().xyz();
            ActivePlaneSet  tomeActivePlanes, collectionActivePlanes;
            ActivePlaneSet* collectionActivePlanesPtr = NULL;
            tomeActivePlanes.numPlanes = 0;
            bool depthMapsEnabled = !(flags & Query::QUERYFLAG_IGNORE_OBJECT_OPTIMIZATIONS);
            bool depthMapsPresent = false;

            if (ctx.getState()->getNumTomeContexts() > 1)
            {
                SIMDRegister vmn = SIMDLoadW1(ctx.getTome()->getTreeMin());
                SIMDRegister vmx = SIMDLoadW1(ctx.getTome()->getTreeMax());
                transform.computeActivePlaneSet(collectionActivePlanes, vmn, vmx);
                collectionActivePlanesPtr = &collectionActivePlanes;
            }

            bool distanceScaleIsZero = objDist && IMPL(objDist)->m_scale == 0.f;

            DepthmapReader reader;
            Vector3i       fwdMapped;

            ObjectIterator<true> objectIterator(&ctx, false, threadId, numThreads);
            while (objectIterator.hasMoreTomes() || objectIterator.hasMoreObjects())
            {
                if (!objectIterator.hasMoreObjects())
                {
                    UMBRA_ASSERT(objectIterator.hasMoreTomes());
                    bool inFrustum = false;
                    do
                    {
                        objectIterator.nextTome();
                        const ImpTome* current = objectIterator.getCurrentTome();
                        SIMDRegister vmn = SIMDLoadW1(current->getTreeMin());
                        SIMDRegister vmx = SIMDLoadW1(current->getTreeMax());
                        inFrustum = transform.frustumTestBounds(collectionActivePlanesPtr, vmn, vmx);
                        if (inFrustum)
                            transform.computeActivePlaneSet(tomeActivePlanes, vmn, vmx);
                    } while (!inFrustum && objectIterator.hasMoreTomes());
                        
                    if (!inFrustum || (!objectIterator.hasMoreTomes() && !objectIterator.hasMoreObjects()))
                        break;

                    if (depthMapsEnabled)
                    {
                        depthMapsPresent = objectIterator.getCurrentTome()->hasObjectDepthmaps();
                        if (depthMapsPresent)
                            reader.init(objectIterator.getCurrentTome());
                        fwdMapped = DepthmapReader::map(fwd);
                    }

                    if (UMBRA_OPT_LARGE_FOOTPRINT && 
                        tomeActivePlanes.numPlanes == 0 && 
                        (depthMapsPresent || objectIterator.getCurrentTome()->hasObjectShadowmaps()) &&
                        isOrtho && 
                        (!objectIterator.hasDistances() || distanceScaleIsZero) &&
                        vis.m_objectDistances.getBuf() == NULL && 
                        !computeContributions)
                    {
                        DepthmapReaderDirectional reader(fwdMapped);
                        reader.init(objectIterator.getCurrentTome());

                        while (objectIterator.hasMoreObjects())
                        {
                            objectIterator.nextObject();
                            //objectIterator.fetchBounds();

                            int localIdx  = objectIterator.getLocalIdx();
                            int globalIdx = objectIterator.getGlobalIdx();
                                                        
                            if (!reader.test(localIdx))
                                continue;

                            UMBRA_PREFETCH(vis.m_processedObjectVector + (globalIdx >> 5));
                            if (testBit(vis.m_processedObjectVector, globalIdx))
                                continue;
                            setBit(vis.m_processedObjectVector, globalIdx);

                            if (vis.m_objects)
                                vis.m_objects->pushBack(globalIdx);
                            if (vis.m_visibleObjectVector)
                                setBit(vis.m_visibleObjectVector, globalIdx);
                        }

                        continue;
                    }
                }

                objectIterator.nextObject();
                objectIterator.fetchBounds();

                int localIdx  = objectIterator.getLocalIdx();
                int globalIdx = objectIterator.getGlobalIdx();

                if (testBit(vis.m_processedObjectVector, globalIdx))
                    continue;

                const ObjectBounds&   o    = objectIterator.getObjectBounds();
                SIMDRegister vmn = SIMDLoadW1(o.mn);
                SIMDRegister vmx = SIMDLoadW1(o.mx);

                const ObjectDistance& dist = objectIterator.getObjectDistance();

                if (!transform.frustumTestBounds(&tomeActivePlanes, vmn, vmx))
                    continue;

                if (objectIterator.hasDistances() && !distanceScaleIsZero && !distanceInRange(lodRef, dist, lodScaleSIMDSqr))
                    continue;

                if (depthMapsPresent && !isOrtho && !reader.testPosition(localIdx, cam))
                    continue;
                if (depthMapsPresent && isOrtho  && !reader.testDirection(localIdx, fwdMapped, transform.getNearPlane()))
                    continue;

                float contribution = 1.f;
                if (computeContributions)
                {
                    Vector4i UMBRA_ATTRIBUTE_ALIGNED16(mnmx);
                    transform.transformBox(mnmx, vmn, vmx, true, scissor, contribution);

                    if (contribution < minRelativeContribution)
                        continue;
                }

                setBit(vis.m_processedObjectVector, globalIdx);
                if (vis.m_objects)
                    vis.m_objects->pushBack(globalIdx);
                if (vis.m_visibleObjectVector)
                    setBit(vis.m_visibleObjectVector, globalIdx);

                if (vis.m_objectDistances.getBuf())
                {                    
                    SIMDRegister distMn = SIMDLoad(objectIterator.hasDistances() ? (float*)&dist.boundMin : (float*)&o.mn);
                    SIMDRegister distMx = SIMDLoad(objectIterator.hasDistances() ? (float*)&dist.boundMax : (float*)&o.mx);
                    float d;
                    SIMDStore(distanceAABBPointSqrSIMD(lodRef, distMn, distMx), d);
                    vis.m_objectDistances.pushBack(d);
                }

                if (vis.m_objectContributions.getBuf())
                    vis.m_objectContributions.pushBack(contribution);
            }
        }
        /* \todo [antti 18.11.2011]: clusters */
    }
    else
    {
        // QueryWrapper doesn't support contribution stuff due to SPU size restriction. The SPU lib itself does though.
        if (ImpObjectDistanceParams::getEffectiveMinContribution(IMPL(objDist)) > 0.f || params.getOutputContributions() != NULL)
            return Query::ERROR_UNSUPPORTED_OPERATION;

        QueryWrapper wrapper(ctx);
        wrapper.put(&flags, QueryParam_Input);
        wrapper.put(&params);
        wrapper.put(&src, QueryParam_Input);
        wrapper.put(&distance, QueryParam_Input);
        wrapper.put(objDist, QueryParam_Input);
        wrapper.put(&threadId, QueryParam_Input);
        wrapper.put(&numThreads, QueryParam_Input);
        wrapper.dispatch(QID_QUERY_FRUSTUMVISIBILITY);
    }
    return (Query::ErrorCode)ctx.getError();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode Query::queryPortalVisibility (Umbra::UINT32 flags, const Visibility& params,
    const CameraTransform& src, float distance, float clusterThreshold, 
    const ObjectDistanceParams* objDist, int threadId, int numThreads, int xSplits)
{
    QueryContext ctx(IMPL(this), flags);
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;
    if (!validateThreadId(threadId, numThreads, xSplits))
        return Query::ERROR_INVALID_ARGUMENT;

    const ImpCameraTransform& cam = *IMPL(&src);
    // \todo [Hannu] 7380 hang fix, portal query hangs with scaled identity matrix
            
#if 1
    if (!cam.m_separate &&
        cam.m_transform[0][0] == cam.m_transform[1][1] && cam.m_transform[1][1] == cam.m_transform[2][2] && // diagonal
        cam.m_transform[0][1] == 0.f && cam.m_transform[0][2] == 0.f && cam.m_transform[1][2] == 0.f &&
        cam.m_transform[1][0] == 0.f && cam.m_transform[2][0] == 0.f && cam.m_transform[2][1] == 0.f)
    {
        ctx.setError(Query::ERROR_INVALID_ARGUMENT);
        return (Query::ErrorCode)ctx.getError();
    }
#endif

    if (!HAS_WRAPPER ||
        (ctx.getState()->getSpuUsage() == Query::SPU_USAGE_NONE) ||
        (flags & Query::QUERYFLAG_REFERENCE))
    {
        Transformer transform(cam, distance, threadId, numThreads, xSplits);

#if UMBRA_ARCH != UMBRA_SPU
        // spu binary size optimization
        if (flags & DEBUGFLAG_VIEW_FRUSTUM)
            ctx.visualizeFrustum(transform);
#endif

        VisibilityResult vis(ctx, params, transform, true);
        if (ctx.hasError())
            return (Query::ErrorCode)ctx.getError();

        if (!(flags & Query::QUERYFLAG_REFERENCE))
        {
            AABB invalid;
            PortalCuller* pr = UMBRA_HEAP_NEW(ctx.getAllocator(), PortalCuller,
                &ctx, &transform, clusterThreshold, IMPL(objDist));
            if (!pr)
                return Query::ERROR_OUT_OF_MEMORY;
            if (!ctx.hasError())
                ctx.setError(pr->execute(vis, (flags & Query::QUERYFLAG_IGNORE_OBJECT_OPTIMIZATIONS) == 0, (flags & Query::QUERYFLAG_IGNORE_CAMERA_POSITION) != 0, invalid, -1));
            UMBRA_HEAP_DELETE(ctx.getAllocator(), pr);
        }
        else
        {
#if UMBRA_ARCH != UMBRA_SPU
            PortalRayTracer* pr = UMBRA_HEAP_NEW(ctx.getAllocator(), PortalRayTracer, &ctx, transform.getCameraPos(), IMPL(objDist), &transform);
            if (!pr)
                return Query::ERROR_OUT_OF_MEMORY;
            ctx.setError(pr->execute(vis));
            UMBRA_HEAP_DELETE(ctx.getAllocator(), pr);
#else
            ctx.setError(Query::ERROR_UNSUPPORTED_OPERATION);
#endif
        }
    }
    else
    {
        // QueryWrapper doesn't support contribution stuff due to SPU size restriction. The SPU lib itself does though.
        if (ImpObjectDistanceParams::getEffectiveMinContribution(IMPL(objDist)) > 0.f || params.getOutputContributions() != NULL)
            return Query::ERROR_UNSUPPORTED_OPERATION;

        QueryWrapper wrapper(ctx);
        wrapper.put(&flags, QueryParam_Input);
        wrapper.put(&params);
        wrapper.put(&src, QueryParam_Input);
        wrapper.put(&distance, QueryParam_Input);
        wrapper.put(&clusterThreshold, QueryParam_Input);
        wrapper.put(objDist, QueryParam_Input);
        wrapper.put(&threadId, QueryParam_Input);
        wrapper.put(&numThreads, QueryParam_Input);
        wrapper.put(&xSplits, QueryParam_Input);
        wrapper.dispatch(QID_QUERY_PORTALVISIBILITY_CAMERA);
    }

#if UMBRA_ARCH != UMBRA_SPU
    // Resulting hull visualization
    if (!ctx.hasError() && ctx.debugEnabled(Query::DEBUGFLAG_VISIBILITY_LINES))
    {
        const ImpVisibility* prms = IMPL(&params);
        if (prms->m_occlusionBuffer)
            IMPL(prms->m_occlusionBuffer)->visualizeHull(&ctx);
    }
#endif

    return (Query::ErrorCode)ctx.getError();
}

/////////////////////////////////////////////////////////////////////////////
//// QueryExt ///////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

QueryExt::QueryExt(void) :
    Query()
{
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

QueryExt::QueryExt(const Tome* tome) :
    Query(tome)
{
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

QueryExt::QueryExt(const TomeCollection* tomes) :
    Query(tomes)
{
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode QueryExt::setWorkMem(Umbra::UINT8* workMem, size_t workMemSize)
{
#if UMBRA_ARCH == UMBRA_SPU
	return Query::ERROR_UNSUPPORTED_OPERATION;
#else
    if (workMem && !workMemSize)
        return Query::ERROR_INVALID_ARGUMENT;

    IMPL(this)->setWorkMem(workMem, workMemSize);

    return Query::ERROR_OK;
#endif
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

size_t QueryExt::getMemoryRequirement (QueryType type, const Tome* tome)
{
#if UMBRA_ARCH == UMBRA_SPU
    return 0;
#else
    if (type != QUERYTYPE_SHORTEST_PATH)
        return 0;
    if (!tome)
        return 0;

    /* \todo [antti 6.11.2012]: use QueryState for this */
    UINT8 UMBRA_ATTRIBUTE_ALIGNED(16, tmp[sizeof(ImpTome) + 32]);
    StackAlloc alloc(tmp, sizeof(tmp));
    const ImpTome* t = (const ImpTome*)QueryState::importRemoteObj(alloc, tome, sizeof(ImpTome));
    if (((const Tome*)t)->getStatus() != Tome::STATUS_OK)
    {
        QueryState::freeRemoteObj(alloc, (void*)t);
        return 0;
    }
    size_t ret = PathFinder::getMemoryRequirement(t);
    QueryState::freeRemoteObj(alloc, (void*)t);
    return ret;
#endif
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode QueryExt::queryShortestPath (Umbra::UINT32 flags, Path& p_, const Umbra::Vector3& start, const Umbra::Vector3& end)
{
    QueryContext ctx(IMPL(this), flags);
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;

    if (!HAS_WRAPPER || (ctx.getState()->getSpuUsage() == Query::SPU_USAGE_NONE))
    {
        ImpPath& p = *IMPL(&p_);
        p.reset();
        PathFinder* pf = UMBRA_HEAP_NEW(ctx.getAllocator(), PathFinder, ctx);
        if (!pf)
            return Query::ERROR_OUT_OF_MEMORY;
        if (ctx.getError() == Query::ERROR_OK)
            pf->find(p, start, end);
        UMBRA_HEAP_DELETE(ctx.getAllocator(), pf);
    }
    else
    {
        QueryWrapper wrapper(ctx);
        wrapper.put(&flags, QueryParam_Input);
        wrapper.put(&p_, QueryParam_Input | QueryParam_Output);
        wrapper.put(&start, QueryParam_Input);
        wrapper.put(&end, QueryParam_Input);
        wrapper.dispatch(QID_QUERY_SHORTESTPATH);
    }
    return (Query::ErrorCode)ctx.getError();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode QueryExt::queryClusterForPoint (const Umbra::Vector3& pt, int& cluster)
{
    QueryContext ctx(IMPL(this));
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;
    cluster = ctx.findCluster(pt);
    cluster = ctx.getDefaultTome().mapLocalCluster(cluster);
    if (!ctx.hasError() && (cluster == -1))
        return Query::ERROR_OUTSIDE_SCENE;
    return (Query::ErrorCode)ctx.getError();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode QueryExt::queryConnectedRegion (Umbra::UINT32 flags, IndexList& clustersOut, int cluster,
    const Umbra::Vector3& pt, float distance, float* confidenceBound, FloatList* clusterPathDistances_,
    FloatList* clusterPathModifiers_, IndexList* clusterEntryPortals_)
{
    QueryContext ctx(IMPL(this), flags);
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;
    if (!HAS_WRAPPER || (ctx.getState()->getSpuUsage() == Query::SPU_USAGE_NONE))
    {
        UserList<int>& clusters = *IMPL(&clustersOut);
        UserList<float>* clusterPathDistances = IMPL(clusterPathDistances_);
		UserList<float>* clusterPathModifiers = IMPL(clusterPathModifiers_);
        UserList<int>* clusterEntryPortals = IMPL(clusterEntryPortals_);
        if (cluster == -1)
        {
            cluster = ctx.findCluster(pt);
        }
        else
        {
            // map, so that ctx.getDefaultTome() will be the cluster's tome            
            MappedTome tome;
            int tomeIdx = ctx.getState()->findTomeByCluster(cluster);
            ctx.getState()->mapTome(tome, tomeIdx);
            cluster = tome.mapGlobalCluster(cluster);
        }

        if (cluster == -1)
            return Query::ERROR_OUTSIDE_SCENE;

        clusters.clear();

        if ((flags & QueryExt::QUERYFLAG_PATH_DISTANCE) && (distance >= 0.f))
        {
            RegionFinder* rf = UMBRA_HEAP_NEW(ctx.getAllocator(), RegionFinder, &ctx, cluster, pt, distance, !!(flags & QueryExt::QUERYFLAG_DISTANCE_FROM_CLUSTER));
            if (!rf)
                return Query::ERROR_OUT_OF_MEMORY;
            if (ctx.getError() == Query::ERROR_OK)
                rf->execute(&clusters, clusterPathDistances, clusterPathModifiers, clusterEntryPortals);
            UMBRA_HEAP_DELETE(ctx.getAllocator(), rf);
        }
        else // euclidean distance
        {
            DepthFirstRegionFinder* rf = UMBRA_HEAP_NEW(ctx.getAllocator(), DepthFirstRegionFinder, &ctx, flags);
            if (!rf)
                return Query::ERROR_OUT_OF_MEMORY;
            if (ctx.getError() == Query::ERROR_OK)
                rf->execute(&clusters, NULL, cluster, pt, distance, confidenceBound);
            UMBRA_HEAP_DELETE(ctx.getAllocator(), rf);
        }

        if (clusters.isMaxed())
            return Query::ERROR_OUT_OF_MEMORY;
    }
    else
    {
        QueryWrapper wrapper(ctx);
        wrapper.put(&flags, QueryParam_Input);
        wrapper.put(&clustersOut, QueryParam_Input | QueryParam_Output);
        wrapper.put(&cluster, QueryParam_Input);
        wrapper.put(&pt, QueryParam_Input);
        wrapper.put(&distance, QueryParam_Input);
        wrapper.put(confidenceBound, QueryParam_Output);
        wrapper.put(clusterPathDistances_, QueryParam_Input | QueryParam_Output);
		wrapper.put(clusterPathModifiers_, QueryParam_Input | QueryParam_Output);
        wrapper.put(clusterEntryPortals_, QueryParam_Input | QueryParam_Output);
        wrapper.dispatch(QID_QUERY_CONNECTEDREGION);
    }
    return (Query::ErrorCode)ctx.getError();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode QueryExt::clusterPortals (IndexList& portals_, int clusterIdx)
{
    QueryContext ctx(IMPL(this));
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;

    UserList<int>* portals = IMPL(&portals_);
    portals->clear();

    MappedTome mappedTome;

    if (!ctx.getState()->tilesArePointers())
        mappedTome = ctx.getState()->getDefaultTome();
    else
    {
        int tomeIdx = ctx.getState()->findTomeByCluster(clusterIdx);
        ctx.getState()->mapTome(mappedTome, tomeIdx);
        clusterIdx  = mappedTome.mapGlobalCluster(clusterIdx);
    }

    if (clusterIdx >= 0 && clusterIdx < mappedTome.getTome()->getNumClusters())
    {
        ArrayMapper clusters(&ctx, mappedTome.getTome()->getClusterNodes());
        ArrayMapper extClusters(&ctx, mappedTome.getExtClusterNodes());
        ArrayMapper clusterPortals(&ctx, mappedTome.getTome()->getClusterPortals());

        ClusterNode cluster;
        clusters.get(cluster, clusterIdx);
        for (int i = 0; i < cluster.getPortalCount(); i++)
        {
            Portal portal;
            clusterPortals.get(portal, cluster.getPortalIndex() + i);
            if (!portal.isOutside())
                portals->pushBack(mappedTome.mapLocalClusterPortal(cluster.getPortalIndex() + i));
        }

        ExtClusterNode extCluster;
        if (extClusters.getCount())
            extClusters.get(extCluster, clusterIdx);
        for (int i = 0; i < extCluster.getPortalCount(); i++)
        {
            Portal portal;
            portals->pushBack(mappedTome.mapLocalClusterPortal(mappedTome.getTome()->getNumClusterPortals() + extCluster.getPortalIndex() + i));
        }
    }

    if (portals->isMaxed())
        ctx.setError(Query::ERROR_OUT_OF_MEMORY);

    return (Query::ErrorCode)ctx.getError();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode QueryExt::getPortalInfo(PortalInfo& out_, int idx)
{
    QueryContext ctx(IMPL(this));
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;
    ImpPortalInfo& out = *IMPL(&out_);

    MappedTome mappedTome;

    if (!ctx.getState()->tilesArePointers())
        mappedTome = ctx.getState()->getDefaultTome();
    else
    {
        int tomeIdx = ctx.getState()->findTomeByClusterPortal(idx);
        ctx.getState()->mapTome(mappedTome, tomeIdx);
        idx  = mappedTome.mapGlobalClusterPortal(idx);
    }

    Portal portal;

    int numClusterPortals = mappedTome.getTome()->getNumClusterPortals();
    if (idx < numClusterPortals)
        mappedTome.getTome()->getClusterPortals().getElem(portal, idx);
    else
        mappedTome.getExtPortals().getElem(portal, idx - numClusterPortals);

    out.m_targetCluster = portal.getTargetCluster();
    out.m_tome = ctx.getDefaultTome().getTome();

    if (portal.isUser())
    {
        out.m_isUser = true;
        out.m_numGateIndices = 0;

        for (int i = 0; i < portal.getUserObjCount(); i++)
        {
            if (i < UMBRA_MAX_GATE_INDICES)
            {
                mappedTome.getTome()->getGateIndices().getElem(out.m_gateIndices[out.m_numGateIndices], portal.getUserObjOfs()+i);
                out.m_gateIndices[out.m_numGateIndices] = mappedTome.mapLocalGate(out.m_gateIndices[out.m_numGateIndices]);
                out.m_numGateIndices++;
            } else
                ctx.setError(Query::ERROR_OUT_OF_MEMORY);
        }

        out.m_numVertices = portal.getVertexCount() - 3;
        out.m_portalGeometryOfs = portal.getGeometryOfs() + 3;

        // Center, plane equation, radius
        DataArray vertices = out.m_tome->getGateVertices();
        vertices.getElem(out.m_center, portal.getGeometryOfs());
        Vector3 tmp1;
        Vector3 tmp2;
        vertices.getElem(tmp1, portal.getGeometryOfs() + 1);
        vertices.getElem(tmp2, portal.getGeometryOfs() + 2);

        out.m_planeEq = Vector4(tmp1.x, tmp1.y, tmp1.z, tmp2.x);
        out.m_minRadius = tmp2.y;
        out.m_maxRadius = tmp2.z;
    }
    else
    {
        out.m_isUser = false;
        out.m_numGateIndices = 0;
        out.m_numVertices = 0;
        portal.getMinMax(mappedTome.getTome()->getTreeMin(), mappedTome.getTome()->getTreeMax(),
            0.f, // \todo return expanded portal
            out.m_aabbMin, out.m_aabbMax);
        out.m_center = (out.m_aabbMin + out.m_aabbMax) * 0.5f;
    }
    return (Query::ErrorCode)ctx.getError();
}

APICLASS_COMMON(LineSegmentQuery)

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

LineSegmentQuery::LineSegmentQuery(void)
{
    new (IMPL(this)) ImpLineSegmentQuery();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

LineSegmentQuery::LineSegmentQuery(const Vector3& a, const Vector3& b, IndexList* ob)
{
    new (IMPL(this)) ImpLineSegmentQuery(a, b, ob);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Query::ErrorCode QueryExt::queryLineSegment(LineSegmentQuery* queries, int count)
{
    if (!queries || (count == 0))
        return Query::ERROR_INVALID_ARGUMENT;

    QueryContext ctx(IMPL(this));
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;

#if 0 // - wrapper use disabled for now
    if (!HAS_WRAPPER || (ctx.getState()->getSpuUsage() == Query::SPU_USAGE_NONE) || (count < 2))
#endif
    {
        LineSegmentFinder* ls = UMBRA_HEAP_NEW(ctx.getAllocator(), LineSegmentFinder, &ctx);
        if (!ls)
            return Query::ERROR_OUT_OF_MEMORY;
        // TODO: this violates alignment, need to unpack line segment queryies separately
        ls->execute((ImpLineSegmentQuery*)queries, count);
        UMBRA_HEAP_DELETE(ctx.getAllocator(), ls);
    }
#if 0
    else
    {
        /* \todo [antti 26.6.2012]: break into 16*1024 batches -- wrapper param size limit */
        QueryWrapper wrapper(ctx);
        wrapper.put(queries, count * sizeof(LineSegmentQuery), QueryParam_Input | QueryParam_Output);
        wrapper.put(&count, QueryParam_Input);
        wrapper.dispatch(QID_QUERY_LINESEGMENT);
    }
#endif
    return (Query::ErrorCode)ctx.getError();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void LineSegmentQuery::setStart(const Vector3& start)
{
    ImpLineSegmentQuery* imp = IMPL(this);
    imp->q.m_start = start;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

const Vector3& LineSegmentQuery::getStart(void) const
{
    return IMPL(this)->q.m_start;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void LineSegmentQuery::setEnd(const Vector3& end)
{
    IMPL(this)->q.m_end = end;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

const Vector3& LineSegmentQuery::getEnd(void) const
{
    return IMPL(this)->q.m_end;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

LineSegmentQuery::ResultCode LineSegmentQuery::getResult(void) const
{
    return IMPL(this)->q.m_result;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void LineSegmentQuery::setObjectSet(IndexList* objectSet)
{
    IMPL(this)->q.m_objectSet = objectSet;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

IndexList* LineSegmentQuery::getObjectSet(void) const
{
    return IMPL(this)->q.m_objectSet;
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

#define USE_VISIBLE_WORLD_BOUNDS 0

APICLASS_COMMON(ShadowCullerExt)

ShadowCullerExt::ShadowCullerExt (void)
{
    new (IMPL(this)) ImpShadowCuller();
}

namespace Umbra
{

static QueryExt::ErrorCode buildShadowCullerImpl (
    QueryExt*               query,
    ImpShadowCuller*        culler,
    const Visibility*       visibility,
    const CameraTransform*  camera,
    const Vector3&          lightDir,
    const Vector3*          dynBounds,
    int                     numDynBounds,
    float*                  farPlaneDistance,
    UINT32                  flags,
    const CameraTransform** cascades, 
    int                     numCascades)
{
    QueryContext ctx(IMPL(query));
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;

    if (numCascades > ImpShadowCuller::MAX_CASCADES ||   // too many cascades
        numCascades < 0 ||                               // invalid cascade count
        (!cascades && numCascades > 0))                  // null transforms pointer but cascade count non-zero
        return Query::ERROR_INVALID_ARGUMENT;

    if (!culler->initCascades(cascades, numCascades))
        return Query::ERROR_INVALID_ARGUMENT;            // null cascade pointer

    culler->setFlags(flags);

    UMBRA_ASSERT(culler);
    culler->setLightDir(lightDir);

    const ImpVisibility* vis = IMPL(visibility);
    Matrix4x4 worldToClip;
    Vector3 cameraPos;

    if (vis)
    {
        UMBRA_ASSERT(!camera);
        if (!vis->m_occlusionBuffer)
            return Query::ERROR_UNSUPPORTED_OPERATION;
        const ImpOcclusionBuffer* occlusionBuffer = IMPL(vis->m_occlusionBuffer);
        if (!occlusionBuffer->isValid() || !occlusionBuffer->getDepthBufferPtr(false))
            return Query::ERROR_UNSUPPORTED_OPERATION;
        const Transformer* transformer = occlusionBuffer->getTransformer();
        worldToClip = transformer->getWorldToClip();
        cameraPos = transformer->getCameraPos();
    }
    else
    {
        UMBRA_ASSERT(!visibility);
        UMBRA_ASSERT(camera);
        camera->get(worldToClip, CameraTransform::DEPTHRANGE_ZERO_TO_ONE, MF_ROW_MAJOR);
        cameraPos = IMPL(camera)->m_position;
    }

    const Vector3 inLightDir = normalize(lightDir);
    float maxDistance = farPlaneDistance ? *farPlaneDistance : FLT_MAX;

    Matrix4x4 clipToWorld;
    clipToWorld.invert(worldToClip);
    AABB      worldBounds;
    Matrix4x4 worldToLight;
    AABB      lightSpaceAABB;

    bool hasFarPlane = true;
    Vector4 frustumPlanes[6];
    ShadowUtils::getClipPlanes(worldToClip, frustumPlanes, hasFarPlane);

    if (farPlaneDistance)
    {
        // Adjust max far plane to max distance
        // TODO: clamp or error when custom distance is more than original?
        const Vector4& farPlane = frustumPlanes[ShadowUtils::FAR];
        frustumPlanes[ShadowUtils::FAR].w = -dot(-farPlane.xyz()*maxDistance+cameraPos, farPlane.xyz());
        culler->setCustomFarPlane(true);
    }
    else if (!hasFarPlane)
    {
        return Query::ERROR_INVALID_ARGUMENT;
    }

    Vector4 planeArray[ShadowUtils::MaxShadowClipPlanes];
    int     planeCount = 0;

    ShadowUtils::getShadowClipPlanes(inLightDir, frustumPlanes, planeArray, planeCount);
    culler->getPlaneCuller().init(planeArray, planeCount);
    culler->getSinglePlaneCuller().init(frustumPlanes[ShadowUtils::FAR]);
    culler->setCameraPos(cameraPos);

    // Initialize receiver mask

    if (vis)
    {
        UMBRA_ASSERT(vis && vis->m_occlusionBuffer);

        AABB worldBounds;

        // TODO: change back to using visible receiver bounds when the occlusion query generates
        // this data
        if (USE_VISIBLE_WORLD_BOUNDS)
        {
            worldBounds = SceneBounds(ctx, vis->m_objects, dynBounds, numDynBounds).getAABB();
        }
        else
        {
            Vector3 mn = ctx.getTome()->getAABB().getMin();
            Vector3 mx = ctx.getTome()->getAABB().getMax();
            for (int i = 0; i < numDynBounds; i++)
            {
                mn = min(mn, dynBounds[2*i]);
                mx = max(mx, dynBounds[2*i+1]);
            }
            worldBounds = AABB(mn, mx);
        }

        if (!worldBounds.isOK())
            return QueryExt::ERROR_OK;

        // Construct receiver mask culler

        Matrix4x4 worldToLightClip;
        Matrix4x4 lightToLightClip;

        ShadowUtils::getWorldToLightMatrix(worldToLight, worldToClip, lightDir);
        ShadowUtils::getLightSpaceAABB(lightSpaceAABB, worldToLight, frustumPlanes, worldBounds);

        // Reject invalid and zero width AABBs. 
        // Returning ERROR_OK here effectively results in plane culler being used.
        if (lightSpaceAABB.getMax().x <= lightSpaceAABB.getMin().x ||
            lightSpaceAABB.getMax().y <= lightSpaceAABB.getMin().y ||
            lightSpaceAABB.getMax().z <= lightSpaceAABB.getMin().z) 
            return QueryExt::ERROR_OK;

        ShadowUtils::getOrthoProjection(lightToLightClip, lightSpaceAABB.getMin(), lightSpaceAABB.getMax());

        worldToLightClip = worldToLight*lightToLightClip;

#if 0
        // visualize light frustum
        {
            Matrix4x4 invlight = worldToLightClip;
            invlight.invert();
            Vector3 v[8];

            v[0] = invlight.transformDivByW(Vector3(-1,-1, 0));
            v[1] = invlight.transformDivByW(Vector3(-1, 1, 0));
            v[2] = invlight.transformDivByW(Vector3( 1,-1, 0));
            v[3] = invlight.transformDivByW(Vector3( 1, 1, 0));

            v[4] = invlight.transformDivByW(Vector3(-1,-1, 1));
            v[5] = invlight.transformDivByW(Vector3(-1, 1, 1));
            v[6] = invlight.transformDivByW(Vector3( 1,-1, 1));
            v[7] = invlight.transformDivByW(Vector3( 1, 1, 1));


            ctx.addQueryDebugLine(v[0], v[1], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[1], v[3], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[3], v[2], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[2], v[0], Vector4(1.f, 1.f, 0.f, 1.f));

            ctx.addQueryDebugLine(v[4], v[5], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[5], v[7], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[7], v[6], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[6], v[4], Vector4(1.f, 1.f, 0.f, 1.f));

            ctx.addQueryDebugLine(v[0], v[4], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[1], v[5], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[2], v[6], Vector4(1.f, 1.f, 0.f, 1.f));
            ctx.addQueryDebugLine(v[3], v[7], Vector4(1.f, 1.f, 0.f, 1.f));
        }
#endif

        culler->initReceiverMask(cameraPos, worldToClip, clipToWorld, worldToLightClip, IMPL(vis->m_occlusionBuffer));

        // Render visible dynamic objects into receiver mask
        for (int i = 0; i < numDynBounds; i++)
        {
            const Vector3& mn = dynBounds[2*i];
            const Vector3& mx = dynBounds[2*i+1];

            culler->getReceiverMaskCuller().addAABB(mn, mx);
        }

        // Cache visible static objects

        if (vis->m_objects)
        {
            const ImpTome* tome = ctx.getTome();
            if (culler->setVisibleStaticObjs(tome->getNumObjects()))
            {
                UINT32* visObjs = culler->getVisibleStaticObjs();
                UMBRA_ASSERT(visObjs);
                // we don't want to abort if this fails as it's just an optimization
                memset(visObjs, 0, UMBRA_BITVECTOR_SIZE(tome->getNumObjects()));

                IndexList* visStaticIndices = vis->m_objects;
                int* ptr = visStaticIndices->getPtr();
                int numObjs = visStaticIndices->getSize();
                for (int i = 0; i < numObjs; i++)
                    setBit(visObjs, *(ptr+i));
            }
        }
    }


    return Query::ERROR_OK;
}

}

QueryExt::ErrorCode QueryExt::buildMaskShadowCuller (
    ShadowCullerExt&        shadowCuller,
    const Visibility&       visibility,
    const Vector3&          lightDir,
    const Vector3*          dynBounds,
    int                     numDynBounds,
    float*                  farPlaneDistance,
    Umbra::UINT32           flags,
    const CameraTransform** cascades, 
    int                     numCascades)
{
    ImpShadowCuller* impl = new (IMPL(&shadowCuller)) ImpShadowCuller();
    return buildShadowCullerImpl(
        this,
        impl,
        &visibility,
        NULL,
        lightDir,
        dynBounds, numDynBounds,
        farPlaneDistance,
        flags,
        cascades,
        numCascades);
}

QueryExt::ErrorCode QueryExt::buildPlaneShadowCuller (
    ShadowCullerExt&        shadowCuller,
    const CameraTransform&  camera,
    const Vector3&          lightDir,
    float*                  farPlaneDistance,
    Umbra::UINT32           flags,
    const CameraTransform** cascades, 
    int                     numCascades)
{
    ImpShadowCuller* impl = new (IMPL(&shadowCuller)) ImpShadowCuller();
    return buildShadowCullerImpl(
        this,
        impl,
        NULL,
        &camera,
        lightDir,
        NULL,
        0,
        farPlaneDistance,
        flags,
        cascades,
        numCascades);
}


namespace Umbra
{
    struct Cascade
    {        
        ActivePlaneSet planeSet;
        bool           isActive;
    };

    template<bool stopOnFullCascade>
    static UMBRA_INLINE bool testCascades(const ImpShadowCuller* culler, SIMDRegister mn, SIMDRegister mx, Cascade* cascades, IndexList* cascadeMasks)
    {
        UMBRA_ASSERT(culler->getNumCascades() > 0);

        int mask = 0;
        for (int i = 0; i < culler->getNumCascades(); i++)
        {
            if (!cascades[i].isActive)
                continue;
            if (culler->getCascade(i).frustumTestBounds(&cascades[i].planeSet, mn, mx))
            {
                mask |= (1 << i);
                if (stopOnFullCascade && culler->getCascade(i).frustumTestBoundsFully(&cascades[i].planeSet, mn, mx))
                    break;
            }
        }

        if (!mask)
            return false;

        UMBRA_ASSERT(cascadeMasks);
        UserList<int>* outList = IMPL(cascadeMasks);
        outList->pushBack(mask);

        return true;
    }
}

QueryExt::ErrorCode QueryExt::queryStaticShadowCasters (const ShadowCullerExt& shadowCuller, IndexList& out, const ObjectDistanceParams* inObjDistanceParams, int jobIdx, int numJobs, IndexList* cascadeMasks)
{
    QueryContext ctx(IMPL(this));
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;

    // id must be within range
    if (jobIdx < 0 || jobIdx >= numJobs)
        return Query::ERROR_INVALID_ARGUMENT;
    
    const ImpShadowCuller* culler = IMPL(&shadowCuller);
    Vector3 cameraPos = culler->getCameraPos();
    bool depthMapsPresent = false;

    Cascade* cascadeData = NULL;
    bool     doCascadeTests = false;

    if (cascadeMasks && !culler->getNumCascades())
        return Query::ERROR_INVALID_ARGUMENT;

    if (culler->getNumCascades() && cascadeMasks)
    {
        doCascadeTests = true;
        cascadeData = (Cascade*)UMBRA_HEAP_ALLOC(ctx.getAllocator(), sizeof(Cascade) * culler->getNumCascades());

        for (int i = 0; i < culler->getNumCascades(); i++)
            cascadeData[i].planeSet.numPlanes = 0;
    }

    bool stopOnFullCascade = !!(culler->getFlags() & QueryExt::QUERYFLAG_EXCLUSIVE_CASCADES);

    ObjectIterator<true> objectIterator(&ctx, false, jobIdx, numJobs);

    ArrayMapper* globalBounds = NULL;

    // Cascade assignment must be performed against a global object bound
    // read them, but only if streaming and cascade tests enabled
    if (!objectIterator.isGlobal() && doCascadeTests)
        globalBounds = UMBRA_HEAP_NEW(ctx.getAllocator(), ArrayMapper, &ctx, ctx.getTome()->getObjectBounds(), 64);
        
    UserList<int>* outList = IMPL(&out);
    outList->clear();

    const ImpObjectDistanceParams* objDist = IMPL(inObjDistanceParams);
    float objDistScale = ImpObjectDistanceParams::getEffectiveScale(objDist);
    SIMDRegister lodDistanceScaleSqr = SIMDLoad(objDistScale * objDistScale); 
    SIMDRegister lodRef = SIMDLoadW1(ImpObjectDistanceParams::getEffectiveReference(objDist, cameraPos));

    DepthmapReaderDirectional reader(-culler->getLightDir());

    UINT32* activeObjs = NULL;
    if (ctx.getState()->getNumTomeContexts() > 1)
    {
        size_t size = UMBRA_BITVECTOR_SIZE(ctx.getTome()->getNumObjects());
        activeObjs = (UINT32*)UMBRA_HEAP_ALLOC(ctx.getAllocator(), size);
        if (activeObjs)
            memset(activeObjs, 0, size);
    }

    ActivePlaneSet activePlaneSet;
    while (objectIterator.hasMoreTomes() || objectIterator.hasMoreObjects())
    {
        if (!objectIterator.hasMoreObjects())
        {
            UMBRA_ASSERT(objectIterator.hasMoreTomes());
            bool active = true;
            do
            {
                objectIterator.nextTome();
                AABB tomeAABB = objectIterator.getCurrentTome()->getAABB();
                SIMDRegister vmn = SIMDLoadW1(tomeAABB.getMin());
                SIMDRegister vmx = SIMDLoadW1(tomeAABB.getMax());
                active = culler->getPlaneCuller().frustumTestBounds(vmn, vmx);
                if (active)
                {
                    culler->getPlaneCuller().computeActivePlaneSet(activePlaneSet, vmn, vmx);
                    if (doCascadeTests)
                    {
                        bool activeCascades = false;
                        for (int i = 0; i < culler->getNumCascades(); i++)
                        {
                            culler->getCascade(i).computeActivePlaneSet(cascadeData[i].planeSet, vmn, vmx);
                            cascadeData[i].isActive = culler->getCascade(i).frustumTestBounds(&cascadeData[i].planeSet, vmn, vmx);
                            activeCascades = activeCascades || cascadeData[i].isActive;
                        }
                        if (!activeCascades)
                            active = false;
                    }
                }
            } while (!active && objectIterator.hasMoreTomes());

            if (!active || (!objectIterator.hasMoreTomes() && !objectIterator.hasMoreObjects()))
                break;

            depthMapsPresent = !!(objectIterator.getCurrentTome()->getFlags() & (ImpTome::TOMEFLAG_DEPTHMAPS | ImpTome::TOMEFLAG_SHADOW_DEPTHMAPS));
            if (depthMapsPresent)
                reader.init(objectIterator.getCurrentTome());
        }

        objectIterator.nextObject();
        objectIterator.fetchBounds();

        int localIdx  = objectIterator.getLocalIdx();
        int globalIdx = objectIterator.getGlobalIdx();

        if(activeObjs && testBit(activeObjs, globalIdx))
            continue;

        const ObjectBounds&   mnmx = objectIterator.getObjectBounds();
        SIMDRegister vmn = SIMDLoadW1(mnmx.mn);
        SIMDRegister vmx = SIMDLoadW1(mnmx.mx);

        const ObjectDistance& dist = objectIterator.getObjectDistance();

        if (culler->getVisibleStaticObjs() && testBit(culler->getVisibleStaticObjs(), globalIdx))
        {
            SIMDRegister vmn = SIMDLoadW1(mnmx.mn);
            SIMDRegister vmx = SIMDLoadW1(mnmx.mx);

            if (!culler->hasCustomFarPlane() || culler->getSinglePlaneCuller().isVisible(vmn, vmx))
            {
                if (depthMapsPresent && !reader.test(localIdx))
                    continue;

                if (activeObjs)
                    setBit(activeObjs, globalIdx);                

                if (doCascadeTests) 
                {
                    if (globalBounds)
                    {
                        ObjectBounds mnmxGlobal;
                        globalBounds->get(mnmxGlobal, globalIdx);
                        vmn = SIMDLoadW1(mnmxGlobal.mn);
                        vmx = SIMDLoadW1(mnmxGlobal.mx);
                    }

                    if ((stopOnFullCascade  && testCascades<true>(culler, vmn, vmx, cascadeData, cascadeMasks)) ||
                        (!stopOnFullCascade && testCascades<false>(culler, vmn, vmx, cascadeData, cascadeMasks)))
                    {
                        if (!outList->pushBack(globalIdx))
                            break;
                    }
                } else
                {
                    if (!outList->pushBack(globalIdx))
                        break;
                }
                continue;
            }
        } else
        if (objectIterator.hasDistances() && !distanceInRange(lodRef, dist, lodDistanceScaleSqr))
            continue;

        if (!culler->isAABBActivePlanes(vmn, vmx, &activePlaneSet))
            continue;

        if (depthMapsPresent && !reader.test(localIdx))
            continue;

        if (culler->isAABBActiveMask(vmn, vmx))
        {
            if (activeObjs)
                setBit(activeObjs, globalIdx);

            if (doCascadeTests) 
            {
                if (globalBounds)
                {
                    ObjectBounds mnmxGlobal;
                    globalBounds->get(mnmxGlobal, globalIdx);
                    vmn = SIMDLoadW1(mnmxGlobal.mn);
                    vmx = SIMDLoadW1(mnmxGlobal.mx);
                }

                if ((stopOnFullCascade  && testCascades<true>(culler, vmn, vmx, cascadeData, cascadeMasks)) ||
                    (!stopOnFullCascade && testCascades<false>(culler, vmn, vmx, cascadeData, cascadeMasks)))
                {
                    if (!outList->pushBack(globalIdx))
                        break;
                }
            } else
            {
                if (!outList->pushBack(globalIdx))
                    break;
            }
            
        }
    }

    if (activeObjs)
        UMBRA_HEAP_FREE(ctx.getAllocator(), activeObjs);

    UMBRA_HEAP_DELETE(ctx.getAllocator(), globalBounds);
    UMBRA_HEAP_FREE(ctx.getAllocator(), cascadeData);

    return outList->isMaxed() ? Query::ERROR_OUT_OF_MEMORY : Query::ERROR_OK;
}

bool ShadowCullerExt::isAABBActive (const Vector3& mn, const Vector3& mx) const
{
    return IMPL(this)->isAABBActive(SIMDLoadW1(mn), SIMDLoadW1(mx));
}

bool ShadowCullerExt::isAABBActive (const Vector3& mn, const Vector3& mx, Umbra::UINT32& cascadeMask) const
{
    const ImpShadowCuller& culler = *IMPL(this);
    
    cascadeMask = 0;

    bool stopOnFullCascade = !!(culler.getFlags() & QueryExt::QUERYFLAG_EXCLUSIVE_CASCADES);

    SIMDRegister simdMn = SIMDLoadW1(mn);
    SIMDRegister simdMx = SIMDLoadW1(mx);

    if (culler.isAABBActive(simdMn, simdMx))
    {
        if (!culler.getNumCascades())
            return true;
        
        for (int i = 0; i < culler.getNumCascades(); i++)
        {
            if (culler.getCascade(i).frustumTestBounds(simdMn, simdMx))
            {
                cascadeMask |= (1 << i);
                if (stopOnFullCascade && culler.getCascade(i).frustumTestBoundsFully(simdMn, simdMx))
                    break;
            }
        }

        return !!cascadeMask;
    }

    return false;
}

QueryExt::ErrorCode ShadowCullerExt::getReceiverMaskBuffer (ReceiverMaskBuffer& out) const
{
    return IMPL(this)->getBuffer(IMPL(&out));
}

Query::ErrorCode QueryExt::queryLocalLights(
    IndexList&          outVisibleLights,
    Umbra::UINT32       /*flags*/,
    const SphereLight*  sphereLights,
    int                 lightCount,
    const IndexList&    visibleClusters,
    const IndexList*    visibleLightFilter)
{
    QueryContext ctx(IMPL(this), 0);
    if (!ctx.hasData())
        return Query::ERROR_NO_TOME;
    UserList<int>& output = (UserList<int>&)outVisibleLights;
    output.clear();

    // Allocate

    int clusterCount = ctx.getState()->getRootTome()->getNumClusters();
    Umbra::UINT32* clusterBV = ctx.bitVectorFromIndexList(clusterCount, visibleClusters);
    DepthFirstRegionFinder* finder = UMBRA_HEAP_NEW(ctx.getAllocator(), DepthFirstRegionFinder, &ctx, 0);
    if (!clusterBV || !finder)
        ctx.setError(Query::ERROR_OUT_OF_MEMORY);

    if (ctx.getError() == Query::ERROR_OK)
    {
        // Loop over lights

        int count = visibleLightFilter ? visibleLightFilter->getSize() : lightCount;

        for (int i = 0; i < count; i++)
        {
            int lightIndex = i;
            if (visibleLightFilter)
            {
                lightIndex = visibleLightFilter->getPtr()[i];
                if (lightIndex < 0 || lightIndex >= lightCount)
                {
                    ctx.setError(Query::ERROR_INVALID_ARGUMENT);
                    break;
                }
            }
            const SphereLight& light = sphereLights[lightIndex];
            // TODO: easy optimization opportunity here, use findMultipleCells or equivalent
            int cluster = ctx.findCluster(light.center);
            if (cluster == -1 || finder->execute(NULL, clusterBV, cluster, light.center, light.radius, NULL))
            {
                output.pushBack(lightIndex);
            }
        }

        if (output.isMaxed())
            ctx.setError(Query::ERROR_OUT_OF_MEMORY);
    }

    UMBRA_HEAP_DELETE(ctx.getAllocator(), finder);
    UMBRA_HEAP_DELETE_ARRAY(ctx.getAllocator(), clusterBV);
    return (Query::ErrorCode)ctx.getError();
}

void DebugRenderer::addAABBLines(DebugRenderer* dbg, const Vector3& mn, const Vector3& mx, const Vector4& color)
{
#define ADD_LINE(a, b, c, d, e, f) \
    { \
        Vector3 start(a[0], b[1], c[2]); \
        Vector3 end(d[0], e[1], f[2]); \
        dbg->addLine(start, end, color); \
    }

    ADD_LINE(mn, mn, mn, mx, mn, mn); ADD_LINE(mn, mx, mn, mx, mx, mn);
    ADD_LINE(mn, mn, mx, mx, mn, mx); ADD_LINE(mn, mx, mx, mx, mx, mx);
    ADD_LINE(mn, mn, mn, mn, mx, mn); ADD_LINE(mn, mn, mx, mn, mx, mx);
    ADD_LINE(mx, mn, mn, mx, mx, mn); ADD_LINE(mx, mn, mx, mx, mx, mx);
    ADD_LINE(mn, mn, mn, mn, mn, mx); ADD_LINE(mn, mx, mn, mn, mx, mx);
    ADD_LINE(mx, mn, mn, mx, mn, mx); ADD_LINE(mx, mx, mn, mx, mx, mx);
#undef ADD_LINE
}

#undef THIS
