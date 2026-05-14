// Copyright (c) 2010-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRADPVSSHARED_HPP
#define UMBRADPVSSHARED_HPP

/*!
 * \file
 * \brief
 */

#include "umbraPrivateDefs.hpp"
#include "umbraVector.hpp"
#include "optimizer/umbraScene.hpp"
#include "optimizer/umbraDPVSBuilder.hpp"

#include <string.h>
#include <float.h>

#define UMBRA_DPVS_MAX_OBJECT_COUNT 16*1024*8

namespace Umbra
{

//------------------------------------------------------------------------

Allocator*  DPVSGetAllocator        (void);
void        DPVSSetAllocator        (Allocator* allocator);

//------------------------------------------------------------------------

class DPVSScopedAllocator
{
public:

    DPVSScopedAllocator(Allocator* a)
    {
        DPVSSetAllocator(a);
    }

    ~DPVSScopedAllocator(void)
    {
        DPVSSetAllocator(NULL);
    }
};

#define DPVS_ALLOCATOR(x) DPVSScopedAllocator __dpvs_scoped_allocator(x)
#define DPVS_DEFAULT_ALLOCATOR() DPVS_ALLOCATOR(m_imp->getPlatformServices().allocator)

//------------------------------------------------------------------------

class Stream
{
public:

	virtual ~Stream(void) {}

	virtual bool isLoading(void) const = 0;
	virtual void serialize(void* buffer, int size) = 0;

	UMBRA_INLINE friend Stream& operator<<(Stream& stream, UINT8& value)
	{
		stream.serialize(&value, sizeof(value));
		return stream;
	}

	UMBRA_INLINE friend Stream& operator<<(Stream& stream, UINT16& value)
	{
		stream.serialize(&value, sizeof(value));
		return stream;
	}

	UMBRA_INLINE friend Stream& operator<<(Stream& stream, UINT32& value)
	{
		stream.serialize(&value, sizeof(value));
		return stream;
	}

	UMBRA_INLINE friend Stream& operator<<(Stream& stream, INT8& value)
	{
		stream.serialize(&value, sizeof(value));
		return stream;
	}

	UMBRA_INLINE friend Stream& operator<<(Stream& stream, INT16& value)
	{
		stream.serialize(&value, sizeof(value));
		return stream;
	}

	UMBRA_INLINE friend Stream& operator<<(Stream& stream, INT32& value)
	{
		stream.serialize(&value, sizeof(value));
		return stream;
	}

	UMBRA_INLINE friend Stream& operator<<(Stream& stream, float& value)
	{
		stream.serialize(&value, sizeof(value));
		return stream;
	}
};

//------------------------------------------------------------------------

class SizeHelper : public Stream
{
public:

    SizeHelper(void)
    :   m_size(0)
    {
    }

	void serialize(void*, int inBufferSize)
	{
        m_size += inBufferSize;
	}

	UMBRA_INLINE bool isLoading(void) const
	{
		return false;
	}

    UMBRA_INLINE int getSize(void) const
    {
        return m_size;
    }

private:

    int m_size;
};

//------------------------------------------------------------------------

template <class T>
int streamGetByteSize(T& t)
{
    SizeHelper size;
    size << t;
    return size.getSize();
}

//------------------------------------------------------------------------

class MemoryReader : public Stream
{
public:

    MemoryReader(void)
    :   m_ptr       (NULL)
    ,   m_buffer    (NULL)
    ,   m_bufferSize(0)
    ,   m_bReadOnly (false)
    {
    }

	MemoryReader(const UINT8* inBuffer, int inBufferSize, bool bReadOnly = false)
	{
		init(inBuffer, inBufferSize, bReadOnly);
	}

	~MemoryReader(void)
	{
		deinit();
	}

	void init(const UINT8* inBuffer, int inBufferSize, bool bReadOnly)
	{
        m_buffer     = inBuffer;
        m_ptr        = (UINT8*)inBuffer;
        m_bufferSize = inBufferSize;
        m_bReadOnly  = bReadOnly;
	}

	void deinit(void)
	{
        m_ptr        = NULL;
        m_buffer     = NULL;
        m_bufferSize = 0;
	}

