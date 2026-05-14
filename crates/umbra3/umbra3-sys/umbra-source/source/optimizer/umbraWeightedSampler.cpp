#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraPrivateDefs.hpp"
#include "umbraWeightedSampler.hpp"
#include "umbraMemory.hpp"

using namespace Umbra;

WeightedSampler::WeightedSampler(Allocator* a, int size)
:   m_weights(0, a),
    m_cumulated(a),
    m_firstDirty(0)
{
    resize(size);
}

WeightedSampler::~WeightedSampler()
{
}

void WeightedSampler::resize(int n)
{
    m_weights.reset(n);
    m_cumulated.reset(n);

    for (int i = 0; i < n; i++)
        m_weights[i] = 1.0 / n;

    m_firstDirty = 0;
}

void WeightedSampler::setWeight(int i, double weight)
{
    UMBRA_ASSERT(i >= 0 && i < m_weights.getSize());
    UMBRA_ASSERT(weight >= 0.0);

    if (m_weights[i] != weight)
    {
        m_firstDirty = min2(m_firstDirty, i);
        m_weights[i] = weight;
    }
}

double WeightedSampler::getWeight(int i)
{
    return m_weights[i];
}

void WeightedSampler::normalizeSampleWeights()
{
    double sum = 0.0;

    for (int i = 0; i < m_weights.getSize(); i++)
        sum += m_weights[i];

    if (sum == 0.0 || sum == 1.0)
        return;

    for (int i = 0; i < m_weights.getSize(); i++)
        m_weights[i] /= sum;

    m_firstDirty = 0;
}

void WeightedSampler::update()
{
    UMBRA_ASSERT(m_weights.getSize() > 0);

    double lastCumulated = (m_firstDirty == 0) ? 0.0 : m_weights[m_firstDirty-1];

    for (; m_firstDirty < m_weights.getSize(); m_firstDirty++)
    {
        lastCumulated += m_weights[m_firstDirty];
        m_cumulated[m_firstDirty] = (lastCumulated == 0.0) ? -1.0 : lastCumulated;
    }
}

int WeightedSampler::pickSample(double rnd)
{
    UMBRA_ASSERT(m_weights.getSize() > 0);
    UMBRA_ASSERT(rnd >= 0.0 && rnd <= 1.0);

    if (!m_weights.getSize())
        return -1;

    update();

    // Pick sample.

    double val = rnd * m_cumulated[m_weights.getSize() - 1];

    // \todo do binary search

    for (int i = 0; i < m_weights.getSize(); i++)
        if (val <= m_cumulated[i])
            return i;

    // Just in case there was some floating-point mistake: uniform sampling.

    return max2(0, min2(int(rnd * m_weights.getSize()), m_weights.getSize() - 1));
}

#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)
