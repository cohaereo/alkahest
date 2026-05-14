#if !defined(UMBRA_EXCLUDE_COMPUTATION)

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
 * \brief   DPVS preprocess implementation
 *
 */

#include "umbraPrivateDefs.hpp"
#include "optimizer/umbraDPVSBuilder.hpp"
#include "umbraDPVSShared.hpp"
#include "umbraAABB.hpp"
#include "umbraRT.hpp"
#include "umbraMatrix.hpp"
#include "umbraRandom.hpp"
#include "umbraTimer.hpp"
#include "umbraLogger.hpp"
#include "umbraWeightedSampler.hpp"
#include "umbraThread.hpp"
#include "umbraPoolAllocator.hpp"
#include "optimizer/umbraObjectGrouper.hpp"
#include "runtime/umbraTome.hpp"
#include "umbraSort.hpp"
#include "umbraHeap.hpp"
#include "umbraSerializer.hpp"
#include <standard/Sampling.hpp>

#include <cstring>
#include <cfloat>
#include <cmath>

#define UMBRA_DPVS_DEBUG 0
#define UMBRA_DPVS_COMPUTE_PVS 1
#define UMBRA_DPVS_ENABLE_GROUPS 1
#define UMBRA_DPVS_VERSION 2

namespace Umbra
{

//------------------------------------------------------------------------

#if UMBRA_DPVS_DEBUG

#include <cstdio>

class FileReader : public Stream
{
public:

    FileReader(void)
    :   m_file  (NULL)
    {
    }

    FileReader(const char* path)
    :   m_file  (NULL)
    {
        init(path);
    }

    ~FileReader(void)
    {
        deinit();
    }

    void init(const char* path)
    {
        m_file = fopen(path, "rb");
        UMBRA_ASSERT(m_file);
    }

    void deinit(void)
    {
        if (m_file)
        {
            fclose(m_file);
            m_file = NULL;
        }
    }

    void serialize(void* buffer, int size)
    {
        UMBRA_ASSERT(m_file);
        size_t s = fread(buffer, sizeof(UINT8), size, m_file); // GCC build fix
        UMBRA_UNREF(s);
    }

    bool isLoading(void) const
    {
        return true;
    }

private:

    FILE* m_file;
};

//------------------------------------------------------------------------

class FileWriter : public Stream
{
public:

    FileWriter(void)
    :   m_file  (NULL)
    {
    }

    FileWriter(const char* path)
    :   m_file  (NULL)
    {
        init(path);
    }

    ~FileWriter(void)
    {
        deinit();
    }

    void init(const char* path)
    {
        m_file = fopen(path, "wb");
        m_bytes = 0;
    }

    void deinit(void)
    {
        if (m_file)
        {
            fclose(m_file);
            m_file = NULL;
        }
    }

    void serialize(void* buffer, int size)
    {
        UMBRA_ASSERT(m_file);
        m_bytes += fwrite(buffer, sizeof(UINT8), size, m_file);
    }

    bool isLoading(void) const
    {
        return false;
    }

    size_t getBytes(void) const
    {
        return m_bytes;
    }

private:

    size_t  m_bytes;
    FILE*   m_file;
};

#endif // UMBRA_DPVS_DEBUG

//------------------------------------------------------------------------

class ImpDPVSInputPath
{
public:

    ImpDPVSInputPath(void)
    {
    }

    ~ImpDPVSInputPath(void)
    {
    }

    void init(const PlatformServices& inPlatformServices, const UINT8* inBuffer, int inBufferSize)
    {
        deinit();

        m_platformServices = inPlatformServices;
        m_nodeArray.setAllocator(inPlatformServices.allocator);

        if (inBuffer)
        {
            MemoryReader reader((UINT8*)inBuffer, inBufferSize);
            reader << *this;
        }
    }

    void deinit(void)
    {
        m_nodeArray.clear();
    }

    void addNode(float inTime, const Vector3& inDirection)
    {
        m_nodeArray.pushBack(Node(inTime, inDirection));
    }

    int getNodeCount(void) const
    {
        return m_nodeArray.getSize();
    }

    void getNode(float& outTime, Vector3& outDirection, int inIndex)
    {
        outTime      = m_nodeArray[inIndex].time;
        outDirection = m_nodeArray[inIndex].direction;
    }

    int getBuffer(UINT8* outBuffer, int outBufferSize)
    {
        if (!outBuffer)
        {
            SizeHelper size;
            size << *this;
            return size.getSize();
        }

        MemoryWriter writer(outBuffer, outBufferSize);
        writer << *this;

        return writer.getByteCount();
    }

    void setBuffer(const UINT8* inBuffer, int inBufferSize)
    {
        MemoryReader reader((UINT8*)inBuffer, inBufferSize);
        reader << *this;
    }

    float getMedianArcLength(void) const
    {
        if (m_nodeArray.getSize() < 2)
            return 0.0f;

        // Apply locally linear approximation

        Array<float> lenSqrArray;
        lenSqrArray.reset(m_nodeArray.getSize()-1);

        for (int i = 0; i < m_nodeArray.getSize()-1; i++)
            lenSqrArray[i] = ((m_nodeArray[i+1].direction-m_nodeArray[i].direction).lengthSqr());

        float m = select((float*)lenSqrArray.getPtr(), lenSqrArray.getSize(), lenSqrArray.getSize()/2);
        return std::sqrt(m);
    }

    static int median3(const float* valueArray, int valueCount)
    {
        if (valueCount < 3)
            return 0;

        int l = 0;
        int c = valueCount/2;
        int h = valueCount;

        if(valueArray[l] > valueArray[h]) swap(l, h);
        if(valueArray[l] > valueArray[c]) swap(l, c);
        if(valueArray[c] > valueArray[h]) swap(c, h);

        return c;
    }

    static float select(float* valueArray, int valueCount, int k)
    {
        int pivot = median3(valueArray, valueCount);
        int size  = 0;

        swap(valueArray[pivot], valueArray[valueCount-1]);
        for (int i = 0; i < valueCount-1; i++)
        {
            if (valueArray[i] < valueArray[valueCount-1])
                swap(valueArray[size++], valueArray[i]);
        }
        swap(valueArray[pivot], valueArray[valueCount-1]);

        if (k < size) return select(valueArray, size, k);
        if (k > size) return select(valueArray+size, valueCount-size, k-size);

        return valueArray[pivot];
    }

    const PlatformServices& getPlatformServices(void) const
    {
        return m_platformServices;
    }

    friend Stream& operator<<(Stream& stream, ImpDPVSInputPath& path)
    {
        int nodeCount = path.getNodeCount();
        stream << nodeCount;

        if (stream.isLoading())
            path.m_nodeArray.reset(nodeCount);

        for (int i = 0; i < path.m_nodeArray.getSize(); i++)
            stream << path.m_nodeArray[i];

        return stream;
    }

private:

    struct Node
    {
        Node(void)
        {
        }

        Node(float inTime, const Vector3& inDirection)
        :   direction   (inDirection)
        ,   time        (inTime)
        {
        }

        Vector3 direction;
        float   time;

        friend Stream& operator<<(Stream& stream, Node& node)
        {
            stream << node.direction.x << node.direction.y << node.direction.z << node.time;
            return stream;
        }
    };

