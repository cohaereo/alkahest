#if !defined(UMBRA_EXCLUDE_COMPUTATION)

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2007-2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Depthmap solver implementation
 *
 */

#include "umbraVolumetricQuery.hpp"
#include "umbraAABB.hpp"
#include "umbraCubemap.hpp"
#include "umbraRasterDefs.hpp"
#include "umbraQueryContext.hpp"
#include "umbraPortalCull.hpp"

using namespace Umbra;

namespace Umbra
{

// Computes camera placement on an AABB face
class SampleCamera
{
public:
    SampleCamera(const AABB& world, const AABB& object, int face);

    float           m_accurateDistance; // query accurate distance
    CameraTransform m_transform;        // Computed CameraTransform, can be used with a query
    Matrix4x4       m_clipToWorld;      // Inverse transformation. Used to unproject query depth buffer.
    Vector3         m_position;         // Camera position
};

// AABBs are split to quarantee working, unambiguous tile traverse order.
// This traverse object splits given AABB using all intersecting splits.
class SplitObjectTraverse : public TraverseFilter<>
{
public:
    SplitObjectTraverse(void) {}
    SplitObjectTraverse(Allocator* a, const AABB& aabb)
        : m_aabb(aabb), m_split(a) 
    {
        m_split.pushBack(m_aabb);
    }

    SplitObjectTraverse& operator=(const SplitObjectTraverse& other)
    {
        m_aabb = other.m_aabb;
        m_split.setAllocator(other.m_split.getAllocator());
        m_split = other.m_split;
        return *this;
    }

    KDTraverseEntry splitNode (const NodeType& node)
    {        
        int axis = node.treeNode().getSplit();
        float mid = node.getSplitValue();

        // If this split intersects original AABB
        if (mid > m_aabb.getMin()[axis] &&
            mid < m_aabb.getMax()[axis])
        {
            // Process all current parts
            int size = m_split.getSize();
            for (int i = 0; i < size; i++)
            {
                // If this part intersects the split plane ...
                if (mid > m_split[i].getMin()[axis] &&
                    mid < m_split[i].getMax()[axis])
                {
                    // Split into two
                    AABB left  = m_split[i];
                    AABB right = m_split[i];
                    left.setMax(axis, mid);
                    right.setMin(axis, mid);
                    m_split[i] = left;
                    m_split.pushBack(right);
                }
            }
        }

        // Must find all splits everywhere.
        return ENTER_BOTH;
    }

    const Array<AABB>& getSplits() const { return m_split; }

    AABB        m_aabb;
    Array<AABB> m_split;
};
}

// Partition AABB into suitable computation units, where no tile split
// intersects the AABB.
void DepthmapSolver::partitionComputationUnits(const AABB& aabb, Array<AABB>& split)
{
    // Setup tree and the traverse
    KDTraversal<SplitObjectTraverse> traversal;
    DataArray treeData = m_tome->getTreeData();
    KDTree tree(m_tome->getTreeNodeCount(), (const Umbra::UINT32*)treeData.m_ofs.getAddr(treeData.m_base), m_tome->getTreeSplits());
    traversal.init(tree, AABB(m_tome->getTreeMin(), m_tome->getTreeMax()), SplitObjectTraverse(split.getAllocator(), aabb));
    traversal.perform();
    split = traversal.getSpec().getSplits();
}

// Convert buffer coordinates into clip space
static UMBRA_INLINE Vector3 getClipSpaceCoord(int x, int y, float z)
{
    float xc = -1.f + ((float)x / (UMBRA_PORTAL_RASTER_SIZE >> 1));
    float yc = -1.f + ((float)y / (UMBRA_PORTAL_RASTER_SIZE >> 1));
    return Vector3(xc, yc, z);
}

// Return float sign
static UMBRA_INLINE float getFloatSign(float f)
{
    return f < 0 ? -1.f : 1.f;
}

