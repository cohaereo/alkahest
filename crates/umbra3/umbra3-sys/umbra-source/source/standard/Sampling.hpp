// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

#include <standard/Vector.hpp>

namespace Umbra
{

Vec3f uniformPointOnSphere(const Vec2f& uv);
int   intShuffler         (int i, int n);

template<int PRIME, typename T>
inline T halton(int k)
{
    int pp = PRIME;
    int kk = k;
    T res = 0;

    while (kk > 0)
    {
        int a = kk % PRIME;
        res += T(a) / pp;
        kk /= PRIME;
        pp *= PRIME;
    }

    return res;
}

template<int PRIME>
static inline float haltonf(int i) { return halton<PRIME, float>(i); }

}
