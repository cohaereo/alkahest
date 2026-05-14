/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \brief   Utility to copy chunks of data between processes.
 *
 */

#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraPrivateDefs.hpp"
#include "umbraProcessDataCopy.hpp"
#include "umbraString.hpp"
#include "umbraProcess.hpp"
#include "umbraThread.hpp"
#include "umbraBinStream.hpp"

using namespace Umbra;

#if UMBRA_OS != UMBRA_WINDOWS
const UINT32 ProcessDataCopy::BufferSize;
#endif

// stupid GCC hack for volatile

static size_t min2 (volatile size_t a, size_t b)    { return a<=b ? a : b; }

/*-------------------------------------------------------------------*//*!
 * \brief   ProcessDataCopy base class constructor.
 *
 * \param   sharedId        String identifier of the shared memory.
 * \param   firstProcess    Whether this it the first process created
 *                          (out of the reader/writer pair).
 *
 *//*-------------------------------------------------------------------*/

ProcessDataCopy::ProcessDataCopy(const String& sharedId, bool firstProcess)
{
    m_shared    = processAlloc<SharedData>(sharedId);

    if (firstProcess)
    {
        String sem1 = generateProcessUID();
        String sem2 = generateProcessUID();
        String sem3 = generateProcessUID();

        if( sem1.length() >= 64)
            sem1 = String(sem1, 0, 63);

        if( sem2.length() >= 64)
            sem2 = String(sem2, 0, 63);

        if( sem3.length() >= 64)
            sem3 = String(sem3, 0, 63);

        memcpy((void*)m_shared.p->m_semaphoreName, sem1.toCharPtr(), sem1.length()+1);
        memcpy((void*)m_shared.p->m_semaphoreName2, sem2.toCharPtr(), sem2.length()+1);
        memcpy((void*)m_shared.p->m_semaphoreName3, sem3.toCharPtr(), sem3.length()+1);
    }

    m_readSemaphore  = UMBRA_NEW(Semaphore, (const char*)m_shared.p->m_semaphoreName, 0, 100);
    m_writeSemaphore = UMBRA_NEW(Semaphore, (const char*)m_shared.p->m_semaphoreName2, 0, 100);
    m_reqSemaphore   = UMBRA_NEW(Semaphore, (const char*)m_shared.p->m_semaphoreName3, 0, 100);
}

/*-------------------------------------------------------------------*//*!
 * \brief   ProcessDataCopy destructor.
 *//*-------------------------------------------------------------------*/

ProcessDataCopy::~ProcessDataCopy(void)
{
    processFree<SharedData>(m_shared);

    UMBRA_DELETE(m_readSemaphore);
    UMBRA_DELETE(m_writeSemaphore);
    UMBRA_DELETE(m_reqSemaphore);
}

/*-------------------------------------------------------------------*//*!
 * \brief   ProcessDataWriter constructor.
 *
 * \param   sharedId        String identifier of the shared memory.
 * \param   firstProcess    Whether this it the first process created
 *                          (out of the reader/writer pair).
 *
 *//*-------------------------------------------------------------------*/

ProcessDataWriter::ProcessDataWriter(const String& sharedId, bool firstProcess, ProcessBase* otherProcess)
: ProcessDataCopy(sharedId, firstProcess),
  m_localBytesLeft(0),
  m_otherProcess(otherProcess)
{
}

/*-------------------------------------------------------------------*//*!
 * \brief   ProcessDataWriter destructor.
 *//*-------------------------------------------------------------------*/

ProcessDataWriter::~ProcessDataWriter(void)
{
    flush();
}

/*-------------------------------------------------------------------*//*!
 * \brief   ProcessDataReader constructor.
 *
 * \param   sharedId        String identifier of the shared memory.
 * \param   firstProcess    Whether this it the first process created
 *                          (out of the reader/writer pair).
 *
 *//*-------------------------------------------------------------------*/

ProcessDataReader::ProcessDataReader(const String& sharedId, bool firstProcess, ProcessBase* otherProcess)
: ProcessDataCopy(sharedId, firstProcess),
  m_mustWait(false),
  m_request(RT_NONE),
  m_readRequestActive(false),
  m_otherProcess(otherProcess)
{
}