// "Roundify" object AABB to quarantee that the AABB matches 
// cubemap (which is a cube) better
static UMBRA_INLINE void roundifyAABB(AABB& object)
{
    float epsilon = object.getMaxAxisLength() * 0.5f;

    if (object.getMinAxisLength() < epsilon)
    {
        float   len = epsilon;
        Vector3 dim = object.getDimensions();
        if (dim[0] < epsilon) dim[0] = len;
        if (dim[1] < epsilon) dim[1] = len;
        if (dim[2] < epsilon) dim[2] = len;

        object = AABB(object.getCenter(), object.getCenter());
        object.inflate(dim * 0.5f);
    }
}

// Helper function for mapFarValues
static UMBRA_INLINE void mapValue(const Vector3& in, Vector3& out, const Vector3& cameraPos, const Vector3& center, Vector3i& m)
{
    out = in;

    if (((m.i == 0 && center.x < cameraPos.x)  ||
         (m.i == 1 && center.x > cameraPos.x)))
         out.x = out.x - cameraPos.x + center.x;

    if (((m.j == 0 && center.y < cameraPos.y)  ||
         (m.j == 1 && center.y > cameraPos.y)))
         out.y = out.y - cameraPos.y + center.y;

    if (((m.k == 0 && center.z < cameraPos.z)  ||
         (m.k == 1 && center.z > cameraPos.z)))
         out.z = out.z - cameraPos.z + center.z;
}

// Input is a world-space depth pixel quad, that has an "infinite" (outside/far) value. 
// To represent infinity correctly relative to cubemap center position, some values must be mapped to be relative to cubemap's 
// center position instead of camera's position. This effectively stretches the pixel.
static UMBRA_INLINE void mapFarValues(const Vector3* in, Vector3* out, const Vector3& cameraPos, const Vector3& center, int face)
{
    int axis  = getFaceAxis(face);
    int axisX = (axis + 1) % 3;
    int axisY = (axis + 2) % 3;

    Vector3i a;
    a[axis] = -1;

    a[axisX] = 0;
    a[axisY] = 1;
    mapValue(in[0], out[0], cameraPos, center, a);
    
    a[axisX] = 1;
    a[axisY] = 1;
    mapValue(in[1], out[1], cameraPos, center, a);

    a[axisX] = 0;
    a[axisY] = 0;
    mapValue(in[2], out[2], cameraPos, center, a);

    a[axisX] = 1;
    a[axisY] = 0;
    mapValue(in[3], out[3], cameraPos, center, a);
}

// Computes camera placement on an AABB face
SampleCamera::SampleCamera(const AABB& world, const AABB& object, int face)
{
    int faceAxis = getFaceAxis(face);  // Fwd axis
    int axisX    = (faceAxis + 1) % 3; // Right axis
    int axisY    = (faceAxis + 2) % 3; // Down axis

    // Compute camera near quad
    AABB faceAABB = object;
    faceAABB.flattenToFace(face);
    AABB nearQuad = faceAABB;
    
    // Near distance
    float dim = object.getDimensions()[faceAxis];
    float nearDistance = dim / 2.f;
    //nearDistance = max2(object.getMaxAxisLength() * 0.05f, nearDistance);

    // Compute far distance
    AABB worldFlattened = world;
    worldFlattened.flattenToFace(face);
    float farDistance  = nearDistance + 2.f * fabsf(worldFlattened.getMax()[faceAxis] - nearQuad.getMax()[faceAxis]);
    farDistance = max2(farDistance, object.getDimensions()[faceAxis]);
    farDistance = max2(farDistance, nearDistance * 2.f);

    if (nearDistance == 0.f)
        nearDistance = 0.5f;
    if (farDistance == 0.f)
        farDistance = 1.f;

    // Accurate distance
    m_accurateDistance = farDistance;

    // Camera position
    Vector3 off;
    off[faceAxis]  = -nearDistance * (float)getFaceDirectionSign(face);
    Vector3 pos    = nearQuad.getCenter() + off;
    m_position = pos;
    // Camera relative near quad 
    Vector3 nearMn = nearQuad.getMin() - pos;
    Vector3 nearMx = nearQuad.getMax() - pos;

    // Compute projection
    float nearScale = 1.f;
    nearMn *= nearScale;
    nearMx *= nearScale;
    Matrix4x4 proj = MatrixFactory::frustumLH(nearMn[axisX], nearMx[axisX], nearMx[axisY], nearMn[axisY], nearDistance * nearScale, farDistance);

    // Compute camera matrix
    Vector3   fwd, right, up;
    fwd[faceAxis] = -(float)getFaceDirectionSign(face);
    right[axisX]  = (float)getFaceDirectionSign(face);
    up[axisY]     = (float)getFaceDirectionSign(face);
    Matrix4x4 cam  = MatrixFactory::transformBase(pos, fwd, right, up);
    
    // Combined projection
    Matrix4x4 worldToClip = cam * proj;
    
    // Inverted combined projection
    m_clipToWorld = worldToClip;
    m_clipToWorld.invert();

    // Initialize CameraTransform
    m_transform = CameraTransform(worldToClip, pos, CameraTransform::DEPTHRANGE_ZERO_TO_ONE, MF_ROW_MAJOR);
}

