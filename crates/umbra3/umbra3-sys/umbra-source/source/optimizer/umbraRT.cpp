#if !defined(UMBRA_EXCLUDE_COMPUTATION)

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2006-2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Ray tracer interface
 *
 *
 */

#include <cfloat>
#include <cstring>
#include <algorithm>
#include "umbraRT.hpp"
#include "umbraAABB.hpp"
#include "umbraArray.hpp"
#include "umbraFPUControl.hpp"
#include "umbraGeometryBlock.hpp"
#include "umbraBitOps.hpp"
#include "umbraSort.hpp"

namespace Umbra
{
    int g_bvhMaxLeafTriangles = 10;
}

#define BVH_SPLIT_SAMPLES 16
#define BVH_USE_SSE

#if defined(BVH_USE_SSE)
#include <emmintrin.h>
#endif

#if defined(BVH_USE_SSE)
#   define BVH_MIN(a, b, c)  _mm_store_ss(&a, _mm_min_ss(_mm_set1_ps(b), _mm_set1_ps(c)));
#   define BVH_MAX(a, b, c)  _mm_store_ss(&a, _mm_max_ss(_mm_set1_ps(b), _mm_set1_ps(c)));
#else
#   define BVH_MIN(a, b, c) a = min2(b, c);
#   define BVH_MAX(a, b, c) a = max2(b, c);
#endif

namespace Umbra
{

//------------------------------------------------------------------------

struct BVHNode
{
    BVHNode()
    {
        RightChild      = -1;
        FirstTriangle   = 0;
        NumTriangles    = 0;
    }

    AABB            Bound;
    // \todo [Hannu] remove one of these indices to fit in 32 bytes
    int             RightChild;
    int             FirstTriangle; // Note: when building this is remapping through m_Buckets, when tracing this points to m_Triangles
    int             NumTriangles;
};

//------------------------------------------------------------------------

UMBRA_FORCE_INLINE static bool intersectRayTriangle(
    const Vector3& orig,
    const Vector3& dir,
    const Vector3& vert0,
    const Vector3& vert1,
    const Vector3& vert2,
    float& t, float& u, float& v)
{
    static const float EPSILON = 0.000001f;
    float det,inv_det;

    Vector3 qvec;
    Vector3 edge1(vert1 - vert0);
    Vector3 edge2(vert2 - vert0);
    Vector3 pvec(cross(dir, edge2));
    det = dot(edge1, pvec);

    if (det > EPSILON)
    {
        Vector3 tvec(orig-vert0);
        u = dot(tvec, pvec);

        if (u < 0.0 || u > det)
            return false;

        qvec = cross(tvec, edge1);
        v = dot(dir, qvec);
        if (v < 0.0 || u + v > det)
            return false;
    }
    else if(det < -EPSILON)
    {
        Vector3 tvec(orig-vert0);
        u = dot(tvec, pvec);

        if (u > 0.0 || u < det)
            return false;

        qvec = cross(tvec, edge1);
        v = dot(dir, qvec) ;
        if (v > 0.0 || u + v < det)
            return false;
    }
    else
        return false;

    inv_det = 1.f / det;

    t = dot(edge2, qvec) * inv_det;
    u *= inv_det;
    v *= inv_det;

    return true;
}

//------------------------------------------------------------------------

struct BVHRayCastData
{
    static const int MAX_TRIANGLES = 2;

    BVHRayCastData(const Vector3& o, const Vector3& d, float enter, float exit) :
        Origin      (o),
        Dir         (d),
        Enter       (enter),
        Exit        (exit),
        NumTriangles(0),
        MinDistance (FLT_MAX)
    {
        // Div by zero and overflow is OK and leads to +-inf time bounds for the ray.

        InvDir.x = 1.0f / Dir.x;
        InvDir.y = 1.0f / Dir.y;
        InvDir.z = 1.0f / Dir.z;

        DirSgn[0] = floatBitPattern(Dir.x) >> 31;
        DirSgn[1] = floatBitPattern(Dir.y) >> 31;
        DirSgn[2] = floatBitPattern(Dir.z) >> 31;

        for (int i = 0; i < MAX_TRIANGLES; i++)
            Triangles[i] = 0;
    }