    Array<Node>         m_nodeArray;
    PlatformServices    m_platformServices;
};

//------------------------------------------------------------------------

DPVSInputPath::DPVSInputPath(void)
:   m_imp(NULL)
{
}

//------------------------------------------------------------------------

DPVSInputPath::~DPVSInputPath(void)
{
    if (!m_imp)
        return;

    Allocator* allocator = m_imp->getPlatformServices().allocator;
    m_imp->~ImpDPVSInputPath();
    allocator->deallocate(m_imp);
    m_imp = NULL;
}

//------------------------------------------------------------------------

void DPVSInputPath::init(const PlatformServices& inPlatformServices, const UINT8* inData, int inDataSize)
{
    PlatformServices services = inPlatformServices;
    if (!services.allocator)
        services.allocator = getAllocator();

    m_imp = UMBRA_HEAP_NEW(services.allocator, ImpDPVSInputPath);
    m_imp->init(inPlatformServices, inData, inDataSize);
}

//------------------------------------------------------------------------

void DPVSInputPath::addNode(float inTime, const Vector3& inDirection)
{
    UMBRA_ASSERT(m_imp);

    m_imp->addNode(inTime, inDirection);
}

//------------------------------------------------------------------------

void DPVSInputPath::setNodeArray(const float* inTimeArray, const Vector3* inDirectionArray, int inArraySize)
{
    UMBRA_ASSERT(m_imp);

    for (int i = 0; i < inArraySize; i++)
        m_imp->addNode(inTimeArray[i], inDirectionArray[i]);
}

//------------------------------------------------------------------------

int DPVSInputPath::getNodeCount(void) const
{
    UMBRA_ASSERT(m_imp);

    return m_imp->getNodeCount();
}

//------------------------------------------------------------------------

void DPVSInputPath::getNode(float& outTime, Vector3& outDirection, int inIndex) const
{
    UMBRA_ASSERT(m_imp);

    m_imp->getNode(outTime, outDirection, inIndex);
}

//------------------------------------------------------------------------

int DPVSInputPath::getBuffer(UINT8* outBuffer, int outBufferSize)
{
    UMBRA_ASSERT(m_imp);

    return m_imp->getBuffer(outBuffer, outBufferSize);
}

//------------------------------------------------------------------------

void DPVSInputPath::setBuffer(const UINT8* inBuffer, int inBufferSize)
{
    UMBRA_ASSERT(m_imp);

    m_imp->setBuffer(inBuffer, inBufferSize);
}

//------------------------------------------------------------------------

class ImpDPVSResult: public Base
{
public:
    ImpDPVSResult(Allocator* a): Base(a) {}

    int getObjectCount(void) const
    {
        return objectIDs.getSize();
    }

    PVSVector* lookup(float time)
    {
        for (int i = 0; i < pvsArray.getSize(); i++)
        {
            if (pvsArray[i].contains(time))
                return pvsArray[i].lookup(time);
        }
        return NULL;
    }

    template<typename OP> void streamOp (OP& op)
    {
        int version = UMBRA_DPVS_VERSION;
        stream(op, version);
        op.require(version == UMBRA_DPVS_VERSION);
        stream(op, objectIDs);
        stream(op, pvsArray);
    }

    FixedArray<UINT32> objectIDs;
    FixedArray<UncompressedDPVS> pvsArray;
};

//------------------------------------------------------------------------

DPVSResult::DPVSResult(void)
{
    m_imp = NULL;
}

//------------------------------------------------------------------------

DPVSResult::~DPVSResult(void)
{
    if (!m_imp)
        return;
    Allocator* allocator = m_imp->getAllocator();
    DPVS_ALLOCATOR(allocator);
    UMBRA_HEAP_DELETE(allocator, m_imp);
    m_imp = NULL;
}

//------------------------------------------------------------------------

bool DPVSResult::serialize(OutputStream& out) const
{
    if (!m_imp)
        return false;
    Serializer serializer(&out);
    stream(serializer, *m_imp);
    return serializer.isOk();
}

//------------------------------------------------------------------------

class StratifiedDirectionalSampler
{
public:

    void init(const Vector3& d0, const Vector3& d1, int binCount)
    {
        deinit();

        Vector3 delta = (d1-d0)/float(binCount);

        for (int i = 0; i < binCount; i++)
            m_sampleArray.pushBack(normalize(d0 + float(i)*delta));

        m_sampleArray.pushBack(d1);
    }

    void deinit(void)
    {
        m_sampleArray.clear();
    }

    int getBinCount(void) const
    {
        return m_sampleArray.getSize() - 1;
    }

    Vector3 getSample(void)
    {
        int index = m_random.getI()%(m_sampleArray.getSize()-1);
        return normalize(getRandomVector3(m_sampleArray[index], m_sampleArray[index+1]));
    }

private:

    Vector3 getRandomVector3(const Vector3& mn, const Vector3& mx)
    {
        return Vector3(
            getRandomFloat(mn[0], mx[0]),
            getRandomFloat(mn[1], mx[1]),
            getRandomFloat(mn[2], mx[2]));
    }

    float getRandomFloat(float lo, float hi)
    {
        return m_random.get()*(hi-lo) + lo;
    }

    Random         m_random;
    Array<Vector3> m_sampleArray;
};

//------------------------------------------------------------------------

class AABBSampler
{
public:

    void init(const AABB& aabb, float randomRotation, const Vector3& Direction, int TargetCount)
    {
        deinit();

        Matrix4x4 Basis = MatrixFactory::orthonormalBasis(Direction);
        Matrix4x4 Rotation = MatrixFactory::rotateZ(randomRotation);
        Basis = Rotation*Basis;

        Vector3 BasisU = Basis.getRight();
        Vector3 BasisV = Basis.getUp();
        Vector3 BasisW = Basis.getDof();

        // Compute AABB in the new basis

        Vector3 Max(-FLT_MAX, -FLT_MAX, -FLT_MAX);
        Vector3 Min(FLT_MAX, FLT_MAX, FLT_MAX);

        for (int I = 0; I < 8; I++)
        {
            float DU = dot(BasisU, aabb.getCorner((AABB::Corner)I));
            float DV = dot(BasisV, aabb.getCorner((AABB::Corner)I));
            float DW = dot(BasisW, aabb.getCorner((AABB::Corner)I));

            Min = min(Min, Vector3(DU, DV, DW));
            Max = max(Max, Vector3(DU, DV, DW));
        }

        // Hammersley p2

        for (int I = 0; I < TargetCount; I++)
        {
            float   FX     = I / float(TargetCount);
            float   FY     = haltonf<2>(I);
            Vector3 S      = (FX*(Max.x-Min.x) + Min.x)*BasisU + (FY*(Max.y-Min.y) + Min.y)*BasisV + Min.z*BasisW;
            float   TEnter = 0.0f;
            float   TExit  = FLT_MAX;

            for (int Axis = 0; Axis < 3; Axis++)
            {
                float T[2] =
                {
                    (aabb.getMin()[Axis] - S[Axis])/Direction[Axis],
                    (aabb.getMax()[Axis] - S[Axis])/Direction[Axis],
                };

                int S  = Direction[Axis] >= 0.0f ? 0 : 1;

                TEnter = max2(TEnter, T[S]);
                TExit  = min2(TExit, T[S^1]);
            }

            if (TEnter <= TExit)
                m_sampleArray.pushBack(S + TEnter*Direction - 0.0001f*Direction);
        }
    }

    void deinit(void)
    {
        m_sampleArray.clear();
    }

    int getSampleCount(void) const
    {
        return m_sampleArray.getSize();
    }

    const Vector3& getSample(int index) const
    {
        return m_sampleArray[index];
    }

    friend Stream& operator<<(Stream& stream, AABBSampler& sampler)
    {
        int size = sampler.getSampleCount();
        stream << size;

        if (stream.isLoading())
            sampler.m_sampleArray.reset(size);

        for (int i = 0; i < size; i++)
        {
            Vector3& v = sampler.m_sampleArray[i];
            stream << v[0] << v[1] << v[2];
        }

        return stream;
    }

private:

    Array<Vector3> m_sampleArray;
};

//------------------------------------------------------------------------

class PVSCompressor
{
private:

    struct Node
    {
        Node(void)
        {
            child[0] = NULL;
            child[1] = NULL;
        }

        PVSVector       PVS;
        Array<UINT32>   sparseDifference;
        Vector2         timeBounds;
        Node*           child[2];

        bool isLeaf(void) const
        {
            return child[0] == NULL && child[1] == NULL;
        }
    };

private:

    struct HierarchicalDeltaEncodingTreeBuilder
    {
        HierarchicalDeltaEncodingTreeBuilder(Allocator* a)
        :   nodeAllocator(a), root(NULL)
        {
        }

        HierarchicalDeltaEncodingTreeBuilder(Allocator* a, UncompressedDPVS& pvs)
        :   nodeAllocator(a), root(NULL)
        {
            init(pvs);
        }

        void init(UncompressedDPVS& pvs)
        {
            root = build(pvs.getCellPtr(), pvs.getCellCount());

            pullVisibility(root);
            pushVisibility(root);

#if UMBRA_DPVS_DEBUG
            PVSVector PVS;
            copy(PVS, root->PVS);
            checkConsistency(root, PVS);
#endif // UMBRA_DPVS_DEBUG
        }

        Node* build(UncompressedDPVS::Cell* cellArray, int cellCount)
        {
            if (cellCount == 0)
                return NULL;

            Node* node = new(nodeAllocator.allocate()) Node;

            if (cellCount == 1)
            {
                node->PVS        = cellArray->pvs;
                node->timeBounds = Vector2(cellArray->time[0], cellArray->time[1]);
                return node;
            }

            int middle = cellCount/2;

            node->child[0] = build(cellArray, middle);
            node->child[1] = build(&cellArray[middle], cellCount - middle);

            node->timeBounds[0] = node->child[0]->timeBounds[0];
            node->timeBounds[1] = node->child[1]->timeBounds[1];

            return node;
        }

        void pullVisibility(Node* node)
        {
            if (!node || node->isLeaf())
                return;

            if (node->child[0])
                pullVisibility(node->child[0]);
            if (node->child[1])
                pullVisibility(node->child[1]);

            if (node->child[0] && node->child[1])
                computeUnion(node->PVS, node->child[0]->PVS, node->child[1]->PVS);
            else if (node->child[0])
                node->PVS = node->child[0]->PVS;
            else
                node->PVS = node->child[1]->PVS;
        }

        void pushVisibility(Node* node)
        {
            if (!node)
                return;

            PVSVector XOR;
            for (int i = 0; i < 2; i++)
            {
                if (!node->child[i])
                    continue;

                computeDifference(XOR, node->PVS, node->child[i]->PVS);

                for (int j = 0; j < node->PVS.getSize(); j++)
                {
                    if (node->PVS.get(j) != node->child[i]->PVS.get(j))
                        node->child[i]->sparseDifference.pushBack(j);
                }

                UMBRA_ASSERT(computeHammingDistance(node->PVS, node->child[i]->PVS) == node->child[i]->sparseDifference.getSize());
                pushVisibility(node->child[i]);
            }
        }

        void checkConsistency(Node* node, PVSVector& PVS)
        {
            if (!node)
                return;

            int before = computeHammingDistance(PVS, node->PVS);
            PVS.eor(node->sparseDifference.getPtr(), node->sparseDifference.getSize());
            int after = computeHammingDistance(PVS, node->PVS);

            UMBRA_UNREF(before);
            UMBRA_UNREF(after);

            if (node->isLeaf())
            {
                UMBRA_ASSERT(computeHammingDistance(PVS, node->PVS) == 0);
                PVS.eor(node->sparseDifference.getPtr(), node->sparseDifference.getSize());
                return;
            }

            checkConsistency(node->child[0], PVS);
            checkConsistency(node->child[1], PVS);

            PVS.eor(node->sparseDifference.getPtr(), node->sparseDifference.getSize());
        }

        PoolAllocator<Node> nodeAllocator;
        Node*               root;
    };

    struct LinearTreeBuilder
    {
        LinearTreeBuilder(void)
        {
        }

        LinearTreeBuilder(Node* node)
        {
            init(node);
        }

        void init(Node* node)
        {
            if (!node)
                return;

            nodeArray.pushBack(CompressedDPVS::Node());
            buildBSP(node, 0);

            // Pad to 4-byte boundary

            if (diffArray.getSize()&1)
                diffArray.pushBack(0xFFFF);
        }

        CompressedDPVS::Node buildBSP(Node* node, int index)
        {
            if (!node)
                return CompressedDPVS::Node();

            // Copy sparse difference array

            nodeArray[index].diffIndex     = diffArray.getSize();
            nodeArray[index].diffSize      = node->sparseDifference.getSize();
            nodeArray[index].timeBounds[0] = node->timeBounds[0];
            nodeArray[index].timeBounds[1] = node->timeBounds[1];

            for (int i = 0; i < node->sparseDifference.getSize(); i++)
                diffArray.pushBack((UINT16)(node->sparseDifference[i]&0xFFFF));

            for (int i = 0; i < 2; i++)
            {
                if (!node->child[i])
                    continue;

                nodeArray[index].child[i] = (UINT16)(nodeArray.getSize()&0xFFFF);
                nodeArray.pushBack(CompressedDPVS::Node());

                buildBSP(node->child[i], nodeArray[index].child[i]);
            }

            return nodeArray[index];
        }

        Array<UINT16>               diffArray;
        Array<CompressedDPVS::Node> nodeArray;
        PVSVector                   rootPVS;
    };

public:

    PVSCompressor(Allocator* a, CompressedDPVS& outPVS, UncompressedDPVS& inPVS)
    {
        init(a, outPVS, inPVS);
    }

    void init(Allocator* a, CompressedDPVS& outPVS, UncompressedDPVS& inPVS)
    {
        // Build hiearchical delta encoding tree

        HierarchicalDeltaEncodingTreeBuilder hierarchicalBuilder(a, inPVS);

        if (!hierarchicalBuilder.root)
            return;

        // Linearize tree

        LinearTreeBuilder linearBuilder(hierarchicalBuilder.root);

        // Copy to output data

        outPVS.rootPVS = hierarchicalBuilder.root->PVS;
        outPVS.diffArray.reset(linearBuilder.diffArray.getSize());
        std::memcpy(outPVS.diffArray.getPtr(), linearBuilder.diffArray.getPtr(), outPVS.diffArray.getByteSize());
        outPVS.nodeArray.reset(linearBuilder.nodeArray.getSize());
        std::memcpy(outPVS.nodeArray.getPtr(), linearBuilder.nodeArray.getPtr(), outPVS.nodeArray.getByteSize());
    }
};

//------------------------------------------------------------------------

class ImpDPVSOutputWriter
{
    struct Data
    {
        Data(void)
        {
        }

        Data(int inSize, int inOffset, const void* inSourceData, int inSourceDataSize, int inPaddedDataSize, int* inPointer)
        :   size            (inSize)
        ,   offset          (inOffset)
        ,   sourceData      (inSourceData)
        ,   sourceDataSize  (inSourceDataSize)
        ,   paddedDataSize  (inPaddedDataSize)
        ,   pointer         (inPointer)
        {
        }

        int         size;     // number of elements
        int         offset;
        const void* sourceData;
        int         sourceDataSize;
        int         paddedDataSize;
        int*        pointer;
    };

    struct DataArrayBuilder
    {
        DataArrayBuilder(const PlatformServices& platformServices)
        {
            m_dataArray.setAllocator(platformServices.allocator);
        }