// Draw a quad (a world-space depth pixel) to the depth map
void DepthmapSolver::drawDepthmapQuad(RawDepthmap& raw, const Vector3* v, int dstFace, bool updateValue, bool updateInf, bool accurateIntersect)
{
    // Quad relative to the cubemap center
    Vector3 v2[4] = 
    {
        v[0] - raw.center,
        v[1] - raw.center,
        v[2] - raw.center,
        v[3] - raw.center,
    };

    // Check that quad is in front of cubemap face
    int posVert = 0;
    int axis    = getFaceAxis(dstFace);
    int faceDir = getFaceDirection(dstFace);
    posVert += (floatSignBit(v2[0][axis]) ^ faceDir);
    posVert += (floatSignBit(v2[1][axis]) ^ faceDir);
    posVert += (floatSignBit(v2[2][axis]) ^ faceDir);
    posVert += (floatSignBit(v2[3][axis]) ^ faceDir);

    if (posVert == 0)
        return;

    // Map coordinates on cubemap, construct an integer rect of values
    Recti rect;
    Vector3i aa = DepthmapReader::map(v2[0], dstFace);
    Vector3i bb = DepthmapReader::map(v2[1], dstFace);
    Vector3i cc = DepthmapReader::map(v2[2], dstFace);
    Vector3i dd = DepthmapReader::map(v2[3], dstFace);
    rect.grow(Vector2i(aa.i, aa.j));
    rect.grow(Vector2i(bb.i, bb.j));
    rect.grow(Vector2i(cc.i, cc.j));
    rect.grow(Vector2i(dd.i, dd.j));

    // Reject rectangles completely outside the cubemap face
    if (!rect.isOK() || 
        rect.getMin().i >= (int)DepthmapData::Resolution ||
        rect.getMin().j >= (int)DepthmapData::Resolution ||
        rect.getMax().i < 0 ||
        rect.getMax().j < 0)
        return;

    // Clamp rect to face
    Recti fullRect(Vector2i(0, 0), Vector2i((int)DepthmapData::Resolution-1, (int)DepthmapData::Resolution-1));
    rect.clamp(fullRect);
    
    // Compute depth value
    float farthestVertDistance = 0.f;
    farthestVertDistance = max2(farthestVertDistance, fabsf(v2[0][getFaceAxis(dstFace)]));
    farthestVertDistance = max2(farthestVertDistance, fabsf(v2[1][getFaceAxis(dstFace)]));
    farthestVertDistance = max2(farthestVertDistance, fabsf(v2[2][getFaceAxis(dstFace)]));
    farthestVertDistance = max2(farthestVertDistance, fabsf(v2[3][getFaceAxis(dstFace)]));

    // Iterate depthmap pixel rectangle
    for (int y2 = rect.getMin().j; y2 <= rect.getMax().j; y2++)
    for (int x2 = rect.getMin().i; x2 <= rect.getMax().i; x2++)                    
    {
/*
#if 0
        SIMDRegister mask2 = SIMDMaskXYZW();

        for (int i = 0; i < 4; i++)
        {            
            const SIMDRegister& plane = m_pixelPlanes[dstFace][y2][x2][i];

            SIMDRegister mask;
            mask = SIMDCompareGE(SIMDDot4(plane, vSIMD[0]), SIMDZero());
            mask = SIMDBitwiseOr(mask, SIMDCompareGE(SIMDDot4(plane, vSIMD[1]), SIMDZero()));
            mask = SIMDBitwiseOr(mask, SIMDCompareGE(SIMDDot4(plane, vSIMD[2]), SIMDZero()));
            mask = SIMDBitwiseOr(mask, SIMDCompareGE(SIMDDot4(plane, vSIMD[3]), SIMDZero()));

            mask2 = SIMDBitwiseAnd(mask2, mask);

        }

        if (!SIMDNotZero(mask2))
            continue;
#else

        bool ok2 = true;
        for (int i = 0; i < 4; i++)
        {
            bool ok = false;
            for (int j = 0; j < 4; j++)
            {
                const SIMDRegister& plane = m_pixelPlanes[dstFace][y2][x2][i];
                if (!SIMDCompareGTTestAny(SIMDZero(), SIMDDot4(plane, vSIMD[j])))
                {
                    ok = true;
                    break;
                }
            }

            if (!ok)
            {
                ok2 = false;
                break;
            }
        }

        if (!ok2)
            continue;
#endif
*/

        // If requested accurate intersection test, use dot product to test vertices against 
        // current pixel's planes.

        if (accurateIntersect)
        {
            bool ok2 = true;
            for (int i = 0; i < 4; i++)
            {
                bool ok = false;
                for (int j = 0; j < 4; j++)
                {
                    const Vector4& plane = m_pixelPlanes[dstFace][y2][x2][i];
                    if (dot(plane, v2[j]) >= 0.f)
                    {
                        ok = true;
                        break;
                    }
                }

                if (!ok)
                {
                    ok2 = false;
                    break;
                }
            }

            if (!ok2)
                continue;
        }

        // Update infinite value if requested
        if (updateInf)
            setBit(raw.inf[dstFace], x2 + y2 * (int)DepthmapData::Resolution);
        // Update depth value if requested
        if (updateValue)
            raw.depthmap[dstFace][x2][y2] = max2(raw.depthmap[dstFace][x2][y2], farthestVertDistance);
    }
}

