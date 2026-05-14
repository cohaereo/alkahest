// Copyright (c) 2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRAFILESTREAM_HPP
#define UMBRAFILESTREAM_HPP

#include "umbraPrivateDefs.hpp"
#include "umbraFile.hpp"
#include "umbraBinStream.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief   File IO stream base class.
 *//*-------------------------------------------------------------------*/

class FileIOStream
{
public:
    bool    isOpen  (void) { return m_fp.isOpen(); }
    void    close   (void);

protected:
    enum Mode
    {
        READ  = (1<<0), //!< read-only
        WRITE = (1<<1), //!< write-only
        TEXT  = (1<<2)  //!< text
    };

    FileIOStream    (void): m_fp(), m_ownsFp(false) {}
    FileIOStream    (const char* fname, int mode): m_fp(), m_ownsFp(false) { open(fname, mode); }
    FileIOStream    (FILE* fp);
    ~FileIOStream   (void) { close(); }

    void    open    (const char* fname, int m);

private:
    FileIOStream(const FileIOStream&); //!< disallowed

protected:
    File    m_fp;
    bool    m_ownsFp;
};

/*-------------------------------------------------------------------*//*!
 * \brief   Input file stream class.
 *//*-------------------------------------------------------------------*/

class FileInputStream : public FileIOStream, public InputStream
{
public:
    FileInputStream (void) {}
    FileInputStream (const char* fname): FileIOStream(fname, READ) {}
    FileInputStream (FILE* fp): FileIOStream(fp) {}

    void    open    (const char* fname, bool text = false) { FileIOStream::open(fname, READ | (text ? TEXT : 0)); }
    UINT32  read    (void* ptr, UINT32 numBytes);
    void    reset   (void);
};

/*-------------------------------------------------------------------*//*!
 * \brief   Binary output file stream.
 *//*-------------------------------------------------------------------*/

class FileOutputStream : public FileIOStream, public OutputStream
{
public:
    FileOutputStream    (void) {}
    FileOutputStream    (const char* fname): FileIOStream(fname, WRITE) {}
    FileOutputStream    (FILE* fp): FileIOStream(fp) {}

    void    open        (const char* fname, bool text = false) { FileIOStream::open(fname, WRITE | (text ? TEXT : 0)); }
    UINT32  write       (const void* ptr, UINT32 numBytes);
    void    flush       (void);
};


FileOutputStream* getStdoutStream (void);

} // namespace Umbra

#endif // UMBRAFILESTREAM_HPP

//--------------------------------------------------------------------
