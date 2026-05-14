#ifndef __UMBRAUUID_HPP
#define __UMBRAUUID_HPP

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2012 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra UUID
 *
 */

#include "umbraPrivateDefs.hpp"

namespace Umbra
{

class UUID
{
public:

    static const int        charLength = 37;    // number of bytes required for string

                            UUID        (void);
                            ~UUID       (void) {}

    static UUID             generate    (void); // with common only

    bool                    valid       (void);
    void UMBRA_INLINE       string      (char* string) const; 
    bool UMBRA_INLINE       operator==  (const UUID& other) const;
    const Umbra::UINT32*    value       (void) const  { return m_uuid; }
    Umbra::UINT32           value       (int i) const { UMBRA_ASSERT(i >= 0 && i < 4); return m_uuid[i]; }
    
private:
    Umbra::UINT32   m_uuid[4];
};

UMBRA_INLINE UUID::UUID(void)
{
    m_uuid[0] = 0;
    m_uuid[1] = 0;
    m_uuid[2] = 0;
    m_uuid[3] = 0;
}

bool UMBRA_INLINE UUID::valid(void)
{
    return m_uuid[0] > 0 ||
           m_uuid[1] > 0 ||
           m_uuid[2] > 0 ||
           m_uuid[3] > 0;
}

void UMBRA_INLINE UUID::string(char* string) const
{
    sprintf(string, "%08x-%04x-%04x-%04x-%04x%08x", 
        m_uuid[0],
        (m_uuid[1] >> 16) & 0xffff,
        m_uuid[1] & 0xffff,
        (m_uuid[2] >> 16) & 0xffff,
        m_uuid[2] & 0xffff,
        m_uuid[3]);
}

bool UMBRA_INLINE UUID::operator== (const UUID& other) const
{
    return m_uuid[0] == other.m_uuid[0] &&
           m_uuid[1] == other.m_uuid[1] &&
           m_uuid[2] == other.m_uuid[2] &&
           m_uuid[3] == other.m_uuid[3];
}

template <> UMBRA_FORCE_INLINE unsigned int getHashValue (const UUID& u)
{
    return ((u.value(0) ^ u.value(1)) ^ u.value(2)) ^ u.value(3);
}

} // namespace Umbra

#endif // __UMBRAUUID_HPP

//--------------------------------------------------------------------
