// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

/*!
 * Silence select compiler warnings
 */

#include <standard/Config.hpp>

#if UMBRA_COMPILER == UMBRA_MSC
#   pragma warning (disable:4514)  // unreferenced inline function has been removed
#   pragma warning (disable:4710)  // function not inlined
#   pragma warning (disable:4711)  // function selected for inline expansion
#   pragma warning (disable:4714)  // function marked as __forceinline not inlined (TODO!)
#   pragma warning (disable:4725)  // instruction may be inaccurate on some Pentiums (as if we cared)
#   pragma warning (disable:4311)  // pointer truncation
#   pragma warning (disable:4324)  // structure was padded due to __declspec(align())
#   pragma warning (disable:4127)  // conditional expression is constant
#   pragma warning (disable:4447)  // 'main' signature found without threading model
#endif

#if UMBRA_COMPILER == UMBRA_SNC
#   pragma diag_suppress=237       // controlling expression is constant
#endif

#if UMBRA_COMPILER == UMBRA_GHS
#   pragma ghs nowarning 177       // unused variable
#   pragma ghs nowarning 76        // argument to macro is empty (varargs macros)
#   pragma ghs nowarning 1
#endif

#if UMBRA_COMPILER == UMBRA_CLANG
#   pragma GCC diagnostic ignored "-Wunused-private-field"
#endif
