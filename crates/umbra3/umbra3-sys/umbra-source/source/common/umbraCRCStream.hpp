#pragma once

#include "umbraPrivateDefs.hpp"
#include "umbraBinStream.hpp"
#include "umbraHash.hpp"
#include "umbraChecksum.hpp"
#include <string.h>
#include <stdio.h>

namespace Umbra
{

class CRCOutputStream : public OutputStream
{
public:

    enum { BUFFER_SIZE = 1024*4 };
    static const UINT32 MAGIC;

    CRCOutputStream(OutputStream& os) : m_os(os), m_size(0)
    {
    }

    ~CRCOutputStream()
    {
        // What to do if flush fails?!
        bool ret = flush();
        UMBRA_UNREF(ret);
    }

    UINT32 write(const void* ptr, UINT32 n)
    {
        // \todo [Hannu] optimize
        for (int i = 0; i < (int)n; i++)
        {
            if (m_size >= (int)sizeof(m_buffer) && !flush())
                return 0;

            UMBRA_ASSERT(m_size < (int)sizeof(m_buffer));
            m_buffer[m_size++] = ((const UINT8*)ptr)[i];
        }

        return n;
    }

    bool flush()
    {
        if (!m_size)
            return true;

        StreamWriter sw(&m_os);

        UINT32 hash = crc32Hash(m_buffer, m_size);
        UINT32 size = (UINT32)m_size;

        sw.put(MAGIC);
        sw.put(hash);
        sw.put(size);
        if (m_os.write(m_buffer, (UINT32)m_size) != (UINT32)m_size)
            return false;

        m_size = 0;

        return true;
    }

private:
    OutputStream&   m_os;
    UINT8           m_buffer[BUFFER_SIZE];
    int             m_size;
    
    CRCOutputStream& operator= (const CRCOutputStream&) { return *this; }
};

class CRCInputStream : public InputStream
{
public:
    CRCInputStream(Allocator* a, InputStream& is) : m_is(is), m_pos(0), m_buffer(a), m_failed(false)
    {
    }

    ~CRCInputStream()
    {
    }

    bool isOk() const { return !m_failed; }

    UINT32 read(void* ptr, UINT32 n)
    {
        if (!isOk())
            return 0;

        while (m_pos + (int)n > m_buffer.getSize())
            if (readChunk() == false)
            {
                m_failed = true;
                return 0;
            }

        UMBRA_ASSERT(m_pos+(int)n <= m_buffer.getSize());
        memcpy(ptr, m_buffer.getPtr() + m_pos, n);
        m_pos += (int)n;

        return n;
    }

    bool readChunk()
    {
        // Move data to beginning of the buffer.

        memmove(m_buffer.getPtr(), m_buffer.getPtr() + m_pos, m_buffer.getSize() - m_pos);
        m_buffer.resize(m_buffer.getSize() - m_pos);
        m_pos = 0;

        // Read header.

        StreamReader sr(&m_is);

        UINT32 magic = 0, hash = 0, size = (UINT32)-1;
        sr.get(magic);
        sr.get(hash);
        sr.get(size);

        if (!sr.isOk())
            return false;

        if (magic != CRCOutputStream::MAGIC)
            return false;

        if (size > (UINT32)CRCOutputStream::BUFFER_SIZE)
            return false;

        // Read and check data.

        int oldSize = m_buffer.getSize();
        m_buffer.resize(oldSize + (int)size);

        UINT32 ret = m_is.read(m_buffer.getPtr() + oldSize, size);
        if (ret != size)
            return false;

        UINT32 h = crc32Hash(m_buffer.getPtr() + oldSize, size);
        if (hash != h)
            return false;

        return true;
    }

private:
    InputStream& m_is;
    int          m_pos;
    Array<UINT8> m_buffer;
    bool         m_failed;

    CRCInputStream& operator= (const CRCInputStream&) { return *this; }
};

}