	void serialize(void* outBuffer, int outBufferSize)
	{
        UMBRA_ASSERT(getByteCount() + outBufferSize <= m_bufferSize);

        memcpy(outBuffer, m_ptr, outBufferSize);
        m_ptr += outBufferSize;
	}

    bool isReadOnly(void) const
    {
        return m_bReadOnly;
    }

	UMBRA_INLINE bool isLoading(void) const
	{
		return true;
	}

    int getByteCount(void) const
    {
        return (int)(m_ptr-m_buffer);
    }

private:

    UINT8*        m_ptr;
    const UINT8*  m_buffer;
    int           m_bufferSize;
    bool          m_bReadOnly;
};

//------------------------------------------------------------------------

class MemoryWriter : public Stream
{
public:

    MemoryWriter(void)
    :   m_ptr       (NULL)
    ,   m_buffer    (NULL)
    ,   m_bufferSize(0)
    {
    }

	MemoryWriter(UINT8* inBuffer, int inBufferSize)
	{
		init(inBuffer, inBufferSize);
	}

	~MemoryWriter(void)
	{
		deinit();
	}

	void init(UINT8* inBuffer, int inBufferSize)
	{
        m_buffer     = inBuffer;
        m_ptr        = inBuffer;
        m_bufferSize = inBufferSize;
	}

	void deinit(void)
	{
        m_ptr = 0;
	}

	void serialize(void* inBuffer, int inBufferSize)
	{
        UMBRA_ASSERT(getByteCount() + inBufferSize <= m_bufferSize);

        memcpy(m_ptr, inBuffer, inBufferSize);
        m_ptr += inBufferSize;
	}

	UMBRA_INLINE bool isLoading(void) const
	{
		return false;
	}

    int getByteCount(void) const
    {
        return (int)(m_ptr-m_buffer);
    }

private:

    UINT8*  m_ptr;
    UINT8*  m_buffer;
    int     m_bufferSize;
};

//------------------------------------------------------------------------

template <class T>
class FixedArray
{
public:

    FixedArray(void)
    :   m_size      (0)
    ,   m_data      (NULL)
    {
    }

    explicit FixedArray(int size)
    :   m_size      (0)
    ,   m_data      (NULL)
    {
        reset(size);
    }

    ~FixedArray(void)
    {
        deinit();
    }

    FixedArray(const FixedArray<T>& other)
    :   m_size      (0)
    ,   m_data      (NULL)
    {
        *this = other;
    }

    FixedArray<T>& operator=(const FixedArray<T>& other)
    {
        if (other.m_size == 0)
        {
            m_size = 0;
            m_data = 0;
            return *this;
        }

        reset(other.getSize());
		for (int i = 0; i < other.getSize(); i++)
			m_data[i] = other.m_data[i];
        return *this;
    }

    void deinit(void)
    {
        if (m_data)
        {
            for (int i = 0; i < m_size; i++)
                m_data[i].~T();

            DPVSGetAllocator()->deallocate(m_data);

            m_data = NULL;
            m_size = 0;
        }
    }

    void reset(int size)
    {
        deinit();

        m_data = (T*)DPVSGetAllocator()->allocate(size*sizeof(T));
        m_size = size;

        for (int i = 0; i < size; i++)
            m_data[i] = T();
    }

    T& operator[](int index)
    {
        UMBRA_ASSERT(index < m_size);
        return m_data[index];
    }

    const T& operator[](int index) const
    {
        return m_data[index];
    }

    T* getPtr(void)
    {
        return m_data;
    }

    const T* getPtr(void) const
    {
        return m_data;
    }

    void setPtr(void* ptr)
    {
        m_data = (T*)ptr;
    }

    void set(int size, void* ptr)
    {
        m_size = size;
        m_data = (T*)ptr;
    }

    int getSize(void) const
    {
        return m_size;
    }

    int getByteSize(void) const
    {
        return m_size*sizeof(T);
    }

    friend Stream& operator<<(Stream& stream, FixedArray<T>& fixedArray)
    {
        int size = fixedArray.getSize();
        stream << size;

        if (stream.isLoading())
            fixedArray.reset(size);

        for (int i = 0; i < size; i++)
            stream << fixedArray.m_data[i];

        return stream;
    }

private:

    int m_size;
    T*  m_data;
};

template<typename OP, typename T> static UMBRA_INLINE void stream (OP& op, FixedArray<T>& t, int size = -1)
{
    op.prepare(t);
    if (size == -1)
    {
        size = t.getSize();
        stream(op, size);
    }
    if (OP::IsWrite)
        t.reset(size);
    streamArray(op, t.getPtr(), size);
}

//------------------------------------------------------------------------

class PVSVector
{
public:

    PVSVector(void)
    {
    }

    UMBRA_INLINE void reset(int size)
    {
        m_size = size;
		m_data.reset((size + 31)>>5);
        clearAll();
    }

	UMBRA_INLINE void set(int bit)
	{
		m_data[bit>>5] |= 1<<(bit&31);
	}

	UMBRA_INLINE bool get(int bit) const
	{
		return (m_data[bit>>5] & (1<<(bit&31))) != 0;
	}

    UMBRA_INLINE void eor(int bit)
    {
        m_data[bit>>5] ^= (UINT32)(1<<((UINT32)(bit)&31));
    }

    UMBRA_INLINE void eor(const UINT32* bitArray, int bitCount)
    {
        for (int i = 0; i < bitCount; i++)
        {
            UINT32 bit = bitArray[i];
            m_data[bit>>5] ^= (UINT32)(1<<((UINT32)(bit)&31));
        }
    }

    UMBRA_INLINE void eor16(const UINT32* base, int bitStart, int bitCount)
    {
        for (int i = 0; i < bitCount; i++)
        {
            UINT32 word = base[(bitStart+i)>>1];
            int mod = (bitStart+i)&1;
            UINT32 bit = ((word >> (16*mod)) & 0xFFFF);
            m_data[bit>>5] ^= (UINT32)(1<<((UINT32)(bit)&31));
        }
    }

	UMBRA_INLINE void clear(int bit)
	{
		m_data[bit>>5] &= ~(1<<(bit&31));
	}

    UMBRA_INLINE void clearAll(void)
    {
        memset(m_data.getPtr(), 0, m_data.getByteSize());
    }

    UMBRA_INLINE void setAll(void)
    {
        memset(m_data.getPtr(), 0xFFFFFFFF, m_data.getByteSize());
    }

    UMBRA_INLINE int countOnes(void) const
    {
        int n = 0;
        for (int i = 0; i < getSize(); i++)
            if (get(i)) n++;
        return n;
    }

    UMBRA_INLINE int getSize(void) const
    {
        return m_size;
    }

    UMBRA_INLINE int countZeros(void) const
    {
        int n = 0;
        for (int i = 0; i < getSize(); i++)
            if (!get(i)) n++;
        return n;
    }

    UMBRA_INLINE const UINT32* getPtr(void) const
    {
        return m_data.getPtr();
    }

    UMBRA_INLINE UINT32* getPtr(void)
    {
        return m_data.getPtr();
    }

    void set(int size, void* ptr)
    {
        m_size = size;
        m_data.set((size+31)/32, ptr);
    }

    int getByteSize(void) const
    {
        return m_data.getByteSize();
    }

    friend void fastCopy(PVSVector& dst, const void* src)
    {
        memcpy(dst.getPtr(), src, dst.getByteSize());
    }

    friend void copy(PVSVector& dst, const PVSVector& src)
    {
        dst.reset(src.getSize());
        memcpy(dst.getPtr(), src.getPtr(), src.m_data.getByteSize());
    }

    friend int computeHammingDistance(const PVSVector& a, const PVSVector& b)
    {
        if (a.getSize() != b.getSize())
            return 0;

        int result = 0;
        for (int i = 0; i < a.getSize(); i++)
        {
            if (a.get(i) != b.get(i))
                result++;
        }
        return result;
    }

    friend void computeUnion(PVSVector& result, const PVSVector& a, const PVSVector& b)
    {
        result.reset(a.getSize());
        for (int i = 0; i < result.m_data.getSize(); i++)
            result.m_data[i] = a.m_data[i] | b.m_data[i];
    }

