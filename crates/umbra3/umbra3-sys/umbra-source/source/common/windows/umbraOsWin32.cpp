// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraOs.hpp"

#if UMBRA_IS_WIN32

#if UMBRA_OS == UMBRA_XBOX360
#   define NOD3D
#   define NONET
#   include <xtl.h>
#else
#   ifndef WIN32_LEAN_AND_MEAN
#       define WIN32_LEAN_AND_MEAN
#   endif
#   include <windows.h>
#endif

#if defined(WINAPI_FAMILY) && (WINAPI_FAMILY == WINAPI_FAMILY_TV_TITLE)
#include <xdk.h>
#if _XDK_VER < 9260
#   error Older XDK version!
#endif
#endif

namespace Umbra
{

// TODO static Umbra::OS init
static Umbra::UINT64 s_TickBase = 0LL;

double Umbra::OS::getCurrentTime (void)
{
    LARGE_INTEGER freq, count;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&count);
    if (s_TickBase == 0LL)
        s_TickBase = count.QuadPart;
    count.QuadPart -= s_TickBase;
    return (double)count.QuadPart / (double)freq.QuadPart;
}

// Thread local storage

__declspec(thread) static void* tlsStorage;

void* Umbra::OS::tlsGetValue()
{
    return tlsStorage;
}

void Umbra::OS::tlsSetValue(void* value)
{
    tlsStorage = value;
}

}

#endif // UMBRA_IS_WIN32
