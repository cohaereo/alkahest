// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

/*!
 * This is our private version of umbraDefs.hpp.
 */

#if defined(UMBRADEFS_HPP)
#   error Public header included directly instead of apidefs.hpp!
#endif
/* We explicitly don't want to include the public umbraDefs */
#define UMBRADEFS_HPP

#include <umbraVersion.hpp>
#include <standard/Config.hpp>

//------------------------------------------------------------------------
// Public API function declaration macros
//------------------------------------------------------------------------

#if (UMBRA_OS == UMBRA_WINDOWS) && defined(UMBRA_DLLEXPORT)
#   define UMBRADEC __declspec(dllexport)
#elif (UMBRA_OS == UMBRA_WINDOWS) && defined(UMBRA_DLLIMPORT)
#   define UMBRADEC __declspec(dllimport)
#else
#   define UMBRADEC
#endif

#if UMBRA_COMPILER == UMBRA_MSC
#   define UMBRACALL __cdecl
#else
#   define UMBRACALL
#endif

//------------------------------------------------------------------------
// Version
//------------------------------------------------------------------------

#if !defined(UMBRA_BUILD_ID)
#define UMBRA_BUILD_ID 0
#endif

//------------------------------------------------------------------------
// Umbra namespace common types, duplicated from umbraDefs.hpp.
// These should really live in another header file.
//------------------------------------------------------------------------

namespace Umbra
{

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
