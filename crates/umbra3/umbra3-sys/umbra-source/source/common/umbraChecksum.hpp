// Copyright (c) 2009-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRACHECKSUM_HPP
#define UMBRACHECKSUM_HPP

/*!
 * \file    umbraChecksum.hpp
 * \brief   Data checksums.
 */

#include "umbraDefs.hpp"
#include "umbraPlatform.hpp"
#include "umbraString.hpp"


namespace Umbra
{

/// Generate the CRC-32 hash for the given data
UINT32 crc32Hash(const UINT8* ptr, size_t length);

/// Generate the CRC-32 hash for the given data (dword)
UINT32 crc32Hash(const UINT32* ptr, size_t length);

/// Generate the 64-bit FNV-1a hash for the given data.
UINT64 fnv64Hash(const UINT8* buffer, size_t length);


/*!
 * \brief   Helper for generating hash value
 */
class HashGenerator: public OutputStream
{
public:
    HashGenerator (Allocator* a);

    UINT32  write (const void* ptr, UINT32 numBytes);
    String  getHashValue (void);
    void    setForward (OutputStream* forward) { m_forward = forward; }

private:
    Allocator*      m_allocator;
    OutputStream*   m_forward;
    int             m_bytes;
    UINT32          m_values[3];
};

/// Generate the SHA1 hash for the given data
struct Sha1Digest
{
    UINT32 uints[5];
    void str (char* dst) const; // get hex digest str, length 40+'\0'
};

Sha1Digest sha1Hash (const UINT8* buffer, size_t length);

}

#endif