        void add(int size, const void* data, int dataSize, int* pointer = NULL)
        {
            int offset = 8;

            for (int i = 0; i < m_dataArray.getSize(); i++)
            {
                m_dataArray[i].offset += 8;
                offset = m_dataArray[i].offset + m_dataArray[i].paddedDataSize;
            }

            m_dataArray.pushBack(Data(size, offset, data, dataSize, dataSize + (dataSize&3), pointer));
        }

        int getBufferSize(void) const
        {
            int dataSize = 8*m_dataArray.getSize();
            for (int i = 0; i < m_dataArray.getSize(); i++)
                dataSize += m_dataArray[i].paddedDataSize;
            return dataSize;
        }

        void getBuffer(Array<UINT8>& buffer)
        {
            // Init working memory

            int bufferSize = getBufferSize();
            buffer.reset(bufferSize);
            std::memset(buffer.getPtr(), 0, buffer.getSize());

            // Update pointers

            for (int i = 0; i < m_dataArray.getSize(); i++)
            {
                if (m_dataArray[i].pointer)
                {
                    m_dataArray[i].pointer[0] = m_dataArray[i].size;
                    m_dataArray[i].pointer[1] = m_dataArray[i].offset;
                }
            }

            // Write data

            for (int i = 0; i < m_dataArray.getSize(); i++)
            {
                const Data& data = m_dataArray[i];

                int* header = (int*)&buffer[8*i];
                *header++   = data.size;
                *header     = data.offset;

                std::memcpy(&buffer[data.offset], data.sourceData, data.sourceDataSize);
            }
        }

        Array<Data> m_dataArray;
    };

public:

    void init(const PlatformServices& platformServices, const FixedArray<UINT32>& pvsIndexRemapTable, const FixedArray<CompressedDPVS>& pvsArray)
    {
        m_buffer.setAllocator(platformServices.allocator);

        DataArrayBuilder builder(platformServices);
        builder.add(pvsIndexRemapTable.getSize(), pvsIndexRemapTable.getPtr(), pvsIndexRemapTable.getSize()*sizeof(UINT32));

        Array<DPVSRuntimeDeltaEncodeTree> treeArray(platformServices.allocator);
        treeArray.reset(pvsArray.getSize());
        builder.add(treeArray.getSize(), treeArray.getPtr(), treeArray.getSize()*sizeof(DPVSRuntimeDeltaEncodeTree));
        int* ptr = (int*)&treeArray[0];

        for (int i = 0; i < pvsArray.getSize(); i++)
        {
            builder.add(pvsArray[i].rootPVS.getSize(), pvsArray[i].rootPVS.getPtr(), pvsArray[i].rootPVS.getByteSize(), ptr);
            ptr += 2;
            builder.add(pvsArray[i].diffArray.getSize(), pvsArray[i].diffArray.getPtr(), pvsArray[i].diffArray.getByteSize(), ptr);
            ptr += 2;
            builder.add(pvsArray[i].nodeArray.getSize(), pvsArray[i].nodeArray.getPtr(), pvsArray[i].nodeArray.getByteSize(), ptr);
            ptr += 2;
        }

        builder.getBuffer(m_buffer);

#if 0

        // Check consistency

        UINT8* base = m_buffer.getPtr();
        DPVSRuntimeData* data = (DPVSRuntimeData*)m_buffer.getPtr();
        DPVSRuntimeArrayView<int> indexRemapTableView(data->remapTable, base);

        UMBRA_ASSERT(indexRemapTableView.getSize() == pvsIndexRemapTable.getSize());
        for (int i = 0; i < indexRemapTableView.getSize(); i++)
        {
            UMBRA_ASSERT(indexRemapTableView[i] == pvsIndexRemapTable[i]);
        }

        DPVSRuntimeArrayView<DPVSRuntimeDeltaEncodeTree> treeArrayView(data->deltaEncodeTreeArray, base);
        UMBRA_ASSERT(treeArrayView.getSize() == pvsArray.getSize());

        for (int i = 0; i < treeArrayView.getSize(); i++)
        {
            const DPVSRuntimeDeltaEncodeTree& tree = treeArrayView[i];
            const CompressedDPVS& dpvs = pvsArray[i];

            // Check root pvs

            DPVSRuntimeArrayView<UINT32> rootPVSView(tree.rootPVS, base);
            PVSVector rootPVS;
            rootPVS.reset(rootPVSView.getSize());
            fastCopy(rootPVS, rootPVSView.getPtr());

            UMBRA_ASSERT(rootPVS.getSize() == dpvs.rootPVS.getSize());
            for (int j = 0; j < rootPVS.getSize(); j++)
            {
                UMBRA_ASSERT(rootPVS.get(j) == dpvs.rootPVS.get(j));
            }

            // Check diff array

            DPVSRuntimeArrayView<UINT32> diffView(tree.diffArray, base);
            UMBRA_ASSERT(diffView.getSize() == dpvs.diffArray.getSize());
            //for (int j = 0; j < diffView.getSize(); j++)
            //    UMBRA_ASSERT(diffView.decode16(j) == dpvs.diffArray[j]);

            // Check node array

            DPVSRuntimeArrayView<DPVSRuntimeDeltaEncodeTreeNode> nodeView(tree.nodeArray, base);

            UMBRA_ASSERT(nodeView.getSize() == dpvs.nodeArray.getSize());

            for (int j = 0; j < nodeView.getSize(); j++)
            {
                const DPVSRuntimeDeltaEncodeTreeNode& a = nodeView[j];
                const CompressedDPVS::Node& b = dpvs.nodeArray[j];

                UMBRA_ASSERT(a.getChild(0) == b.child[0]);
                UMBRA_ASSERT(a.getChild(1) == b.child[1]);
                UMBRA_ASSERT(a.getDiffIndex() == b.diffIndex);
                UMBRA_ASSERT(a.getDiffSize() == b.diffSize);
                UMBRA_ASSERT(a.getMinTime() == b.timeBounds[0]);
                UMBRA_ASSERT(a.getMaxTime() == b.timeBounds[1]);
                UMBRA_ASSERT(a.isLeaf() == b.isLeaf());

                // Check diffs

                //for (UINT32 k = 0; k < a.getDiffSize(); k++)
                //{
                //    UMBRA_ASSERT(diffView.decode16(a.getDiffIndex() + k) == dpvs.diffArray[b.diffIndex+k]);
                //}
            }
        }
#endif
    }

    int getBufferSize(void) const
    {
        return m_buffer.getSize();
    }

    void getBuffer(UINT8* buffer)
    {
        std::memcpy(buffer, m_buffer.getPtr(), m_buffer.getSize());
    }

    Allocator* getAllocator(void) const
    {
        return m_buffer.getAllocator();
    }

private:
    Array<UINT8> m_buffer;
};

//------------------------------------------------------------------------

DPVSOutputWriter::DPVSOutputWriter(void)
{
    m_imp = NULL;
}

//------------------------------------------------------------------------

DPVSOutputWriter::~DPVSOutputWriter(void)
{
    if (!m_imp)
        return;
    Allocator* a = m_imp->getAllocator();
    DPVS_ALLOCATOR(a);
    UMBRA_HEAP_DELETE(a, m_imp);
    m_imp = NULL;
}

//------------------------------------------------------------------------

int DPVSOutputWriter::getBuffer(UINT8* outBuffer, int outBufferSize)
{
    if (!m_imp)
        return 0;
    int s = m_imp->getBufferSize();
    if (outBuffer != NULL)
    {
        if (outBufferSize < s)
            return 0;
        m_imp->getBuffer(outBuffer);
    }
    return s;
}

//------------------------------------------------------------------------

class CellCompressor
{
    struct Cell
    {
        int                    pvsIndex;
        bool                   bValid;
        UncompressedDPVS::Cell cellData;

