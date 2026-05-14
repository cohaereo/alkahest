// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRAPRIVATEDEFS_HPP
#define UMBRAPRIVATEDEFS_HPP

#include <standard/Base.hpp>

#undef UMBRA_COMP_NO_EXCEPTIONS
#if !UMBRA_EXCEPTIONS_SUPPORTED
#   define UMBRA_COMP_NO_EXCEPTIONS
#endif

//------------------------------------------------------------------------
// Include compiler intrinsic declarations
//------------------------------------------------------------------------

#if UMBRA_OS == UMBRA_XBOX360
#   include <ppcintrinsics.h>
#elif UMBRA_IS_WIN32 && (UMBRA_COMPILER == UMBRA_MSC)
#   include <math.h>    // must be included before intrin.h
#   include <intrin.h>
// make sure we don't break Unity build with un-namespaced UINT32s
// \todo remove this and rename our own types to something unambiguous
// in the next major version update!
#   include <BaseTsd.h>
#elif UMBRA_IS_WIN32 && (UMBRA_COMPILER == UMBRA_GCC)
#   include <xmmintrin.h>
#elif UMBRA_ARCH == UMBRA_X86 && UMBRA_OS != UMBRA_IOS
#   if defined(__clang__)
#       include <x86intrin.h>
#   else
#   if (UMBRA_OS != UMBRA_NACL) // \todo remove once compiling with -msse2
#       include <emmintrin.h>
#   endif
#   endif
#endif

//------------------------------------------------------------------------
// Data alignment
//------------------------------------------------------------------------

#define UMBRA_ATTRIBUTE_ALIGNED(X,T) UMBRA_ALIGNED(X) T
#define UMBRA_ATTRIBUTE_ALIGNED8(T) UMBRA_ALIGNED(8) T
#define UMBRA_ATTRIBUTE_ALIGNED16(T) UMBRA_ALIGNED(16) T
#define UMBRA_ATTRIBUTE_ALIGNED32(T) UMBRA_ALIGNED(32) T

#ifndef UMBRA_CACHE_LINE_SIZE
#   define UMBRA_CACHE_LINE_SIZE 32 // this is conservative and valid assumption on modern hardware
#endif

#define UMBRA_ALIGN(x, a) (((Umbra::UINTPTR)(x) + (a) - 1) & ~((Umbra::UINTPTR)(a) - 1))
#define UMBRA_ALIGN_INT(x) UMBRA_ALIGN(x, sizeof(int))

//------------------------------------------------------------------------
// Prefetch macros
//------------------------------------------------------------------------

#if UMBRA_ARCH == UMBRA_X86 && !defined(__flash__) && (UMBRA_OS != UMBRA_NACL)
#   define UMBRA_PREFETCH(x) _mm_prefetch((const char*)(x), _MM_HINT_T0) // Pull into L1 cache
#   define UMBRA_PREFETCH_RANGE(x, y) UMBRA_PREFETCH(x)
#elif UMBRA_OS == UMBRA_XBOX360
#   define UMBRA_PREFETCH(x) __dcbt(0, x)
#   define UMBRA_PREFETCH_RANGE(x, y) UMBRA_PREFETCH(x)
#else
#   define UMBRA_PREFETCH(x)
#   define UMBRA_PREFETCH_RANGE(x, y)
#endif

//------------------------------------------------------------------------
// Utilities in the Umbra namespace
//------------------------------------------------------------------------