    Vector3         Origin;
    Vector3         Dir;
    Vector3         InvDir;
    int             DirSgn[3];
    float           Enter;
    float           Exit;
    int             Triangles[MAX_TRIANGLES];
    int             NumTriangles;
    float           MinDistance;
};

class ImpRayTracerDefs : public RayTracerDefs
{
public:

    enum
    {
        StackSize = 8*1024
    };

    struct StackEntry
    {
        UMBRA_FORCE_INLINE StackEntry(int idx, float a, float b) :
            Node(idx),
            Enter(a),
            Exit(b)
        {
            UMBRA_ASSERT(idx >= 0);
        }

        StackEntry(void) {}

        int         Node;
        float       Enter;
        float       Exit;
    };

};

//------------------------------------------------------------------------

class ImpRayTracer : public ImpRayTracerDefs
{
public:

    struct HitTriangle
    {
        HitTriangle(void)
        {
        }

        HitTriangle(const RayTracer::Triangle InTriangle, float InHitTime)
        :   Triangle    (InTriangle)
        ,   HitTime     (InHitTime)
        {
        }

        friend bool operator<(const HitTriangle& a, const HitTriangle& b)
        {
            return a.HitTime < b.HitTime;
        }

        friend bool operator>(const HitTriangle& a, const HitTriangle& b)
        {
            return a.HitTime > b.HitTime;
        }

        RayTracer::Triangle Triangle;
        float               HitTime;
    };

public:
                                ImpRayTracer        (const PlatformServices& services);
                                ~ImpRayTracer       (void);

    void                        buildBVH            (const AABB& aabb, const Vector3* vertices, const RayTracer::Triangle* triangles, int numVertices, int numTriangles);
    RayTracer::RayTraceResult   rayTrace            (StackEntry* stack, const Vector3& origin, const Vector3& dir, float maxDist, float& dist, Vector3* vertices = NULL);
    bool                        rayCastFirst        (StackEntry* stack, const Vector3& origin, const Vector3& dir, RayTracer::Triangle& outTriangle);
    PlatformServices&           getPlatform         (void) { return m_platform; }

private:

    int                         buildBVHRecursive   (int* triangles, int numTriangles, const AABB& bound);
    float                       computeBestSplit    (float& split, int& leftSize, int* triangles, int numTriangles, int axis);
    void                        computeSplitCost    (float& cost, int& leftSize, int* triangles, int numTriangles, float split, int axis);
    void                        computeAABB         (AABB& aabb, const int* pTriangles, int iNumTriangles);
    void                        rayCast             (StackEntry* stack, BVHRayCastData* data, int nodeIdx);

