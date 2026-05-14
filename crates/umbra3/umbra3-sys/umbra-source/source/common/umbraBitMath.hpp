#ifndef UMBRABITMATH_HPP
#define UMBRABITMATH_HPP

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
 * \brief   Umbra bit math. BitVector currently.
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraMemory.hpp"
#include "umbraBitOps.hpp"

namespace Umbra
{
    bool    memEqual    (const void* s0, const void* s1, size_t bytes);
    void    fillByte    (void* destination, unsigned char value, size_t nBytes);
    void    fillDWord   (uint32* destination, uint32 pattern, size_t numDWords);

    namespace BitMath
    {
        extern const signed char s_highestLUT[256];
    }

/*-------------------------------------------------------------------*//*!
 * \brief           Class for representing a variable-size bit vector
 *                  and performing fast operations on the bit data.
 *//*-------------------------------------------------------------------*/

    class BitVector : public Base
    {
    private:
        uint32*     m_array;        //!< data in uint32 format
        size_t      m_dwords;       //!< number of dwords
    public:
                        explicit    BitVector       (size_t size=0, Allocator* a = NULL);
                                    BitVector       (const BitVector& s);
                                    ~BitVector      (void);
                        void        free            (void)                  { UMBRA_DELETE_ARRAY(m_array); m_array = 0; m_dwords = 0; }
    UMBRA_FORCE_INLINE  void        clearAll        (void)                  { fillDWord (m_array,0x00000000,m_dwords);  }
    UMBRA_FORCE_INLINE  void        setAll          (void)                  { fillDWord (m_array,0xFFFFFFFF,m_dwords);  }
    UMBRA_FORCE_INLINE  bool        test            (size_t bit) const      { size_t offs = bit>>5; UMBRA_ASSERT (offs < m_dwords); return (m_array[offs]&(1<<((uint32)(bit)&31))) ? true : false;  }
    UMBRA_FORCE_INLINE  void        set             (size_t bit)            { size_t offs = bit>>5; UMBRA_ASSERT (offs < m_dwords); m_array[offs] |= (uint32)(1<<((uint32)(bit)&31));       }
    UMBRA_FORCE_INLINE  void        clear           (size_t bit)            { size_t offs = bit>>5; UMBRA_ASSERT (offs < m_dwords); m_array[offs] &= ~(uint32)(1<<((uint32)(bit)&31));      }
    UMBRA_FORCE_INLINE  void        eor             (size_t bit)            { size_t offs = bit>>5; UMBRA_ASSERT (offs < m_dwords); m_array[offs] ^= (uint32)(1<<((uint32)(bit)&31));       } // XOR
    UMBRA_FORCE_INLINE  void        set             (size_t bit, int value) { UMBRA_ASSERT(value == 0 || value == 1); if (value) set(bit); else clear(bit); }
    UMBRA_FORCE_INLINE  bool        operator[]      (size_t bit) const      { return test(bit); }
    UMBRA_FORCE_INLINE  size_t      getSize         (void) const            { return m_dwords << 5; }
    UMBRA_FORCE_INLINE  size_t      numBlocks       (void) const            { return m_dwords; }
    UMBRA_FORCE_INLINE  uint32*     getArray        (void) const            { return m_array; }
    UMBRA_FORCE_INLINE  uint32      getBlock        (size_t idx) const      { UMBRA_ASSERT(idx < m_dwords); return m_array[idx]; }
                        void        reset           (size_t size);
                        BitVector&  operator=       (const BitVector& s);
                        void        resize          (size_t size, bool clear, bool value);
                        void        setRange        (size_t start, size_t end);
                        void        _and            (const BitVector& s);
                        void        andNot          (const BitVector& s);
                        void        _or             (const BitVector& s);
                        void        orNot           (const BitVector& s);
                        void        _xor            (const BitVector& s);
                        void        _not            (void);
                        bool        test            (const BitVector& s) const;
                        bool        testNot         (const BitVector& s) const;
                        int         countOnes       (void) const            { int r = 0; for (size_t i = 0; i < m_dwords; i++) r += Umbra::countOnes(m_array[i]); return r; }
                        int         countZeros      (void) const            { int n = 0; for (int i = 0; i < (int)getSize(); i++) if (!test(i)) n++; return n; }
    UMBRA_FORCE_INLINE  size_t      hammingDistance (const BitVector& s) const;
    UMBRA_FORCE_INLINE  void        set             (const BitVector& s)    { for (size_t i = 0; i < min2(m_dwords, s.m_dwords); i++) m_array[i] = s.m_array[i]; }
    };

    /*-------------------------------------------------------------------*//*!
     * \brief
     *//*-------------------------------------------------------------------*/

    class BitInputStream
    {
    public:
        BitInputStream(const BitVector& data) :
            m_data(data), m_pos(0)
            {}