// Similiar to drawDepthmapQuad, but draw the quad in several parts.
// This is used to draw pixel "sides" (nonlateral faces between pixels). Updating in parts is required for 
// accurate enough depth value.
void DepthmapSolver::drawDepthmapQuadSplit(RawDepthmap& raw, int srcFace, const Vector3* v, int dstFace, bool inf)
{
    // Reject quads behind camera
    int   posVert = 0;
    int   dstAxis = getFaceAxis(dstFace);
    int   faceDir = getFaceDirection(dstFace);
    float center  = raw.center[dstAxis];    
    posVert += (floatSignBit(v[0][dstAxis] - center) ^ faceDir);
    posVert += (floatSignBit(v[1][dstAxis] - center) ^ faceDir);
    posVert += (floatSignBit(v[2][dstAxis] - center) ^ faceDir);
    posVert += (floatSignBit(v[3][dstAxis] - center) ^ faceDir);

    if (posVert == 0)
        return;

    // Compute step
    int axis  = getFaceAxis(srcFace);
    int axisX = (axis + 1) % 3;
    int axisY = (axis + 2) % 3;
    // Select some suitable step size
    float step = fabsf(v[0][axisX] - v[1][axisX]) * 3.f;
    step = max2(step, fabsf(v[0][axisY] - v[1][axisY]) * 3.f);
    
    // Skip quad if there's no depth
    float mn = min2(v[0][axis], v[3][axis]);
    float mx = max2(v[0][axis], v[3][axis]);

    if (mn == mx || step == 0.f)
        return;

    // Setup loop. Split along the "long" depth axis.
    Vector3 base1 = v[0];
    Vector3 base2 = v[1];
    Vector3 n1 = v[2] - base1;
    Vector3 n2 = v[3] - base2;
    n1 *= 1.f / fabsf(n1[axis]);
    n2 *= 1.f / fabsf(n2[axis]);

    if ((mx - mn) / step > 16.f)
        step = (mx - mn) / 16.f;

    for (float d = mn; d <= mx; d += step)
    {
        // Get split quad
        Vector3 v2[4];
        v2[0] = base1;
        v2[1] = base2;
        v2[2] = base2 + n2 * min2(mx - d, step);
        v2[3] = base1 + n1 * min2(mx - d, step);

        // Draw current part.
        drawDepthmapQuad(raw, v2, dstFace, true, inf, true);

        base1 = v2[3];
        base2 = v2[2];
    }
}

