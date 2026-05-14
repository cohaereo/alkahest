#pragma once

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2007-2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Utility to copy chunks of data between processes.
 *
 */

#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraString.hpp"
#include "umbraProcess.hpp"
#include "umbraThread.hpp"
#include "umbraBinStream.hpp"

namespace Umbra
{

class Semaphore;
class BinIMStream;

/*-------------------------------------------------------------------*//*!
 * \brief       Base class for reader and writer.
 *
 *//*-------------------------------------------------------------------*/

class ProcessDataCopy
{
public:

    static const UINT32 BufferSize = 1024;

    enum RequestType
    {
        RT_NONE,
        RT_RUNTIME,
        RT_VISUALIZATIONS
    };

    virtual ~ProcessDataCopy();

protected:

    ProcessDataCopy(const String& sharedId, bool firstProcess);

    /*-------------------------------------------------------------------*//*!
     * \brief       Data that is common to the two processes.
     *//*-------------------------------------------------------------------*/
#if UMBRA_COMPILER == UMBRA_MSC
#pragma warning(disable:4324)
    __declspec(align(8))
#endif
    struct SharedData
    {
        SharedData()
        : m_request(RT_NONE),
          m_available(false),
          m_size(0),
          m_bytesLeft(0)
        {
            m_semaphoreName[0] = 0;
            m_semaphoreName2[0] = 0;
            m_semaphoreName3[0] = 0;
        }

        volatile RequestType m_request;
        volatile bool        m_available;
        volatile UINT32      m_size;
        volatile UINT32      m_bytesLeft;
        volatile char        m_semaphoreName[64];
        volatile char        m_semaphoreName2[64];
        volatile char        m_semaphoreName3[64];
        volatile char        m_data[BufferSize];
    };

#if UMBRA_COMPILER == UMBRA_MSC
#pragma warning(default:4324)
#endif

    ProcessSharedMemory<SharedData> m_shared;

    Semaphore*               m_reqSemaphore;
    Semaphore*               m_readSemaphore;
    Semaphore*               m_writeSemaphore;
};

/*-------------------------------------------------------------------*//*!
 * \brief       Writer, which copies data from process to process.
 *
 * \note        Two processes: one process reads and other process writes.
 *              Reader makes requests, which writer delivers.
 *
 *//*-------------------------------------------------------------------*/

class ProcessDataWriter : public ProcessDataCopy
{
public:

    ProcessDataWriter           (const String& sharedId, bool firstProcess, ProcessBase* otherProcess);
    ~ProcessDataWriter          (void);

    RequestType     checkRequest    (void);                     // Check whether request is available
    RequestType     waitRequest     (int ms);                   // Wait for request

    // Write an entire buffer
    void            write           (const UINT8* buffer, UINT32 size);

    // Write by a page at a time, use either these or the function above
    void            beginWrite      (UINT32 size);
    void            writePage       (const UINT8* buffer, UINT32& bytesWritten);
    bool            waitWrite       (int waitMS);

private:

    void            flush           ();                         // Flush requests. Used only on exit.

    UINT32          m_localBytesLeft;
    ProcessBase*    m_otherProcess;

};

/*-------------------------------------------------------------------*//*!
 * \brief       Reader, which copies data from process to process.
 *
 * \note        Two processes: one process reads and other process writes.
 *              Reader makes requests, which writer delivers.
 *
 *//*-------------------------------------------------------------------*/

class ProcessDataReader : public ProcessDataCopy
{
public:

    ProcessDataReader           (const String& sharedId, bool firstProcess, ProcessBase* otherProcess);
    ~ProcessDataReader          (void);

    void            request         (RequestType requestType);  // Make a request

    bool            available       (void);                     // Check whether requested data is available
    bool            wait            (int waitMS);               // Wait for data
    bool            wait            (void);
    UINT32          getSize         (void);                     // Get size of available data

    bool            active          (void);                     // If request is active (i.e. request was called)
    RequestType     getActive       (void) { return m_request; }

    // Read buffer
    void            read            (UINT8* buffer, UINT32& bytesLeft, UINT32& bytesWritten);

private:

    void            flush           ();                         // Flush requests. Used only on exit.

    bool            m_mustWait;
    RequestType     m_request;
    bool            m_readRequestActive;
    ProcessBase*    m_otherProcess;
};

class ProcessInputStream : public InputStream
{
public:

    ProcessInputStream              (ProcessDataCopy::RequestType type, ProcessDataReader* reader, Allocator* a = NULL);
    ProcessInputStream              (ProcessDataReader* reader, Allocator* a = NULL);
    ~ProcessInputStream             (void);

    void                communicate (void);
    UINT32              getSize     (void) { return m_reader->getSize(); }

    UINT32              read        (void* ptr, UINT32 numBytes);

private:

    Allocator*          m_allocator;
    ProcessDataReader*  m_reader;
    UINT8*              m_buf;
    MemInputStream*     m_stream;
};

class ProcessOutputStream : public OutputStream
{
public:
    ProcessOutputStream             (ProcessDataWriter* writer, Allocator* a = NULL);
    ~ProcessOutputStream            (void);

    bool                communicate (void);
    UINT32              write       (const void* ptr, UINT32 numBytes);

private:

    Allocator*          m_allocator;
    ProcessDataWriter*  m_writer;
    MemOutputStream     m_stream;
    UINT32              m_bytesLeft;
    UINT32              m_offset;
    bool                m_communicate;
};

} // namespace Umbra

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)

//--------------------------------------------------------------------
