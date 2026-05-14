// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Portability.hpp>

//------------------------------------------------------------------------
// Static (compile time) assertion
//------------------------------------------------------------------------

// Does the compiler support standard c++0x static_assert?
#if UMBRA_COMPILER == UMBRA_MSC
#   define HAS_STATIC_ASSERT (_MSC_VER >= 1600)
#elif UMBRA_COMPILER == UMBRA_GCC
#   define HAS_STATIC_ASSERT ((__GNUC__ > 4 || (__GNUC__ == 4 && __GNUC_MINOR__ > 2)) && defined(__GXX_EXPERIMENTAL_CXX0X__))
#elif UMBRA_COMPILER == UMBRA_CLANG
#   define HAS_STATIC_ASSERT __has_feature(cxx_static_assert)
#else
#   define HAS_STATIC_ASSERT 0
#endif

#if HAS_STATIC_ASSERT
#   define UMBRA_CT_ASSERT_MSG(...) static_assert(__VA_ARGS__)
#   define UMBRA_CT_ASSERT(x) static_assert(x, #x)
#else
#if UMBRA_GCC_INTRINSICS
#   define UMBRA_UNUSED __attribute__((unused))
#else
#   define UMBRA_UNUSED
#endif
#define UMBRA_CT_ASSERT(x) \
  struct UMBRA_CONCAT_NAMES(__static_assertion_at_line_, __LINE__) \
  { \
      Umbra::StaticAssertion<static_cast<bool>((x))> UMBRA_CONCAT_NAMES(STATIC_ASSERTION_FAILED_AT_LINE_, __LINE__); \
  }; \
  typedef Umbra::StaticAssertionTest<sizeof(UMBRA_CONCAT_NAMES(__static_assertion_at_line_, __LINE__))> \
  UMBRA_CONCAT_NAMES(__static_assertion_test_at_line_, __LINE__) UMBRA_UNUSED
#define UMBRA_CT_ASSERT_MSG(x, Msg) UMBRA_CT_ASSERT(x)

namespace Umbra
{
    template<bool> struct StaticAssertion;
    template<> struct StaticAssertion<true> {};
    template<int i> struct StaticAssertionTest {};
}
#endif

//------------------------------------------------------------------------
// Runtime assertion
//------------------------------------------------------------------------

// Our runtime assertions are enabled whenever UMBRA_DEBUG is defined. It is
// completely possible to compile with all optimizations and still have
// UMBRA_DEBUG enabled. Code that gets compiled under UMBRA_DEBUG should
// still be reasonably fast, development sanity checking etc should be stripped
// away or disabled permanently in production code.

#if !defined(UMBRA_DEBUG)
#define UMBRA_ASSERT(x) ((void)0)
#define UMBRA_ASSERT_MSG(...) ((void)0)
#else
#undef NDEBUG
#include <assert.h>
#define UMBRA_ASSERT(x) assert(x)
#define UMBRA_ASSERT_MSG(x, Msg) assert((x)&&(Msg))
#endif
