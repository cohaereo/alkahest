#ifndef UMBRAPROGRESS_HPP
#define UMBRAPROGRESS_HPP

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
 * \brief   Umbra progress helper
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{
    class Progress : public Base
    {
    public:
        Progress(Allocator* a) 
            : Base(a), m_dst(NULL), m_start(0.f), m_end(1.f), 
              m_phaseStart(0.f), m_phaseEnd(0.f), 
              m_phaseProgress(0.f), m_lastValue(0.f),
              m_phaseIdx(0), m_phases(a)
        {}
        
        void reset()
        {
            m_phases.clear();
        }

        void addPhase(float length, const char*)
        {
            m_phases.pushBack(length);
        }

        void start(float* dst = NULL, float start = 0.f, float end = 1.f, Progress* parent = NULL)
        {
            UMBRA_ASSERT(m_phases.getSize() > 0);
            m_dst = dst;
            m_start = start;
            m_end = end;
            m_phaseStart = 0.f;
            m_phaseEnd = 0.f;
            m_phaseProgress = 0.f;
            m_parent = parent;
            UMBRA_ASSERT(m_start >= 0.f && m_start <= 1.f && m_start < m_end);
            UMBRA_ASSERT(m_end >= 0.f && m_end <= 1.f);

            float sum = 0.f;
            for (int i = 0; i < m_phases.getSize(); i++)
                sum += m_phases[i];
            float multiplier = 1.f / sum;
            for (int i = 0; i < m_phases.getSize(); i++)
                m_phases[i] *= multiplier;
        }
        
        // Start next phase
        void  nextPhase (void)  
        { 
            float length = m_phases[m_phaseIdx++];
            m_phaseStart = m_phaseEnd; 
            m_phaseEnd = m_phaseStart + length; 
            //UMBRA_ASSERT(m_phaseEnd <= 1.f);
            m_phaseProgress = 0.f;
            setPhaseProgress(0.f);
        }

        // Set inner progress of current phase [0,1]
        void   setPhaseProgress (float p) 
        { 
            UMBRA_ASSERT(p >= 0.f && p <= 1.f);
            UMBRA_ASSERT(p >= m_phaseProgress);
            m_phaseProgress = p; 
            m_lastValue = computeValue(); 
            if (m_dst) 
                *m_dst = getValue();
            if (m_parent)
                m_parent->setPhaseProgress(getValue());
        }

        // Advance complete phase without measuring it's inner progress
        void advancePhase ()
        {
            nextPhase();
            setPhaseProgress(1.f);
        }
        
        float  getPhaseProgress (void)    { return m_phaseProgress; }
        float  getValue         (void)    { return m_lastValue; }

    private:

        // Get current progress
        float  computeValue     (void)    { return m_start + (m_end - m_start) * (m_phaseStart + (m_phaseEnd - m_phaseStart) * m_phaseProgress);}

        float*          m_dst;
        float           m_start;
        float           m_end;
        float           m_phaseStart;
        float           m_phaseEnd;
        float           m_phaseProgress;
        float           m_lastValue;
        int             m_phaseIdx;
        Array<float>    m_phases;
        Progress*       m_parent;
    };

} // namespace Umbra

#endif // UMBRAFLOAT_HPP

//--------------------------------------------------------------------
