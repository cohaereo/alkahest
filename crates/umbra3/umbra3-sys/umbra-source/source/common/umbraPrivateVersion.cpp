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
#include "umbraPrivateVersion.hpp"

namespace Umbra
{

const char* getAPIInfoString(InfoString info)
{
    switch(info)
    {
    case INFOSTRING_VERSION:
        return UMBRA_VERSION;

    case INFOSTRING_COPYRIGHT:
        return UMBRA_COPYRIGHT;

    case INFOSTRING_BUILD_TIME:
        return UMBRA_BUILD_TIME;

    default:
        return "";
    }
}

int getAPIInfoValue(InfoValue info)
{
    switch(info)
    {
    case INFOVALUE_VERSION_MAJOR:
        return UMBRA_VERSION_MAJOR;

    case INFOVALUE_VERSION_MINOR:
        return UMBRA_VERSION_MINOR;

    case INFOVALUE_VERSION_REVISION:
        return UMBRA_VERSION_REVISION;

    default:
        return 0;
    }
}

} // namespace Umbra
