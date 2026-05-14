// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

// We use standard exact-width integer type defs (a la uint32_t),
// usually defined in stdint.h

#include <standard/Config.hpp>

#if UMBRA_COMPILER == UMBRA_MSC
#   define HAS_STDINT_H (_MSC_VER >= 1600)
#else
#   define HAS_STDINT_H 1
#endif

#if HAS_STDINT_H
#include <stdint.h>
#elif UMBRA_COMPILER == UMBRA_MSC
#ifndef _W64
#  if !defined(__midl) && (defined(_X86_) || defined(_M_IX86)) && _MSC_VER >= 1300
#     define _W64 __w64
#  else
#     define _W64
#  endif
#endif
typedef signed __int8     int8_t;
typedef signed __int16    int16_t;
typedef signed __int32    int32_t;
typedef unsigned __int8   uint8_t;
typedef unsigned __int16  uint16_t;
typedef unsigned __int32  uint32_t;
typedef signed __int64    int64_t;
typedef unsigned __int64  uint64_t;
#ifdef _WIN64
   typedef signed __int64    intptr_t;
   typedef unsigned __int64  uintptr_t;
#else
   typedef _W64 signed int   intptr_t;
   typedef _W64 unsigned int uintptr_t;
#endif
#endif