DepthmapSolver::DepthmapSolver(Allocator* a, const ImpTome* tome, const AABB& worldAABB) 
    : Base(a)
    , m_tome(tome)
    , m_worldAABB(worldAABB)     
{
    // Compute plane equations for each cubemap pixels.
    for (int face = 0; face < 6; face++)
    {
        float dist = m_worldAABB.getDiagonalLength();

        Vector3 origin(0,0,0);

        for (int y = 0; y < (int)DepthmapData::Resolution; y++)
        for (int x = 0; x < (int)DepthmapData::Resolution; x++)
        {
            Vector3 a = DepthmapReader::map(Vector3i(x,   y,   face)) * dist;
            Vector3 b = DepthmapReader::map(Vector3i(x+1, y,   face)) * dist;
            Vector3 c = DepthmapReader::map(Vector3i(x+1, y+1, face)) * dist;
            Vector3 d = DepthmapReader::map(Vector3i(x,   y+1, face)) * dist;

            Vector3 center2 = a + b + c + d;
            center2 *= 0.25f;

            Vector4* plane = m_pixelPlanes[face][y][x];

            plane[0] = getPlaneEquation(origin, a, b);
            plane[1] = getPlaneEquation(origin, b, c);
            plane[2] = getPlaneEquation(origin, c, d);
            plane[3] = getPlaneEquation(origin, d, a);

            // invert plane direction if required
            for (int i = 0; i < 4; i++)
            {
                if (dot(plane[i], center2) < 0)
                    plane[i].scale(Vector4(-1.f, -1.f, -1.f, -1.f));

                plane[i] *= 1.f / (plane[i].xyz().length());
            }
        }
    }

    // Query object with higher work mem.
    size_t workSize = (1024 + 512) * 1024;
    m_query    = UMBRA_HEAP_NEW(getAllocator(), QueryExt, (const Tome*)tome);
    m_workMem  = (UINT8*)UMBRA_HEAP_ALLOC_16(getAllocator(), workSize);
    m_query->setWorkMem(m_workMem, workSize);

    // QueryContext
    m_queryContext = UMBRA_HEAP_NEW(getAllocator(), QueryContext, IMPL(m_query), 0);

    for (int i = 0; i < 6; i++)
    {
        m_faceDatas[i].buffer = UMBRA_HEAP_NEW(getAllocator(), OcclusionBuffer);
    }

    // Buffer for reading depth values
    size_t floatSize = sizeof(float) * UMBRA_PORTAL_RASTER_SIZE * UMBRA_PORTAL_RASTER_SIZE;
    m_floatBuffer = (float*)UMBRA_HEAP_ALLOC_16(getAllocator(), floatSize);
}

DepthmapSolver::~DepthmapSolver(void)
{
    UMBRA_HEAP_FREE_16(getAllocator(), m_floatBuffer);

    for (int i = 0; i < 6; i++)
    {
        UMBRA_HEAP_DELETE(getAllocator(), m_faceDatas[i].buffer);
    }

    UMBRA_HEAP_DELETE(getAllocator(), m_queryContext);
    UMBRA_HEAP_FREE_16(getAllocator(), m_workMem);
    UMBRA_HEAP_DELETE(getAllocator(), m_query);
}