        friend bool operator<(const Cell& a, const Cell& b)
        {
            if (a.pvsIndex == b.pvsIndex)
                return a.cellData.time[0] < b.cellData.time[0];
            return a.pvsIndex < b.pvsIndex;
        }

        friend bool operator>(const Cell& a, const Cell& b)
        {
            if (a.pvsIndex == b.pvsIndex)
                return a.cellData.time[0] > b.cellData.time[0];
            return a.pvsIndex > b.pvsIndex;
        }
    };

    struct Item
    {
        int left;
        int right;
        int cost;

        bool isValid(Cell* cellArray, int* segmentCellCountArray) const
        {
            return cellArray[left].bValid && cellArray[right].bValid && segmentCellCountArray[cellArray[left].pvsIndex] > 1;
        }

        friend bool operator<(const Item& a, const Item& b)
        {
            return a.cost < b.cost;
        }

        friend bool operator>(const Item& a, const Item& b)
        {
            return a.cost > b.cost;
        }
    };

    UncompressedDPVS::Cell collapse(const UncompressedDPVS::Cell& left, UncompressedDPVS::Cell& right)
    {
        UncompressedDPVS::Cell result = left;

        result.dir[1]  = right.dir[1];
        result.time[1] = right.time[1];

        computeUnion(result.pvs, left.pvs, right.pvs);

        for (int i = 0; i < left.pvs.getSize(); i++)
        {
            if (left.pvs.get(i) || right.pvs.get(i))
                UMBRA_ASSERT(result.pvs.get(i));
        }

        return result;
    }

    void findNeighbors(int& left, int& right, const Cell& cell, const Cell* cellArray, int cellCount)
    {
        left = right = -1;

        for (int i = 0; i < cellCount; i++)
        {
            if (!cellArray[i].bValid || cellArray[i].pvsIndex != cell.pvsIndex)
                continue;

            if (cellArray[i].cellData.time[1] == cell.cellData.time[0])
                left = i;

            if (cellArray[i].cellData.time[0] == cell.cellData.time[1])
                right = i;
        }
    }

public:

    CellCompressor(const PlatformServices& services, ImpDPVSResult& inoutChunk, int maxCellCount)
    {
        init(services, inoutChunk, maxCellCount);
    }

    void init(const PlatformServices& services, ImpDPVSResult& inoutChunk, int maxCellCount)
    {
        if (maxCellCount < inoutChunk.pvsArray.getSize())
            maxCellCount = inoutChunk.pvsArray.getSize();

        // Build priority queue for collapse operations

        Array<Cell> cellArray(services.allocator);
        Array<int>  segmentCellCount(services.allocator);

        segmentCellCount.reset(inoutChunk.pvsArray.getSize());
        for (int i = 0; i < segmentCellCount.getSize(); i++)
            segmentCellCount[i] = 0;

        for (int i = 0; i < inoutChunk.pvsArray.getSize(); i++)
        {
            UncompressedDPVS& pvs = inoutChunk.pvsArray[i];

            for (int j = 0; j < pvs.getCellCount(); j++)
            {
                Cell cell;
                cell.bValid   = true;
                cell.pvsIndex = i;
                cell.cellData = pvs.getCell(j);

                cellArray.pushBack(cell);
                segmentCellCount[i]++;
            }
        }

        int cellCount = cellArray.getSize();
        const int initialCellCount = cellArray.getSize();

        Heap<int, Item> queue;

        for (int i = 0; i < cellArray.getSize() - 1; i++)
        {
            if (cellArray[i].pvsIndex != cellArray[i+1].pvsIndex)
                continue;

            Item item;
            item.left  = i;
            item.right = i+1;
            item.cost  = computeHammingDistance(cellArray[i].cellData.pvs, cellArray[i+1].cellData.pvs);

            queue.insert(item.cost, item);
        }

        while (queue.getSize() && cellCount > maxCellCount)
        {
            Item item = queue.getValue(0);
            queue.removeFirst();

            if (!item.isValid(cellArray.getPtr(), segmentCellCount.getPtr()))
                continue;

            // Collapse two cells

            Cell cell;
            cell.bValid = true;
            cell.pvsIndex = cellArray[item.left].pvsIndex;
            cell.cellData = collapse(cellArray[item.left].cellData, cellArray[item.right].cellData);

            cellArray[item.left].bValid  = false;
            cellArray[item.right].bValid = false;

            cellCount--;
            segmentCellCount[cellArray[item.left].pvsIndex]--;

            cellArray.pushBack(cell);

            // Create more collapse candidates

            int left, right;
            findNeighbors(left, right, cell, cellArray.getPtr(), cellArray.getSize());

            if (left != -1)
            {
                Item item;
                item.left  = left;
                item.right = cellArray.getSize() - 1;
                item.cost  = computeHammingDistance(cellArray[item.left].cellData.pvs, cellArray[item.right].cellData.pvs);

                queue.insert(item.cost, item);
            }

            if (right != -1)
            {
                Item item;
                item.left  = cellArray.getSize() - 1;
                item.right = right;
                item.cost  = computeHammingDistance(cellArray[item.left].cellData.pvs, cellArray[item.right].cellData.pvs);

                queue.insert(item.cost, item);
            }
        }

        // Partition into valid / nonvalid

        int validIndex = 0;
        for (int i = 0; i < cellArray.getSize(); i++)
        {
            if (cellArray[i].bValid)
                swap(cellArray[validIndex++], cellArray[i]);
        }

        // Copy output

        for (int i = 0; i < inoutChunk.pvsArray.getSize(); i++)
        {
            int size = 0;

            for (int j = 0; j < validIndex; j++)
            {
                if (cellArray[j].pvsIndex == i)
                    size++;
            }

            inoutChunk.pvsArray[i].m_cellArray.reset(size);
            size = 0;

            for (int j = 0; j < validIndex; j++)
            {
                if (cellArray[j].pvsIndex == i)
                {
                    const UncompressedDPVS::Cell& cell = cellArray[j].cellData;
                    inoutChunk.pvsArray[i].m_cellArray[size++] = cell;
                }
            }

            quickSort(inoutChunk.pvsArray[i].m_cellArray.getPtr(), inoutChunk.pvsArray[i].m_cellArray.getSize());
        }

        UMBRA_LOG_I(services.logger, "Collapsed %d cells to %d cells\n", initialCellCount, cellCount);
    }
};

//------------------------------------------------------------------------

class ImpDPVSBuilder
{
private:

    enum
    {
        DirectionSampleCount = 256*256,    // Phase I:          Number of samples per each direction
        GroupSampleCount     = 512,        // Phase II:         Number of samples per object group
        ObjectSampleCount    = 128,        // Phase III:        Number of samples per each hidden object

        DirectionalBinCount  = 16
    };

private:

    struct Object
    {
        Object(void)
        :   bVisiblePointTracking   (false)
        {
        }

        Object(const AABB& inAABB, int inLinearIndex, int inTriangleCount)
        :   aabb                    (inAABB)
        ,   linearIndex             (inLinearIndex)
        ,   triangleCount           (inTriangleCount)
        ,   groupIndex              (-1)
        ,   bVisiblePointTracking   (false)
        {
        }

        AABB    aabb;
        int     linearIndex;
        int     triangleCount;
        int     groupIndex;

        void enableVisiblePointTracking(const Vector3& p)
        {
            visiblePoint = p;
            bVisiblePointTracking = true;
        }