    PlatformServices            m_platform;
    AABB                        m_aabb;
    Array<Vector3>              m_Vertices;
    Array<RayTracer::Triangle>  m_Triangles; // NOTE: first triangle has a magic bit set when tracing
    Array<BVHNode>              m_Nodes;
    Array<AABB>                 m_TriangleAABBs;
    Array<int>                  m_Buckets;
    Array<Vector3>              m_Centers;
    Array<bool>                 m_Flipped;
};

//------------------------------------------------------------------------

ImpRayTracer::ImpRayTracer(const PlatformServices& platform)
:   m_platform(platform),
    m_Vertices(platform.allocator),
    m_Triangles(platform.allocator),
    m_Nodes(platform.allocator),
    m_TriangleAABBs(platform.allocator),
    m_Buckets(platform.allocator),
    m_Centers(platform.allocator),
    m_Flipped(platform.allocator)
{
}

//------------------------------------------------------------------------

ImpRayTracer::~ImpRayTracer()
{
}

//------------------------------------------------------------------------

void ImpRayTracer::computeAABB(
    AABB&               aabb,
    const int*          pTriangles,
    int                 iNumTriangles)
{
#if defined(BVH_USE_SSE)

    __m128 vmn = _mm_set1_ps(FLT_MAX);
    __m128 vmx = _mm_set1_ps(-FLT_MAX);

    for (int i = 0; i < iNumTriangles; i++)
    {
        vmn = _mm_min_ps(vmn, _mm_loadu_ps(&m_TriangleAABBs[pTriangles[i]].getMin().x));
        vmx = _mm_max_ps(vmx, _mm_loadu_ps(&m_TriangleAABBs[pTriangles[i]].getMax().x));
    }

    {
        Vector4 mn, mx;
        _mm_storeu_ps(&mn.x, vmn);
        _mm_storeu_ps(&mx.x, vmx);
        aabb.set(Vector3(mn.x, mn.y, mn.z), Vector3(mx.x, mx.y, mx.z));
    }
#else

    aabb = AABB();

    UMBRA_ASSERT(iNumTriangles > 0);

    for (int i = 0; i < iNumTriangles; i++)
    {
        aabb.grow(m_TriangleAABBs[pTriangles[i]]);
    }

#endif // BVH_USE_SSE
}

//------------------------------------------------------------------------

void ImpRayTracer::buildBVH(
    const AABB&                 aabb,
    const Vector3*              vertices,
    const RayTracer::Triangle*  triangles,
    int                         numVertices,
    int                         numTriangles)
{
    UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY

    // Copy vertices & triangles

    m_aabb = aabb;

    m_Vertices.set(vertices, numVertices);
    m_Triangles.reset(numTriangles);
    m_Nodes.resize(0);

    if (!numTriangles)
        return;

    m_Centers.reset(numTriangles);
    m_Flipped.reset(numTriangles);
    // note: extra entry in aabb array to support unaligned
    // quadword load of vec3 elements
    m_TriangleAABBs.reset(numTriangles + 1);

    const float ONE_OVER_THREE = 1.0f / 3.0f;
    for (int i = 0; i < numTriangles; i++)
    {
        // Sort triangle indices by vertex values to get consistent results for double-sided triangles.

        Vector3i tri = triangles[i].Vertex;

        UMBRA_ASSERT(tri[0] >= 0 && tri[0] < numVertices);
        UMBRA_ASSERT(tri[1] >= 0 && tri[1] < numVertices);
        UMBRA_ASSERT(tri[2] >= 0 && tri[2] < numVertices);

        bool flipped = false;

        if (memcmp(&m_Vertices[tri[0]], &m_Vertices[tri[1]], sizeof(Vector3)) > 0)
        {
            swap(tri[0], tri[1]);
            flipped = !flipped;
        }
        if (memcmp(&m_Vertices[tri[1]], &m_Vertices[tri[2]], sizeof(Vector3)) > 0)
        {
            swap(tri[1], tri[2]);
            flipped = !flipped;
        }
        if (memcmp(&m_Vertices[tri[0]], &m_Vertices[tri[1]], sizeof(Vector3)) > 0)
        {
            swap(tri[0], tri[1]);
            flipped = !flipped;
        }

        m_Triangles[i].Vertex = tri;
        m_Triangles[i].UserData = triangles[i].UserData;

        m_TriangleAABBs[i].grow(m_Vertices[tri[0]]);
        m_TriangleAABBs[i].grow(m_Vertices[tri[1]]);
        m_TriangleAABBs[i].grow(m_Vertices[tri[2]]);

        m_Centers[i] = ONE_OVER_THREE * (
            m_Vertices[tri[0]] +
            m_Vertices[tri[1]] +
            m_Vertices[tri[2]]);

        m_Flipped[i] = flipped;
    }

    // Build BVH

    Array<int> indices(numTriangles, m_platform.allocator);
    for (int i = 0; i < indices.getSize(); i++)
        indices[i] = i;

    //AABB bound;
    //computeAABB(bound, &indices[0], numTriangles);
    int rootIdx = buildBVHRecursive(&indices[0], numTriangles, aabb);
    UMBRA_UNREF(rootIdx);
    UMBRA_ASSERT(rootIdx == 0);

    // Sort m_Triangles according to m_Buckets and mark flipped flag.

    UMBRA_ASSERT(m_Buckets.getSize() == m_Triangles.getSize());

    Array<RayTracer::Triangle> tmpTriangles(m_Triangles);

    for (int i = 0; i < m_Buckets.getSize(); i++)
    {
        m_Triangles[i] = tmpTriangles[m_Buckets[i]];
        if (m_Flipped[m_Buckets[i]])
            m_Triangles[i].Vertex[0] |= 0x40000000;
    }

    // Clean build-time memory.

    m_TriangleAABBs.reset(0);
    m_Buckets.reset(0);
    m_Centers.reset(0);
    m_Flipped.reset(0);
}

//------------------------------------------------------------------------

int ImpRayTracer::buildBVHRecursive(
    int*            triangles,
    int             numTriangles,
    const AABB&     bound)
{
    int currentIdx = m_Nodes.getSize();
    m_Nodes.pushBack(BVHNode());

    m_Nodes[currentIdx].Bound = bound;

    int axis = bound.getLongestAxis();

    if (numTriangles <= g_bvhMaxLeafTriangles)
    {
        m_Nodes[currentIdx].FirstTriangle = m_Buckets.getSize();
        m_Nodes[currentIdx].NumTriangles = numTriangles;
        for (int i = 0; i < numTriangles; i++)
            m_Buckets.pushBack(triangles[i]);
    }
    else
    {
        float c[3];
        float splits[3];
        int lsizes[3];

        if (numTriangles == m_Triangles.getSize()) // First split axis is the longest axis.
        {
            int i = bound.getLongestAxis();
            computeBestSplit(splits[i], lsizes[i], triangles, numTriangles, i);
            c[i] = 0.f;
            c[(i+1)%3] = 1.f;
            c[(i+2)%3] = 1.f;
        }
        else
        {
            for (int i = 0; i < 3; i++)
                c[i] = computeBestSplit(splits[i], lsizes[i], triangles, numTriangles, i);
        }

        if (c[0] < c[1])
            if (c[0] < c[2])
                axis = 0;
            else
                axis = 2;
        else
            if (c[1] < c[2])
                axis = 1;
            else
                axis = 2;

        float split = splits[axis];
        int lsize = lsizes[axis];

        if (lsize == 0 || lsize == numTriangles)
        {
            // No split available

            m_Nodes[currentIdx].FirstTriangle = m_Buckets.getSize();
            m_Nodes[currentIdx].NumTriangles = numTriangles;
            for (int i = 0; i < numTriangles; i++)
                m_Buckets.pushBack(triangles[i]);
        }
        else
        {
            // Partition triangles

#if 0 // \note [Hannu] I had some problems with this on Linux
            int l = 0;
            int r = numTriangles-1;

            while (l < r)
            {
                while (triangles[l].Center[axis] <= split) l++;
                while (triangles[r].Center[axis] > split) r--;

                if (l < r)
                    swap(triangles[l], triangles[r]);
            }
#else
            int l = 0;
            for (int i = 0; i < numTriangles; i++)
                if (m_Centers[triangles[i]][axis] <= split)
                {
                    if (i != l)
                        swap(triangles[l], triangles[i]);
                    l++;
                }
#endif

            UMBRA_ASSERT(l >= 0 && l <= numTriangles);
            // \todo [Hannu] jopefix, can't handle border cases
            if (l == 0)
                l++;
            if (l == numTriangles)
                l--;

            AABB left, right;
            computeAABB(left, triangles, l);
            computeAABB(right, triangles + l, numTriangles - l);

            int leftIdx = buildBVHRecursive(triangles, l, left);
            UMBRA_UNREF(leftIdx);
            int rightIdx = buildBVHRecursive(triangles + l, numTriangles - l, right);

            UMBRA_ASSERT(leftIdx == currentIdx+1);
            m_Nodes[currentIdx].RightChild = rightIdx;
        }
    }

    return currentIdx;
}

//------------------------------------------------------------------------

float ImpRayTracer::computeBestSplit(
    float&                  fSplit,
    int&                    iLeftSize,
    int*                    pTriangles,
    int                     iNumTriangles,
    int                     axis)
{
    AABB left, right;

    if (iNumTriangles > BVH_SPLIT_SAMPLES)
    {
        // Generate split samples

        float fpSplitTestValues[BVH_SPLIT_SAMPLES];
        for (int i = 0, j = 0; i < BVH_SPLIT_SAMPLES; i++, j += iNumTriangles/BVH_SPLIT_SAMPLES)
            fpSplitTestValues[i] = m_Centers[pTriangles[j]][axis];

        // Compute best split value

        float fMinCost = FLT_MAX;
        for (int i = 0; i < BVH_SPLIT_SAMPLES; i++)
        {
            float s = fpSplitTestValues[i];
            float c;
            int l;

            computeSplitCost(c, l, pTriangles, iNumTriangles, s, axis);

            if (c < fMinCost)
            {
                fMinCost    = c;
                fSplit      = s;
                iLeftSize   = l;
            }
        }

        return fMinCost;
    }
    else
    {
        // Brute force sampling

        float fMinCost = FLT_MAX;
        for (int i = 0; i < iNumTriangles; i++)
        {
            float s = m_Centers[pTriangles[i]][axis];
            float c;
            int l;

            computeSplitCost(c, l, pTriangles, iNumTriangles, s, axis);

            if (c < fMinCost)
            {
                fMinCost    = c;
                fSplit      = s;
                iLeftSize   = l;
            }
        }

        return fMinCost;
    }
}

//------------------------------------------------------------------------

void ImpRayTracer::computeSplitCost(
    float&                  cost,
    int&                    leftSize,
    int*                    triangles,
    int                     numTriangles,
    float                   split,
    int                     axis)
{
#if defined(BVH_USE_SSE)

    //--------------------------------------------------------------------
    // Compute AABB
    //--------------------------------------------------------------------

    __m128 vmn0 = _mm_set1_ps(FLT_MAX);
    __m128 vmx0 = _mm_set1_ps(-FLT_MAX);
    __m128 vmn1 = _mm_set1_ps(FLT_MAX);
    __m128 vmx1 = _mm_set1_ps(-FLT_MAX);

    int l = 0;
    for (int i = 0; i < numTriangles; i++)
    {
        if (m_Centers[triangles[i]][axis] <= split)
        {
            vmn0 = _mm_min_ps(vmn0, _mm_loadu_ps(&m_TriangleAABBs[triangles[i]].getMin().x));
            vmx0 = _mm_max_ps(vmx0, _mm_loadu_ps(&m_TriangleAABBs[triangles[i]].getMax().x));
            l++;
        }
        else
        {
            vmn1 = _mm_min_ps(vmn1, _mm_loadu_ps(&m_TriangleAABBs[triangles[i]].getMin().x));
            vmx1 = _mm_max_ps(vmx1, _mm_loadu_ps(&m_TriangleAABBs[triangles[i]].getMax().x));
        }
    }

    //--------------------------------------------------------------------
    // Compute AABB surface area
    //--------------------------------------------------------------------

    float fAreaLeft = 0.f;
    float fAreaRight = 0.f;

    if (l>0)
    {
        __m128 d0xyz    = _mm_sub_ps(vmx0, vmn0);                                                                   // d0xyz = vmx1 - vmn1
        __m128 d0xxy    = _mm_shuffle_ps(d0xyz, d0xyz, 0x50);
        __m128 d0yzz    = _mm_shuffle_ps(d0xyz, d0xyz, 0x5A);
        __m128 d0       = _mm_mul_ps(_mm_mul_ps(d0xxy, d0yzz), _mm_set1_ps(2.0f));                                  // d0 = [2*xy, 2*xz, 2*yz]
        __m128 sa0      = _mm_add_ss(_mm_add_ss(d0, _mm_shuffle_ps(d0, d0, 0x55)), _mm_shuffle_ps(d0, d0, 0xAA));   // sa0 = x + y + z
        _mm_store_ss(&fAreaLeft,sa0);
    }

    if (l < numTriangles)
    {
        __m128 d1xyz    = _mm_sub_ps(vmx1, vmn1);                                                                   // d1xyz = vmx1 - vmn1
        __m128 d1xxy    = _mm_shuffle_ps(d1xyz, d1xyz, 0x50);
        __m128 d1yzz    = _mm_shuffle_ps(d1xyz, d1xyz, 0x5A);
        __m128 d1       = _mm_mul_ps(_mm_mul_ps(d1xxy, d1yzz), _mm_set1_ps(2.0f));                                  // d1 = [2*xy, 2*xz, 2*yz]
        __m128 sa1      = _mm_add_ss(_mm_add_ss(d1, _mm_shuffle_ps(d1, d1, 0x55)), _mm_shuffle_ps(d1, d1, 0xAA));   // sa1 = x + y + z
        _mm_store_ss(&fAreaRight, sa1);
    }

    leftSize = l;
    cost = fAreaLeft * l + fAreaRight*(numTriangles-l);

#else

    //--------------------------------------------------------------------
    // Non-SSE version
    //--------------------------------------------------------------------

    AABB left;
    AABB right;

    int l = 0;

    for (int i = 0; i < numTriangles; ++i)
    {
        if (m_Centers[triangles[i]][axis] <= split)
        {
            left.grow(m_TriangleAABBs[triangles[i]]);
            ++l;
        }
        else
        {
            right.grow(m_TriangleAABBs[triangles[i]]);
        }
    }

    float fAreaLeft = 0.f;
    float fAreaRight = 0.f;

    if (l>0)
        fAreaLeft = left.getSurfaceArea();

    if (l < numTriangles)
        fAreaRight = right.getSurfaceArea();

    leftSize = l;
    cost = fAreaLeft * l + fAreaRight*(numTriangles-l);
#endif
}

//------------------------------------------------------------------------

void ImpRayTracer::rayCast(StackEntry* stack, BVHRayCastData* data, int nodeOrigIdx)
{
    if (!m_Nodes.getSize())
        return;

    int sp = 0;
    stack[sp++] = StackEntry(nodeOrigIdx, data->Enter, data->Exit);

    const int sx = data->DirSgn[0];
    const int sy = data->DirSgn[1];
    const int sz = data->DirSgn[2];

    UMBRA_ASSERT(m_Nodes.getSize());

    while (sp)
    {
        UMBRA_ASSERT(sp < StackSize);

        StackEntry entry    = stack[--sp];
        int currentIdx      = entry.Node;
        const BVHNode& node = m_Nodes[currentIdx];

        // Early exit when possible

        if (entry.Enter > data->MinDistance)
            continue;

        if (node.NumTriangles)
        {
            // Leaf

            for (int i = 0; i < node.NumTriangles; i++)
            {
                float u, v, t;

                // Intersect

                bool isect = intersectRayTriangle(
                    data->Origin,
                    data->Dir,
                    m_Vertices[m_Triangles[node.FirstTriangle+i].Vertex[0] & ~0x40000000],
                    m_Vertices[m_Triangles[node.FirstTriangle+i].Vertex[1]],
                    m_Vertices[m_Triangles[node.FirstTriangle+i].Vertex[2]],
                    t, u, v);

                if (isect && t >= 0.0f)
                {
                    if (t < data->MinDistance)
                    {
                        data->MinDistance   = t;
                        data->NumTriangles  = 1;
                        data->Triangles[0]  = node.FirstTriangle + i;
                    }
                    else if (t == data->MinDistance && data->NumTriangles < BVHRayCastData::MAX_TRIANGLES)
                    {
                        data->Triangles[data->NumTriangles++] = node.FirstTriangle + i;
                    }
                }
            }
        }
        else
        {
            // Recurse

            int child = 0;
            float t[2];
            float dEnter = entry.Enter;
            float dExit;

            for (int c = 0; c < 2; c++)
            {
                int childIdx             = (c == 0) ? currentIdx+1 : node.RightChild;
                const BVHNode& childNode = m_Nodes[childIdx];

                dEnter  = entry.Enter;
                BVH_MIN(dExit, entry.Exit, data->MinDistance);

                // Potential NaN (Zero times Inf) is OK -- in that case ray misses the box

                t[0] = (childNode.Bound.getMin().x - data->Origin.x) * data->InvDir.x;
                t[1] = (childNode.Bound.getMax().x - data->Origin.x) * data->InvDir.x;

                if (t[sx] <= dExit)
                {
                    BVH_MAX(dEnter, t[sx], dEnter);
                    BVH_MIN(dExit, t[sx^1], dExit);

                    t[0] = (childNode.Bound.getMin().y - data->Origin.y) * data->InvDir.y;
                    t[1] = (childNode.Bound.getMax().y - data->Origin.y) * data->InvDir.y;

                    if (t[sy] <= dExit)
                    {
                        BVH_MAX(dEnter, t[sy], dEnter);
                        BVH_MIN(dExit, t[sy^1], dExit);

                        t[0] = (childNode.Bound.getMin().z - data->Origin.z) * data->InvDir.z;
                        t[1] = (childNode.Bound.getMax().z - data->Origin.z) * data->InvDir.z;

                        if (t[sz] <= dExit)
                        {
                            BVH_MAX(dEnter, t[sz], dEnter);
                            BVH_MIN(dExit, t[sz^1], dExit);

                            if (dEnter <= data->MinDistance && dExit > 0.0f)
                            {
                                stack[sp++] = StackEntry(childIdx, dEnter, dExit);
                                child++;
                            }
                        }
                    }
                }
            }

            // Sort children

            if (child == 2 && stack[sp-2].Enter < dEnter)
            {
                UMBRA_ASSERT(sp >= 2);
                swap(stack[sp-2], stack[sp-1]);
            }
        }
    }
}

//------------------------------------------------------------------------

static void getRayEnterExit(float& enter, float& exit, const AABB& aabb, const Vector3& origin, const Vector3& dir)
{
    // Compute ray enter and exit distances
    // Div by zero and overflow is OK and leads to +-inf time bounds for the ray.

    Vector3 invDir(1.0f / dir.x, 1.0f / dir.y, 1.0f / dir.z);

    float mn[3];
    float mx[3];

    for (int i = 0; i < 3; i++)
    {
        mn[i]   = (aabb.getMin()[i] - origin[i])*invDir[i];
        mx[i]   = (aabb.getMax()[i] - origin[i])*invDir[i];
        if (dir[i] < 0.0f)
            swap(mn[i], mx[i]);
    }

    enter = mn[0];
    if (mn[1] > enter) enter = mn[1];
    if (mn[2] > enter) enter = mn[2];

    exit = mx[0];
    if (mx[1] < exit) exit = mx[1];
    if (mx[2] < exit) exit = mx[2];
}

//------------------------------------------------------------------------

RayTracer::RayTraceResult ImpRayTracer::rayTrace(StackEntry* stack, const Vector3& origin, const Vector3& dir, float maxDist, float& dist, Vector3* vert)
{
    UMBRA_ASSERT(maxDist >= 0.f);

    RayTracer::RayTraceResult res = RayTracer::NO_HIT;

    if (!m_Nodes.getSize())
        return res;

    // Let's go!

    float enter, exit;
    getRayEnterExit(enter, exit, m_aabb, origin, dir);

    BVHRayCastData data(origin, dir, enter, exit);

    rayCast(stack, &data, 0);

    if (data.NumTriangles)
        dist = data.MinDistance;

    if (data.NumTriangles && (maxDist == 0.f || data.MinDistance < maxDist))
    {
        // Check if we hit any front faces

        res = RayTracer::HIT_BACKFACE;

        for (int i = 0; i < data.NumTriangles; i++)
        {
            bool flipped = !!(m_Triangles[data.Triangles[i]].Vertex[0] & 0x40000000);

            float sgn = dot(dir, cross(
                m_Vertices[m_Triangles[data.Triangles[i]].Vertex[1]] - m_Vertices[m_Triangles[data.Triangles[i]].Vertex[0] & ~0x40000000],
                m_Vertices[m_Triangles[data.Triangles[i]].Vertex[2]] - m_Vertices[m_Triangles[data.Triangles[i]].Vertex[0] & ~0x40000000]));

            if (vert)
            {
                vert[0] = m_Vertices[m_Triangles[data.Triangles[i]].Vertex[0] & ~0x40000000];
                vert[1] = m_Vertices[m_Triangles[data.Triangles[i]].Vertex[1]];
                vert[2] = m_Vertices[m_Triangles[data.Triangles[i]].Vertex[2]];
            }

            if (flipped ? sgn > 0.0f : sgn < 0.f)
            {
                res = RayTracer::HIT_FRONTFACE;
                break;
            }
        }
    }

    return res;
}

//------------------------------------------------------------------------

bool ImpRayTracer::rayCastFirst(StackEntry* stack, const Vector3& origin, const Vector3& dir, RayTracer::Triangle& outTriangle)
{
    float enter, exit;
    getRayEnterExit(enter, exit, m_aabb, origin, dir);
    BVHRayCastData data(origin, dir, enter, exit);
    rayCast(stack, &data, 0);

    if (data.NumTriangles)
    {
        outTriangle = m_Triangles[data.Triangles[0]];
        return true;
    }

    return false;
}

//------------------------------------------------------------------------

RayTracer::RayTracer(const PlatformServices& platform)
{
    m_imp = UMBRA_HEAP_NEW(platform.allocator, ImpRayTracer, platform);
}

//------------------------------------------------------------------------

RayTracer::~RayTracer()
{
    UMBRA_HEAP_DELETE(m_imp->getPlatform().allocator, m_imp);
}

//------------------------------------------------------------------------

RayTracer::RayTracer(const RayTracer& other)
{
    *m_imp = *other.m_imp;
}

//------------------------------------------------------------------------

RayTracer& RayTracer::operator=(const RayTracer& other)
{
    *m_imp = *other.m_imp;
    return *this;
}

//------------------------------------------------------------------------

void RayTracer::buildBVH(const Vector3* vertices, const RayTracer::Triangle* triangles, int numVertices, int numTriangles)
{
    UMBRA_ASSERT(vertices && triangles && numVertices > 0 && numTriangles > 0);

    AABB aabb;
    for (int i = 0; i < numVertices; i++)
        aabb.grow(vertices[i]);

    m_imp->buildBVH(aabb, vertices, triangles, numVertices, numTriangles);
}

//------------------------------------------------------------------------

void RayTracer::buildBVH(const GeometryBlock& gb)
{
    Array<Triangle> triangles(m_imp->getPlatform().allocator);

    for (int i = 0; i < gb.getTriangleCount(); i++)
    {
        if (gb.getTriangleObject(i).isOccluder())
            triangles.pushBack(Triangle(gb.getTriangle(i).m_vertices));
    }

    m_imp->buildBVH(gb.getOccluderAABB(), gb.getVertices().getPtr(), triangles.getPtr(), gb.getVertices().getSize(), triangles.getSize());
}

//------------------------------------------------------------------------

class ImpRayTracerTraversal : public ImpRayTracerDefs
{
public:

