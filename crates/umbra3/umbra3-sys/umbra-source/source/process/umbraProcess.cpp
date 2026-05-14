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
 * \file Entry point for background computation processes.
 *
 */

#include "umbraDefs.hpp"
#include <stdio.h>

namespace Umbra
{
    UMBRADEC void startFromProcess(const char* sharedMemoryId, void* parameters);
}

// TODO: print info text if called improperly? MessageBox under windows?

#if defined(_WIN32) || defined(_WIN64)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

int main (int argc, const char* argv[])
{
    char sharedMemoryId[512];
    HANDLE parentProcess = 0;
    if (argc != 3)
        return 1;

    // parse memory id
    if (argv[1][0] == '\"')
    {        
        if (sscanf_s(argv[1], "\"%[^\"]\"", sharedMemoryId, 512) != 1)
            return 1;
    } else
    {
        if (sscanf_s(argv[1], "%s", sharedMemoryId, 512) != 1)
            return 1;
    }

    // parse parent process handle
    if (sscanf_s(argv[2], "%p", &parentProcess, sizeof(HANDLE)) != 1)
        return 1;

    Umbra::startFromProcess(sharedMemoryId, (void*)&parentProcess);

    return 0;
}
#else
#include <unistd.h>    // getppid
#include <sys/types.h> // pid_t

int main(int argc, const char* argv[])
{
    // executable + memory id == 2 (executable path included)
    // No parent process handle here.
    if (argc != 2)
        return 1;

    // Parse quotes from arqument
    char sharedMemoryId[512];
    int parsed = sscanf(argv[1], "\"%[^\"]\"", sharedMemoryId);

    if (parsed != 1)
        return 1;

    Umbra::startFromProcess(sharedMemoryId, NULL);

    return 0;

}
#endif
