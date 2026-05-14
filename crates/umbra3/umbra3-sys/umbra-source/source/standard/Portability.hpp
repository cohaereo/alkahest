// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

/*!
 * Portable compiler keywords (c++ extensions we use)
 */

#include <standard/Config.hpp>

//------------------------------------------------------------------------
// UMBRA_ALIGNED
//------------------------------------------------------------------------

// Use UMBRA_ALIGNED when you need to guarantee that an element is allocated
// at a memory offset greater than that of the natural alignment.
// The preferred way is to use this in the class declaration, like this:
//
// class UMBRA_ALIGNED(16) MyClassThatNeedsAlignment;
//
// This will also, however, work with variable declarations, like this:
//
// Vector4 UMBRA_ALIGNED(16) MyVariableThatNeedsAlignment;

#if UMBRA_COMPILER == UMBRA_MSC
#   define UMBRA_ALIGNED(X) __declspec(align(X))
#else
#   define UMBRA_ALIGNED(X) __attribute__((aligned(X)))
#endif

//------------------------------------------------------------------------
// UMBRA_RESTRICT
//------------------------------------------------------------------------

#if UMBRA_COMPILER == UMBRA_MSC
#   define UMBRA_RESTRICT(T) T __restrict
#elif UMBRA_GCC_INTRINSICS
#   define UMBRA_RESTRICT(T) T __restrict__
#else
#   define UMBRA_RESTRICT(T) T
#endif

//------------------------------------------------------------------------
// UMBRA_ALIGNOF
//------------------------------------------------------------------------

// UMBRA_ALIGNOF returns the natural alignment of a type.

#if UMBRA_COMPILER == UMBRA_GHS
#   define UMBRA_ALIGNOF(t) __alignof__(t)
#else
#   define UMBRA_ALIGNOF(t) __alignof(t)
#endif

//------------------------------------------------------------------------
// UMBRA_FORCE_INLINE
//------------------------------------------------------------------------

// Use UMBRA_FORCE_INLINE when a function absolutely must be inlined at
// all times (for example, so that it compiles correctly). The less forceful
// method is to just use the standard "inline" keyword.

#if UMBRA_COMPILER == UMBRA_MSC
#   define UMBRA_FORCE_INLINE __forceinline
#elif UMBRA_GCC_INTRINSICS
#   define UMBRA_FORCE_INLINE __attribute__((always_inline)) inline
#else
#   define UMBRA_FORCE_INLINE inline
#endif

// deprecated macro that now always means FORCE_INLINE, use plain inline
// if you don't actually want force inlining
#define UMBRA_INLINE UMBRA_FORCE_INLINE

//------------------------------------------------------------------------
// UMBRA_EXPECT
//------------------------------------------------------------------------

#if UMBRA_GCC_INTRINSICS
#define UMBRA_EXPECT(expr, result) __builtin_expect(expr, result)
#else
#define UMBRA_EXPECT(expr, result) expr
#endif

//------------------------------------------------------------------------
// Preprocessor string utils (these don't quite belong here?)
//------------------------------------------------------------------------

// Generate a new token that concatenates two tokens together. Eg. UMBRA_CONCAT_NAMES(Foo, Bar) -> FooBar
#define UMBRA_CONCAT_NAMES(a, b) UMRBA_CONCAT_NAMES_HELP(a, b)
#define UMRBA_CONCAT_NAMES_HELP(x, y) x##y
#define UMBRA_STRINGIFY(x) UMBRA_STRINGIFY_HELP(x)
#define UMBRA_STRINGIFY_HELP(x) #x