        Vector3 visiblePoint;
        bool    bVisiblePointTracking;
    };

    struct Group
    {
        Group(void)
        :   hiddenObjectCount (0)
        {
        }

        AABB       aabb;
        Array<int> objectArray;
        int        hiddenObjectCount;

        bool isFullyHidden(void) const
        {
            return objectArray.getSize() == hiddenObjectCount;
        }
    };

    class PVSIDRemapper
    {
        struct RemapEntry
        {
            RemapEntry(void)
            {
            }

            RemapEntry(int inPVSID, int inTomeID)
            :   pvsID   (inPVSID)
            ,   tomeID  (inTomeID)
            {
            }

            int pvsID;
            int tomeID;
        };

    public:

        PVSIDRemapper(const PlatformServices& services, ImpDPVSResult& pvs, const Tome* tome)
        {
            int inObjectCount = pvs.getObjectCount();
            if (!inObjectCount || !tome)
                return;

            // Build reverse pvs object id mapping

            Hash<UINT32, int> reverseMap(services.allocator);
            for (int i = 0; i < pvs.getObjectCount(); i++)
                reverseMap.insert(pvs.objectIDs[i], i);

            // Build remap table

            Array<UINT32> groupIds(services.allocator);
            Array<RemapEntry> entryArray(services.allocator);
            Array<int> ranges(tome->getObjectCount(), services.allocator);
            pvs.objectIDs.reset(tome->getObjectCount());

            for (int i = 0; i < tome->getObjectCount(); i++)
            {
                int n = tome->getObjectUserIDs(i, NULL, 0);
                groupIds.reset(n);
                tome->getObjectUserIDs(i, groupIds.getPtr(), groupIds.getSize());
                for (int j = 0; j < n; j++)
                {
                    UMBRA_ASSERT(reverseMap.contains(groupIds[j]));
                    entryArray.pushBack(RemapEntry(*reverseMap.get(groupIds[j]), i));
                }
                ranges[i] = n;
                // note that this makes the remapping completely redundant
                pvs.objectIDs[i] = i;
            }

            // Collapse PVS vectors

            for (int i = 0; i < pvs.pvsArray.getSize(); i++)
            for (int j = 0; j < pvs.pvsArray[i].getCellCount(); j++)
            {
                UncompressedDPVS::Cell& cell = pvs.pvsArray[i].getCell(j);

                PVSVector collapsedVector;
                collapsedVector.reset(tome->getObjectCount());
                collapsedVector.clearAll();
                int entryIdx = 0;

                for (int k = 0; k < tome->getObjectCount(); k++)
                {
                    bool bVisible = false;

                    // Compute bitwise OR over the original PVS vector range

                    for (int l = 0; l < ranges[k]; l++)
                    {
                        if (cell.pvs.get(entryArray[entryIdx + l].pvsID))
                        {
                            bVisible = true;
                            break;
                        }
                    }

                    if (bVisible)
                        collapsedVector.set(k);
                    entryIdx += ranges[k];
                }

                cell.pvs = collapsedVector;
            }

            UMBRA_LOG_I(services.logger, "Collapsed %d objects to %d objects based on tome indices\n", inObjectCount, tome->getObjectCount());
        }
    };

private:

    class DPVSWorker : public Runnable
    {
    public:

        struct Data
        {
            int                     index;
            UncompressedDPVS::Cell* cellArray;
            int                     cellCount;
            RayTracer*              rayTracer;
            Array<Object>           sceneObjectArray;
            Array<Group>            sceneGroupArray;
            AABB                    sceneAABB;
            PlatformServices        platformServices;
        };

    public:

        unsigned long run(void* arg)
        {
            Data* data = (Data*)arg;
            Random random;
            RayTracerTraversal traversal(*data->rayTracer);

            for (int j = 0; j < data->cellCount; j++)
            {
                computeCellPVS(random, traversal, data->cellArray[j], data->sceneObjectArray, data->sceneGroupArray, data->sceneAABB, j==0);
                UMBRA_LOG_I(data->platformServices.logger, "[%d:%d/%d] done, %d visible\n", data->index, j+1, data->cellCount, data->cellArray[j].pvs.countOnes());
            }

            return 0;
        }

        void computeCellPVS(
            Random&                     random,
            const RayTracerTraversal&   RT,
            UncompressedDPVS::Cell&     cell,
            Array<Object>&              sceneObjectArray,
            Array<Group>&               sceneGroupArray,
            const AABB&                 sceneAABB,
            bool                        bBootstrap)
        {
            cell.pvs.clearAll();

            // Phase I: Sample direction cell and shoot rays to the scene

            Vector3 lightToWorld = -cell.getAverageDirection();

            AABBSampler aabbSampler;
            aabbSampler.init(sceneAABB, 0.0f, lightToWorld, bBootstrap ? 16*DirectionSampleCount : DirectionSampleCount);

            for (int j = 0; j < aabbSampler.getSampleCount(); j++)
            {
                int objectIndex = -1;
                if (rayCast(RT, objectIndex, aabbSampler.getSample(j), lightToWorld) && objectIndex != -1)
                {
                    cell.pvs.set(objectIndex);
                    sceneObjectArray[objectIndex].enableVisiblePointTracking(aabbSampler.getSample(j));
                    sceneGroupArray[sceneObjectArray[objectIndex].groupIndex].hiddenObjectCount = 0;
                }
            }

            // Phase II: Sample hidden groups

            StratifiedDirectionalSampler directionalSampler;
            directionalSampler.init(cell.dir[0], cell.dir[1], DirectionalBinCount);

#if UMBRA_DPVS_ENABLE_GROUPS
            for (int i = 0; i < sceneGroupArray.getSize(); i++)
            {
                if (!sceneGroupArray[i].isFullyHidden())
                    continue;

                AABBSampler aabbSampler;
                aabbSampler.init(sceneGroupArray[i].aabb, 2.0f*3.14159f*random.get(), lightToWorld, GroupSampleCount);

                bool bOccluded = false;

                for (int j = 0; j < aabbSampler.getSampleCount(); j++)
                {
                    const Vector3& o = aabbSampler.getSample(j);
                    const Vector3& d = directionalSampler.getSample();
                    bOccluded        = rayCast(RT, o, d);

                    if (!bOccluded)
                    {
                        sceneGroupArray[i].hiddenObjectCount = 0;
                        break;
                    }
                }
            }
#endif // UMBRA_DPVS_ENABLE_GROUPS

            // Phase III: Sample each invisible object by shooting rays from the AABB surface

            for (int i = 0; i < sceneObjectArray.getSize(); i++)
            {
                if (cell.pvs.get(i))
                    continue;

                Object& object = sceneObjectArray[i];

#if UMBRA_DPVS_ENABLE_GROUPS
                if (sceneGroupArray[object.groupIndex].isFullyHidden())
                    continue;
#endif // UMBRA_DPVS_ENABLE_GROUPS

                if (object.bVisiblePointTracking && !rayCast(RT, object.visiblePoint, -lightToWorld))
                {
                    cell.pvs.set(i);
                    continue;
                }

                AABBSampler aabbSampler;
                aabbSampler.init(sceneObjectArray[i].aabb, 2.0f*3.14159f*random.get(), lightToWorld, bBootstrap ? 16*ObjectSampleCount : ObjectSampleCount);

                bool bOccluded = true;

                for (int j = 0; j < aabbSampler.getSampleCount(); j++)
                {
                    const Vector3& o = aabbSampler.getSample(j);
                    const Vector3& d = directionalSampler.getSample();
                    bOccluded        = rayCast(RT, o, d);

                    if (!bOccluded)
                    {
                        object.enableVisiblePointTracking(o);
                        cell.pvs.set(i);
                        break;
                    }
                }

                if (bOccluded)
                    object.bVisiblePointTracking = false;
            }

#if UMBRA_DPVS_ENABLE_GROUPS

            // Update group visibility status

            for (int i = 0; i < sceneGroupArray.getSize(); i++)
                sceneGroupArray[i].hiddenObjectCount = 0;

            for (int i = 0; i < sceneObjectArray.getSize(); i++)
            {
                if (!cell.pvs.get(i))
                    sceneGroupArray[sceneObjectArray[i].groupIndex].hiddenObjectCount++;
            }

#else
            UMBRA_UNREF(sceneGroupArray);
#endif // UMBRA_DPVS_ENABLE_GROUPS

        }

