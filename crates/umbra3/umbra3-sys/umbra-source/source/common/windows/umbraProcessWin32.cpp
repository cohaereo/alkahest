/*!
 *
 * Umbra PVS Base
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
 * \brief   Threading library.
 *
 */

#include "umbraPrivateDefs.hpp"

#if UMBRA_OS == UMBRA_WINDOWS

#include <windows.h>
#include <rpc.h>
#include <time.h>
#include <rpc.h>

#include "umbraProcess.hpp"
#include "umbraString.hpp"
#include "umbraThread.hpp"
#include "umbraUUID.hpp"

using namespace Umbra;

static String processError;

static void setProcessError(const char* err)
{
    processError = err;
}

const String& Umbra::getProcessError(void)
{
    return processError;
}

static void WindowsAPIError(LPCTSTR lpszFunction)
{
    // Retrieve the system error message for the last-error code

    LPVOID lpMsgBuf;
    LPVOID lpDisplayBuf;
    DWORD dw = GetLastError();

    FormatMessage(
        FORMAT_MESSAGE_ALLOCATE_BUFFER |
        FORMAT_MESSAGE_FROM_SYSTEM |
        FORMAT_MESSAGE_IGNORE_INSERTS,
        NULL,
        dw,
        MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
        (LPTSTR) &lpMsgBuf,
        0, NULL );

    // Display the error message and exit the process

    lpDisplayBuf = (LPVOID)LocalAlloc(LMEM_ZEROINIT,
        (lstrlen((LPCTSTR)lpMsgBuf) + lstrlen((LPCTSTR)lpszFunction) + 256) * sizeof(TCHAR));
    _snprintf((LPTSTR)lpDisplayBuf, LocalSize(lpDisplayBuf) / sizeof(TCHAR), 
              "%s failed (%d): %s",
              lpszFunction, dw, lpMsgBuf);

    //MessageBox(NULL, (LPCTSTR)lpDisplayBuf, TEXT("PVS calculation failed"), MB_OK);
    //printf("%s\n", (LPCTSTR)lpDisplayBuf);

    setProcessError((LPCTSTR)lpDisplayBuf);

    LocalFree(lpMsgBuf);
    LocalFree(lpDisplayBuf);

    return;
}

namespace Umbra
{
    class ProcessSharedMemoryImpl
    {
    public:
        ProcessSharedMemoryImpl() :
          mapObjectHandle(NULL) {}

        HANDLE mapObjectHandle;
    };
} // namespace Umbra

void* Umbra::processAlloc(String identifier, unsigned int size, bool& first, ProcessSharedMemoryImpl** impl)
{
    setProcessError("");

    void* data = NULL;

    (*impl) = UMBRA_NEW(ProcessSharedMemoryImpl);

    (*impl)->mapObjectHandle =
        CreateFileMapping(
            INVALID_HANDLE_VALUE,
            NULL,
            PAGE_READWRITE,
            0,
            size,
            identifier.toCharPtr());

    if(!(*impl)->mapObjectHandle)
    {
        WindowsAPIError("CreateFileMapping");
        return NULL;
    }

    first = (GetLastError() != ERROR_ALREADY_EXISTS);

    data = MapViewOfFile(
                (*impl)->mapObjectHandle,
                FILE_MAP_WRITE,
                0,
                0,
                0);

    if (data == NULL)
    {
        WindowsAPIError("MapViewOfFile");
        return NULL;
    }

    if(first)
        memset(data, 0, size);

    return data;
}

void Umbra::processFree(void* data, ProcessSharedMemoryImpl* impl)
{
    UnmapViewOfFile(data);
    CloseHandle(impl->mapObjectHandle);
    UMBRA_DELETE(impl);
}

namespace Umbra
{
    class StdOutRunnable;

    /*-------------------------------------------------------------------*//*!
     * \brief   Platform-specific process data.
     *//*-------------------------------------------------------------------*/

    class ImplProcess
    {
    public:

        ImplProcess()
        : started(false),
          aborted(false),
          stdoutThread(NULL),
          stdoutRunnable(NULL),
          childStdoutRead(NULL)
        {
            memset(&processInfo, 0, sizeof(PROCESS_INFORMATION));
        }

        // Whether this process was started
        bool                 started;
        // Whether the output processing thread should abort
        bool                 aborted;
        // Executable image path
        String               executable;
        // Command line elements
        Array<String>        commandline;
        // Details of the executed process
        PROCESS_INFORMATION  processInfo;

