// Copyright (c) 2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraFileStream.hpp"
#include <stdio.h>
#include <string.h>

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

FileIOStream::FileIOStream(FILE* fp) :
    m_ownsFp(false)
{
    m_fp = File((FILE*)fp);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void FileIOStream::open (const char* fname, int mode)
{
    UMBRA_ASSERT(fname);
    if (isOpen()) // signal error
        return;

    char modestring[4];
    int i = 0;

    if (mode & READ)
        modestring[i++] = 'r';
    else
    if (mode & WRITE)
        modestring[i++] = 'w';

    if (!(mode & TEXT))
        modestring[i++] = 'b';
    modestring[i] = '\0';

    m_fp.open(fname, modestring);
    m_ownsFp = true;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void FileIOStream::close()
{
    if (m_fp.isOpen() && m_ownsFp)
        m_fp.close();
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

void FileInputStream::reset (void)
{
    if (m_ownsFp)
        m_fp.seek(0, SEEK_SET);
}

/*----------------------------------------------------------------------*//*!
 * \brief
 *//*----------------------------------------------------------------------*/

Umbra::UINT32 FileInputStream::read (void* ptr, Umbra::UINT32 size)
{
    if (!size || !isOpen())
        return 0;
    if (!ptr)
    {
        if (m_fp.seek(size, SEEK_CUR) == 0)
            return size;
        else
            return 0;
    }
    else
    {
        return (UINT32)m_fp.read(ptr, 1, size);
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 FileOutputStream::write (const void* ptr, Umbra::UINT32 size)
{
    if (!ptr)
        return 0;
    return (UINT32)m_fp.write(ptr, 1, size);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void FileOutputStream::flush (void)
{
    m_fp.flush();
}

static FileOutputStream g_stdout(stdout);

FileOutputStream* getStdoutStream (void)
{
    return &g_stdout;
}

} // namespace Umbra

