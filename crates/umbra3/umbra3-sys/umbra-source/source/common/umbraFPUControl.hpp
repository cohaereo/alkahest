#ifndef UMBRAFPUCONTROL_HPP
#define UMBRAFPUCONTROL_HPP

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
 * \brief   Floating Point Unit Control routines
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{

/*----------------------------------------------------------------------*//*!
 * \brief   Class for controlling FPU state
 *
 * \note    To port for different platforms, implement the functions get()
 *          and set().
 *//*----------------------------------------------------------------------*/

#if (HG_COMPILER == HG_COMPILER_MSC) && (HG_CPU == HG_CPU_X86_64)
// TODO [sampo@umbrasoftware.com] modifying the fpu state is not supported in x64. Is this needed?
static uint32 s_stateX64 = 0;
#endif

class FPUControl
{
public:
    enum Precision
    {
        PRECISION_24    = 0,                // 24-bit precision
        PRECISION_53    = 512,              // 53-bit precision
        PRECISION_64    = 512|256,          // 64-bit precision
        PRECISION_MASK  = 512|256
    };

    enum Rounding
    {
        ROUND_NEAR = 0,                 // round to near
        ROUND_DOWN = 1024,              // round down
        ROUND_UP   = 2048,              // round up
        ROUND_CHOP = 1024|2048,         // truncate (i.e. chop)
        ROUND_MASK = 1024|2048
    };

#if defined (UMBRA_X86_ASSEMBLY)

    static UMBRA_FORCE_INLINE uint32 get    (void)
    {
        unsigned int tmp;
        __asm
        {
            fwait
            fstcw dword ptr [tmp]
        }
        return tmp;
    }
    static UMBRA_FORCE_INLINE void set (uint32 a)
    {
        __asm
        {
            fwait
            fldcw dword ptr [a]
        }
    }
#elif (HG_COMPILER == HG_COMPILER_MSC) && (HG_CPU == HG_CPU_X86_64)
    static UMBRA_FORCE_INLINE uint32 get (void)
    {
        return s_stateX64;
    }

    static UMBRA_FORCE_INLINE void   set (uint32 a)
    {
        s_stateX64 = a;
    }
#else // need to be implemented on this platform
    static UMBRA_FORCE_INLINE uint32 get (void)
    {
        unsigned short tmp;
        asm volatile ("fstcw %0" : "=m" (tmp) );
        return tmp;
    }

    static UMBRA_FORCE_INLINE void   set (uint32 a)
    {
        asm volatile ("fldcw %0" : : "m" (*&a));
    }
#endif

    static UMBRA_FORCE_INLINE Rounding getRounding (void)
    {
        return (Rounding)(get() & ROUND_MASK);
    }

    static UMBRA_FORCE_INLINE Precision getPrecision (void)
    {
        return (Precision)(get() & PRECISION_MASK);
    }

    UMBRA_FORCE_INLINE FPUControl (Precision p) :
        m_oldMode(get()),
        m_changed(false)
    {
        uint32 tmp = (m_oldMode & ~PRECISION_MASK) | (uint32)(p);
        if (tmp != m_oldMode)
        {
            set(tmp);
            m_changed = true;
        }
    }

    UMBRA_FORCE_INLINE FPUControl (Rounding r) :
        m_oldMode(get()),
        m_changed(false)
    {
        uint32 tmp = (m_oldMode & ~ROUND_MASK) | (uint32)(r);
        if (tmp != m_oldMode)
        {
            set(tmp);
            m_changed = true;
        }
    }

    UMBRA_FORCE_INLINE FPUControl (Precision p, Rounding r) :
        m_oldMode(get()),
        m_changed(false)
    {
        uint32 tmp = (m_oldMode & ~(ROUND_MASK|PRECISION_MASK)) | (uint32)(r) | (uint32)(p);
        if (tmp != m_oldMode)
        {
            set(tmp);
            m_changed = true;
        }
    }

    UMBRA_FORCE_INLINE ~FPUControl (void)
    {
        if (m_changed)
            set(m_oldMode);
    }


private:
    FPUControl              (const FPUControl&);
    FPUControl& operator=   (const FPUControl&);

    uint32  m_oldMode;                                      //!< store old FPU mode here
    bool    m_changed;                                      //!< if zero, value didn't change (speedup)
};

//------------------------------------------------------------------------
// FPU Control macros.
//------------------------------------------------------------------------

#define UMBRA_SET_DEFAULT_FPU_MODE \
    ::Umbra::FPUControl::set((::Umbra::FPUControl::get() & \
                             ~(::Umbra::FPUControl::PRECISION_MASK | ::Umbra::FPUControl::ROUND_MASK)) | \
                            ::Umbra::FPUControl::PRECISION_24 | ::Umbra::FPUControl::ROUND_NEAR);

#define UMBRA_SET_DEFAULT_FPU_MODE_TEMPORARY \
    ::Umbra::FPUControl precisionFPUControl(::Umbra::FPUControl::PRECISION_24); \
    ::Umbra::FPUControl roundFPUControl(::Umbra::FPUControl::ROUND_NEAR);

#define UMBRA_ASSERT_DEFAULT_FPU_MODE /*\
    UMBRA_ASSERT((::Umbra::FPUControl::get() & ::Umbra::FPUControl::PRECISION_MASK) == ::Umbra::FPUControl::PRECISION_53 && \
                (::Umbra::FPUControl::get() & ::Umbra::FPUControl::ROUND_MASK) == ::Umbra::FPUControl::ROUND_UP);*/

} // namespace Umbra

#endif // UMBRAFPUCONTROL_HPP
//------------------------------------------------------------------------
