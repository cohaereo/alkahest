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
 * \brief   Text scan implementation
 *
 */

#include "umbraDirScan.hpp"
#include "umbraPrivateDefs.hpp"

#include <stdio.h>
#include <stdlib.h>
#include <errno.h>

#if UMBRA_OS == UMBRA_WINDOWS || UMBRA_IS_POSIX

namespace Umbra
{

String canonicalPath(const String& path)
{
    bool isDir = checkPath(path) == CHECKPATH_DIRECTORY;

#if UMBRA_IS_POSIX
    bool isAbsolute = path.length() >= 1 && path[0] == '/';
#endif

    Array<String> pathSplit = path.pathSplit();
    Array<String> outPath;

    int first = 0;

#if UMBRA_OS == UMBRA_WINDOWS
    if (path.length() >= 2 && path[1] == ':')
        first = 1;
#endif

    for (int i = 0; i < pathSplit.getSize(); i++)
    {
        if (pathSplit[i] == ".")
            continue;
        else if (pathSplit[i] == ".." && outPath.getSize() > first && outPath[outPath.getSize() - 1] != "..")
            outPath.resize(outPath.getSize() - 1);
        else
            outPath.pushBack(pathSplit[i]);
    }

    String result;
#if UMBRA_IS_POSIX
    if (isAbsolute)
        result += "/";
#endif
    for (int i = 0; i < outPath.getSize(); i++)
    {
        result += outPath[i];
        if (i != outPath.getSize() - 1)
            result += "/";
    }
    if (isDir)
        result += "/";

#if UMBRA_OS == UMBRA_WINDOWS
    result.lower();
#endif
    return result;
}

String joinPaths(const String& a, const String& b)
{
    String result(a);
    result.replace("\\", "/");
    if (result[result.length() - 1] != '/')
        result += '/';
    result += b;
    result.replace("\\", "/");
    return canonicalPath(result);
}

String absolutePath(const String& path)
{
#if UMBRA_OS == UMBRA_WINDOWS
    if (path.length() >= 2 && path[1] == ':')
#else
    if (path.length() >= 1 && path[0] == '/')
#endif
        return canonicalPath(path);

    bool isDir = checkPath(path) == CHECKPATH_DIRECTORY;

    Array<String> currentSplit = getCurrentDirectory().pathSplit();
    Array<String> pathSplit = path.pathSplit();

    for (int i = 0; i < pathSplit.getSize(); i++)
    {
        if (pathSplit[i] == ".")
            continue;
        else if (pathSplit[i] == "..")
            currentSplit.resize(currentSplit.getSize() - 1);
        else
            currentSplit.pushBack(pathSplit[i]);
    }

    String result;
#if UMBRA_IS_POSIX
    result += "/";
#endif
    for (int i = 0; i < currentSplit.getSize(); i++)
    {
        result += currentSplit[i];
        if (i != currentSplit.getSize() - 1)
            result += "/";
    }
    if (isDir)
        result += "/";

    return canonicalPath(result);
}

String relativePath(const String& relativeTo, const String& filename)
{
    if (filename.length() == 0)
        return "";

    String cur(absolutePath(relativeTo));
    String abs(absolutePath(filename));
    String res;

#if UMBRA_OS == UMBRA_WINDOWS
    cur.lower();
    abs.lower();
#endif

    bool isDir = checkPath(filename) == CHECKPATH_DIRECTORY;

    cur.replace("\\", "/");
    abs.replace("\\", "/");

    Array<String> source = cur.pathSplit();
    Array<String> target = abs.pathSplit();

    int n = 0;
    for (; n < min2(source.getSize(), target.getSize()); n++)
    {
        if (source[n] != target[n])
            break;
    }

    if (n == 0)
        return abs;

    String result("");

    for (int i = 0; i < source.getSize() - n; i++)
        result += "../";

    for (int i = n; i < target.getSize(); i++)
        result += target[i] + "/";

    if (!isDir)
        result = String(result, 0, result.length() - 1);

    return result;
}
}

#endif

#if UMBRA_OS == UMBRA_WINDOWS