        UMBRA_INLINE bool rayCast(const RayTracerTraversal& RT, const Vector3& origin,  const Vector3& direction)
        {
            RayTracer::Triangle triangle;
            return RT.rayCastFirst(origin, direction, triangle);
        }

        UMBRA_INLINE bool rayCast(const RayTracerTraversal& RT, int& objectIndex, const Vector3& origin,  const Vector3& direction)
        {
            RayTracer::Triangle triangle;

            if (RT.rayCastFirst(origin, direction, triangle))
            {
                objectIndex = triangle.UserData;
                return true;
            }

            return false;
        }
    };

public:

    ImpDPVSBuilder(const PlatformServices& platform): m_platformServices(platform) {}

    bool build(
        DPVSResult&				outResult,
        const Scene&            inScene,
        const DPVSInputPath*    inPathArray,
        int                     inPathCount,
        const DPVSParams&       inParams)
    {
        // Init result object

        ImpDPVSResult* result = UMBRA_HEAP_NEW(m_platformServices.allocator, ImpDPVSResult, m_platformServices.allocator);
        if (!result)
            return false;

        // Init ray tracer

        AABB          sceneAABB;
        Array<Object> sceneObjectArray(m_platformServices.allocator);
        Array<Group>  sceneGroupArray(m_platformServices.allocator);

        RayTracer RT(m_platformServices);
        initRayTracer(RT, sceneObjectArray, sceneAABB, result->objectIDs, inScene);

        // Init groups

        initGroups(sceneObjectArray, sceneGroupArray);

        // Compute PVS for each cell

        Timer timer(m_platformServices.allocator);
        result->pvsArray.reset(inPathCount);

        for (int i = 0; i < inPathCount; i++)
        {
            timer.resetTimer("DPVS");
            timer.startTimer("DPVS");

            // Launch threads

            result->pvsArray[i].init(sceneObjectArray.getSize(), inPathArray[i]);
            computePathPVS(result->pvsArray[i], &RT, sceneAABB, sceneObjectArray, sceneGroupArray, inParams);

            timer.stopTimer("DPVS");
            UMBRA_LOG_I(m_platformServices.logger, "Computed PVS in %.3f s\n", float(timer.getTimerValue("DPVS")));
        }
        if (outResult.m_imp)
            UMBRA_HEAP_DELETE(outResult.m_imp->getAllocator(), outResult.m_imp);
        outResult.m_imp = result;
        return true;
    }

    bool generateOutput(DPVSOutputWriter& out,
                        const DPVSResult& resultIn,
                        const class Tome* tome,
                        int maxCells)
    {
        if (!resultIn.m_imp)
            return false;

        // Remap result

        ImpDPVSResult result = *resultIn.m_imp;
        PVSIDRemapper(m_platformServices, result, tome);

        // Compress cells to hit maxCells limit

        if (maxCells != -1)
            CellCompressor(m_platformServices, result, maxCells);

        // Compress pvs

        FixedArray<CompressedDPVS> compressedDPVSArray(result.pvsArray.getSize());

        for (int i = 0; i < result.pvsArray.getSize(); i++)
        {
            PVSCompressor(m_platformServices.allocator, compressedDPVSArray[i], result.pvsArray[i]);

            int uncompressedSize = streamGetByteSize(result.pvsArray[i]);
            int compressedSize   = streamGetByteSize(compressedDPVSArray[i]);

            UMBRA_LOG_I(m_platformServices.logger, "Compressed PVS [%d/%d] from %d bytes to %d bytes\n", i+1,
                result.pvsArray.getSize(), uncompressedSize, compressedSize);
        }

        // Init output data

        ImpDPVSOutputWriter* writer = UMBRA_HEAP_NEW(m_platformServices.allocator, ImpDPVSOutputWriter);
        if (!writer)
            return false;
        writer->init(m_platformServices, result.objectIDs, compressedDPVSArray);
        if (out.m_imp)
            UMBRA_HEAP_DELETE(out.m_imp->getAllocator(), out.m_imp);
        out.m_imp = writer;

        UMBRA_LOG_I(m_platformServices.logger, "Final PVS size %d bytes\n", out.m_imp->getBufferSize());
        return true;
    }

    bool loadResult(DPVSResult& result, InputStream& in)
    {
        ImpDPVSResult* imp = UMBRA_HEAP_NEW(m_platformServices.allocator, ImpDPVSResult, m_platformServices.allocator);
        if (!imp)
            return false;
        Deserializer d(&in, m_platformServices.allocator);
        stream(d, *imp);
        if (!d.isOk())
            return false;
        if (result.m_imp)
            UMBRA_HEAP_DELETE(result.m_imp->getAllocator(), result.m_imp);
        result.m_imp = imp;
        return true;
    }

    void initGroups(Array<Object>& sceneObjectArray, Array<Group>& groupArray)
    {
        // Compute object grouping

        ObjectGrouperInput input(getPlatformServices());
        for (int i = 0; i < sceneObjectArray.getSize(); i++)
            input.add(i, sceneObjectArray[i].aabb.getMin(), sceneObjectArray[i].aabb.getMax(), float(sceneObjectArray[i].triangleCount));

        ObjectGrouper grouper(getPlatformServices(), input, ObjectGrouperParams());

        groupArray.reset(grouper.getGroupCount());
        for (int i = 0; i < grouper.getGroupCount(); i++)
        {
            Vector3 mn, mx;
            grouper.getGroupAABB(mn, mx, (UINT32)i);
            groupArray[i].aabb.set(mn,mx);
        }

        for (int i = 0; i < sceneObjectArray.getSize(); i++)
        {
            int groupIndex = grouper.getGroupIndex(i);
            sceneObjectArray[i].groupIndex = groupIndex;
            groupArray[groupIndex].objectArray.pushBack(i);
        }
    }