namespace Umbra
{

//------------------------------------------------------------------------
// Portable definitions of basic memory types for backwards compatibility
//------------------------------------------------------------------------

typedef uint8_t             UINT8;            /*!< 8-bit unsigned integer */
typedef int8_t              INT8;             /*!< 8-bit signed integer */
typedef int16_t             INT16;            /*!< 16-bit signed integer */
typedef uint16_t            UINT16;           /*!< 16-bit unsigned integer */
typedef int32_t             INT32;            /*!< 32-bit signed integer */
typedef uint32_t            UINT32;           /*!< 32-bit unsigned integer */
typedef int64_t             INT64;            /*!< 64-bit signed integer */
typedef uint64_t            UINT64;           /*!< 64-bit unsigned integer */
typedef uintptr_t           UINTPTR;          /*!< unsigned integer for storing pointer value */

//------------------------------------------------------------------------
// Names of things
//------------------------------------------------------------------------

static UMBRA_INLINE const char* getOsName (void)
{
    switch (UMBRA_OS)
    {
#define RETURN_OSNAME(x) case UMBRA_ ## x: return #x;
    RETURN_OSNAME(WINDOWS);
    RETURN_OSNAME(LINUX);
    RETURN_OSNAME(OSX);
    RETURN_OSNAME(XBOX360);
    RETURN_OSNAME(PS3);
    RETURN_OSNAME(PSVITA);
    RETURN_OSNAME(IOS);
    RETURN_OSNAME(METRO);
    RETURN_OSNAME(CAFE);
    RETURN_OSNAME(PS4);
    RETURN_OSNAME(ANDROID);
    RETURN_OSNAME(XBOXONE);
    RETURN_OSNAME(NACL);
#undef RETURN_OSNAME
    }
    return "Unknown";
}

//------------------------------------------------------------------------
// Align utils
//------------------------------------------------------------------------

UMBRA_FORCE_INLINE bool isDWordAligned (const void* p) { return !(reinterpret_cast<UINTPTR>(p)&3);     }
UMBRA_FORCE_INLINE bool isQWordAligned (const void* p) { return !(reinterpret_cast<UINTPTR>(p)&7);     }
UMBRA_FORCE_INLINE bool is128Aligned   (const void* p) { return !(reinterpret_cast<UINTPTR>(p)&15);    }

template <class T> UMBRA_FORCE_INLINE T* alignPow2      (T* ptr, int ALIGN) { return reinterpret_cast<T*>(UMBRA_ALIGN(ptr, ALIGN)); }
template <class T> UMBRA_FORCE_INLINE T* alignCacheLine (T* ptr)            { return alignPow2(ptr, UMBRA_CACHE_LINE_SIZE);   }
template <class T> UMBRA_FORCE_INLINE T* align128       (T* ptr)            { return alignPow2(ptr, 16);                }

//------------------------------------------------------------------------
// Default hash functions
//------------------------------------------------------------------------

// Prototype for hash functions
template <class Key> inline UINT32 getHashValue (const Key& k);

// A few hash value specializations

template <> inline UINT32 getHashValue (const float& s)
{
    union {
        UINT32 ui;
        float  f;
    } value;
    value.f = s;
    if (value.f == -0.f) value.f = 0.f;
    return ((value.ui>>22)+(value.ui>>12)+(value.ui));
}

template <> inline UINT32 getHashValue (const unsigned int& s)
{
    UINT32 hval = s;
    hval = hval + (hval>>5) + (hval>>10) + (hval>>20);
    return hval;
}

template <> inline UINT32 getHashValue (const int& s)
{
    return getHashValue<unsigned int>(s);
}

template <typename T> inline UINT32 getHashValue (T* const & s)
{
    UINTPTR ptr = (UINTPTR)s;
    ptr >>= 3;
    UINT32 hval = (UINT32)(ptr + (ptr>>5) + (ptr>>10) + (ptr>>20) + (ptr>>25));
    return hval;
}

template <typename T> inline UINT32 getHashValue (const T* const & s)
{
    UINTPTR ptr = (UINTPTR)s;
    ptr >>= 3;
    UINT32 hval = (UINT32)(ptr + (ptr>>5) + (ptr>>10) + (ptr>>20) + (ptr>>25));
    return hval;
}

template <typename T> inline void swap (T& a, T& b)
{
    T t = a;
    a = b;
    b = t;
}

UMBRA_FORCE_INLINE int getByteOrder(void)
{
    union {
        UINT32 i;
        UINT8 c[4];
    } testval = {0x01020304};

    return testval.c[0] == 1 ? UMBRA_BIG_ENDIAN : UMBRA_LITTLE_ENDIAN;
}

UMBRA_FORCE_INLINE UINT32 swapBytes_4 (void* addr)
{
    UINT32 val = *(UINT32*)addr;
    return (val >> 24) | ((val >> 8) & 0xFF00) | ((val << 8) & 0xFF0000) | (val << 24);
}

UMBRA_FORCE_INLINE UINT64 swapBytes_8 (void* addr)
{
    UINT64 val = *(UINT64*)addr;
    return ((val >> 56) & 0x00000000000000FFULL) | ((val >> 40) & 0x000000000000FF00ULL) |
           ((val >> 24) & 0x0000000000FF0000ULL) | ((val >>  8) & 0x00000000FF000000ULL) |
           ((val <<  8) & 0x000000FF00000000ULL) | ((val << 24) & 0x0000FF0000000000ULL) |
           ((val << 40) & 0x00FF000000000000ULL) | ((val << 56) & 0xFF00000000000000ULL);
}

typedef union FloatUInt_u
{
    float  f;
    UINT32 i;
} FloatUInt;

static UMBRA_FORCE_INLINE UINT32 floatBitPattern(float f)
{
    FloatUInt fi;
    fi.f = f;
    return fi.i;
}

static UMBRA_FORCE_INLINE float bitPatternFloat (UINT32 i)
{
    FloatUInt fi;
    fi.i = i;
    return fi.f;
}

static UMBRA_FORCE_INLINE int floatSignBit(const float& f)
{
	return (floatBitPattern(f) & 0x80000000) >> 31;
}

//------------------------------------------------------------------------
// Forward declarations
//------------------------------------------------------------------------

class Vector2i;
class Vector3i;
class Vector2;
class Vector3;
class Vector4;
class Matrix4x3;
class Matrix4x4;

//------------------------------------------------------------------------
// Legacy support, should be removed
//------------------------------------------------------------------------



typedef INT8     int8;
typedef UINT8    uint8;
typedef INT16    int16;
typedef UINT16   uint16;
typedef INT32    int32;
typedef UINT32   uint32;
typedef INT64    int64;
typedef UINT64   uint64;
typedef UINTPTR  uptr;

/** \brief Enumeration of the faces of a cube */
enum Face
{
    NEGATIVE_X  = 0,
    POSITIVE_X  = 1,
    NEGATIVE_Y  = 2,
    POSITIVE_Y  = 3,
    NEGATIVE_Z  = 4,
    POSITIVE_Z  = 5
};

/** \brief Enumeration of 3D axes */
enum Axis
{
    AXIS_X = 0,
    AXIS_Y = 1,
    AXIS_Z = 2
};

// Utilities for construction / destruction.

template<typename T> inline void callDestructor(T* p)
{
    UMBRA_UNREF(p);
    p->~T();
}

template<typename T> inline T* newArray(void* ptr, int n)
{
#if defined(UMBRA_COMP_NO_EXCEPTIONS)
        if (!ptr)
            return NULL;
#endif
    int* iptr = (int*)ptr;
    *iptr = n;
    T* t = (T*)(iptr+4);    // +4 instead of +1 not to break 16 byte alignment
    for (int i = 0; i < n; i++)
        new (&t[i]) T;
    return t;
}

template<typename T> inline void* deleteArray(T* t)
{
    UMBRA_ASSERT(t);
    int32* ptr = (int32*)t - 4;
    int32 n = *ptr;
    UMBRA_ASSERT(n >= 0);
    for (int i = 0; i < n; i++)
        t[i].~T();
    return ptr;
}

template<typename T> inline void* allocThrow(T* heap, size_t size, const char* info)
{
    return heap->allocate(size, info);
}

template<typename T> inline void* allocThrowAligned(T* heap, size_t size, UINT32 alignment, const char* info)
{
    // room for alignment + HEADER
    size += alignment + sizeof(UINT32);
    UINT8* buf = (UINT8*)heap->allocate(size, info);
    if (!buf)
        return NULL;
    UINT8* aligned = (UINT8*)UMBRA_ALIGN((UINTPTR)(buf + sizeof(UINT32)), alignment);
    UINT32* header = ((UINT32*)aligned) - 1;
    *header = (UINT32)((UINT8*)aligned - buf);
    return aligned;
}

template<typename T> inline void freeAligned(T* heap, void* ptr)
{
    if (!ptr)
        return;
    UINT32 ofs = *((UINT32*)ptr - 1);
    UINT8* buf = (UINT8*)ptr - ofs;
    heap->deallocate(buf);
}

} // namespace Umbra