///////////////////////////////////////////////////////////////////
//////////// WINDOWS //////////////////////////////////////////////
///////////////////////////////////////////////////////////////////

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <direct.h>

namespace Umbra
{

CheckPathResult Umbra::checkPath (const char* path)
{
    DWORD attr = GetFileAttributes(path);
    if (attr == INVALID_FILE_ATTRIBUTES)
        return CHECKPATH_NOT_FOUND;

    if (attr & FILE_ATTRIBUTE_DIRECTORY)
        return CHECKPATH_DIRECTORY;

    return CHECKPATH_FILE;
}

String Umbra::getCurrentDirectory  (void)
{
    char temp[512] = "";
    GetCurrentDirectory(512, temp);
    return String(temp);
}


bool Umbra::createDir(const char* dir)
{
#if UMBRA_OS == UMBRA_WINDOWS || UMBRA_OS == UMBRA_XBOXONE
    if (_mkdir(dir))
#elif UMBRA_OS == UMBRA_OSX || UMBRA_OS == UMBRA_IOS
    if (mkdir(dir, 0777))
#else
    if (mkdir(dir, S_IRUSR | S_IWUSR | S_IXUSR))
#endif
    {
        int err;
#if defined(_MSC_VER)
        _get_errno(&err);
#else
        err = errno;
#endif
        if (err != EEXIST)
            return false;
    }
    return true;
}

DirScan::DirScan(const char* path, const char* expr, Allocator* a)
    : Base(a)
{
    m_numFiles = 0;

    UMBRA_ASSERT(expr);
    UMBRA_ASSERT(!path || strlen(path) <= DIRSCAN_MAX_PATH);
    UMBRA_ASSERT(strlen(expr) <= DIRSCAN_MAX_PATH);
    if(!expr)
        return;
    
    m_files.setAllocator(getAllocator());

    char    tmp[DIRSCAN_MAX_PATH + 1 + DIRSCAN_MAX_PATH];
    char    p[DIRSCAN_MAX_PATH+DIRSCAN_MAX_PATH];

    if(path && *path)
    {
        strcpy(tmp, path);
        strcat(tmp, "\\");
        strcat(tmp, expr);
    }
    else
        strcpy(tmp, expr);

    // Find all file names in the directory matching
    // the mask.. We use here the Win32 API file-finding
    // calls as all compilers should support them.
    WIN32_FIND_DATA fileData;
    HANDLE          h = FindFirstFile(tmp,&fileData);

    if(h != INVALID_HANDLE_VALUE)
    {
        do
        {
            bool ok = true;

            // Don't list directories
            if(fileData.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY)
                ok = false;

            if(ok)
            {
                *p = 0;
                fullPath(p, fileData.cFileName, DIRSCAN_MAX_PATH);

                char* ptr = strrchr(p, '/');
                if(ptr)
                    ptr++;
                else
                    ptr = p;

                char* str = UMBRA_NEW_ARRAY(char, strlen(ptr) + 1);
                strcpy(str, ptr);
                m_files.pushBack(str);
                m_numFiles++;
            }
        } while(FindNextFile(h,&fileData));

        FindClose(h);
    }
}

char* DirScan::getCwd(char* buffer, int maxlen)
{
    UMBRA_ASSERT(buffer && maxlen > 0);
    GetCurrentDirectory(maxlen, buffer);
    return buffer;
}

int DirScan::chDir(const char* path)
{
    UMBRA_ASSERT(path);
    return SetCurrentDirectory(path);
}

DirScan::~DirScan()
{
    for(int i=0; i<m_numFiles; i++)
        UMBRA_DELETE_ARRAY(m_files[i]);
}

int DirScan::getNumFiles() const
{
    return m_numFiles;
}

const char* DirScan::getFile(int num) const
{
    UMBRA_ASSERT(num >= 0 && num < m_numFiles);
    return m_files[num];
}

void DirScan::fullPath(char* absPath, const char* relPath, int /*maxLength*/)
{
    UMBRA_ASSERT(absPath && relPath);

    getCwd(absPath, DIRSCAN_MAX_PATH);      // get current working directory
    strcat(absPath, "/");
    strcat(absPath, relPath);
}

Umbra::UINT64 DirScan::getFileSize(const char* p)
{
    WIN32_FILE_ATTRIBUTE_DATA data;
    if (!GetFileAttributesEx(p, GetFileExInfoStandard, &data))
        return 0;

    return (UINT64)data.nFileSizeLow | ((UINT64)(data.nFileSizeHigh) << 32LL);
}

static Umbra::UINT64 filetimeToUINT64(FILETIME ft)
{
    return ((Umbra::UINT64)ft.dwHighDateTime << 32LL) | (Umbra::UINT64)ft.dwLowDateTime;
}

Umbra::UINT64 DirScan::getFileATime(const char* p)
{
    WIN32_FILE_ATTRIBUTE_DATA data;
    if (!GetFileAttributesEx(p, GetFileExInfoStandard, &data))
        return 0;

    return max2(filetimeToUINT64(data.ftCreationTime), max2(filetimeToUINT64(data.ftLastAccessTime), filetimeToUINT64(data.ftLastWriteTime)));
}

void DirScan::getFileAttrib(const char* p, Umbra::UINT64& accessTime, Umbra::UINT64& size)
{
    accessTime = 0;
    size = 0;

    WIN32_FILE_ATTRIBUTE_DATA data;
    if (!GetFileAttributesEx(p, GetFileExInfoStandard, &data))
        return;

    accessTime = max2(filetimeToUINT64(data.ftCreationTime), max2(filetimeToUINT64(data.ftLastAccessTime), filetimeToUINT64(data.ftLastWriteTime)));
    size       = (UINT64)data.nFileSizeLow | ((UINT64)(data.nFileSizeHigh) << 32LL);
}

void DirScan::removeFile(const char* p)
{
    _unlink(p);
}

}

