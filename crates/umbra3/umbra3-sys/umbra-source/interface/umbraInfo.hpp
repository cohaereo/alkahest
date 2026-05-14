// Copyright (c) 2009-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRAINFO_HPP
#define UMBRAINFO_HPP

#include "umbraDefs.hpp"

namespace Umbra
{

/*!
 * \file
 * \brief     Library information API
 */

enum InfoString
{
    INFOSTRING_VERSION              = 0,    /*!< version string of the library (format: major.minor.build, "3.1.2") */
    INFOSTRING_COPYRIGHT            = 1,    /*!< library copyright info string */
    INFOSTRING_BUILD_TIME           = 2     /*!< library build date & time */
};

enum InfoValue
{
    INFOVALUE_VERSION_MAJOR         = 0,    /*!< version format: major.minor.revision */
    INFOVALUE_VERSION_MINOR         = 1,
    INFOVALUE_VERSION_REVISION      = 2
};

/*!
 * \brief   Optimizer library info getters
 */

UMBRADEC const char*    getOptimizerInfoString    (InfoString);
UMBRADEC int            getOptimizerInfoValue     (InfoValue);

/*!
 * \brief   Runtime library info getters (always statically linked)
 */

const char*             getRuntimeInfoString    (InfoString);
int                     getRuntimeInfoValue     (InfoValue);

}
#endif
