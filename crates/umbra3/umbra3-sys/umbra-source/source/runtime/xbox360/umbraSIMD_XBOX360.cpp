/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   SIMD vector instruction wrapper
 *
 */

#include "../umbraSIMD.hpp"

#if (UMBRA_OS == UMBRA_XBOX360) && defined(UMBRA_SIMD_CODE)

namespace Umbra
{

const SIMDRegister UMBRA_SIMD_CONST_XBOX360 = { -0.f, 1.f, -1.f, bitPatternFloat(0x0004080C) };
const SIMDRegister UMBRA_SIMD_PIXEL_MASKS_XBOX360 = SIMDLoad32(0x1, 0x2, 0x4, 0x8);

} // namespace Umbra

#endif
