// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRAPLATFORM_HPP
#define UMBRAPLATFORM_HPP

/*!
 * \file   umbraPlatform.hpp
 * Umbra platform integration callback functions.
 * At the moment these are used solely by the Builder interface, except
 * for the use of Allocator in Tome loading.
 */

#include "umbraDefs.hpp"

namespace Umbra
{

/*!
 * \brief       Allocator interface for dynamic memory allocations
 *
 * The Umbra classes that require dynamic allocations do so by callbacks
 * to the user provided Allocator implementation. This is to give the user
 * full control of the memory management. When no user implementation is
 * provided, a default implementation that uses std::malloc and std::free
 * is used.
 *
 * \note    The Umbra::Task implementation does not use the Allocator
 *          interface but uses the OS services directly instead.
 */
class Allocator
{
public:
    virtual         ~Allocator (void) {}

    /*!
     * \brief   Allocate a block of memory
     *
     * The implementation of this class should reserve and return a block
     * contiguous block of memory of size indicated by the size parameter.
     * The semantics are similar to std::malloc. Note that Umbra does internal
     * memory management on top of the allocations through this interface to
     * reduce the number of calls.
     *
     * Umbra makes no effort of mutually excluding calls to the allocation
     * interface, whether or not the implementations should do this depends
     * on the usage pattern of the Umbra public interfaces: if calls are made
     * to the Umbra modules simulatenously from multiple threads then thread
     * safety needs to be implemented on this level as well.
     *
     * \param   size    The requested allocation size, in bytes.
     * \param   info    A freeform description of the allocation, for
     *                  optional tracking of where the allocations are being
     *                  made from.
     * \return          A pointer to the allocated block of memory.
     */
    virtual void*   allocate    (size_t size, const char* info = NULL) = 0;

    /*!
     * \brief   Free a block of memory
     *
     * Free a previously allocated block of memory and make it available for reuse.
     *
     * \param   ptr     Pointer to memory block previously allocated with allocate().
     */
    virtual void    deallocate  (void* ptr) = 0;
};

/*!
 * \brief       Logger interface
 *
 * The Umbra Optimizer module provides useful information on the progress and
 * potential issues though the Logger interface. When no user implementation
 * is provided, a default implementation that formats the log messages into
 * STDOUT is used.
 *
 * When contacting Umbra support with issues related to the use of the Umbra
 * Optimizer it is a good idea to include a full log of the computation.
 *
 * \note    The Umbra::Task implementation does not use the Logger
 *          interface but uses the OS services directly instead.
 */
class Logger
{
public:

    /*! Class of log message */
    enum Level
    {
        /*! A debug message, generally only used for Umbra development purposes */
        LEVEL_DEBUG       = 0,
        /*! An informational log message */
        LEVEL_INFO        = 1,
        /*! A warning message for highlighting issues in the input or otherwise
         *  suboptimal operation. A warning message is always non-fatal and does
         *  not result in wrong behavior, but can provide useful clues for improving
         *  the computation performance and/or output quality */
        LEVEL_WARNING     = 2,
        /*! A detailed description of a fatal error. Generally when an error message
         *  is logged the API call causing it will return with an error code. */
        LEVEL_ERROR       = 3
    };

    virtual         ~Logger     (void) {}

    /*!
     * \brief   The single line log message callback
     *
     * Implement this function to display or store log messages as appropriate in your
     * system. The level of the message can be used to filter and/or highlight messages
     * based on the class.
     *
     * \param   level   The class of the log message
     * \param   str     The log message string, as zero-terminated ascii with no newline
     *                  character at the end
     */
    virtual void    log         (Level level, const char* str) = 0;
};

/*!
 * \brief       License key store
 *
 * User implementation of license key storage, used to retrieve the Umbra license key.
 */
class LicenseKey
{
public:
    virtual ~LicenseKey         (void) {}
    virtual void    readKey     (char key[128]) = 0;
};

/*!
 * \brief       Input byte stream abstraction
 *
 * InputStream is an abstraction of a byte stream input to Umbra, hiding the storage of
 * computation resources from the implementation. Typical implementations include input
 * from the filesystem, over the network or directly from a blob of memory.
 */
class InputStream
{
public:
    virtual         ~InputStream    (void) {}

    /*!
     * \brief   Read a given number of bytes from the stream
     *
     * Umbra uses this callback to deserialize computation elements from user storage.
     * The implementation shoud copy 'numBytes' bytes into the memory pointer to by 'ptr'
     * and return the number of bytes actually read. The caller of this function knows
     * how much data to expect, so a read operation should never be requested that passes
     * the end of stream. If this happens, an error has occurred.
     *
     * The caller does not buffer the input in any way so if the individual read operation
     * from the underlying storage is slow the implementation of this function should maintain
     * a read buffer. Note that OS stream read operations such as std::fread may already
     * provide the buffering.
     *
     * The caller also expects a single read operation to always be fully satisfied, returning
     * anything but 'numBytes' from the operation is always treated as an error.
     *
     * \note    The byte data should be read as-is without any type of conversion. Make sure
     *          that you are not, for example, treating the stream as text data for which
     *          operating system functions typically do newline conversions.
     *
     * \param   ptr         A pointer to the destination memory
     * \param   numBytes    Number of bytes to read
     * \return              The value of numBytes if the operation succeeded, anything else
     *                      if an error was encountered.
     */
    virtual uint32_t  read            (void* ptr, uint32_t numBytes) = 0;
};

/*!
 * \brief       Output byte stream abstraction
 *
 * OutputStream is an abstraction of a byte stream output from Umbra, hiding the storage of
 * computation resources from the implementation. Typical implementations include output
 * to the filesystem, over the network or directly to a blob of memory.
 */
class OutputStream
{
public:
    virtual         ~OutputStream   (void) {}

    /*!
     * \brief   Write a given number of data bytes into the output
     *
     * Umbra uses this function to serialize data into user storage. The implementation
     * should copy 'numBytes' bytes from memory pointer to by 'ptr' into the underlying
     * storage.
     *
     * The caller does not buffer the output in any way so if the individual write operation
     * is slow the implementation of this function should maintain a write buffer. Note that
     * OS stream write operations such as std::fwrite may already provide the buffering.
     *
     * The caller also expects a single write operation to always be fully satisfied, returning
     * anything but 'numBytes' from the operation is always treated as an error.
     *
     * \note    The byte data should be read as-is without any type of conversion. Make sure
     *          that you are not, for example, treating the stream as text data for which
     *          operating system functions typically do newline conversions.
     *
     * \param   ptr         A pointer to the source memory
     * \param   numBytes    Number of bytes to write
     * \return              The value of numBytes if the operation succeeded, anything else
     *                      if an error was encountered.
     *
     */
    virtual uint32_t write           (const void* ptr, uint32_t numBytes) = 0;
};


/*!
 * \brief       Collection of platform services
 *
 * A PlatformServices instance is a container of platform integration implementation
 * classes. A NULL pointer can be supplied when a default implementation should be used
 * for any given service.
 */
struct PlatformServices
{
    PlatformServices(Allocator* a = NULL, Logger* l = NULL, LicenseKey* e = NULL)
        : allocator(a), logger(l), licenseKey(e) {}

    /** Allocator implementation */
    Allocator*      allocator;
    /** Logger implementation */
    Logger*         logger;
    /** License key */
    LicenseKey*     licenseKey;
};


} // namespace Umbra

#endif