    friend void computeDifference(PVSVector& result, const PVSVector& a, const PVSVector& b)
    {
        result.reset(a.getSize());
        for (int i = 0; i < result.m_data.getSize(); i++)
            result.m_data[i] = a.m_data[i] ^ b.m_data[i];
    }

	friend Stream& operator<<(Stream& stream, PVSVector& vector)
	{
        stream << vector.m_data;
        return stream;
	}

	template<typename OP> void streamOp (OP& op)
    {
        stream(op, m_size);
        stream(op, m_data);
	}

private:

    int                 m_size;
    FixedArray<UINT32>  m_data;
};

//------------------------------------------------------------------------

class UncompressedDPVS
{
public:

    enum
    {
        StackSize = 256
    };

public:

    struct Cell
    {
        float       time[2];
        Vector3     dir[2];
        PVSVector   pvs;

        void init(int maxSize, const DPVSInputPath& path, int index)
        {
            path.getNode(time[0], dir[0], index);
            path.getNode(time[1], dir[1], index+1);
            pvs.reset(maxSize);
            pvs.clearAll();
        }

        Vector3 getAverageDirection(void) const
        {
            return normalize(dir[0]+dir[1]);
        }

        friend Stream& operator<<(Stream& stream, Cell& interval)
        {
            stream << interval.time[0] << interval.time[1];
            stream << interval.dir[0].x << interval.dir[0].y << interval.dir[0].z;
            stream << interval.dir[1].x << interval.dir[1].y << interval.dir[1].z;
            stream << interval.pvs;
            return stream;
        }

        friend bool operator<(const Cell& a, const Cell& b)
        {
            return a.time[1] <= b.time[0];
        }

        friend bool operator>(const Cell& a, const Cell& b)
        {
            return a.time[0] >= b.time[1];
        }

		template<typename OP> void streamOp (OP& op)
		{
			streamArray(op, time, 2);
			streamArray(op, dir, 2);
			stream(op, pvs);
		}
    };

public:

    UncompressedDPVS(void)
    {
        m_timeBounds[0] = -FLT_MAX;
        m_timeBounds[1] = -FLT_MAX;
    }

    void init(int maxSize, const DPVSInputPath& path)
    {
        if (!path.getNodeCount())
            return;

        // Init intervals

        m_cellArray.reset(path.getNodeCount()-1);

        for (int i = 0; i < path.getNodeCount()-1; i++)
            m_cellArray[i].init(maxSize, path, i);

        // Update time bounds

        m_timeBounds[0] = FLT_MAX;
        m_timeBounds[1] = -FLT_MAX;

        for (int i = 0; i < path.getNodeCount(); i++)
        {
            float   nodeTime;
            Vector3 nodeDir;

            path.getNode(nodeTime, nodeDir, i);

            m_timeBounds[0] = min2(m_timeBounds[0], nodeTime);
            m_timeBounds[1] = max2(m_timeBounds[1], nodeTime);
        }
    }

    bool contains(float time)
    {
        return time >= m_timeBounds[0] && time <= m_timeBounds[1];
    }

    PVSVector* lookup(float time)
    {
        if (!contains(time))
            return NULL;
        return lookupInner(0, m_cellArray.getSize(), time);
    }

    PVSVector* lookupInner(int begin, int end, float time)
    {
        struct StackEntry
        {
            StackEntry(void)
            {
            }

            StackEntry(int b, int e)
            :   begin   (b)
            ,   end     (e)
            {
            }

            int begin;
            int end;
        };

        StackEntry stack[StackSize];
        int        stackIndex = 0;

        stack[stackIndex++] = StackEntry(begin, end);

        while (stackIndex)
        {
            StackEntry entry = stack[--stackIndex];

            if (entry.begin > entry.end)
                continue;

            if (entry.begin == entry.end)
                return &m_cellArray[entry.begin].pvs;

            int middle = (entry.begin+entry.end)/2;

            if (time < m_cellArray[middle].time[0])
                stack[stackIndex++] = StackEntry(entry.begin, middle-1);
            else if (time > m_cellArray[middle].time[1])
                stack[stackIndex++] = StackEntry(middle+1, entry.end);
            else
                return &m_cellArray[middle].pvs;
        }

        return NULL;
    }

    int getCellCount(void) const
    {
        return m_cellArray.getSize();
    }

