#ifndef __UMBRAVERSION_HPP
#define __UMBRAVERSION_HPP

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra Version
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraInfo.hpp"

// Concatenate into version string
#define UMBRA_VERSION_MAKE_STR(ma,mi,rev,sta,bu) (UMBRA_STRINGIFY(ma) "." UMBRA_STRINGIFY(mi) "." UMBRA_STRINGIFY(rev) " " sta " (build: " UMBRA_STRINGIFY(bu) ")\0")

// Version number as a string
#define UMBRA_VERSION           UMBRA_VERSION_MAKE_STR(UMBRA_VERSION_MAJOR, UMBRA_VERSION_MINOR, UMBRA_VERSION_REVISION, UMBRA_VERSION_STATUS, UMBRA_BUILD_ID)
#define UMBRA_COPYRIGHT         "2007-2014 Umbra Software Ltd. All Rights Reserved.\0"
#define UMBRA_BUILD_TIME        __DATE__ " " __TIME__

namespace Umbra
{
    // Can't put the public info API functions here since in MSVC you
    // can't dllexport a function from static lib without .def file.
    // Use umbraVersionAPI.cpp to declare public API wrappers.
    const char*     getAPIInfoString   (InfoString info);
    int             getAPIInfoValue    (InfoValue info);
}

#endif // __UMBRAVERSION_HPP
