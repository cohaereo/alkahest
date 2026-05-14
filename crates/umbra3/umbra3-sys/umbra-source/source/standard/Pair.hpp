// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once

namespace Umbra
{
template<typename TA, typename TB>
struct Pair
{
    Pair() {}
    Pair(const TA& a, const TB& b) : a(a), b(b) {}

    bool operator==(const Pair<TA, TB>& other) const
    {
        return a == other.a && b == other.b;
    }

    bool operator<(const Pair<TA, TB>& other) const
    {
        if (a < other.a)
            return true;
        else if (a == other.a)
            return b < other.b;
        else
            return false;
    }

    bool operator>(const Pair<TA, TB>& other) const
    {
        return !(*this < other);
    }

    TA a;
    TB b;
};

template<typename TA, typename TB>
inline unsigned int getHashValue (const Pair<TA, TB>& p)
{
    unsigned int a = getHashValue(p.a);
    unsigned int b = getHashValue(p.b);
    return a + b;
#if 0
    unsigned int c = 0xe896f700;
    shuffle(a, b, c);
    return a;
#endif
}

template<typename TA, typename TB>
static inline Pair<TA, TB> makePair(TA a, TB b)
{
    return Pair<TA, TB>(a, b);
}

}
