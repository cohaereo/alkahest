// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraOs.hpp"

#if UMBRA_OS == UMBRA_PS3
#if UMBRA_ARCH == UMBRA_PPC

#include <sys/sys_time.h>

double Umbra::OS::getCurrentTime (void)
{
    return sys_time_get_system_time() * .000001;
}
static __thread void* tlsStorage;

void* Umbra::OS::tlsGetValue()
{
    return tlsStorage;
}

void Umbra::OS::tlsSetValue(void* value)
{
    tlsStorage = value;
}
#elif UMBRA_ARCH == UMBRA_SPU

// Since SPU does not support threads, a global variable works.
static void* fakeTlsStorage = NULL;

void* Umbra::OS::tlsGetValue()
{
    return fakeTlsStorage;
}

void Umbra::OS::tlsSetValue(void* value)
{
    fakeTlsStorage = value;
}

#endif // UMBRA_ARCH == UMBRA_PPC
#endif // UMBRA_OS == UMBRA_PS3_PPU
