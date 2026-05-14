#pragma once

/*!
 *
 * Umbra PVS
 * -----------------------------------------
 *
 * (C) 2007-2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief
 *
 */

#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraPrivateDefs.hpp"
#include "umbraMemory.hpp"
#include "umbraArray.hpp"

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/
class WeightedSampler
{
public:
    WeightedSampler(Allocator* a, int size = 0);
    ~WeightedSampler();

    void resize(int n);
    inline int getSize() const { return m_weights.getSize(); }

    void setWeight(int i, double weight);
    double getWeight(int i);
    void normalizeSampleWeights();

    int pickSample(double rnd);

private:
    void update();

private:
    Array<double>   m_weights;
    Array<double>   m_cumulated;
    int             m_firstDirty; // cumulation array needs update after this
};

} // namespace Umbra

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)
