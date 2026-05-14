// Copyright (c) 2009-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRADEFS_HPP
#define UMBRADEFS_HPP

/*!
 * \file
 * \brief     Common type definitions for all classes of Umbra
 */

#ifndef __cplusplus
#   error "C++ compiler required"
#endif

#include "umbraVersion.hpp"
#include <stddef.h>

//------------------------------------------------------------------------
// Fixed-width integer types
//------------------------------------------------------------------------

#if !defined(_MSC_VER) || (_MSC_VER >= 1600)
#   include <stdint.h>
#else
typedef signed __int8     int8_t;
typedef signed __int16    int16_t;
typedef signed __int32    int32_t;
typedef unsigned __int8   uint8_t;
typedef unsigned __int16  uint16_t;
typedef unsigned __int32  uint32_t;
#endif

//------------------------------------------------------------------------
// Define some internal macros affecting how the library is compiled
//------------------------------------------------------------------------

#if defined(_MSC_VER) && defined(UMBRA_DLLIMPORT)
// Note: this is an optional performance optimization when linking against DLL libs
#   define UMBRADEC __declspec(dllimport)
#else
#   define UMBRADEC
#endif

#if defined (_MSC_VER)
#   define UMBRACALL __cdecl
#else
#   define UMBRACALL
#endif

#if defined (_MSC_VER)
#   define UMBRA_ATTRIBUTE_ALIGNED(X,T) __declspec(align(X)) T
#else
#   define UMBRA_ATTRIBUTE_ALIGNED(X,T) __attribute__((aligned(X))) T
#endif

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


//------------------------------------------------------------------------
// Public dummy implementations for vector types in API
//------------------------------------------------------------------------

class Vector2    { public: float v[2];          }; /*!< Default Vector2 implementation */
class Vector2i   { public: int32_t i,j;           }; /*!< Default Vector2i implementation */
class Vector3    { public: float v[3];          }; /*!< Default Vector3 implementation */
class Vector3i   { public: int32_t i,j,k;         }; /*!< Default Vector3i implementation */
class Vector4    { public: float v[4];          }; /*!< Default Vector4 implementation */
class Matrix4x4  { public: float m[4][4];       }; /*!< Default Matrix4x4 implementation */

/*!
 * \brief   The matrix format used when inputting and outputting
 *          matrices through this API
 */
enum MatrixFormat
{
    /*!< column-major matrix format */
    MF_COLUMN_MAJOR = 0,
    /*!< row-major matrix format */
    MF_ROW_MAJOR    = 1
};

/*!
 * \brief   Object triangle winding.
 *
 * \note    default winding is WINDING_CCW
 *
 */
enum TriangleWinding
{
    WINDING_CCW,        /*!< counterclockwise */
    WINDING_CW,         /*!< clockwise */
    WINDING_TWO_SIDED   /*!< double-sided triangles */
};

}

#endif
