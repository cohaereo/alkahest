// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraOs.hpp"

#if UMBRA_OS == UMBRA_PS4

#include <kernel.h>
#include <stdlib.h>
#include <sdk_version.h>

#define UMBRA_ORBIS_SDK_VERSION 0x00911000u

#if SCE_ORBIS_SDK_VERSION < UMBRA_ORBIS_SDK_VERSION
#error Older Orbis SDK version!
#endif

// The current time method is same as POSIX.
double Umbra::OS::getCurrentTime (void)
{
    return (double)sceKernelReadTsc() / (double)sceKernelGetTscFrequency();
}

// Memory allocation config for Sony runtime library. See malloc under
// standard library for ORBIS documentation.
unsigned int sceLibcHeapExtendedAlloc = 1;
size_t       sceLibcHeapSize = SCE_LIBC_HEAP_SIZE_EXTENDED_ALLOC_NO_LIMIT;

// Thread local storage

__thread static void* tlsStorage;

void* Umbra::OS::tlsGetValue()
{
    return tlsStorage;
}

void Umbra::OS::tlsSetValue(void* value)
{
    tlsStorage = value;
}

#endif // UMBRA_PS4
