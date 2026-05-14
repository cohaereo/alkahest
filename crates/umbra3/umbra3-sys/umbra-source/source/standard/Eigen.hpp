// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#define EIGEN_NO_MALLOC
#define EIGEN_NO_AUTOMATIC_RESIZING
#define EIGEN_MPL2_ONLY
#ifndef UMBRA_DEBUG
#   define EIGEN_NO_DEBUG
#endif
#include "../external/eigen3/Eigen/Core"
#include "../external/eigen3/Eigen/Geometry"