    ImpRayTracerTraversal(void)
    :   m_rayTracer(NULL)
    {
    }

    void deinit(void)
    {
        if (m_rayTracer)
        {
            m_stack.reset(0);
            m_stack.shrinkToFit();
            m_rayTracer = NULL;
        }
    }

    void init(ImpRayTracer* imp)
    {
        deinit();

        m_rayTracer = imp;
        m_platformServices = imp->getPlatform();
        m_stack.setAllocator(imp->getPlatform().allocator);
        m_stack.reset(ImpRayTracerDefs::StackSize);
    }

    RayTraceResult rayTrace(const Vector3& origin, const Vector3& dir, float maxDist, float& dist, Vector3* vert) const
    {
        UMBRA_ASSERT(m_rayTracer);
        return m_rayTracer->rayTrace(m_stack.getPtr(), origin, dir, maxDist, dist, vert);
    }

    bool rayCastFirst(const Vector3& origin, const Vector3& dir, RayTracer::Triangle& outTriangle) const
    {
        UMBRA_ASSERT(m_rayTracer);
        return m_rayTracer->rayCastFirst(m_stack.getPtr(), origin, dir, outTriangle);
    }

    const PlatformServices& getPlatformServices(void)
    {
        return m_platformServices;
    }

private:

    ImpRayTracer*                       m_rayTracer;
    PlatformServices                    m_platformServices;
    Array<ImpRayTracerDefs::StackEntry> m_stack;
};

//------------------------------------------------------------------------

RayTracerTraversal::RayTracerTraversal(void)
:   m_imp(NULL)
{
}

//------------------------------------------------------------------------

RayTracerTraversal::RayTracerTraversal(const RayTracer& tracer)
:   m_imp(NULL)
{
    if (&tracer == 0)
        return;
    init(tracer);
}

//------------------------------------------------------------------------

RayTracerTraversal::~RayTracerTraversal(void)
{
    if (!m_imp)
        return;
    Allocator* allocator = m_imp->getPlatformServices().allocator;
    UMBRA_HEAP_DELETE(allocator, m_imp);
}

//------------------------------------------------------------------------

RayTracerDefs::RayTraceResult RayTracerTraversal::rayTrace(const Vector3& origin, const Vector3& dir, float maxDist, float& dist, Vector3* vert) const
{
    UMBRA_ASSERT(m_imp);
    return m_imp->rayTrace(origin, dir, maxDist, dist, vert);
}

//------------------------------------------------------------------------

bool RayTracerTraversal::rayCastFirst(const Vector3& origin, const Vector3& dir, RayTracer::Triangle& outTriangle) const
{
    UMBRA_ASSERT(m_imp);
    return m_imp->rayCastFirst(origin, dir, outTriangle);
}

//------------------------------------------------------------------------

void RayTracerTraversal::init(const RayTracer& rt)
{
    if (m_imp)
    {
        Allocator* allocator = m_imp->getPlatformServices().allocator;
        UMBRA_HEAP_DELETE(allocator, m_imp);
    }

    Allocator* allocator = rt.m_imp->getPlatform().allocator;
    m_imp = UMBRA_HEAP_NEW(allocator, ImpRayTracerTraversal);
    m_imp->init(rt.m_imp);
}

//------------------------------------------------------------------------

} // namespace Umbra

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)

//------------------------------------------------------------------------

