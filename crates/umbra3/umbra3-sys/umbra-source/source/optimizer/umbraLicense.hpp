#ifndef __UMBRALICENSE_HPP
#define __UMBRALICENSE_HPP
/*!
 *
 * Umbra Occlusion Booster
 * -----------------------------------------
 *
 * (C) 2009-2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \brief   Umbra license key
 *
 */

#include <stdlib.h> // atol
#include <time.h>
#include "umbraPrivateDefs.hpp"

namespace Umbra
{
namespace License
{

//--------------------------------------------------------------------
// File globals
//--------------------------------------------------------------------

static const char LICENSE_PASSWORD[]   = {0xc, 0x2d, 0x62, 0x2d, 0x2c, 0x27, 0x62, 0x31, 0x2a, 0x23, 0x2e, 0x2e, 0x62, 0x32, 0x23, 0x31, 0x31, 0x6c, 0x0};
static const char HEX_TABLE[]       = { '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f' };
static const int  HEX_TABLE_SIZE    = sizeof(HEX_TABLE) / sizeof(HEX_TABLE[0]);
extern const int  g_requireValidation;

/******************************************************************************
 *
 * Function:        Umbra::License::hexToChar()
 *
 * Description:     Convert hex value to small integer
 *
 ******************************************************************************/

UMBRA_FORCE_INLINE char hexToChar(char high, char low)
{
    int r = 0;

    for (int i = 0; i < HEX_TABLE_SIZE; i++)
    {
        if (HEX_TABLE[i] == high)
            r += i << 4;
        if (HEX_TABLE[i] == low)
            r += i;
    }

    return (char) (r);
}

/******************************************************************************
 *
 * Function:        Umbra::License::isHexChar()
 *
 ******************************************************************************/

UMBRA_FORCE_INLINE bool isHexChar(char c)
{
    return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f');
}

/******************************************************************************
 *
 * Function:        Umbra::License::isValidKey()
 *
 * Note:            Valid key is divisible by two and contains only
 *                  characters found in HEX_TABLE
 *
 ******************************************************************************/

UMBRA_FORCE_INLINE bool isValidKey(const char* key)
{
    bool result = false;

    // Check key

    if (key)
    {
        // Check key length

        int len = (int) strlen(key);

        if (len % 2 == 0)
        {
            // Check all chars

            bool charsValid = true;

            for (int i = 0; i < len; ++i)
            {
                if (!isHexChar(key[i]))
                {
                    charsValid = false;
                    break;
                }
            }

            // Were all chars valid?

            result = charsValid;
        }
    }

    return result;
}

/******************************************************************************
 *
 * Function:        Umbra::License::decrypt()
 *
 * Description:     Decrypt license time info
 *
 ******************************************************************************/

UMBRA_FORCE_INLINE void decrypt(char* buf, const char *instr, const char *passPhrase)
{
    size_t l1 = strlen(instr);
    size_t l2 = strlen(passPhrase);
    int index = 0;

    // Decrypt

    for (size_t i = 0; i < l1; i+=2)
    {
        char c = hexToChar(instr[i], instr[i+1]);
        buf[index++] += c ^ (passPhrase[(i>>1) % l2] ^ 0x42);
    }
}

/******************************************************************************
 *
 * Function:        Umbra::License::validate()
 *
 * Description:     Compares license time against system time
 *
 ******************************************************************************/

UMBRA_FORCE_INLINE bool validate(const char* passwd)
{
    if (!g_requireValidation)
        return true;
    if (!isValidKey(passwd))
        return false;

    char decipher[128];
    memset(decipher, '\0', sizeof(decipher));

    decrypt(decipher, passwd, LICENSE_PASSWORD);

    time_t expirationtime = atol(decipher);
    time_t rawtime;
    time ( &rawtime );

    return (rawtime < expirationtime);
}

} // License
} // Umbra

#endif // __UMBRALICENSE_HPP
