#include "umbraPrivateDefs.hpp"
#include "umbraChecksum.hpp"
#include "umbraHash.hpp"
#include <cstdlib>
#include <cstdio>


namespace Umbra
{

// crc-32c, table from RFC 3309
const Umbra::UINT32 crcLut[] =
{
    0x00000000, 0xf26b8303, 0xe13b70f7, 0x1350f3f4,
    0xc79a971f, 0x35f1141c, 0x26a1e7e8, 0xd4ca64eb,
    0x8ad958cf, 0x78b2dbcc, 0x6be22838, 0x9989ab3b,
    0x4d43cfd0, 0xbf284cd3, 0xac78bf27, 0x5e133c24,
    0x105ec76f, 0xe235446c, 0xf165b798, 0x030e349b,
    0xd7c45070, 0x25afd373, 0x36ff2087, 0xc494a384,
    0x9a879fa0, 0x68ec1ca3, 0x7bbcef57, 0x89d76c54,
    0x5d1d08bf, 0xaf768bbc, 0xbc267848, 0x4e4dfb4b,
    0x20bd8ede, 0xd2d60ddd, 0xc186fe29, 0x33ed7d2a,
    0xe72719c1, 0x154c9ac2, 0x061c6936, 0xf477ea35,
    0xaa64d611, 0x580f5512, 0x4b5fa6e6, 0xb93425e5,
    0x6dfe410e, 0x9f95c20d, 0x8cc531f9, 0x7eaeb2fa,
    0x30e349b1, 0xc288cab2, 0xd1d83946, 0x23b3ba45,
    0xf779deae, 0x05125dad, 0x1642ae59, 0xe4292d5a,
    0xba3a117e, 0x4851927d, 0x5b016189, 0xa96ae28a,
    0x7da08661, 0x8fcb0562, 0x9c9bf696, 0x6ef07595,
    0x417b1dbc, 0xb3109ebf, 0xa0406d4b, 0x522bee48,
    0x86e18aa3, 0x748a09a0, 0x67dafa54, 0x95b17957,
    0xcba24573, 0x39c9c670, 0x2a993584, 0xd8f2b687,
    0x0c38d26c, 0xfe53516f, 0xed03a29b, 0x1f682198,
    0x5125dad3, 0xa34e59d0, 0xb01eaa24, 0x42752927,
    0x96bf4dcc, 0x64d4cecf, 0x77843d3b, 0x85efbe38,
    0xdbfc821c, 0x2997011f, 0x3ac7f2eb, 0xc8ac71e8,
    0x1c661503, 0xee0d9600, 0xfd5d65f4, 0x0f36e6f7,
    0x61c69362, 0x93ad1061, 0x80fde395, 0x72966096,
    0xa65c047d, 0x5437877e, 0x4767748a, 0xb50cf789,
    0xeb1fcbad, 0x197448ae, 0x0a24bb5a, 0xf84f3859,
    0x2c855cb2, 0xdeeedfb1, 0xcdbe2c45, 0x3fd5af46,
    0x7198540d, 0x83f3d70e, 0x90a324fa, 0x62c8a7f9,
    0xb602c312, 0x44694011, 0x5739b3e5, 0xa55230e6,
    0xfb410cc2, 0x092a8fc1, 0x1a7a7c35, 0xe811ff36,
    0x3cdb9bdd, 0xceb018de, 0xdde0eb2a, 0x2f8b6829,
    0x82f63b78, 0x709db87b, 0x63cd4b8f, 0x91a6c88c,
    0x456cac67, 0xb7072f64, 0xa457dc90, 0x563c5f93,
    0x082f63b7, 0xfa44e0b4, 0xe9141340, 0x1b7f9043,
    0xcfb5f4a8, 0x3dde77ab, 0x2e8e845f, 0xdce5075c,
    0x92a8fc17, 0x60c37f14, 0x73938ce0, 0x81f80fe3,
    0x55326b08, 0xa759e80b, 0xb4091bff, 0x466298fc,
    0x1871a4d8, 0xea1a27db, 0xf94ad42f, 0x0b21572c,
    0xdfeb33c7, 0x2d80b0c4, 0x3ed04330, 0xccbbc033,
    0xa24bb5a6, 0x502036a5, 0x4370c551, 0xb11b4652,
    0x65d122b9, 0x97baa1ba, 0x84ea524e, 0x7681d14d,
    0x2892ed69, 0xdaf96e6a, 0xc9a99d9e, 0x3bc21e9d,
    0xef087a76, 0x1d63f975, 0x0e330a81, 0xfc588982,
    0xb21572c9, 0x407ef1ca, 0x532e023e, 0xa145813d,
    0x758fe5d6, 0x87e466d5, 0x94b49521, 0x66df1622,
    0x38cc2a06, 0xcaa7a905, 0xd9f75af1, 0x2b9cd9f2,
    0xff56bd19, 0x0d3d3e1a, 0x1e6dcdee, 0xec064eed,
    0xc38d26c4, 0x31e6a5c7, 0x22b65633, 0xd0ddd530,
    0x0417b1db, 0xf67c32d8, 0xe52cc12c, 0x1747422f,
    0x49547e0b, 0xbb3ffd08, 0xa86f0efc, 0x5a048dff,
    0x8ecee914, 0x7ca56a17, 0x6ff599e3, 0x9d9e1ae0,
    0xd3d3e1ab, 0x21b862a8, 0x32e8915c, 0xc083125f,
    0x144976b4, 0xe622f5b7, 0xf5720643, 0x07198540,
    0x590ab964, 0xab613a67, 0xb831c993, 0x4a5a4a90,
    0x9e902e7b, 0x6cfbad78, 0x7fab5e8c, 0x8dc0dd8f,
    0xe330a81a, 0x115b2b19, 0x020bd8ed, 0xf0605bee,
    0x24aa3f05, 0xd6c1bc06, 0xc5914ff2, 0x37faccf1,
    0x69e9f0d5, 0x9b8273d6, 0x88d28022, 0x7ab90321,
    0xae7367ca, 0x5c18e4c9, 0x4f48173d, 0xbd23943e,
    0xf36e6f75, 0x0105ec76, 0x12551f82, 0xe03e9c81,
    0x34f4f86a, 0xc69f7b69, 0xd5cf889d, 0x27a40b9e,
    0x79b737ba, 0x8bdcb4b9, 0x988c474d, 0x6ae7c44e,
    0xbe2da0a5, 0x4c4623a6, 0x5f16d052, 0xad7d5351
};

UINT32 crc32Hash(const UINT8* ptr, size_t length)
{
    UINT32 crc = 0xffffffffu;
    while (length--)
        crc = (crc >> 8) ^ crcLut[(crc ^ *ptr++) & 0xff];
    return crc ^ 0xffffffffu;
}

UINT32 crc32Hash(const UINT32* ptr, size_t length)
{
    UMBRA_ASSERT(!(length & 3));

    UINT32 crc = 0xffffffffu;
    while (length)
    {
        UMBRA_ASSERT(length >= sizeof(UINT32));
        UINT32 value = *ptr++;

        for (size_t i = 0; i < sizeof(UINT32); i++)
        {
            crc = (crc >> 8) ^ crcLut[(crc ^ (value & 0xff)) & 0xff];
            value >>= 8;
        }

        length -= sizeof(UINT32);
    }
    return crc ^ 0xffffffffu;
}

UINT64 fnv64Hash(const UINT8* buffer, size_t length)
{
    const UINT64 fnvPrime = 1099511628211ULL;
    UINT64 hash = 14695981039346656037ULL;
    for (size_t i = 0; i < length; i++)
    {
        hash = hash ^ buffer[i];
        hash = hash * fnvPrime;
    }
    return hash;
}


static UINT32 rotl32(UINT32 x, int b)
{
    return (x << b) | (x >> (32-b));
}

static UINT32 get32 (UINT32 u)
{
#if UMBRA_BYTE_ORDER == UMBRA_LITTLE_ENDIAN
    // switch endianness
    const UINT8 *x = (const UINT8*)&u;
    return (x[0] << 24) | (x[1] << 16) | (x[2] << 8) | x[3];
#else
    return u;
#endif
}

static UINT32 f (int t, UINT32 b, UINT32 c, UINT32 d)
{
    UMBRA_ASSERT(0 <= t && t < 80);

    if (t < 20)
        return (b & c) | ((~b) & d);
    if (t < 40)
        return b ^ c ^ d;
    if (t < 60)
        return (b & c) | (b & d) | (c & d);
    //if (t < 80)
        return b ^ c ^ d;
}


static void processBlock (const UINT32* block, UINT32* h)
{
    static const UINT32 k[4] =
    {
        0x5A827999,
        0x6ED9EBA1,
        0x8F1BBCDC,
        0xCA62C1D6
    };

    UINT32 w[80];
    UINT32 a = h[0];
    UINT32 b = h[1];
    UINT32 c = h[2];
    UINT32 d = h[3];
    UINT32 e = h[4];
    int t;

    for (t = 0; t < 16; t++)
        w[t] = get32(*block++);
/*
    for (; t < 80; t++)
        w[t] = rotl32(w[t-3] ^ w[t-8] ^ w[t-14] ^ w[t-16], 1);
        */

    for (t = 0; t < 80; t++)
    {
        int s = t & 0xf;
        UINT32 temp;
        if (t >= 16)
            w[s] = rotl32(w[(s + 13) & 0xf] ^ w[(s + 8) & 0xf] ^ w[(s + 2) & 0xf] ^ w[s], 1);

         temp = rotl32(a, 5) + f(t, b,c,d) + e + w[s] + k[t/20];

         e = d; d = c; c = rotl32(b, 30); b = a; a = temp;
        /*
        UINT32 temp = rotl32(a, 5) + f(t, b,c,d) + e + w[t] + k[t/20];
        e = d;
        d = c;
        c = rotl32(b, 30);
        b = a;
        a = temp;
        */
    }

    h[0] += a;
    h[1] += b;
    h[2] += c;
    h[3] += d;
    h[4] += e;
}

static size_t padMsg (const void* msg, UINT64 bytes, UINT8* lastBlock)
{
    size_t msgBytesInLast   = (size_t)(bytes & 63);
    size_t extraBlocks      = (msgBytesInLast + 9) > 64 ? 2 : 1;
    size_t numZeroBytes     = extraBlocks * 64 - 9 - msgBytesInLast;

    // fill remaining msg bytes
    const UINT8* msgLast = (UINT8*)msg + (bytes & ~63);
    while (msgBytesInLast--)
        *lastBlock++ = *msgLast++;

    // separator
    *lastBlock++ = 0x80;

    while (numZeroBytes--)
        *lastBlock++ = 0;

    // original length in bits (!), switch endianness
    bytes *= 8;
    *lastBlock++ = (UINT8)(bytes >> 56 & 0xff);
    *lastBlock++ = (UINT8)(bytes >> 48 & 0xff);
    *lastBlock++ = (UINT8)(bytes >> 40 & 0xff);
    *lastBlock++ = (UINT8)(bytes >> 32 & 0xff);
    *lastBlock++ = (UINT8)(bytes >> 24 & 0xff);
    *lastBlock++ = (UINT8)(bytes >> 16 & 0xff);
    *lastBlock++ = (UINT8)(bytes >> 8  & 0xff);
    *lastBlock++ = (UINT8)(bytes >> 0  & 0xff);

    return extraBlocks;
}

Sha1Digest sha1Hash (const UINT8* msg, size_t bytes)
{
    Sha1Digest digest;
    UINT32 h[5] =
    {
        0x67452301,
        0xefcdab89,
        0x98badcfe,
        0x10325476,
        0xc3d2e1f0
    };

    size_t totalBlocks = UMBRA_ALIGN(bytes + 9, 64) / 64; // including padding
    const UINT32* block = (const UINT32*)msg;
    size_t b;

    // we could assume that msg is always required to
    // hold padding, but let's not
    UINT8 lastBlocks[128];  // either one or two blocks
    size_t numLast = padMsg(msg, bytes, lastBlocks);

    for (b = 0; b < totalBlocks-numLast; b++)
    {
        processBlock(block, h);
        block += 16;
    }

    // process last block
    for (b = 0; b < numLast; b++)
        processBlock((const UINT32*)(lastBlocks + b*64), h);

    digest.uints[0] = h[0];
    digest.uints[1] = h[1];
    digest.uints[2] = h[2];
    digest.uints[3] = h[3];
    digest.uints[4] = h[4];
    return digest;
}

static inline void appendByteChar (char* d, int byte)
{
    int c0 = byte >> 4;
    int c1 = byte & 0xf;

    UMBRA_ASSERT(0 <= c0 && c0 <= 0xf);
    UMBRA_ASSERT(0 <= c1 && c1 <= 0xf);
    c0 = c0 <= 9 ? '0' + c0 : 'a' + c0 - 0xa;
    c1 = c1 <= 9 ? '0' + c1 : 'a' + c1 - 0xa;

    *d++ = (char)c0;
    *d   = (char)c1;
}

void Sha1Digest::str (char* dst) const
{
    // sprintf would be easier but don't want
    // to add dependencies
    for (int i = 0; i < 5; i++)
    {
        UINT32 u = uints[i];
        appendByteChar(dst, (u >> 24));        dst += 2;
        appendByteChar(dst, (u >> 16) & 0xff); dst += 2;
        appendByteChar(dst, (u >> 8 ) & 0xff); dst += 2;
        appendByteChar(dst, (u      ) & 0xff); dst += 2;
    }
    *dst = '\0';
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

HashGenerator::HashGenerator (Allocator* a)
    : m_allocator(a), m_forward(NULL), m_bytes(0)
{
    m_values[0] = 0x9e3779b9;
    m_values[1] = 0x9e3779b9;
    m_values[2] = 0x9e3779b9;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Umbra::UINT32 HashGenerator::write (const void* ptr, Umbra::UINT32 numBytes)
{
    UMBRA_ASSERT(ptr);
    const UINT8* src = (const UINT8*)ptr;
    UINT32 len = numBytes;

    while (len--)
    {
        int pos = m_bytes++ % 12;
        int idx = pos >> 2;
        int shift = (pos & 0x3) << 3;
        m_values[idx] += (*src++ << shift);

        if (pos == 11)
            shuffle(m_values[0], m_values[1], m_values[2]);
    }

    if (m_forward)
        return m_forward->write(ptr, numBytes);

    return numBytes;
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

String HashGenerator::getHashValue (void)
{
    if ((m_bytes % 12) != 0)
    {
        m_values[2] += m_bytes;
        shuffle(m_values[0], m_values[1], m_values[2]);
        m_bytes = 0;
    }

    char tmp[32];
    std::sprintf(tmp, "%08x%08x%08x", m_values[0], m_values[1], m_values[2]);
    return String(tmp, m_allocator);

}
}
