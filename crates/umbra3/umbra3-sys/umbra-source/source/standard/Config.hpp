// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#include <stddef.h>

/*!
 * In situations where it is necessary to know something about the
 * build or execution environment, always use the UMBRA_OS, UMBRA_COMPILER
 * and UMBRA_ARCH macros provided here. There are additionally some
 * UMBRA_IS_XXX convenience macros that can be used to specify aspects
 * of the operating environment for specific use cases.
 *
 * The environment definition macros expand to a non-zero named integer
 * that is used to specify a single known value. Testing for a particular
 * environment is done like this:
 *
 *      #if UMBRA_OS == UMBRA_WINDOWS
 *
 * With this syntax there is no chance of mixing up code intended for
 * different environments (compare against #ifdef UMBRA_OS_WINDOWS) and
 * compiler warnings for undefined macro expansions can be used to detect
 * typos in the conditionals.
 */

//------------------------------------------------------------------------
// Generic "unknown" value for OS, COMPILER, ARCH etc
//------------------------------------------------------------------------

#define UMBRA_UNKNOWN 0

//------------------------------------------------------------------------
// UMBRA_OS
//
// If not given from the outside, we try to detect the operating system
// from compiler preprocessor defines. The detection routine intentionally
// tries to match all of the supported operating systems and will generate
// an error if there are multiple matches. Do not assume a given compiler
// or architecture based on the operating environment.
//------------------------------------------------------------------------

#define UMBRA_WINDOWS    1
#define UMBRA_LINUX      2
#define UMBRA_OSX        3
#define UMBRA_XBOX360    4
#define UMBRA_PS3        5
#define UMBRA_PSVITA     7
#define UMBRA_IOS        8
#define UMBRA_METRO      9
#define UMBRA_CAFE       10
#define UMBRA_PS4        11
#define UMBRA_ANDROID    12
#define UMBRA_XBOXONE    13
#define UMBRA_NACL       14

#if !defined(UMBRA_OS)

#if defined(_WIN32)
#   if defined(_XBOX_VER)
#       if !defined(_XBOX)
#           error XDK requires _XBOX to be defined!
#       endif
#       if _XBOX_VER < 200
#           error Original XBOX not supported!
#       endif
#       define UMBRA_OS UMBRA_XBOX360
#   elif defined(_XBOX_ONE)
#       define UMBRA_OS UMBRA_XBOXONE
#   elif defined(WINAPI_FAMILY)
#       if WINAPI_FAMILY == WINAPI_FAMILY_DESKTOP_APP
#           define UMBRA_OS UMBRA_WINDOWS
#       else
#           define UMBRA_OS UMBRA_METRO
#       endif
#   else
#       define UMBRA_OS UMBRA_WINDOWS
#   endif
#endif
#if defined(__ANDROID__) || defined(ANDROID)
#   define UMBRA_OS UMBRA_ANDROID
#elif defined(__linux__) || defined(__QNXNTO__) // TODO: add real support for QNX once we have build environment
#   define UMBRA_OS UMBRA_LINUX
#endif
#if defined(__APPLE__)
#   include "TargetConditionals.h"
#   include "AvailabilityMacros.h"
#   if (TARGET_OS_IPHONE || TARGET_IPHONE_SIMULATOR)
#       define UMBRA_OS UMBRA_IOS
#   else
#       define UMBRA_OS UMBRA_OSX
#   endif
#endif
#if defined(__PS3__) || defined(SN_TARGET_PS3)
#   define UMBRA_OS UMBRA_PS3
#endif
#if defined(__psp2__)
#   define UMBRA_OS UMBRA_PSVITA
#endif
#if defined(__ORBIS__)
#   define UMBRA_OS UMBRA_PS4
#endif
#if defined(CAFE)
#   define UMBRA_OS UMBRA_CAFE
#endif
#if defined(__native_client__)
#   define UMBRA_OS UMBRA_NACL
#endif
#endif

#if !defined(UMBRA_OS)
#warning UMBRA_OS was not automatically detected!
#define UMBRA_OS UMBRA_UNKNOWN
#endif

//------------------------------------------------------------------------
// UMBRA_COMPILER
//------------------------------------------------------------------------

#define UMBRA_MSC     1
#define UMBRA_GCC     2
#define UMBRA_SNC     3
#define UMBRA_GHS     4
#define UMBRA_CLANG   5

#if !defined(UMBRA_COMPILER)
#if defined (_MSC_VER)
#   define UMBRA_COMPILER UMBRA_MSC
#endif
#if defined (__clang__)
#   define UMBRA_COMPILER UMBRA_CLANG
#endif
#if defined (__GNUC__) && !defined(__clang__) && !defined(__ghs__)
#   define UMBRA_COMPILER UMBRA_GCC
#endif
#if defined(__SNC__)
#   define UMBRA_COMPILER UMBRA_SNC
#endif
#if defined(__ghs__)
#   define UMBRA_COMPILER UMBRA_GHS
#endif
#endif

#if !defined(UMBRA_COMPILER)
#warning UMBRA_COMPILER was not automatically detected!
#define UMBRA_COMPILER UMBRA_UNKNOWN
#endif

//------------------------------------------------------------------------
// UMBRA_ARCH
//------------------------------------------------------------------------

// Supported architectures