    void computePathPVS(
        UncompressedDPVS&    pvs,
        RayTracer*           RT,
        const AABB&          sceneAABB,
        const Array<Object>& sceneObjectArray,
        const Array<Group>&  sceneGroupArray,
        const DPVSParams&    params)
    {
#if !UMBRA_DPVS_ENABLE_GROUPS
        UMBRA_UNREF(sceneGroupArray);
#endif

#if UMBRA_DPVS_COMPUTE_PVS

        // ###ari TODO: Use threadpool with smaller fixed sized jobs instead for better load balancing

        int threadCount = min2(params.maxThreads, Thread::getNumProcessors());

        Array<DPVSWorker::Data> dataArray;
        Array<DPVSWorker*>      workerArray;
        Array<Thread*>          threadArray;

        dataArray.reset(threadCount);
        workerArray.reset(threadCount);
        threadArray.reset(threadCount);

        int batchSize       = pvs.getCellCount() / threadCount;
        int batchRemainder  = pvs.getCellCount() % threadCount;

        for (int i = 0; i < threadCount; i++)
        {
            int batchFirst = i*batchSize;

            dataArray[i].index                = i;
            dataArray[i].cellArray            = &pvs.getCellPtr()[batchFirst];
            dataArray[i].cellCount            = batchSize;
            dataArray[i].rayTracer            = RT;
            dataArray[i].sceneAABB            = sceneAABB;
            dataArray[i].sceneObjectArray     = sceneObjectArray;
            dataArray[i].sceneGroupArray      = sceneGroupArray;
            dataArray[i].platformServices     = m_platformServices;

            if (i == threadCount - 1)
                dataArray[i].cellCount += batchRemainder;

            workerArray[i] = UMBRA_HEAP_NEW(m_platformServices.allocator, DPVSWorker);
            threadArray[i] = UMBRA_HEAP_NEW(m_platformServices.allocator, Thread);

            threadArray[i]->setFunction(workerArray[i]);
            threadArray[i]->run(&dataArray[i]);
        }

        for (int i = 0; i < threadCount; i++)
            threadArray[i]->waitToFinish();

        for (int i = 0; i < threadCount; i++)
        {
            UMBRA_HEAP_DELETE(m_platformServices.allocator, workerArray[i]);
            UMBRA_HEAP_DELETE(m_platformServices.allocator, threadArray[i]);
        }
#else
        UMBRA_UNREF(RT);
        UMBRA_UNREF(sceneAABB);
        UMBRA_UNREF(params);
        UMBRA_UNREF(sceneGroupArray);

        for (int i = 0; i < pvs.getCellCount(); i++)
        {
            for (int j = 0; j < sceneObjectArray.getSize(); j++)
                pvs.getCell(i).pvs.set(j);
        }
#endif
    }

    void initRayTracer(RayTracer& RT, Array<Object>& objectArray, AABB& sceneAABB, FixedArray<UINT32>& objectIDs, const Scene& scene)
    {
        Array<RayTracer::Triangle>  triangleArray;
        Array<Vector3>              vertexArray;

        int vertexIndexOffset = 0;
        int linearIndex       = 0;

        Vector3 sceneMin(FLT_MAX, FLT_MAX, FLT_MAX);
        Vector3 sceneMax(-FLT_MAX, -FLT_MAX, -FLT_MAX);

        int targetCount = 0;
        for (int i = 0; i < scene.getObjectCount(); i++)
        {
            const SceneObject* object = scene.getObject(i);
            if (object->getFlags() & SceneObject::TARGET)
                targetCount++;
        }
        objectIDs.reset(targetCount);

        // Gather geometry

        for (int i = 0; i < scene.getObjectCount(); i++)
        {
            const SceneObject* object = scene.getObject(i);

            Matrix4x4 matrix;
            object->getMatrix(matrix);

            if (object->getFlags() & SceneObject::OCCLUDER)
            {
                // Add occluder triangle

                const SceneModel* model   = object->getModel();
                const Vector3* vertices   = model->getVertices();
                const Vector3i* triangles = model->getTriangles();

                for (int j = 0; j < model->getVertexCount(); j++)
                {
                    Vector3 v = matrix.transformDivByW(vertices[j]);
                    sceneMin = min(sceneMin,v);
                    sceneMax = max(sceneMax,v);
                    vertexArray.pushBack(v);
                }

                int userData = (object->getFlags() & SceneObject::TARGET) ? linearIndex : -1;

                for (int j = 0; j < model->getTriangleCount(); j++)
                {
                    RayTracer::Triangle triangle;
                    triangle.Vertex   = triangles[j] + Vector3i(vertexIndexOffset, vertexIndexOffset, vertexIndexOffset);
                    triangle.UserData = userData;
                    triangleArray.pushBack(triangle);
                }

                vertexIndexOffset += model->getVertexCount();
            }

            if (object->getFlags() & SceneObject::TARGET)
            {
                // Gather targget objects

                const SceneModel* model   = object->getModel();
                const Vector3* vertices   = model->getVertices();

                Vector3 mn(FLT_MAX, FLT_MAX, FLT_MAX);
                Vector3 mx(-FLT_MAX, -FLT_MAX, -FLT_MAX);

                for (int j = 0; j < model->getVertexCount(); j++)
                {
                    Vector3 v = matrix.transformDivByW(vertices[j]);

                    mn = min(mn, v);
                    mx = max(mx, v);

                    sceneMin = min(sceneMin,v);
                    sceneMax = max(sceneMax,v);
                }

                AABB aabb(mn, mx);
                objectArray.pushBack(Object(aabb, linearIndex, model->getTriangleCount()));
                objectIDs[linearIndex] = object->getID();

                linearIndex++;
            }
        }

        sceneAABB.set(sceneMin, sceneMax);

#if 0
        FileWriter writer("c:/users/ari/desktop/scene.data");
        int vertexCount = triangleArray.getSize()*3;
        writer << vertexCount;
        for (int i = 0; i < triangleArray.getSize(); i++)
        {
            const Vector3i& t = triangleArray[i].Vertex;
            for (int j = 0; j < 3; j++)
            {
                writer << vertexArray[t[j]][0];
                writer << vertexArray[t[j]][1];
                writer << vertexArray[t[j]][2];
            }
        }
#endif
        // Build BVH

#if UMBRA_DPVS_COMPUTE_PVS
        RT.buildBVH(vertexArray.getPtr(), triangleArray.getPtr(), vertexArray.getSize(), triangleArray.getSize());
#else
        UMBRA_UNREF(RT);
#endif // UMBRA_DPVS_COMPUTE_PVS
    }

    const PlatformServices& getPlatformServices(void) const
    {
        return m_platformServices;
    }

private:

    PlatformServices m_platformServices;
};

//------------------------------------------------------------------------

DPVSBuilder::DPVSBuilder(void)
:   m_imp(NULL)
{
}

//------------------------------------------------------------------------

DPVSBuilder::~DPVSBuilder(void)
{
    if (!m_imp)
        return;

    Allocator* allocator = m_imp->getPlatformServices().allocator;
    DPVS_ALLOCATOR(allocator);
    m_imp->~ImpDPVSBuilder();
    allocator->deallocate(m_imp);
    m_imp = NULL;
}

//------------------------------------------------------------------------

void DPVSBuilder::init(
    const PlatformServices& inPlatformServices)
{
    PlatformServices services = inPlatformServices;
    if (!services.allocator)
        services.allocator = getAllocator();
    DPVS_ALLOCATOR(services.allocator);
    m_imp = UMBRA_HEAP_NEW(services.allocator, ImpDPVSBuilder, services);
}

//------------------------------------------------------------------------

bool DPVSBuilder::build(
    DPVSResult&				result,
    const Scene&            inScene,
    const DPVSInputPath*    inPathArray,
    int                     inPathCount,
    const DPVSParams&       inParams)
{
    if (!m_imp)
        return false;
    DPVS_DEFAULT_ALLOCATOR();
    return m_imp->build(result, inScene, inPathArray, inPathCount, inParams);
}

//------------------------------------------------------------------------

bool DPVSBuilder::generateOutput(DPVSOutputWriter& out, const DPVSResult& result, const class Tome* tome, int maxCells)
{
    if (!m_imp)
        return false;
    DPVS_DEFAULT_ALLOCATOR();
    return m_imp->generateOutput(out, result, tome, maxCells);
}

//------------------------------------------------------------------------

bool DPVSBuilder::loadResult(DPVSResult& result, InputStream& input)
{
    if (!m_imp)
        return false;
    DPVS_DEFAULT_ALLOCATOR();
    return m_imp->loadResult(result, input);
}

//------------------------------------------------------------------------

} // namespace Umbra

//------------------------------------------------------------------------

#endif // UMBRA_EXCLUDE_COMPUTATION
