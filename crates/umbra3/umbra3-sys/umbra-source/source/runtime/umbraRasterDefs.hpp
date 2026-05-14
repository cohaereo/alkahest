// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef __UMBRARASTERDEFS_H
#define __UMBRARASTERDEFS_H

#include "umbraPrivateDefs.hpp"

/*-------------------------------------------------------------------*//*!
 * \brief   Size and layout of raster buffers
 *
 * Users: portal raster, occlusion buffer
 *//*-------------------------------------------------------------------*/

// #define UMBRA_PORTAL_LOG_RASTER_SIZE 6
#define UMBRA_PORTAL_LOG_RASTER_SIZE 6

// Note: raster is always square for now
#define UMBRA_PORTAL_RASTER_SIZE (1 << UMBRA_PORTAL_LOG_RASTER_SIZE)
#define UMBRA_HALF_RASTER (UMBRA_PORTAL_RASTER_SIZE * 0.5f)

// Note: portal raster only supports square blocks for now
#define UMBRA_RASTER_BLOCK_X_LOG 2
#define UMBRA_RASTER_BLOCK_Y_LOG 2
#define UMBRA_RASTER_BLOCK_X (1 << UMBRA_RASTER_BLOCK_X_LOG)
#define UMBRA_RASTER_BLOCK_Y (1 << UMBRA_RASTER_BLOCK_Y_LOG)

#endif