#define buildFace(axis, direction)  ((Umbra::Face)(((axis) << 1) | ((direction) & 1)))
#define getFaceAxis(face)           ((Umbra::Axis)(((face) >> 1) & 3))
#define getFaceDirection(face)      ((face & 1))
#define getFaceDirectionSign(face)  ((getFaceDirection(face) << 1) - 1)

#ifdef UMBRA_DEBUG
#define UMBRA_ALLOCINFO __FILE__ ":" UMBRA_STRINGIFY(__LINE__)
#else
#define UMBRA_ALLOCINFO NULL
#endif

#define UMBRA_HEAP_ALLOC(heap, size)            allocThrow((heap), size, UMBRA_ALLOCINFO)
#define UMBRA_HEAP_ALLOC_16(heap, size)         allocThrowAligned((heap), size, 16, UMBRA_ALLOCINFO)
#define UMBRA_MALLOC(size)                      UMBRA_HEAP_ALLOC(getAllocator(), size)
#define UMBRA_HEAP_NEW(heap, C, ...)            (new (UMBRA_HEAP_ALLOC(heap, sizeof(C))) C (__VA_ARGS__))
#define UMBRA_NEW(C, ...)                       UMBRA_HEAP_NEW(getAllocator(), C, __VA_ARGS__)
#define UMBRA_HEAP_NEW_ARRAY(heap, C, n)        (newArray<C>(UMBRA_HEAP_ALLOC(heap, 4*sizeof(int) + sizeof(C)*(n)), (int)(n)))
#define UMBRA_HEAP_NEW_ARRAY_NOINIT(heap, C, n) (UMBRA_HEAP_ALLOC(heap, 4*sizeof(int) + sizeof(C)*(n)))
#define UMBRA_NEW_ARRAY(C, n)                   UMBRA_HEAP_NEW_ARRAY(getAllocator(), C, n)