/*-------------------------------------------------------------------*//*!
 * \brief   ProcessDataReader destructor.
 *//*-------------------------------------------------------------------*/

ProcessDataReader::~ProcessDataReader(void)
{
    flush();
}

/*-------------------------------------------------------------------*//*!
 * \brief   Check's whether a request is available.
 *
 * \note    Called by the writer to see if a request is available.
 *
 * \return  Request type, or RT_NONE if none available.
 *
 *//*-------------------------------------------------------------------*/

ProcessDataCopy::RequestType ProcessDataWriter::checkRequest()
{
    m_reqSemaphore->checkDown();
    return m_shared.p->m_request;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Blocks waiting for available request.
 *
 * \note    Called by the writer to see if a request is available.
 *
 * \param   ms Max. amount of milliseconds to wait.
 *
 * \return  Request type, or RT_NONE if none available.
 *
 *//*-------------------------------------------------------------------*/

ProcessDataCopy::RequestType ProcessDataWriter::waitRequest(int ms)
{
    m_reqSemaphore->tryDown(ms);
    return m_shared.p->m_request;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Makes a request.
 *
 * \param   requestType Type of the request being made.
 *
 * \note    Only one request can be active at a time.
 *
 * \note    Application should call available() to see when data becomes
 *          available to read.
 *
 *//*-------------------------------------------------------------------*/

void ProcessDataReader::request(ProcessDataCopy::RequestType requestType)
{
    if (m_shared.p->m_request != RT_NONE)
        return;

    m_readRequestActive = true;
    m_mustWait = true;
    m_request = requestType;

    m_shared.p->m_request = requestType;
    m_shared.p->m_available = false;
    m_shared.p->m_size = 0;
    m_shared.p->m_bytesLeft = 0;

    m_reqSemaphore->up();
}

/*-------------------------------------------------------------------*//*!
 * \brief   Checks whether data is available from writer.
 *
 * \note    Can be called after request is made to see when data becomes
 *          available.
 *
 *//*-------------------------------------------------------------------*/

bool ProcessDataReader::available()
{
    return m_shared.p->m_available;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Checks whether data is available from writer.
 *
 * \note    Can be called after request is made to see when data becomes
 *          available.
 *
 *//*-------------------------------------------------------------------*/

bool ProcessDataReader::active()
{
    return m_readRequestActive;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Get total size of available data.
 *
 * \note    Only valid to be called once available() returns true.
 *
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ProcessDataReader::getSize()
{
    return m_shared.p->m_size;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Wait for data to be available.
 *
 * \note    Can be called after request is made to block until writer
 *          is ready.
 *
 *//*-------------------------------------------------------------------*/

bool ProcessDataReader::wait(int waitMs)
{
    if (!m_mustWait)
        return true;

    if(m_readSemaphore->tryDown(waitMs))
    {
        m_readSemaphore->up();
        m_mustWait = false;
        return true;
    }

    return false;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Wait for data to be available.
 *
 * \note    Can be called after request is made to block until writer
 *          is ready.
 *
 *//*-------------------------------------------------------------------*/

bool ProcessDataReader::wait()
{
    if (!m_mustWait)
        return true;

    while (!m_readSemaphore->tryDown(200))
    {
        if (m_otherProcess && m_otherProcess->isFinished())
            return false;
    }

    m_readSemaphore->up();
    m_mustWait = false;
    return true;
}


/*-------------------------------------------------------------------*//*!
 * \brief   Reads a "page" of available data.
 *
 * \param   buffer       Buffer to read data to.
 * \param   bytesLeft    Number of bytes left after operation.
 * \param   bytesWritten Number of bytes written during this call.
 *
 * \note    Data is read in ProcessDataCopy::BufferSize size chunks.
 *
 *//*-------------------------------------------------------------------*/

void ProcessDataReader::read(Umbra::UINT8* buffer, Umbra::UINT32& bytesLeft, Umbra::UINT32& bytesWritten)
{
    // Special case for empty data
    if (!m_shared.p->m_size)
    {
        // Consume permission to read
        while(!m_readSemaphore->tryDown(1000))
        {
            if (m_otherProcess && m_otherProcess->isFinished()) break;
        }

        m_shared.p->m_available = false;
        m_readRequestActive = false;
        bytesLeft = 0;
        return;
    }

    if (!m_shared.p->m_bytesLeft)
    {
        m_readRequestActive = false;
        bytesLeft = m_shared.p->m_bytesLeft;
        return;
    }

    // Consume permission to read
    while(!m_readSemaphore->tryDown(1000))
    {
        if (m_otherProcess && m_otherProcess->isFinished())
        {
            m_readRequestActive = false;
            bytesLeft = 0;
            return;
        }
    }

    // Copy data from shared buffer to local buffer
    UINT32 copySize = (UINT32)::min2(m_shared.p->m_bytesLeft, BufferSize);

    if (buffer)
        memcpy(buffer, (const void*)m_shared.p->m_data, copySize);

    // Decrement bytes left
    m_shared.p->m_bytesLeft -= copySize;

    // Output state
    bytesLeft = m_shared.p->m_bytesLeft;
    bytesWritten = copySize;

    m_shared.p->m_available = false;

    // Give permission to write if data left
    if (bytesLeft)
        m_writeSemaphore->up();
    else
        m_readRequestActive = false;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Writes a buffer.
 *
 * \param   buffer  Buffer to write.
 * \param   size    Size of buffer.
 *
 * \note    Note that this function executed syncronously with
 *          ProcessDataReader::read.
 *
 *//*-------------------------------------------------------------------*/

void ProcessDataWriter::write(const Umbra::UINT8* buffer, Umbra::UINT32 size)
{
    m_shared.p->m_size = size;
    m_shared.p->m_bytesLeft = size;

    m_shared.p->m_available = true;

    m_localBytesLeft = size;
    const UINT8 *pointer = buffer;

    // Give one read permission in case of no data
    if (m_localBytesLeft == 0)
        m_readSemaphore->up();
    else
    while (m_localBytesLeft > 0)
    {
        // Copy next chunk of data to shared buffer
        UINT32 copySize = (UINT32)::min2(m_shared.p->m_bytesLeft, BufferSize);
        memcpy( (void*)m_shared.p->m_data, pointer, copySize);
        pointer += copySize;
        m_localBytesLeft -= copySize;

        // Fill rest of the buffer with zero, if this is the last chunk
        if( copySize < BufferSize)
            memset((UINT8*)m_shared.p->m_data + copySize, 0, BufferSize - copySize);

        // Give permission to read
        m_readSemaphore->up();

        // Consume permission to write
        if(m_localBytesLeft > 0)
        {
            while(!m_writeSemaphore->tryDown(1000))
            {
                if (m_otherProcess && m_otherProcess->isFinished())
                {
                    m_shared.p->m_request = RT_NONE;
                    m_shared.p->m_bytesLeft = 0;
                    return;
                }
            }
        }
    }

    m_shared.p->m_request = RT_NONE;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Initializes writing by page.
 *
 * \note    This function should be called before calling
 *          write(const Umbra::UINT8*, size_t&, int).
 *
 * \param   size    Size of buffer that's going to be written.
 *
 *//*-------------------------------------------------------------------*/

void ProcessDataWriter::beginWrite(Umbra::UINT32 size)
{
    m_shared.p->m_size = size;
    m_shared.p->m_bytesLeft = size;
    m_localBytesLeft = size;

    m_shared.p->m_available = true;

    m_shared.p->m_request = RT_NONE;

    // Give one read permission in case of no data
    if (size == 0)
        m_readSemaphore->up();

}

/*-------------------------------------------------------------------*//*!
 * \brief   Writes a "page" of data.
 *
 * \note    Before calling this, initialize write by calling
 *          beginWrite.
 *
 * \note    Caller must keep track of correct buffer position.
 *
 * \param   buffer          Pointer to page start to write.
 * \param   bytesWritten    About of bytes written during this call
 *                          (not total).
 *
 *//*-------------------------------------------------------------------*/

void ProcessDataWriter::writePage(const Umbra::UINT8* buffer, Umbra::UINT32& bytesWritten)
{
    bytesWritten = 0;

    if(!m_localBytesLeft)
        return;

    // Copy next chunk of data to shared buffer
    UINT32 copySize = (UINT32)::min2(m_shared.p->m_bytesLeft, BufferSize);
    memcpy( (void*)m_shared.p->m_data, buffer, copySize);
    bytesWritten = copySize;
    m_localBytesLeft -= copySize;

    // Fill rest of the buffer with zero, if this is the last chunk
    if (copySize < BufferSize)
        memset((UINT8*)m_shared.p->m_data + copySize, 0, BufferSize - copySize);

    // Give permission to read
    m_readSemaphore->up();
}

/*-------------------------------------------------------------------*//*!
 * \brief   Wait for permission to write.
 *
 * \return  true, if permission was granted.
 *
 *//*-------------------------------------------------------------------*/

bool ProcessDataWriter::waitWrite(int waitMs)
{
    // First write is free
    if(m_localBytesLeft == m_shared.p->m_size)
        return true;

    // Try to consume permission to write
    if (m_localBytesLeft > 0)
        return m_writeSemaphore->tryDown(waitMs);

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Flushes reader on exit.
 *//*-------------------------------------------------------------------*/

void ProcessDataReader::flush()
{
    while (m_readRequestActive)
    {
        UINT32 bytesLeft, bytesWritten;
        read(0, bytesLeft, bytesWritten);
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief   Flushes writer on exit.
 *//*-------------------------------------------------------------------*/

void ProcessDataWriter::flush()
{
    checkRequest();
    m_shared.p->m_bytesLeft = 0;
    m_shared.p->m_available = false;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

ProcessInputStream::ProcessInputStream(ProcessDataCopy::RequestType type, ProcessDataReader* reader, Allocator* a) :
    m_allocator(a), m_reader(reader), m_buf(NULL), m_stream(NULL)
{
    UMBRA_ASSERT(m_reader);

    m_reader->request(type);
    m_reader->wait();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

ProcessInputStream::ProcessInputStream(ProcessDataReader* reader, Allocator* a) :
    m_allocator(a), m_reader(reader), m_buf(NULL), m_stream(NULL)
{
    UMBRA_ASSERT(m_reader);
    m_reader->wait();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void ProcessInputStream::communicate(void)
{
    UINT32 dataSize = m_reader->getSize();

    if (!dataSize)
    {
        UINT32 bytesLeft = 0;
        UINT32 bytesWritten = 0;
        // Perform empty read
        m_reader->read( 0, bytesLeft, bytesWritten );
        return;
    }

    m_buf = (UINT8*)UMBRA_HEAP_ALLOC(m_allocator, dataSize);

    UINT32 bytesLeft = 0;
    UINT32 offset = 0;

    // Read data from another process page by page
    do
    {
        UINT32 bytesWritten = 0;
        m_reader->read( (UINT8*)m_buf + offset, bytesLeft, bytesWritten );
        offset += bytesWritten;
    } while (bytesLeft > 0);

    m_stream = UMBRA_HEAP_NEW(m_allocator, MemInputStream, m_buf, dataSize);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

ProcessInputStream::~ProcessInputStream()
{
    UMBRA_HEAP_DELETE(m_allocator, m_stream);
    UMBRA_HEAP_DELETE(m_allocator, m_buf);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ProcessInputStream::read(void* ptr, Umbra::UINT32 numBytes)
{
    if (!m_stream)
        return 0;

    return m_stream->read(ptr, numBytes);
}


/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

ProcessOutputStream::ProcessOutputStream(ProcessDataWriter* writer, Allocator* a) :
    m_allocator(a), m_writer(writer), m_stream(a), m_bytesLeft(0), m_offset(0), m_communicate(false)
{
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

ProcessOutputStream::~ProcessOutputStream(void)
{}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool ProcessOutputStream::communicate(void)
{
    UINT8* data = (UINT8*)m_stream.getPtr();
    UINT32 size = m_stream.getSize();

    if (!m_communicate)
    {
        m_writer->beginWrite(size);
        m_bytesLeft = size;
        m_offset = 0;

        m_communicate = true;
    }

    do
    {
        UINT32 bytesWritten = 0;
        m_writer->writePage( data + m_offset, bytesWritten );

        m_offset    += bytesWritten;
        m_bytesLeft -= bytesWritten;

        // Wait for permission to write
        while (!m_writer->waitWrite(1000))
        {
            return false;
        }
    } while (m_bytesLeft > 0);

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 ProcessOutputStream::write(const void* ptr, Umbra::UINT32 numBytes)
{
    return m_stream.write(ptr, numBytes);
}

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)
