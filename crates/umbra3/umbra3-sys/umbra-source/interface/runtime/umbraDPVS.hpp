// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRADPVS_HPP
#define UMBRADPVS_HPP

/*!
 * \file
 * \brief   Directional PVS interface
 */

#include "umbraDefs.hpp"

#define UMBRA_DPVS_SIZE 16*1024 + 32

namespace Umbra
{

//------------------------------------------------------------------------

class DPVS
{
public:

    enum ErrorCode
    {
        ERRORCODE_OK,
        ERRORCODE_GENERIC_ERROR,
        ERRORCODE_LOOKUP_FAILED,
        ERRORCODE_BAD_ALIGN
    };

public:

    DPVS            (void);

    ErrorCode  init            (const uint8_t* inBuffer, int inBufferSize);
    ErrorCode  lookup          (class IndexList* outIndexList, float inTime) const;
    int        getListCapacity (void) const;

private:

                        DPVS            (const DPVS&);
                        DPVS& operator= (const DPVS&);

    uint8_t             m_mem[UMBRA_DPVS_SIZE];
};

//------------------------------------------------------------------------

} // namespace Umbra

#endif // UMBRADPVS_HPP