        inline UINT32 read(int bits)
        {
            UMBRA_ASSERT(m_pos + bits + 32 <= (int)m_data.getSize());
            UINT32 value = 0;
            if (bits == 32)
                value = unpackElem32(m_data.getArray(), m_pos);
            else
                value = unpackElem(m_data.getArray(), m_pos, bits);
            m_pos += bits;

            return value;
        }

        inline UINT32 read2(void)
        {
            UMBRA_ASSERT(m_pos + 2 <= (int)m_data.getSize());
            UINT32 value = 0;
            if (m_data.test(m_pos++))
                value = 1;
            if (m_data.test(m_pos++))
                value |= 2;
            return value;
        }

        inline UINT32 read3(void)
        {
            UMBRA_ASSERT(m_pos + 3 <= (int)m_data.getSize());
            UINT32 value = 0;
            if (m_data.test(m_pos++))
                value = 1;
            if (m_data.test(m_pos++))
                value |= 2;
            if (m_data.test(m_pos++))
                value |= 4;
            return value;
        }

        int getPosition() const
        {
            return m_pos;
        }

    private:
        // not allowed
        BitInputStream& operator= (const BitInputStream& s);

        const BitVector&   m_data;
        int                m_pos;
    };

    /*-------------------------------------------------------------------*//*!
     * \brief
     *//*-------------------------------------------------------------------*/

    class BitOutputStream
    {
    public:
        BitOutputStream(BitVector& data) :
            m_data(data), m_pos(0)
            {}

        inline void writeToOffset(int srcOffset, UINT32 value, int bits)
        {
            UMBRA_ASSERT((int)m_data.getSize() <= srcOffset + bits);
            copyBitRange(m_data.getArray(), srcOffset, &value, 0, bits);
        }

        inline void ensureSpace (int bits)
        {
            if ((int)m_data.getSize() <= m_pos + bits + 32)
                m_data.resize(max2((size_t)256, (m_data.getSize() + bits)*2 + 32), true, false);
        }

        inline void writeBits(const BitVector& bv, int bits)
        {
            UMBRA_ASSERT(bits >= 0);

            ensureSpace(bits);

            // TODO: optimize!
            for (int i = 0; i < bits; i++)
                write(bv.test(i) ? 1 : 0);
        }

        inline void write(UINT32 value, int bits)
        {
            UMBRA_ASSERT(bits == 32 || value < (1u << bits));
            ensureSpace(bits);
            copyBitRange(m_data.getArray(), m_pos, &value, 0, bits);
            m_pos += bits;
        }

        inline void write(int bit)
        {
            UMBRA_ASSERT(bit == 0 || bit == 1);
            ensureSpace(1);
            m_data.set(m_pos++, (size_t)bit);
        }

        inline void write2(int bits)
        {
            UMBRA_ASSERT(bits >= 0 && bits < 4);
            ensureSpace(2);
            m_data.set(m_pos++, (size_t)bits & 1);
            m_data.set(m_pos++, (size_t)bits >> 1);
        }

        inline void write3(int bits)
        {
            UMBRA_ASSERT(bits >= 0 && bits < 8);
            ensureSpace(3);
            m_data.set(m_pos++, (size_t)bits & 1);
            m_data.set(m_pos++, (size_t)(bits >> 1) & 1);
            m_data.set(m_pos++, (size_t)(bits >> 2) & 1);
        }

        inline int getBitCount() const
        {
            return m_pos;
        }

    private:
        // not allowed
        BitOutputStream& operator= (const BitOutputStream& s);

        BitVector&  m_data;
        int         m_pos;
    };


/*-------------------------------------------------------------------*//*!
 * \brief           Class for representing a variable-size bit matrix
 *                  and performing fast operations on the bit data.
 *//*-------------------------------------------------------------------*/

    class BitMatrix
    {
    public:

                BitMatrix   (void) { reset(0,0); }
                BitMatrix   (int w, int h) { reset(w,h); }

        void    reset       (int w, int h)  { m_width = w; m_height = h; m_bitVector.reset(w*h); m_bitVector.clearAll(); }
        void    set         (int x, int y)  { m_bitVector.set(y*m_width + x); }
        void    clearAll    (void)          { m_bitVector.clearAll(); }
        void    clear       (int x, int y)  { m_bitVector.clear(y*m_width + x); }
        bool    test        (int x, int y)  const { return m_bitVector.test(y*m_width + x); }

    private:

        int         m_width;
        int         m_height;
        BitVector   m_bitVector;
    };

UMBRA_FORCE_INLINE int log2 (uint32 value)
{
    static const UINT8 MultiplyDeBruijnBitPosition[32] =
    {
      0, 9, 1, 10, 13, 21, 2, 29, 11, 14, 16, 18, 22, 25, 3, 30,
      8, 12, 20, 28, 15, 17, 24, 7, 19, 27, 23, 6, 26, 5, 4, 31
    };

    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;

    return (int)MultiplyDeBruijnBitPosition[(uint32)(value * 0x07C4ACDDU) >> 27];
}

UMBRA_FORCE_INLINE int getHighestSetBit (uint32 value)
#if defined (UMBRA_X86_ASSEMBLY)
#pragma warning (disable:4035) // don't whine about no return value (it's returned in eax)
{
#if defined (UMBRA_X86_RETURN_EAX)
    __asm
    {
        mov eax,-1              // if (a==0), bsr doesn't set eax so we store -1 here now
        mov ecx,value
        bsr eax,ecx
    }
#else
    int retVal;
    __asm
    {
        mov eax,-1              // if (a==0), bsr doesn't set eax so we store -1 here now
        mov ecx,value
        bsr eax,ecx
        mov retVal,eax
    }
    return retVal;
#endif //UMBRA_X86_RETURN_EAX
}
#pragma warning (default:4035)
#else
{
    if (value&0xffff0000)
    {
        if (value&0xff000000)
            return BitMath::s_highestLUT[value>>24]+24;
        return BitMath::s_highestLUT[value>>16]+16;
    }
    if (value&0xff00)
        return BitMath::s_highestLUT[value>>8]+8;
    return BitMath::s_highestLUT[value];
}
#endif // UMBRA_X86_ASSEMBLY


UMBRA_FORCE_INLINE uint32 getNextPowerOfTwo(uint32 value)
#if defined (UMBRA_X86_ASSEMBLY)
#pragma warning (disable:4035) // don't whine about no return value (it's returned in eax)
{
#if defined (UMBRA_X86_RETURN_EAX)
    __asm
    {
        mov ecx,-1
        mov eax,value
        dec eax
        bsr ecx,eax
        mov eax,1
        inc ecx
        shl eax,cl              // return value is always in eax
    }
#else //UMBRA_X86_RETURN_EAX
    unsigned int retVal;
    __asm
    {
        mov ecx,-1
        mov eax,value
        dec eax
        bsr ecx,eax
        mov eax,1
        inc ecx
        shl eax,cl              // return value is always in eax
        mov retVal,eax
    }
    return retVal;
#endif //UMBRA_X86_RETURN_EAX
}
#pragma warning (default:4035)
#else
{
    if (value > 1)
        return (uint32)(1<<(getHighestSetBit(value-1)+1));
    else
        return 1;
}
#endif // UMBRA_X86_ASSEMBLY

UMBRA_FORCE_INLINE bool isPowerOfTwo (uint32 value)
{
    // \todo real impl, move elsewhere?
    return getNextPowerOfTwo(value) == value;
}

//------------------------------------------------------------------------
// Generic rotation -> left code and implementation for platforms that
// have native support for it (such as x86)
//------------------------------------------------------------------------

#if defined (UMBRA_X86_ASSEMBLY) && defined(UMBRA_X86_RETURN_EAX)
#pragma warning (disable:4035)
static UMBRA_FORCE_INLINE uint32 rotateLeft (uint32 x, int32 r)
{
    __asm
    {
        mov eax,x
        mov ecx,r
        rol eax,cl
    }
}
#pragma warning (default:4035)
#else
static UMBRA_FORCE_INLINE uint32 rotateLeft (uint32 x, int32 r)  { return (x << r) | (x >> (sizeof(x)*8-r));   }
#endif // UMBRA_X86_ASSEMBLY && UMBRA_X86_RETURN_EAX

UMBRA_FORCE_INLINE uint32 interleaveBits(uint32 x, uint32 y)
{
    UMBRA_ASSERT(x <= 65535 && y <= 65535);

    uint32 ret = 0;
    for (int i = 0; i < 16; i++)
    {
        ret |= (x & (1u << i)) << i;
        ret |= (y & (1u << i)) << (i+1);
    }

    return ret;
}

UMBRA_FORCE_INLINE void deinterleaveBits(uint32 v, uint32& x, uint32& y)
{
    x = y = 0;
    for (int i = 0; i < 16; i++)
    {
        x |= (v & (1u << 2*i)) >> i;
        y |= ((v>>1) & (1u << 2*i)) >> i;
    }
}

UMBRA_FORCE_INLINE size_t BitVector::hammingDistance(const BitVector& s) const
{
    UMBRA_ASSERT(m_dwords == s.m_dwords);
    if (m_dwords != s.m_dwords)
        return 0;

    size_t distance = 0;
    for (size_t i = 0; i < m_dwords; i++)
    {
        // calculate number of ones when xorring the two bitvectors
        uint32 x = m_array[i] ^ s.m_array[i];

        while (x)
        {
            distance++;
            x &= x - 1;
        }
    }

    return distance;
}

} // namespace Umbra

#endif // UMBRABITMATH_HPP

//--------------------------------------------------------------------
