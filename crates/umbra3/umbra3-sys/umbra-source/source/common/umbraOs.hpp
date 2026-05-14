// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef UMBRAOS_HPP
#define UMBRAOS_HPP

#include "umbraPrivateDefs.hpp"

/*!
 * \file    umbraOS.hpp
 * \brief   Umbra operating system abstraction layer
 */

namespace Umbra
{
namespace OS
{

/*!
 * \brief   Get current system time, in seconds
 */
double      getCurrentTime      (void);

/* The simplest possible TLS abstraction - allows storing and
 * retrieving a single pointer. */
/*!
 *  \brief  Get pointer from TLS
 */
void*       tlsGetValue        (void);
/*!
 * \brief   Store pointer to TLS
 */
void        tlsSetValue        (void *value);
}
}

#endif