    const Cell& getCell(int index) const
    {
        return m_cellArray[index];
    }

    Cell& getCell(int index)
    {
        return m_cellArray[index];
    }

    Cell* getCellPtr(void)
    {
        return m_cellArray.getPtr();
    }

    friend Stream& operator<<(Stream& stream, UncompressedDPVS& pvs)
    {
        stream << pvs.m_timeBounds[0] << pvs.m_timeBounds[1];
        stream << pvs.m_cellArray;
        return stream;
    }

    template<typename OP> void streamOp (OP& op)
    {
        streamArray(op, m_timeBounds, 2);
        stream(op, m_cellArray);
	}

    float            m_timeBounds[2];
    FixedArray<Cell> m_cellArray;
};

//------------------------------------------------------------------------

class CompressedDPVS
{
public:

    struct Node
    {
        Node(void)
        :   diffIndex   (0)
        ,   diffSize    (0)
        {
            child[0] = child[1] = 0;
            timeBounds[0] = timeBounds[1] = FLT_MAX;
        }

        bool isLeaf(void) const
        {
            return child[0] == 0 && child[1] == 0;
        }

        bool contains(float t) const
        {
            return t >= timeBounds[0] && t <= timeBounds[1];
        }

        friend Stream& operator<<(Stream& stream, Node& node)
        {
            stream << node.diffIndex << node.diffSize;
            stream << node.child[0] << node.child[1];
            stream << node.timeBounds[0] << node.timeBounds[1];

            return stream;
        }


        UINT32 diffIndex;
        UINT32 diffSize;
        UINT16 child[2];
        float  timeBounds[2];
    };

public:

    friend Stream& operator<<(Stream& stream, CompressedDPVS& pvs)
    {
        stream << pvs.rootPVS << pvs.diffArray << pvs.nodeArray;
        return stream;
    }

public:

    PVSVector          rootPVS;
    FixedArray<UINT16> diffArray;
    FixedArray<Node>   nodeArray;
};

//------------------------------------------------------------------------

UMBRA_CT_ASSERT(sizeof(CompressedDPVS::Node) == 20);

//------------------------------------------------------------------------

struct CompressedDPVSChunk
{
    FixedArray<int>            pvsIndexRemapTable;
    FixedArray<CompressedDPVS> pvsArray;
};

//------------------------------------------------------------------------

struct DPVSRuntimeArray
{
    int size;
    int offset;
};

UMBRA_CT_ASSERT(sizeof(DPVSRuntimeArray) == 8);

//------------------------------------------------------------------------

template <class T>
class DPVSRuntimeArrayView
{
public:

    DPVSRuntimeArrayView(const DPVSRuntimeArray& inArray, const UINT8* inBase)
    {
        init(inArray, inBase);
    }

    void init(const DPVSRuntimeArray& inArray, const UINT8* inBase)
    {
        m_array = inArray;
        m_base  = inBase;
    }

    int getSize(void) const
    {
        return m_array.size;
    }

	UMBRA_INLINE const T& operator[](int index) const
	{
        return *(T*)(m_base + m_array.offset + sizeof(T)*index);
	}

    const T* getPtr(void) const
    {
        return (T*)(m_base + m_array.offset);
    }

private:

    DPVSRuntimeArray m_array;
    const UINT8*     m_base;
};

//------------------------------------------------------------------------

class DPVSRuntime
{
public:

    DPVSRuntime(const UINT8* inBase)
    {
        init(inBase);
    }

    void         init           (const UINT8* inBase);
    PVSVector*   lookup         (float time);
    int          getObjectCount (void) const;
    int          remap          (int index) const;

    int          cacheIndex;
    PVSVector    cachePVS;
    const UINT8* base;

private:

    UINT8        mem[UMBRA_DPVS_MAX_OBJECT_COUNT/8];
};

//------------------------------------------------------------------------

class DPVSRuntimeDeltaEncodeTreeNode
{
public:

    UMBRA_INLINE bool isLeaf(void) const
    {
        return getChild(0) == 0 && getChild(1) == 0;
    }

    UMBRA_INLINE bool contains(float t) const
    {
        return t >= getMinTime() && t <= getMaxTime();
    }