#define UMBRA_HEAP_ALLOC_NOTHROW(heap, size)    ((heap)->allocate(size, UMBRA_ALLOCINFO))
#define UMBRA_MALLOC_NOTHROW(size)              UMBRA_HEAP_ALLOC_NOTHROW(getAllocator(), size)
// On some platforms, placement new crashes with NULL
#define UMBRA_HEAP_NEW_NOTHROW(out, heap, C, ...) do { \
        void* __ptr__ = UMBRA_HEAP_ALLOC_NOTHROW(heap, sizeof(C)); \
        if (__ptr__) (out) = new (__ptr__) C (__VA_ARGS__); \
        else (out) = NULL; \
    } while(false)

#define UMBRA_HEAP_FREE(heap, p)                (heap)->deallocate(p)
#define UMBRA_HEAP_FREE_16(heap, p)             freeAligned((heap), p)
#define UMBRA_FREE(p)                           UMBRA_HEAP_FREE(getAllocator(), p)
#define UMBRA_HEAP_DELETE(heap, p)              { if (p) { callDestructor(p); UMBRA_HEAP_FREE(heap, p); } }
#define UMBRA_HEAP_DELETE2(heap, C, p)          { if (p) { (p)->~C(); UMBRA_HEAP_FREE(heap, p); } }
#define UMBRA_DELETE(p)                         UMBRA_HEAP_DELETE(getAllocator(), p)
#define UMBRA_HEAP_DELETE_ARRAY(heap, p)        { if (p) UMBRA_HEAP_FREE(heap, deleteArray(p)); }
#define UMBRA_DELETE_ARRAY(p)                   UMBRA_HEAP_DELETE_ARRAY(getAllocator(), p)

//--------------------------------------------------------------------
#endif // UMBRAPRIVATEDEFS_HPP

