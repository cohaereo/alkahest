#ifndef UMBRADIRSCAN_HPP
#define UMBRADIRSCAN_HPP

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
 * \brief   Directory scanning class
 *
 */

#if !defined(UMBRAARRAY_HPP)
#   include "umbraArray.hpp"
#endif
#include "umbraString.hpp"

namespace Umbra
{

enum CheckPathResult
{
    CHECKPATH_NOT_FOUND = 0,
    CHECKPATH_FILE,      // Path points to a file
    CHECKPATH_DIRECTORY  // Path is a directory
};

String canonicalPath(const String& path);

String absolutePath(const String& filename);
String relativePath(const String& relativeTo, const String& filename);

String joinPaths(const String& a, const String& b);

CheckPathResult checkPath   (const char* path);
inline CheckPathResult checkPath   (const String& filename)    { return checkPath(filename.toCharPtr()); }

bool fileExists             (const char* filename);
inline bool fileExists             (const String& filename)    { return fileExists(filename.toCharPtr()); }

String getCurrentDirectory  (void);

bool createDir(const char*);

bool createDirRecursive(const char*);

void recursiveDelete(const char*);

/*-------------------------------------------------------------------*//*!
 * \brief           Scans a directory for a specific file mask.
 *
 * \note            Return file names only (no path information).
 *//*-------------------------------------------------------------------*/

class DirScan : public Base
{
public:
                        DirScan         (const char* path, const char* expr, Allocator* a = NULL);
                        ~DirScan        (void);

    int                 getNumFiles     (void) const;
    const char*         getFile         (int num) const;
    static char*        getCwd          (char* str, int maxLength);

    static Umbra::UINT64 getFileSize    (const char*);
    static Umbra::UINT64 getFileATime   (const char*);
    static void          getFileAttrib  (const char*, Umbra::UINT64& accessTime, Umbra::UINT64& size);
    static void          removeFile     (const char*);
private:
    enum
    {
        DIRSCAN_MAX_PATH        = 256
    };

    static int          chDir           (const char* path);
    static void         fullPath        (char* absPath, const char* relPath, int maxLength);

    int                 m_numFiles;
    Array<char*>        m_files;
};

} // namespace Umbra

#endif // UMBRADIRSCAN_HPP
