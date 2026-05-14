// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/IgnoreWarnings.hpp>
#include <standard/Portability.hpp>
#include <standard/Assert.hpp>
#include <standard/IntTypes.hpp>

namespace Umbra
{

//------------------------------------------------------------------------
// Convenience macros
//------------------------------------------------------------------------

#define UMBRA_UNREF(X) /*@-noeffect@*/ ((void)(X))
#define UMBRA_NULL_STATEMENT (static_cast<void>(0))
#define UMBRA_ARRAY_SIZE(x) (int)(sizeof(x) / sizeof(x[0]))
#define UMBRA_EMPTY

#ifdef UMBRA_DEBUG
#   define UMBRA_DEBUG_CODE(X) X
#else
#   define UMBRA_DEBUG_CODE(X)
#endif

//------------------------------------------------------------------------
// Execute a function before the program enters the main function.
//------------------------------------------------------------------------

/**
 * The given expression must be a call of a function that returns a non-void
 * value (the value is discarded). This is used for making compilation units
 * that register data without being explicitly called.
 */
#define UMBRA_EXECUTE_BEFORE_MAIN(expr) static bool UMBRA_CONCAT_NAMES(dummyVar, __LINE__) = ((expr), false)

//------------------------------------------------------------------------
// Properly aligned element storage
//------------------------------------------------------------------------

template<int Size, int Align>
struct AlignedElementMemBase {};

#define ALIGNED_ELEMENT_DECL(Align) \
    template<int Size> struct AlignedElementMemBase<Size,Align> \
    { uint8_t UMBRA_ALIGNED(Align) Bytes[Size]; }

ALIGNED_ELEMENT_DECL(1);
ALIGNED_ELEMENT_DECL(2);
ALIGNED_ELEMENT_DECL(4);
ALIGNED_ELEMENT_DECL(8);
ALIGNED_ELEMENT_DECL(16);
ALIGNED_ELEMENT_DECL(32);

#undef ALIGNED_ELEMENT_DECL

template<typename T>
struct AlignedElementMem: public AlignedElementMemBase<sizeof(T), UMBRA_ALIGNOF(T)> {};

//------------------------------------------------------------------------
// Convenience routines
//------------------------------------------------------------------------

template <class T> static inline const T& min2 (const T& a, const T& b)    { return a<=b ? a : b; }
template <class T> static inline const T& max2 (const T& a, const T& b)    { return a>=b ? a : b; }
template <class T> static inline const T abs2 (const T& a) { return a < 0 ? -a : a; }
template <class T> static inline void swap2 (T& a, T& b)   { T tmp(a); a = b; b = tmp; }


}