#elif UMBRA_IS_POSIX

///////////////////////////////////////////////////////////////////
//////////// POSIX ////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////

#include <fnmatch.h>
#include <dirent.h>
#include <unistd.h>
#include <iostream>
#include <string.h>
#define _FILE_OFFSET_BITS 64
#include <sys/stat.h>


static char* fileMatchPattern; // Not thread-safe! Use glob().

namespace Umbra
{

CheckPathResult checkPath (const char* path)
{
    // posix
    struct stat sbuf;
    if (stat(path, &sbuf) == -1)
        return CHECKPATH_NOT_FOUND;

    if (S_ISDIR(sbuf.st_mode))
        return CHECKPATH_DIRECTORY;

    return CHECKPATH_FILE;
}

String getCurrentDirectory  (void)
{
    char temp[512] = "";
    if (getcwd(temp, sizeof(temp)) == NULL)
      return String();
    temp[sizeof(temp)-1] = '\0';
    return String(temp);
}


bool createDir(const char* dir)
{
#if UMBRA_OS == UMBRA_OSX || UMBRA_OS == UMBRA_IOS
    if (mkdir(dir, 0777))
#else
    if (mkdir(dir, S_IRUSR | S_IWUSR | S_IXUSR))
#endif
    {
        int err;
        err = errno;
        if (err != EEXIST)
            return false;
    }
    return true;
}
}

int isMatchingFile(const struct dirent* dir)
{
    if (FNM_NOMATCH != fnmatch(fileMatchPattern,dir->d_name,FNM_PATHNAME))
        return 1;
    else
        return 0;
}

int isMatchingFile(struct dirent* dir)
{
    return isMatchingFile((const struct dirent*)dir);
}

