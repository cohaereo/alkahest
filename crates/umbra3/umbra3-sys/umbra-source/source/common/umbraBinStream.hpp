// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRABINSTREAM_HPP
#define UMBRABINSTREAM_HPP

#include "umbraPrivateDefs.hpp"
#include "umbraPlatform.hpp"
#include "umbraMemory.hpp"
#include "umbraArray.hpp"


namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   Generic stream handling utilities
 *//*-------------------------------------------------------------------*/

class StreamUtil
{
public:

    static bool needSwap (void)
    {
        return (UMBRA_BYTE_ORDER != UMBRA_LITTLE_ENDIAN);
    }

    template <typename T> static void swap (UINT8* buf)
    {
        if (!needSwap())
            return;

        if (sizeof(T) == 8)
            *((UINT64*)buf) = swapBytes_8(buf);
        else if (sizeof(T) == 4)
            *((UINT32*)buf) = swapBytes_4(buf);
        else
            UMBRA_ASSERT(sizeof(T) == 1);
    }
};

/*-------------------------------------------------------------------*//*!
 * \brief   Helper for input stream data reading
 *//*-------------------------------------------------------------------*/

class StreamReader
{
public:
    StreamReader (InputStream* in): m_stream(in), m_ok(in != NULL) {}

    bool isOk (void) const { return m_ok; }

    template<typename T> void get (T& t)
    {
        if (!m_ok)
            return;
        UINT8 buf[sizeof(T)];
        m_ok = (m_stream->read(buf, sizeof(T)) == sizeof(T));
        StreamUtil::swap<T>(buf);
        t = *((T*)buf);
    }

    void get (bool& t)
    {
        UINT8 v = 0;
        get(v);
        t = (v == 1);
    }

    void rawRead (void* ptr, UINT32 bytes)
    {
        if (!m_ok)
            return;
        m_ok = (m_stream->read(ptr, bytes) == bytes);
    }

    int readLineAppend (Array<char>& out);

    bool readLine (Array<char>& out)
    {
        out.clear();
        return readLineAppend(out) >= 0;
    }

    void skip (UINT32 bytes)
    {
        if (!m_ok || !bytes)
            return;
        m_ok = (m_stream->read(NULL, bytes) == bytes);
    }

private:
    InputStream* m_stream;
    bool m_ok;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Helper for output stream data writing
 *//*-------------------------------------------------------------------*/

class StreamWriter
{
public:
    StreamWriter (OutputStream* out): m_stream(out), m_ok(out != NULL) {}

    bool isOk (void) const { return m_ok; }

    template<typename T> void put (const T& t)
    {
        if (!m_ok)
            return;
        UINT8 buf[sizeof(T)];
        *((T*)buf) = t;
        StreamUtil::swap<T>(buf);
        m_ok = (m_stream->write(buf, sizeof(T)) == sizeof(T));
    }

    void put (const bool& t)
    {
        UINT8 v = t ? 1 : 0;
        put(v);
    }

    void put (const char* str, UINT32 len)
    {
        if (!m_ok)
            return;
        m_ok = (m_stream->write(str, len) == len);
    }

    void putStr (const char* s);

private:
    OutputStream*   m_stream;
    bool            m_ok;
};


/*-------------------------------------------------------------------*//*!
 * \brief   Input stream wrapper for memory block
 *//*-------------------------------------------------------------------*/

class MemInputStream : public InputStream
{
public:
    MemInputStream  (const void* ptr, UINT32 size): m_data((const UINT8*)ptr), m_left(size) {}
    ~MemInputStream (void) {}

    UINT32          read    (void* ptr, UINT32 size);

private:
    const UINT8*    m_data;
    UINT32          m_left;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Dynamically growing binary output memory stream.
 *//*-------------------------------------------------------------------*/

class MemOutputStream : public OutputStream, public Base
{
public:
    MemOutputStream   (Allocator* a = NULL);
    ~MemOutputStream  (void);

    void*   getPtr          (void) { return m_buf; }
    UINT32  getSize         (void) { return m_pos; }
    UINT32  write           (const void* ptr, UINT32 size);

private:
    UINT8*  m_buf;
    UINT32  m_size;
    UINT32  m_pos;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Size calculating filter
 *//*-------------------------------------------------------------------*/

class OutputSizeFilter : public OutputStream
{
public:
    OutputSizeFilter (OutputStream* next): m_next(next), m_size(0) {}

    UINT32  getSize  (void) { return m_size; }

    UINT32  write    (const void* ptr, UINT32 size)
    {
        if (m_next)
            size = m_next->write(ptr, size);
        m_size += size;
        return size;
    }

private:
    OutputStream*   m_next;
    UINT32          m_size;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Endianess swap filter.
 *//*-------------------------------------------------------------------*/

class SwapEndianOutFilter : public OutputStream
{
public:
    SwapEndianOutFilter (OutputStream* next): m_next(next) {}

    UINT32  write    (const void* ptr, UINT32 size)
    {
        if (size & 3)
            return 0;

        const UINT32* ptr32 = (const UINT32*)ptr;
        for (UINT32 i = 0; i < size; i += sizeof(UINT32))
        {
            UINT32 data = *(ptr32++);
            data = swapBytes_4(&data);
            if (m_next->write(&data, sizeof(UINT32)) != sizeof(UINT32))
                return i;
        }

        return size;
    }

private:
    OutputStream*   m_next;
};

class SwapEndianInFilter : public InputStream
{
public:
    SwapEndianInFilter (InputStream* next): m_next(next) {}

    UINT32  read    (void* ptr, UINT32 size)
    {
        if (size & 3)
            return 0;

        UINT32* ptr32 = (UINT32*)ptr;
        for (UINT32 i = 0; i < size; i += sizeof(UINT32))
        {
            UINT32 data = 0;
            if (m_next->read(&data, sizeof(UINT32)) != sizeof(UINT32))
                return i;
            data = swapBytes_4(&data);
            *(ptr32++) = data;
        }

        return size;
    }

private:
    InputStream*   m_next;
};

} // namespace Umbra

#endif // UMBRABINSTREAM_HPP

//--------------------------------------------------------------------
