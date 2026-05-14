#ifndef UMBRASTRING_HPP
#define UMBRASTRING_HPP
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
 * \brief   String class
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraArray.hpp"

#include <stdio.h>
#include <string.h>
#include <ctype.h>

namespace Umbra
{

/*-------------------------------------------------------------------*//*!
 * \brief           Class for representing a string of characters
 *
 * \note            This class is provided in order to simplify some
 *                  basic string operations, such as comparisons,
 *                  insertion of characters in the middle of a string
 *                  and converting strings into upper/lower case.
 *
 * \note            The class has a cast-operator into char*, which allows easy
 *                  mixed usage of String and standard character arrays.
 *
 * \note            Some operations such as querying the length of a string
 *                  are much faster with this class than using an array of
 *                  characters.
 *//*-------------------------------------------------------------------*/

class String: public Base
{
public:
                        String          (Allocator* a = NULL)                       { init(a); }
                        String          (const String& s) : Base(NULL)              { init(s.getAllocator()); if (s.m_string) *this = s.m_string; }
                        String          (const String& s, int i1, int i2);
                        String          (const char* str, int len);
                        String          (const char* str, Allocator* a = NULL)      { init(a); if (str) *this = str; }
    explicit            String          (const char c, Allocator* a = NULL)         { init(a); char s[2]={c,0x00}; *this=s; }
    explicit            String          (const int n, Allocator* a = NULL)          { init(a); char tmp[16]; sprintf(tmp,"%d",n);  *this = tmp; }
    explicit            String          (const unsigned int n, Allocator* a = NULL) { init(a); char tmp[16]; sprintf(tmp,"%u",n);  *this = tmp; }
    explicit            String          (const float n, Allocator* a = NULL)        { init(a); char tmp[32]; sprintf(tmp,"%f",n);  *this = tmp; }
    explicit            String          (const double n, Allocator* a = NULL)       { init(a); char tmp[64]; sprintf(tmp,"%f",n); *this = tmp; }
#if defined(_WIN64)
    explicit            String          (const size_t n)                            { init(); char tmp[16]; sprintf(tmp,"%d",n);  *this = tmp; }
#endif
                        ~String         (void)                                      { clear(); }

    String&             operator=       (const char* str);
    String&             operator=       (const String& s)                           { if (this != &s) *this = s.m_string; return *this; }
    String&             operator+=      (const char c)                              { char s[2] = {c,0x00}; *this += s; return *this; }
    String&             operator+=      (const char* str);
    String&             operator+=      (const String& s)                           { *this += s.m_string; return *this; }
    bool                operator==      (const char* s) const                       { UMBRA_ASSERT(s); return !strcmp(m_string,s); }
    bool                operator!=      (const char* s) const                       { return !(*this==s); }
    bool                operator==      (const String& s) const                     { if(s.m_len!=m_len) return false; return strcmp(s.m_string,m_string)==0 ? true : false; }
    bool                operator!=      (const String& s) const                     { return *this==s ? false : true; }
    bool                operator<       (const String& s) const                     { return strcmp(m_string,s.m_string)<0 ? true : false; }
    bool                operator>       (const String& s) const                     { return strcmp(m_string,s.m_string)>0 ? true : false; }

    char                operator[]      (int i) const                               { UMBRA_ASSERT(i>=0 && i<m_len); return m_string[i]; }
    char&               operator[]      (int i)                                     { UMBRA_ASSERT(i>=0 && i<m_len); return m_string[i]; }

    const char*         toCharPtr       (void) const                                { return !isNull() ? (char*)m_string : (NULL); }

    bool                contains        (const String &s) const                     { return strstr( m_string,s.m_string )!=NULL ? true : false; }

    bool                isEmpty         (void) const                                { return (m_len==1) ? true : false; }
    int                 length          (void) const                                { return m_len-1; }

    int                 find            (const String &s, int i=0) const;
    int                 findFirst       (const String &s) const;
    int                 findLast        (const String &s) const;

    bool                endsWith        (const String &suffix) const                { return findLast(suffix) == length() - suffix.length(); }
    bool                startsWith      (const String &prefix) const                { return findFirst(prefix) == 0; }

    bool                remove          (const String &s);
    void                remove          (int i1, int i2);
    int                 removeAll       (const String &s);
    int                 removeFromHead  (char c);
    int                 removeFromTail  (char c);
    String&             trimWhiteSpace  (void);
    void                wordWrap        (int width, Array<String>& lines) const;

    Array<String>       split           (const String &sep, bool includeEmpty) const;
    Array<String>       splitToLines    () const;
    Array<String>       pathSplit       () const;

    void                clear           (void)                                      { if (!isNull()) UMBRA_DELETE_ARRAY(m_string); init(getAllocator()); }
    bool                replace         (const String &sub, const String &s);
    int                 replaceAll      (const String &sub, const String &s)        { bool v; int c=0; do{ v=replace(sub,s); c+=v; }while(v); return c; }
    void                replace         (const String &s, int i=0);
    void                insert          (const String &s, int i=0);

    void                upper           (void)                                      { for( int i=0;i<m_len-1;i++ ) m_string[i] = (char)toupper(m_string[i]); }
    void                lower           (void)                                      { for( int i=0;i<m_len-1;i++ ) m_string[i] = (char)tolower(m_string[i]); }

    static String       separateInt     (const int n, const char separator);
    static String       formatSize      (size_t size, Allocator* a = NULL); // byte precision ("1 MB 2 kB 345 B")
    static String       formatSize1k    (size_t size, Allocator* a = NULL); // round up to 1k ("1 MB 3 kB")

    String              getPath         (void) const;
    String              getFileName     (void) const;

#if UMBRA_ARCH != UMBRA_SPU
    /// Read the entire contents of an input stream into the string.
    void                readInputStream (InputStream& in);
    void                readFile        (const String& filename);
#endif

    static void         selfTest        (void);
private:
    int                 findPathSep     (void) const;
    void                init            (Allocator* a = NULL)                       { setAllocator(a), m_dummy = 0; m_string = &m_dummy; m_len = 1; }
    bool                isNull          (void) const                                { return (m_string == &m_dummy); }

    char                m_dummy;                    //!< dummy character containing ascii zero
    int                 m_len;                      //!< length of array
    char*               m_string;                   //!< array of characters
};

inline String operator+( const String &s1, const String &s2 )                       { String s(s1); s+=s2; return s; }

template <> inline unsigned int getHashValue (const String& s) // replace this with a better one
{
    uint32 val = 0;
    for (int i = s.length() - 1; i >= 0; i--)
        val = (val>>13)^(val<<19)^s[i];
    return val;
}

OutputStream& operator<<(OutputStream& out, const String& s);

} // namespace Umbra

//------------------------------------------------------------------------
#endif // UMBRASTRING_HPP