namespace Umbra
{

DirScan::DirScan(const char* path, const char* expr, Allocator* a)
    : Base(a)
{
    m_numFiles = 0;

    UMBRA_ASSERT(path && expr);
    if(!path || ! expr)
        return;

    String p = path ? String(path) : String(".");

    struct dirent **namelist;
    fileMatchPattern = (char*)expr;
    int cnt = scandir(p.toCharPtr(), &namelist, isMatchingFile, alphasort);
    if(cnt < 0)
    {
        std::cerr << "WARNINGS: Couldn't scan directory: " << p.toCharPtr() << std::endl;
    }
    else
    {
        for(int i = 0; i< cnt; i++)
        {
            char* str = UMBRA_NEW_ARRAY(char, strlen(namelist[i]->d_name) + 1);
            strcpy(str, namelist[i]->d_name);
            m_files.pushBack(str);
            m_numFiles++;
            free(namelist[i]);
        }

        free(namelist);
    }

    return;
}

char* DirScan::getCwd(char* buffer, int maxlen)
{
    UMBRA_ASSERT(buffer && maxlen > 0);
    return ::getcwd(buffer, maxlen);
}

int DirScan::chDir(const char* path)
{
    UMBRA_ASSERT(path);
    return ::chdir(path);
}

DirScan::~DirScan()
{
    for(int i=0; i<m_numFiles; i++)
        UMBRA_DELETE_ARRAY(m_files[i]);
}

int DirScan::getNumFiles() const
{
    return m_numFiles;
}

const char* DirScan::getFile(int num) const
{
    UMBRA_ASSERT(num >= 0 && num < m_numFiles);
    return m_files[num];
}

void DirScan::fullPath(char* absPath, const char* relPath, int /*maxLength*/)
{
    UMBRA_ASSERT(absPath && relPath);

    getCwd(absPath, DIRSCAN_MAX_PATH);      // get current working directory
    strcat(absPath, "/");
    strcat(absPath, relPath);
}

Umbra::UINT64 DirScan::getFileSize(const char* p)
{
    struct stat s;
    int ret = stat(p, &s);
    return ret < 0 ? 0 : (UINT64)s.st_size;
}

Umbra::UINT64 DirScan::getFileATime(const char* p)
{
    struct stat s;
    int ret = stat(p, &s);
    // \todo [Hannu] how to handle properly if atime is not supported?
    return ret < 0 ? 0 : (UINT64)max2(s.st_atime, max2(s.st_mtime, s.st_ctime));
}

void DirScan::getFileAttrib(const char* p, Umbra::UINT64& accessTime, Umbra::UINT64& size)
{
    accessTime = getFileATime(p);
    size       = getFileSize(p);
}

void DirScan::removeFile(const char* p)
{
    unlink(p);
}

}

#endif

#if UMBRA_ARCH != UMBRA_SPU
bool Umbra::fileExists(const char* filename)
{
    if (!filename)
        return false;

    FILE* f = fopen(filename, "r");

    if(!f)
        return false;

    fclose(f);
    return true;
}
#endif

namespace Umbra
{

bool createDirRecursive(const char* dir)
{
    UMBRA_UNREF(dir);
#if UMBRA_OS == UMBRA_WINDOWS || UMBRA_IS_POSIX // Not supported on consoles.
    Umbra::Array<Umbra::String> parts = String(dir).pathSplit();
    Umbra::String currentPath;
    for (int i = 0; i < parts.getSize(); i++)
    {
        currentPath += parts[i] + "/";
        if (!createDir(currentPath.toCharPtr()))
            return false;
    }
    return true;
#else
    return false;
#endif
}

void recursiveDelete(const char* path)
{
    UMBRA_UNREF(path);
#if UMBRA_OS == UMBRA_WINDOWS || UMBRA_IS_POSIX // Not supported on consoles.
    // XXX: This would be more robust if it used the filesystem API.
    String cmd;
    String pathStr(path);
    String pathSep("/");
#if UMBRA_OS == UMBRA_WINDOWS
    pathStr.replaceAll("/", "\\");
    pathSep = String("\\");
    cmd += "rd /s /q ";
#else
    pathStr.replaceAll("\\", "/");
    cmd += "rm -rf ";
#endif
    if (!pathStr.startsWith(pathSep))
        pathStr = getCurrentDirectory() + pathSep + pathStr;

    if (checkPath(pathStr) != CHECKPATH_NOT_FOUND)
    {
        cmd += pathStr;
        int retval = system(cmd.toCharPtr());
        UMBRA_UNREF(retval);
    }
#endif
}

}