        // Thread that processes redirected output from the child process
        Thread*              stdoutThread;
        // Thread object
        StdOutRunnable*      stdoutRunnable;
        // Read handle from child process stdout pipe
        HANDLE               childStdoutRead;
    };

    /*-------------------------------------------------------------------*//*!
     * \brief   Output processing thread for redirected child process
     *          output
     *//*-------------------------------------------------------------------*/

    class StdOutRunnable : public Runnable
    {
    public:

        unsigned long run(void * param)
        {
            ImplProcess* process = (ImplProcess*)param;

            char buf[256] = "";
            DWORD dwRead, dwWritten;
            BOOL success = FALSE;

            // Loop until abort indicated - should normally exit when ReadFile fails
            while(!process->aborted)
            {

                // Read redirected output from child process
                success = ReadFile( process->childStdoutRead, buf, 256, &dwRead, NULL );

                // Failure should indicate child process exit
                if( !success || dwRead == 0 )
                    break;

                // Write output from child process to current process's stdout
                WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), buf, dwRead, &dwWritten, NULL);

                // Note that WriteFile MUST be allowed to fail without it affecting calls to ReadFile.
                // There's a gotcha here: if it happens that stdout of the parent process
                // becomes invalid (making WriteFile fail), we still must flush output from the
                // child process pipe by calling ReadFile. Otherwise the child process deadlocks
                // on print once the pipe buffer is filled.
            }

            FlushFileBuffers(GetStdHandle(STD_OUTPUT_HANDLE));

            return 0;
        }
    };
} // namespace Umbra


/*-------------------------------------------------------------------*//*!
 * \brief   Process constructor
 *//*-------------------------------------------------------------------*/