#define UMBRA_X86   1
#define UMBRA_PPC   2
#define UMBRA_ARM   3
#define UMBRA_SPU   4

#if !defined(UMBRA_ARCH)
#if defined(_M_IX86) || defined(_M_X64) || defined(i386) || defined(__i386__) || defined(__amd64__)
#   define UMBRA_ARCH UMBRA_X86
#endif
#if defined(__arm__) || defined(__thumb__) || defined(_M_ARM) || defined(__arm64__)
#   define UMBRA_ARCH UMBRA_ARM
#endif
#if defined(_M_PPC) || defined(__ppc__) || defined(__PPC__)
#   define UMBRA_ARCH UMBRA_PPC
#endif
#if defined(__SPU__)
#   define UMBRA_ARCH UMBRA_SPU
#endif
#endif

#if !defined(UMBRA_ARCH)
#warning UMBRA_ARCH was not automatically detected!
#define UMBRA_ARCH UMBRA_UNKNOWN
#endif

//------------------------------------------------------------------------
// UMBRA_BYTE_ORDER
//------------------------------------------------------------------------

#define UMBRA_LITTLE_ENDIAN 1
#define UMBRA_BIG_ENDIAN 2

#if !defined(UMBRA_BYTE_ORDER)
#if (UMBRA_ARCH == UMBRA_PPC) || (UMBRA_ARCH == UMBRA_SPU)
#define UMBRA_BYTE_ORDER UMBRA_BIG_ENDIAN
#else
#define UMBRA_BYTE_ORDER UMBRA_LITTLE_ENDIAN
#endif
#endif

//------------------------------------------------------------------------
// Are C++ exceptions supported?
//------------------------------------------------------------------------

#if !defined(UMBRA_EXCEPTIONS_SUPPORTED)
#define UMBRA_EXCEPTIONS_SUPPORTED ((UMBRA_OS == UMBRA_WINDOWS) || (UMBRA_OS == UMBRA_LINUX) || (UMBRA_OS == UMBRA_OSX))
#endif

//------------------------------------------------------------------------
// UMBRA_CODE_ANALYZER (are we being compiled fot static analysis?)
//------------------------------------------------------------------------

// Visual Studio 2008
#if UMBRA_COMPILER == UMBRA_MSC && _MSC_VER >= 1500 && _MSC_VER < 1600
template<int x = __COUNTER__> struct umbra_increment_counter{};
#   if __COUNTER__ < 1 // detect intellisense
#       define UMBRA_CODE_ANALYZER 1
#   endif
// Visual Studio 2010 -
#elif UMBRA_COMPILER == UMBRA_MSC && _MSC_VER >= 1600
#   if defined(__INTELLISENSE__)
#       define UMBRA_CODE_ANALYZER 1
#   endif
#endif

#if !defined(UMBRA_CODE_ANALYZER)
#   define UMBRA_CODE_ANALYZER 0
#endif

//------------------------------------------------------------------------
// Operating environment helpers
//------------------------------------------------------------------------

#if !defined(UMBRA_IS_POSIX)
#   define UMBRA_IS_POSIX (UMBRA_OS == UMBRA_OSX || \
                           UMBRA_OS == UMBRA_IOS || \
                           UMBRA_OS == UMBRA_LINUX || \
                           UMBRA_OS == UMBRA_ANDROID)
#endif
#if !defined(UMBRA_IS_WIN32)
#   define UMBRA_IS_WIN32 (UMBRA_OS == UMBRA_WINDOWS || \
                           UMBRA_OS == UMBRA_XBOX360 || \
                           UMBRA_OS == UMBRA_METRO || \
                           UMBRA_OS == UMBRA_XBOXONE)
#endif
#if !defined(UMBRA_IS_TARGET)
#   define UMBRA_IS_TARGET (UMBRA_OS == UMBRA_XBOX360 || \
                            UMBRA_OS == UMBRA_PS3     || \
                            UMBRA_OS == UMBRA_IOS     || \
                            UMBRA_OS == UMBRA_PSVITA  || \
                            UMBRA_OS == UMBRA_CAFE    || \
                            UMBRA_OS == UMBRA_PS4     || \
                            UMBRA_OS == UMBRA_METRO   || \
                            UMBRA_OS == UMBRA_XBOXONE || \
                            UMBRA_OS == UMBRA_ANDROID)
#endif
#if !defined(UMBRA_GCC_INTRINSICS)
#define UMBRA_GCC_INTRINSICS (UMBRA_COMPILER == UMBRA_GCC || \
                              UMBRA_COMPILER == UMBRA_SNC || \
                              UMBRA_COMPILER == UMBRA_CLANG)
#endif

//------------------------------------------------------------------------
// Code optimization hints
//------------------------------------------------------------------------

// Optimizations that sacrifice code size substantially
#if !defined(UMBRA_OPT_LARGE_FOOTPRINT)
#   define UMBRA_OPT_LARGE_FOOTPRINT (UMBRA_ARCH != UMBRA_SPU)
#endif
// Avoid branching at all cost
#if !defined(UMBRA_OPT_AVOID_BRANCHES)
#   define UMBRA_OPT_AVOID_BRANCHES ((UMBRA_ARCH == UMBRA_PPC) || UMBRA_ARCH == UMBRA_SPU)
#endif
