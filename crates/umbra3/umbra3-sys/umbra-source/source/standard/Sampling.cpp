// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include <standard/Sampling.hpp>
#include <standard/Assert.hpp>
#include <cmath>

using namespace Umbra;

Vec3f Umbra::uniformPointOnSphere(const Vec2f& uv)
{
    float u = uv.x();
    float v = uv.y();

    UMBRA_ASSERT(u >= 0.f && u <= 1.f && v >= 0.f && v <= 1.f);
    Vec3f result;
    float theta = u * 2.f * 3.14159265f;
    float z = v * 2.f - 1.f;
    result.z() = z;
    result.x() = std::sqrt(1.f - z*z) * std::cos(theta);
    result.y() = std::sqrt(1.f - z*z) * std::sin(theta);
    return result;
}

// When i goes from 0 to n-1, this function returns all values between 0 and
// n-1 in seemingly random order.

int Umbra::intShuffler(int i, int n)
{
    UMBRA_ASSERT(i >= 0 && i < n);
    UMBRA_ASSERT(n >= 0);

    int r = 0;
    int p = n;

    while (i)
    {
        int m = (p+1) / 2;

        if (i & 1)
        {
            r += m;
            p = p - m;
        }
        else
            p = m;

        i >>= 1;
    }

    UMBRA_ASSERT(r < n);

    return r;
}
