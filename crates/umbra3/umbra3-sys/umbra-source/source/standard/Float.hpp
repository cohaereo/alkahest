// Copyright (c) 2009-2015 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <cfloat>

namespace Umbra
{
    static inline bool isFloatFinite(float f)
    {
        return f == f && f >= -FLT_MAX && f <= FLT_MAX;
    }

    static inline uint32_t floatAsInt(float f)
    {
        union
        {
            float    f;
            uint32_t ui;
        } cast;

        cast.f = f;
        return cast.ui;
    }

    static inline float intAsFloat(uint32_t ui)
    {
        union
        {
            float    f;
            uint32_t ui;
        } cast;

        cast.ui = ui;
        return cast.f;
    }
}
