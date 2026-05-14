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

#include "umbraPrivateDefs.hpp"
#include "umbraOptimizerTextCommand.hpp"

#include <stdio.h>
#include <string.h>

namespace Umbra
{
    UMBRADEC void* executeOptimizerCommand (const char* cmd);
    char g_debugVisualizationSpec[1024];
}

void* Umbra::executeOptimizerCommand (const char* cmdString)
{
    if (!cmdString)
        return NULL;

    char cmd[64];
    int n = 0;

    if (sscanf(cmdString, "%s%n", cmd, &n) == 0)
        return 0;
    cmdString += n;

    if (strcmp(cmd, "bvh_max_leaf_triangles") == 0)
    {
        if (sscanf(cmdString, "%d", &g_bvhMaxLeafTriangles) == 1)
            return (void*)1;
    }
#if 1
    else if (strcmp(cmd, "reachability_analysis_threshold") == 0)
    {
        if (sscanf(cmdString, "%f", &g_reachabilityAnalysisThreshold) == 1)
            return (void*)1;
    }
#endif
#if 0
    else if (strcmp(cmd, "occlusion_threshold") == 0)
    {
        if (sscanf(cmdString, "%f", &g_occlusionThreshold) == 1)
            return (void*)1;
    }
#endif
#if 0
    else if (strcmp(cmd, "portal_sampling_width") == 0)
    {
        if (sscanf(cmdString, "%d", &g_portalSamplingWidth) == 1)
            return (void*)1;
    }
#endif
    if (strcmp(cmd, "statsheap_assert_on_memory_leaks") == 0)
    {
        if (sscanf(cmdString, "%d", &g_assertOnMemoryLeaks) == 1)
            return (void*)1;
    }
    if (strcmp(cmd, "debug_visualization") == 0)
    {
        strncpy(g_debugVisualizationSpec, cmdString, sizeof(g_debugVisualizationSpec));
    }

    return NULL;
}
