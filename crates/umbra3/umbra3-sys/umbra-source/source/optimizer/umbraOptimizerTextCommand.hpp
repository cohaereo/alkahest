#pragma once
#ifndef UMBRAOPTIMIZERTEXTCOMMAND_HPP
#define UMBRAOPTIMIZERTEXTCOMMAND_HPP

/*!
 *
 * Umbra PVS
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
 * \brief   Text command implementation
 *
 */

namespace Umbra
{

extern float g_computationAABBClamping;
extern float g_occlusionThreshold;
extern int g_bvhMaxLeafTriangles;
extern int g_portalSamplingWidth;
extern float g_reachabilityAnalysisThreshold;
extern int g_assertOnMemoryLeaks;
extern char g_debugVisualizationSpec[1024];

}   // namespace Umbra


#endif // UMBRAOPTIMIZERTEXTCOMMAND_HPP
