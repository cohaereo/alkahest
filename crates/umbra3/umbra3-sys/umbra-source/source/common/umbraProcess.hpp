#ifndef UMBRAPROCESS_HPP
#define UMBRAPROCESS_HPP
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
 * \brief   Process Library
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraArray.hpp"
#include "umbraString.hpp"

#if UMBRA_OS == UMBRA_WINDOWS
typedef void* HANDLE;
#else
#include <sys/types.h> // pid_t
#endif

namespace Umbra
{
    template<class T>
    class ProcessSharedMemory;

    /*-------------------------------------------------------------------*//*!
     * \brief           Allocate class T for interprocess communication.
     *//*-------------------------------------------------------------------*/
    template<class T>
    ProcessSharedMemory<T> processAlloc(String identifier);

    /*-------------------------------------------------------------------*//*!
     * \brief           Free class T for interprocess communication.
     *//*-------------------------------------------------------------------*/
    template<class T>
    void processFree(ProcessSharedMemory<T> &sharedMemory);

    /*-------------------------------------------------------------------*//*!
     * \brief           Process shared memory wrapper class.
     *//*-------------------------------------------------------------------*/
    template<class T>
    class ProcessSharedMemory
    {
        friend void processFree<>(ProcessSharedMemory<T> &sharedMemory);
        friend ProcessSharedMemory<T> processAlloc<>(String identifier);

    public:
        ProcessSharedMemory();
        T* p;

        bool operator! (void) const {return !p;}

    private:

        class ProcessSharedMemoryImpl* m_impl;

    };

    /*-------------------------------------------------------------------*//*!
     * \brief           Allocate unique identifier to be used with process
     *                  shared memory.
     *//*-------------------------------------------------------------------*/
    String generateProcessUID();

    /*-------------------------------------------------------------------*//*!
     * \brief           Internal process shared memory alloc.
     *//*-------------------------------------------------------------------*/
    void*  processAlloc(String identifier, unsigned int size, bool& first, class ProcessSharedMemoryImpl** impl);

    /*-------------------------------------------------------------------*//*!
     * \brief           Internal process shared memory free
     *//*-------------------------------------------------------------------*/
    void   processFree(void* data, class ProcessSharedMemoryImpl* impl);

    /*-------------------------------------------------------------------*//*!
     * \brief           Base class for process handle and process
     *                  spawned from executable.
     *//*-------------------------------------------------------------------*/
    class ProcessBase
    {
    public:
        virtual             ~ProcessBase    (void) {}
        virtual bool        isFinished      (void) const = 0;
        virtual uint32      getExitCode     (void) const = 0;
        virtual void        waitToFinish    (void)       = 0;
    };

    /*-------------------------------------------------------------------*//*!
     * \brief           Process created from handle.
     *//*-------------------------------------------------------------------*/
    class HandleProcess : public ProcessBase
    {
    public:

#if UMBRA_OS == UMBRA_WINDOWS

        typedef HANDLE      OSProcessHandle;

#else

        enum ParentProcess
        {
            HANDLE_PARENT
        };

        HandleProcess       (ParentProcess);

        typedef pid_t       OSProcessHandle;

#endif

        HandleProcess       (OSProcessHandle handle);

        bool                isFinished      (void) const;
        uint32              getExitCode     (void) const;
        void                waitToFinish    (void);

    private:

        OSProcessHandle     m_handle;
        bool                m_isParent;
    };

    /*-------------------------------------------------------------------*//*!
     * \brief           Process API for managing child processes.
     *//*-------------------------------------------------------------------*/
    class Process : public ProcessBase
    {
    public:
        enum Error
        {
            ERROR_OK,
            ERROR_EXECUTABLE_NOT_FOUND,
            E_OTHER
        };

                            Process         ();
                            Process         (const String& executable);
                            Process         (const String& executable, Array<String>& commandline);

                            ~Process        (void);

        void                setExecutable   (const String& executable);
        void                setCommandLine  (Array<String>& commandline);

        Error               run             (void);

        bool                isFinished      (void) const;
        uint32              getExitCode     (void) const;
        void                waitToFinish    (void);

        static bool         is64BitProcess  (void);
        static bool         is64BitCapable  (void);

    private:

        class ImplProcess*  m_impl;

    };

    const String&           getProcessError (void);

} // namespace Umbra

#include "umbraProcess.inl"


#endif // UMBRAPROCESS_HPP

//--------------------------------------------------------------------