    UMBRA_INLINE UINT32 getDiffIndex(void) const
    {
        return m_diffIndex;
    }

    UMBRA_INLINE UINT32 getDiffSize(void) const
    {
        return m_diffSize;
    }

    UMBRA_INLINE UINT16 getChild(int index) const
    {
        return (UINT16)((m_packedChild>>(16*(index&1)))&0xFFFF);
    }

    UMBRA_INLINE float getMinTime(void) const
    {
        return m_timeBounds[0];
    }

    UMBRA_INLINE float getMaxTime(void) const
    {
        return m_timeBounds[1];
    }

private:

    UINT32 m_diffIndex;
    UINT32 m_diffSize;
    UINT32 m_packedChild;
    float  m_timeBounds[2];
};

//------------------------------------------------------------------------

UMBRA_CT_ASSERT(sizeof(DPVSRuntimeDeltaEncodeTreeNode) == 20);

//------------------------------------------------------------------------

class DPVSRuntimeDeltaEncodeTree
{
public:

    bool contains(const UINT8* base, float time) const
    {
        DPVSRuntimeArrayView<DPVSRuntimeDeltaEncodeTreeNode> nodeView(nodeArray, base);

        if (!nodeView.getSize())
            return false;

        return nodeView[0].contains(time);
    }

    PVSVector* lookup(DPVSRuntime* area, float time) const
    {
        if (!contains(area->base, time))
            return NULL;

        // Cache lookup

        DPVSRuntimeArrayView<DPVSRuntimeDeltaEncodeTreeNode> nodeView(nodeArray, area->base);

        if (area->cacheIndex >= 0 && area->cacheIndex < nodeView.getSize() && nodeView[area->cacheIndex].contains(time))
            return &area->cachePVS;

        // Decode PVS vector

        DPVSRuntimeArrayView<UINT32> rootPVSView(rootPVS, area->base);
        DPVSRuntimeArrayView<UINT32> diffView(diffArray, area->base);

        fastCopy(area->cachePVS, rootPVSView.getPtr());

        int index = 0;

        while (!nodeView[index].isLeaf())
        {
            const DPVSRuntimeDeltaEncodeTreeNode& node = nodeView[index];

            area->cachePVS.eor16(&diffView[0], node.getDiffIndex(), node.getDiffSize());

            if (nodeView[node.getChild(0)].contains(time)) // Every non-leaf node always has left child
                index = node.getChild(0);
            else
                index = node.getChild(1);
        }

        area->cachePVS.eor16(&diffView[0], nodeView[index].getDiffIndex(), nodeView[index].getDiffSize());
        area->cacheIndex = index;

        return &area->cachePVS;
    }

    DPVSRuntimeArray rootPVS;
    DPVSRuntimeArray diffArray;
    DPVSRuntimeArray nodeArray;
};

UMBRA_CT_ASSERT(sizeof(DPVSRuntimeDeltaEncodeTree) == 24);

//------------------------------------------------------------------------

class DPVSRuntimeData
{
public:

    int getObjectCount(const UINT8* base) const
    {
        DPVSRuntimeArrayView<int> remapTableView(remapTable, base);
        return remapTableView.getSize();
    }

    PVSVector* lookup(DPVSRuntime* area, float time) const
    {
        DPVSRuntimeArrayView<DPVSRuntimeDeltaEncodeTree> deltaEncodeTreeArrayView(deltaEncodeTreeArray, area->base);

        for (int i = 0; i < deltaEncodeTreeArrayView.getSize(); i++)
        {
            if (deltaEncodeTreeArrayView[i].contains(area->base, time))
                return deltaEncodeTreeArrayView[i].lookup(area, time);
        }

        return NULL;
    }

    UMBRA_FORCE_INLINE int remap(const UINT8* base, int index) const
    {
        DPVSRuntimeArrayView<int> remapTableView(remapTable, base);
        return remapTableView[index];
    }

    DPVSRuntimeArray remapTable;
    DPVSRuntimeArray deltaEncodeTreeArray;
};

UMBRA_CT_ASSERT(sizeof(DPVSRuntimeData) == 16);

//------------------------------------------------------------------------

} // namespace Umbra

#endif // UMBRADPVSSTREAM_HPP
