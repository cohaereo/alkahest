#pragma once

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
 * \brief   Tile computation frontend
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraBuildContext.hpp"

namespace Umbra
{

class ImpTileResult;
class ImpTileInput;

class TileProcessor : public BuilderBase
{
public:
    TileProcessor (BuildContext* ctx);
    ~TileProcessor (void);

    ImpTileResult* execute (const ImpTileInput& in);
};

} // namespace Umbra