Process::Process()
{
    m_impl = UMBRA_NEW(ImplProcess);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Process constructor with executable path
 *//*-------------------------------------------------------------------*/

Process::Process(const String& executable)
{
    m_impl = UMBRA_NEW(ImplProcess);

    m_impl->executable = executable;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Process constructor with executable path and parameters
 *//*-------------------------------------------------------------------*/

Process::Process(const String& executable, Array<String>& commandline)
{
    m_impl = UMBRA_NEW(ImplProcess);

    m_impl->executable = executable;
    m_impl->commandline = commandline;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Set executable for this process
 *//*-------------------------------------------------------------------*/

void Process::setExecutable(const String& executable)
{
    // Fail if already started
    if(m_impl->started)
        return;

    m_impl->executable = executable;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Set command line
 *//*-------------------------------------------------------------------*/

void Process::setCommandLine(Array<String>& commandline)
{
    // Fail if already started
    if(m_impl->started)
        return;

    m_impl->commandline = commandline;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Destructor
 *
 * Doesn't kill the process, waits for it to exit.
 *//*-------------------------------------------------------------------*/

Process::~Process(void)
{
    waitToFinish();

    if(m_impl->childStdoutRead)
        CloseHandle(m_impl->childStdoutRead);

    m_impl->aborted = true;
    if(m_impl->stdoutThread)
        m_impl->stdoutThread->waitToFinish(0);

    UMBRA_DELETE(m_impl->stdoutThread);
    UMBRA_DELETE(m_impl->stdoutRunnable);

    CloseHandle(m_impl->processInfo.hProcess);

    UMBRA_DELETE(m_impl);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Run the process.
 *//*-------------------------------------------------------------------*/

Process::Error Process::run(void)
{
    setProcessError("");

    if(m_impl->started)
    {
        setProcessError("process already started.");
        return E_OTHER;
    }

    m_impl->started = true;

    FILE* f = fopen(m_impl->executable.toCharPtr(), "rb");
    if(f == NULL)
    {
        setProcessError("process executable not found.");
        return ERROR_EXECUTABLE_NOT_FOUND;
    }
    fclose(f);

    bool redirectOutput = true;

    HANDLE stdoutHandle = GetStdHandle(STD_OUTPUT_HANDLE);

    // Test if STDOUT is valid
    if( stdoutHandle == INVALID_HANDLE_VALUE ||
        stdoutHandle == NULL )
        // If current process's stdout is invalid, there's no point in redirecting
        // stdout and stderr from child process at all.
        redirectOutput = false;

    // Initialize process startup data
    STARTUPINFO startupInfo;
    GetStartupInfoA(&startupInfo);

    HANDLE childStdoutRead  = NULL;
    HANDLE childStdoutWrite = NULL;

    // If we wish to redirect child process output
    if(redirectOutput)
    {
        // Configure security attributes for creating inheritable pipe
        SECURITY_ATTRIBUTES securityAttrib;
        securityAttrib.nLength = sizeof(SECURITY_ATTRIBUTES);
        securityAttrib.bInheritHandle = TRUE;
        securityAttrib.lpSecurityDescriptor = NULL;

        // Create pipe for redirecting child stdout
        if ( !CreatePipe(&childStdoutRead, &childStdoutWrite, &securityAttrib, 0) )
        {
            WindowsAPIError("CreatePipe");
            return E_OTHER;
        }

        // Prevent inheriting the read handle
        if ( !SetHandleInformation(childStdoutRead, HANDLE_FLAG_INHERIT, 0) )
        {
            CloseHandle(childStdoutRead);
            CloseHandle(childStdoutWrite);
            WindowsAPIError("SetHandleInformation");
            return E_OTHER;
        }

        // Configure stdout and stderror for the child process, so that
        // they write to the pipe
        startupInfo.hStdError  = childStdoutWrite;
        startupInfo.hStdOutput = childStdoutWrite;
        startupInfo.hStdInput  = INVALID_HANDLE_VALUE;
        startupInfo.dwFlags    = STARTF_USESTDHANDLES;
    }

    HANDLE processPseudoHandle = GetCurrentProcess();
    HANDLE inheritableHandle;

    // Create inheritable duplicate of current process handle for the child process
    if(!DuplicateHandle(processPseudoHandle, processPseudoHandle, processPseudoHandle, &inheritableHandle, 0, TRUE, DUPLICATE_SAME_ACCESS))
    {
        CloseHandle(childStdoutRead);
        CloseHandle(childStdoutWrite);
        WindowsAPIError("DuplicateHandle");
        return E_OTHER;
    }

    // Formulate final command line, where first token is the executable
    String commandline2 = String("\"")+ m_impl->executable + String("\" ");// + m_impl->commandline;

    for(int i = 0; i < m_impl->commandline.getSize(); i++)
    {
        commandline2 += m_impl->commandline[i] + String(" ");
    }

    // Add the duplicatable handle as last parameter in command line
    // Be careful here, the type HANDLE is 32bit/64bit depending on configuration
    char processHandle[32] = "";
    sprintf(processHandle, "%p", inheritableHandle);
    commandline2 += String(processHandle);

    // Command line given to process must be changeable,
    // make a changeable copy
    char* commandlineParam = UMBRA_NEW_ARRAY(char, commandline2.length()+1);
    memcpy(commandlineParam, commandline2.toCharPtr(), commandline2.length()+1);

    // Start child process
    BOOL result;
    result = CreateProcessA(
        m_impl->executable.toCharPtr(),
        commandlineParam,
        NULL,
        NULL,
        TRUE,
        0,
        NULL,
        NULL,
        &startupInfo,
        &m_impl->processInfo);

    UMBRA_DELETE_ARRAY(commandlineParam);

    if(!result)
    {
        if(redirectOutput)
        {
            CloseHandle(childStdoutRead);
            CloseHandle(childStdoutWrite);
        }
        WindowsAPIError("CreateProcess");
        return E_OTHER;
    }

    if(redirectOutput && !CloseHandle(childStdoutWrite))
    {
        WindowsAPIError("CloseHandle");
        return E_OTHER;
    }

    if(!CloseHandle(inheritableHandle))
    {
        WindowsAPIError("CloseHandle");
        return E_OTHER;
    }

    // If child process output was redirected
    if(redirectOutput)
    {
        // Start thread for processing the output
        m_impl->childStdoutRead = childStdoutRead;
        m_impl->stdoutRunnable = UMBRA_NEW(StdOutRunnable);
        m_impl->stdoutThread = ThreadPool::get();
        m_impl->stdoutThread->setFunction(m_impl->stdoutRunnable);

        if(!m_impl->stdoutThread->run(m_impl))
        {
            CloseHandle(childStdoutRead);
            return E_OTHER;
        }
    }

    return ERROR_OK;

}

/*-------------------------------------------------------------------*//*!
 * \brief   Is process finished?
 *//*-------------------------------------------------------------------*/

bool Process::isFinished(void) const
{
    if(m_impl->processInfo.hProcess == NULL)
        return true;

    DWORD exitcode;
    GetExitCodeProcess(m_impl->processInfo.hProcess, &exitcode);
    return exitcode != STILL_ACTIVE;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Get process exit code
 *//*-------------------------------------------------------------------*/

uint32 Process::getExitCode(void) const
{
    if(m_impl->processInfo.hProcess == NULL)
        return 0;

    DWORD exitcode;
    GetExitCodeProcess(m_impl->processInfo.hProcess, &exitcode);
    if(exitcode == STILL_ACTIVE)
        exitcode = 0;
    return exitcode;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Wait for process to exit
 *//*-------------------------------------------------------------------*/

void Process::waitToFinish(void)
{
    if(m_impl->processInfo.hProcess == NULL)
        return;

    WaitForMultipleObjects(1, &m_impl->processInfo.hProcess, TRUE, INFINITE);
}

HandleProcess::HandleProcess(OSProcessHandle handle)
    : m_handle(handle),
      m_isParent(false) // unused
{
}

/*-------------------------------------------------------------------*//*!
 * \brief   Is process finished?
 *//*-------------------------------------------------------------------*/

bool HandleProcess::isFinished(void) const
{
    if(m_handle == NULL)
        return false;

    DWORD exitcode;
    GetExitCodeProcess(m_handle, &exitcode);
    return exitcode != STILL_ACTIVE;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Get process exit code
 *//*-------------------------------------------------------------------*/

uint32 HandleProcess::getExitCode(void) const
{
    if(m_handle == NULL)
        return 0;

    DWORD exitcode;
    GetExitCodeProcess(m_handle, &exitcode);
    if(exitcode == STILL_ACTIVE)
        exitcode = 0;
    return exitcode;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Wait for process to exit
 *//*-------------------------------------------------------------------*/

void HandleProcess::waitToFinish(void)
{
    if(m_handle == NULL)
        return;

    WaitForMultipleObjects(1, &m_handle, TRUE, INFINITE);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Generates identifiers for shared allocs.
 *//*-------------------------------------------------------------------*/

String Umbra::generateProcessUID(void)
{
    UUID uuid = UUID::generate();
    char str[UUID::charLength] = "";
    uuid.string(str);
    return String(str);
}

bool Process::is64BitProcess()
{
#if defined(_WIN64)
    return true;
#else
    return false;
#endif
}

bool Process::is64BitCapable()
{
#if defined(_WIN64)
    return true;  // 64-bit programs run only on Win64
#else

    // 32-bit programs run on both 32-bit and 64-bit Windows so must sniff
    BOOL isWow64 = FALSE;

    typedef BOOL (WINAPI *FuncIsWow64Process) (HANDLE, PBOOL);
    HMODULE kernel = GetModuleHandle(TEXT("kernel32"));
    if( kernel ) {
        FuncIsWow64Process funcWow64 = (FuncIsWow64Process)GetProcAddress( kernel,"IsWow64Process" );
        if( NULL != funcWow64 )
            funcWow64( GetCurrentProcess(),&isWow64 );
    }
    return isWow64 ? true : false;
#endif
}

// Expect Window's UUIDs to be 128 bits
UMBRA_CT_ASSERT(sizeof(::UUID) == 16);

Umbra::UUID Umbra::UUID::generate(void)
{
    typedef long (WINAPI* PUuidCreateSequential) (::UUID *Uuid);
    PUuidCreateSequential UuidCreateSequential = 0;

    HMODULE lib = LoadLibraryA("rpcrt4.DLL");
    if (lib)
        UuidCreateSequential = (PUuidCreateSequential)GetProcAddress(lib, "UuidCreateSequential");

    // Should always be present
    if (!UuidCreateSequential)
    {
        UMBRA_ASSERT(false);
        static int counter = 0;
        Umbra::UUID result;
        result.m_uuid[0] = counter++;
        result.m_uuid[1] = counter++;
        result.m_uuid[2] = counter++;
        result.m_uuid[3] = counter++;
        counter++;
        return result;
    }        

    Umbra::UUID result;
    ::UUID      system;
    if (UuidCreateSequential(&system) != RPC_S_UUID_NO_ADDRESS)
    {
        result.m_uuid[0] = system.Data1;
        result.m_uuid[1] = (system.Data2 << 16) | system.Data3;
        result.m_uuid[2] = (system.Data4[0] << 24) | (system.Data4[1] << 16) | (system.Data4[2] << 8) | system.Data4[3];
        result.m_uuid[3] = (system.Data4[4] << 24) | (system.Data4[5] << 16) | (system.Data4[6] << 8) | system.Data4[7];    
    }
    return result;
}

#endif