void DepthmapSolver::solve(RawDepthmap& raw, const AABB& objAABB, int objectIdx, float targetInflation)
{

    // Query object's each face

    for (int face = 0; face < 6; face++)
    {
        FaceData& fd = m_faceDatas[face];

        // Round AABB
        AABB rounded = objAABB;
        roundifyAABB(rounded);

        // Compute camera matrices etc.
        SampleCamera camera(m_worldAABB, rounded, face);

        // Prepare visibility object
        Visibility visibility;
        visibility.setOutputBuffer(fd.buffer);

        // Store some values for later
        fd.inverse     = camera.m_clipToWorld;
        fd.pos         = camera.m_position;
            
        // Start cell lookup AABB. Inflated with given targetInflation.
        AABB cellLookup = objAABB;
        cellLookup.inflate(Vector3(targetInflation, targetInflation, targetInflation));

        // Prediction aabb.
        AABB prediction = objAABB;

        // We're calling PortalCuller directly to avoid exposing some parameters in the public API.
        // We must duplicate some work done by queryPortalVisibility here, such as setting up 
        // Transformer and VisibilityResult.

        const ImpCameraTransform& impTransform = *IMPL(&camera.m_transform);
        Transformer               transformer(impTransform, prediction.getDimensions() * 0.5f, 0, 1, 0);
        VisibilityResult          result(*m_queryContext, visibility, transformer, true);
                       
        for (int i = 0; i < 8; i++)
            fd.frustumCorners[i] = transformer.getFrustumCorner(i);

        // Construct PortalCuller, note higher maxCells and maxTreeNodes limits made possible by bigger work mem.
        m_queryContext->setError(Query::ERROR_OK);
        PortalCuller* pr = UMBRA_HEAP_NEW(m_queryContext->getAllocator(), PortalCuller, m_queryContext, &transformer, camera.m_accurateDistance, NULL, 32768, 32768);

        // Make query with specialized start cell lookup by AABB and object index.
        Query::ErrorCode errorCode = pr->execute(result, false, true, cellLookup, objectIdx);
        UMBRA_UNREF(errorCode);

#if 0
        if (errorCode != Query::ERROR_OK && errorCode != Query::ERROR_OUTSIDE_SCENE)
        {
            printf("depthmap error %d/%d: %d\n", objectIdx, face, errorCode);
        }
#endif

        UMBRA_HEAP_DELETE(m_queryContext->getAllocator(), pr);
    }

    Recti fullRect(Vector2i(0, 0), Vector2i((int)DepthmapData::Resolution-1, (int)DepthmapData::Resolution-1));

    int mapFaces[6];
    int numMapFaces = 0;

    // Update cubemap using query results.

    for (int face = 0; face < 6; face++)
    {
        FaceData& fd = m_faceDatas[face];

        // Nonlateral faces (pixel sides) are visible from cubemap if cubemap center point is different from projection point.
        // This happens when object has been splitted and query performed in multiple parts.
        bool renderSides = raw.center != fd.pos;

        numMapFaces = 0;

        // Find which cubemap faces to update for current query.

        for (int mapFace = 0; mapFace < 6; mapFace++)
        {
            Recti r;
            for (int i = 0; i < 8; i++)
            {                
                if (getFloatSign(fd.frustumCorners[i][getFaceAxis(mapFace)] - raw.center[getFaceAxis(mapFace)]) != getFaceDirectionSign(mapFace))
                    continue;

                Vector3i mapped1 = DepthmapReader::map(fd.frustumCorners[i] - raw.center, mapFace);
                r.grow(Vector2i(mapped1.i, mapped1.j));
            }

            if (!r.isOK() || !r.intersects(fullRect))
                continue;

            mapFaces[numMapFaces++] = mapFace;
        }

        // Get pixels
        OcclusionBuffer::BufferDesc desc;
        desc.width  = UMBRA_PORTAL_RASTER_SIZE;
        desc.height = UMBRA_PORTAL_RASTER_SIZE;
        desc.format = OcclusionBuffer::FORMAT_NDC_FLOAT;
        desc.stride = desc.width * (UMBRA_FORMAT_BPP(desc.format) / 8);

        memset(m_floatBuffer, 0, sizeof(float) * UMBRA_PORTAL_RASTER_SIZE * UMBRA_PORTAL_RASTER_SIZE);
        fd.buffer->getBuffer(m_floatBuffer, &desc);

        // Iterate all pixels
        for (int y = 0; y < UMBRA_PORTAL_RASTER_SIZE; y++)
        for (int x = 0; x < UMBRA_PORTAL_RASTER_SIZE; x++)
        {
            int   idx    = y * UMBRA_PORTAL_RASTER_SIZE + x;
            float value  = m_floatBuffer[idx];

            // Pixel in world space
            Vector3 v[] = 
            {
                fd.inverse.transformDivByW(getClipSpaceCoord(x,   y,   value)),
                fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y,   value)),
                fd.inverse.transformDivByW(getClipSpaceCoord(x,   y+1, value)),
                fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y+1, value)),
            };

            // Right side in world space
            Vector3 vR[4];
            if (renderSides)
            {
                float valueR = x < UMBRA_PORTAL_RASTER_SIZE - 1 ? m_floatBuffer[idx+1] : 0.f;
                vR[0] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y+1, value));
                vR[1] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y,   value));
                vR[2] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y,   valueR));
                vR[3] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y+1, valueR));
            }

            // Lower side in world space
            Vector3 vD[4];
            if (renderSides)
            {
                float valueD = y < UMBRA_PORTAL_RASTER_SIZE - 1 ? m_floatBuffer[idx+UMBRA_PORTAL_RASTER_SIZE] : 0.f;
                vD[0] = fd.inverse.transformDivByW(getClipSpaceCoord(x,   y+1, value));
                vD[1] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y+1, value));
                vD[2] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y+1, valueD));
                vD[3] = fd.inverse.transformDivByW(getClipSpaceCoord(x,   y+1, valueD));
            }

            // Left side only if x == 0
            Vector3 vL[4];
            if (renderSides && x == 0)
            {
                vL[0] = fd.inverse.transformDivByW(getClipSpaceCoord(x, y+1, value));
                vL[1] = fd.inverse.transformDivByW(getClipSpaceCoord(x, y,   value));
                vL[2] = fd.inverse.transformDivByW(getClipSpaceCoord(x, y,   0.f));
                vL[3] = fd.inverse.transformDivByW(getClipSpaceCoord(x, y+1, 0.f));
            }

            // Upper side only if y == 0
            Vector3 vU[4];
            if (renderSides && y == 0)
            {
                vU[0] = fd.inverse.transformDivByW(getClipSpaceCoord(x,   y, value));
                vU[1] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y, value));
                vU[2] = fd.inverse.transformDivByW(getClipSpaceCoord(x+1, y, 0.f));
                vU[3] = fd.inverse.transformDivByW(getClipSpaceCoord(x,   y, 0.f));
            }

            for (int i = 0; i < numMapFaces; i++)
            { 
                int face2 = mapFaces[i];
            
                // Render infinite pixels only
                if (value >= 1.f)
                {
                    Vector3 r[4];
                    mapFarValues(v, r, fd.pos, raw.center, face);
                    drawDepthmapQuad(raw, r,  face2, false, true, renderSides || face != face2);
                }

                // Update depth value only
                drawDepthmapQuad(raw, v, face2, true, false, renderSides || face != face2);

                // Update sides if requested
                if (renderSides)
                {
                    drawDepthmapQuadSplit(raw, face, vR, face2, false);
                    drawDepthmapQuadSplit(raw, face, vD, face2, false);

                    if (x == 0)
                        drawDepthmapQuadSplit(raw, face, vL, face2, false);
                    if (y == 0)
                        drawDepthmapQuadSplit(raw, face, vU, face2, false);
                }
            }
        }
    }
}

#endif