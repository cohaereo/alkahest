// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraBinStream.hpp"
#include "umbraMemory.hpp"

#include <stdio.h>
#include <string.h>

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 MemInputStream::read (void* ptr, Umbra::UINT32 n)
{
    n = min2(n, m_left);
    if (ptr)
        memcpy(ptr, m_data, n);
    m_data += n;
    m_left -= n;
    return n;
}

/*-------------------------------------------------------------------*//*!*
 * \brief
 *//*-------------------------------------------------------------------*/

MemOutputStream::MemOutputStream(Allocator* a)
    : Base(a), m_buf(NULL), m_size(0), m_pos(0)
{
}

/*-------------------------------------------------------------------*//*!*
 * \brief
 *//*-------------------------------------------------------------------*/

MemOutputStream::~MemOutputStream()
{
    UMBRA_DELETE(m_buf);
}

/*-------------------------------------------------------------------*//*!*
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 MemOutputStream::write(const void* ptr, Umbra::UINT32 n)
{
    UINT32 last = m_pos + n;
    UINT32 newSize = m_size;

    while (last > newSize)
        newSize = max2(64u, newSize * 2);

    if (newSize != m_size)
    {
        UINT8* buf = (UINT8*)UMBRA_MALLOC(newSize);
        if (!buf)
            return 0;
        if (m_buf)
        {
            if (m_pos)
                memcpy(buf, m_buf, m_pos);
            UMBRA_FREE(m_buf);
        }
        m_buf = buf;
        m_size = newSize;
    }

    memcpy(&m_buf[m_pos], ptr, n);
    m_pos = last;
    return n;
}

/*-------------------------------------------------------------------*//*!*
 * \brief
 *//*-------------------------------------------------------------------*/

int StreamReader::readLineAppend (Array<char>& out)
{
    int start = out.getSize();
    char c = '\0';
    do
    {
        get(c);
        if (!m_ok)
            break;
        out.pushBack(c);
    } while (c != '\n');

    int read = out.getSize() - start;
    if (!read)
        return -1;
    out.pushBack('\0');
    return read;
}

/*-------------------------------------------------------------------*//*!*
 * \brief
 *//*-------------------------------------------------------------------*/

void StreamWriter::putStr (const char* str)
{
    put(str, (UINT32)strlen(str));
}

/*-------------------------------------------------------------------*//*!*
 * \brief
 *//*-------------------------------------------------------------------*/



}
