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
 * \brief   String class implementation (short functions are in .hpp)
 *
 */

#include "umbraString.hpp"
#include "umbraArray.hpp"
#include "umbraFileStream.hpp"

using namespace std;

// Re-implemented strncpy because code below incorrectly assumes that pointers may overlap.
static char* strncpy2(char* dst, const char* src, size_t n)
{
    n = Umbra::min2(n, strlen(src)+1);
    memmove(dst, src, n);
    return dst;
}

namespace Umbra
{

String::String  (const String& s, int i1, int i2)
{
    UMBRA_ASSERT(i1<i2 && i2<s.m_len);
    init();
    int dstSize = i2-i1+2;
    char* str=UMBRA_NEW_ARRAY(char, dstSize);
    //strncpy_s(str, dstSize, &s.m_string[i1],i2-i1);
    strncpy2(str, &s.m_string[i1],i2-i1);
    str[i2-i1]=0;
    *this=str; UMBRA_DELETE_ARRAY(str);
}

String::String (const char* s, int len)
{
    init();
    m_len = len + 1;
    m_string = UMBRA_NEW_ARRAY(char, m_len);
    memcpy(m_string, s, len);
    m_string[len] = 0;
}

String String::separateInt(int n, const char separator)
{
    String temp;

    if(n < 0)
    {
        temp += "-";
        n *= -1;
    }

    bool first = true;
    char tmp[16];

    do
    {
        if(first)
            first = false;
        else
            temp = String(separator) + temp;

        tmp[0] = '\0';

        if( n/1000 > 0)
            sprintf(tmp,"%03d",n%1000);
        else
            sprintf(tmp,"%d",n%1000);
        temp = tmp + temp;
        n /= 1000;
    } while(n > 0);

    return temp;
}

String String::formatSize1k(size_t s, Allocator* a)
{
    String res(a);
    s = (s + 1023) / 1024;
    size_t kb = s % 1024;
    size_t mb = s / 1024;
    if (mb > 0)
        res += String((unsigned int)mb, a) + String(" MB ", a);
    res += String((unsigned int)kb, a) + String(" kB", a);
    return res;
}

String String::formatSize(size_t s, Allocator* a)
{
    String res(a);
    size_t b  = s % 1024;
    s/= 1024;
    size_t kb = s % 1024;
    size_t mb = s / 1024;
    if (mb > 0)
        res += String((unsigned int)mb, a) + String(" MB ", a);
    if (kb > 0)
        res += String((unsigned int)kb, a) + String(" kB ", a);
    if (b > 0 || (s == 0))
        res += String((unsigned int)b, a) + String(" B", a);
    return res;
}

String& String::operator=   (const char* str)
{
    if (str != m_string)
    {
        clear();
        if (str && *str)
        {
            m_len=(int)strlen(str)+1;
            if (!isNull())
                UMBRA_DELETE_ARRAY(m_string); // Is this correct? Added after a Bounds Checker run. [Kalle]
            m_string=UMBRA_NEW_ARRAY(char, m_len);
            //strcpy_s(m_string, m_len, str);
            strcpy(m_string, str);
        }
    }
    return *this;
}

String& String::operator+=  (const char* str)
{
    if (!str || !(*str))
        return *this;

    int l=m_len+(int)strlen(str);
    char* s=UMBRA_NEW_ARRAY(char, l);
    strcpy(s, m_string);
    strcpy(&s[m_len-1], str);
    clear();
    m_string=s;
    m_len=l;
    return *this;
}


int     String::find        (const String &s, int i) const
{
    UMBRA_ASSERT(i<m_len);
    const char* str=strstr( &m_string[i],s.m_string );
    return str ? (int)(uptr(str)-uptr(m_string)) : -1;
}

int     String::findFirst   (const String &s) const
{
    const char* str=strstr( m_string,s.m_string );
    return str ? (int)(uptr(str)-uptr(m_string)) : -1;
}

int     String::findLast    (const String &s) const
{
    int i2,i;
    i=i2=find(s);
    while(i!=-1 && i<length())
    {
        i2=i;
        i=find(s,i+1);
    }
    return i2;
}

/* NOTE: removed due to public pressure
String  String::getExtension(void) const
{
    int i=findLast(".");
    return i!=-1 ? String(*this,i+1,length()) : String();
}
*/


bool String::remove         (const String &s)
{
    int i=find(s);
    if(i==-1)
        return false;
    remove(i,i+s.length());
    m_len=(int)strlen(m_string)+1;
    return true;
}

void String::remove         (int i1, int i2)
{
    if(i1==i2) return;
    UMBRA_ASSERT(i1<=i2 && i2<m_len);
    strncpy2(&m_string[i1],&m_string[i2],m_len-i2);
    m_len -= i2 - i1;
}

int String::removeAll       (const String &s)
{
    bool v;
    int c=0;

    do
    {
        v=remove(s);
        c+=v;
    }while(v);

    m_len=(int)strlen(m_string)+1;
    return c;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Removes first characters if they are given character
 * \param   c   Character to remove
 * \return  Count of characters actually removed.
 *//*-------------------------------------------------------------------*/

int String::removeFromHead  (char c)
{
    int i=0;

    for(i=0;i<length();i++)
    {
        if(m_string[i]!=c)
            break;
    }

    if(i==0)
        return 0;

    *this = (i==length()) ? String("") : String(*this,i,length());
    return i;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Removes last characters if they are given character
 * \param   c   Character to remove
 * \return  Count of characters actually removed.
 *//*-------------------------------------------------------------------*/

int String::removeFromTail  (char c)
{
    int l=length();
    int i=l-1;

    for(;i>=0;i--){ if(m_string[i]!=c) break; }

    if(i==l-1)
        return 0;

    *this = (i<0) ? String("") : String(*this,0,i+1);
    return l-1-i;
}

/*-------------------------------------------------------------------*//*!
 * \brief   Removes white space characters (space and tab) from the
 *          start and the end of the String.
 * \return  Reference to self.
 *//*-------------------------------------------------------------------*/

String& String::trimWhiteSpace(void)
{
    int count = removeFromHead(' ');
    count += removeFromHead('\t');
    count += removeFromTail(' ');
    count += removeFromTail('\t');
    return *this;
}

void String::wordWrap(int width, Array<String>& lines) const
{
    Array<char> lineBuf;
    Array<char> wordBuf;
    for (int i = 0; i < length(); i++)
    {
        int c = (int)(operator[](i));
        if (c == '\n' || c == ' ')
        {
            for (int j = 0; j < wordBuf.getSize(); j++)
                lineBuf.pushBack(wordBuf[j]);
            wordBuf.reset(0);

            if (c == '\n')
            {
                lineBuf.pushBack('\0');
                lines.pushBack(String(lineBuf.getPtr()));
                lineBuf.reset(0);
            } else
            {
                lineBuf.pushBack((char)c);
            }
        }
        else
        {
            if (lineBuf.getSize() + wordBuf.getSize() + 1 > width)
            {
                if (lineBuf.getSize() > 0)
                {
                    lineBuf.pushBack('\0');
                    lines.pushBack(String(lineBuf.getPtr()));
                    lineBuf.reset(0);
                } else if (wordBuf.getSize() > 0)
                {
                    for (int j = 0; j < wordBuf.getSize(); j++)
                        lineBuf.pushBack(wordBuf[j]);
                    lineBuf.pushBack('\0');
                    lines.pushBack(String(lineBuf.getPtr()));
                    lineBuf.reset(0);
                    wordBuf.reset(0);
                }
            }
            wordBuf.pushBack((char)c);
        }
    }
    for (int i = 0; i < wordBuf.getSize(); i++)
    {
        lineBuf.pushBack(wordBuf[i]);
    }
    if (lineBuf.getSize())
    {
        lineBuf.pushBack('\0');
        lines.pushBack(String(lineBuf.getPtr()));
    }
}

Array<String> String::split(const String &sep, bool includeEmpty) const
{
    int begin = 0;
    Array<String> result;
    while (begin < length())
    {
        int end = find(sep, begin);
        if (end < 0)
        {
            result.pushBack(String(*this, begin, length()));
            break;
        }
        if (end == begin)
        {
            if (includeEmpty)
                result.pushBack(String(""));
            begin = end + sep.length();
        }
        if (end > begin)
        {
            result.pushBack(String(*this, begin, end));
            begin = end + sep.length();
        }
    }
    return result;
}

Array<String> String::splitToLines() const
{
    // Remove all carriage returns in case the string uses Windows linebreaks.
    String noCR(*this);
    noCR.removeAll("\r");
    // Then return the string split on line feed.
    return noCR.split("\n", false);
}

Array<String> String::pathSplit() const
{
    // Convert Windows-style path separators to Unix-style.
    String normalized(*this);
    normalized.replaceAll("\\", String("/"));

    return normalized.split("/", false);
}

bool String::replace(const String &sub, const String &s)
{
    int i=find(sub);
    if(i==-1)
        return false;

    remove( i,i+sub.length() );
    insert( s,i );
    return true;
}


void String::replace(const String &s, int i)
{
    UMBRA_ASSERT(i+s.m_len<=m_len);
    strncpy2(&m_string[i],s.m_string,s.m_len-1);
    m_len=(int)strlen(m_string)+1;
}


void String::insert (const String &s, int i)
{
    UMBRA_ASSERT(i<m_len);
    String s1;
    if(i==0) { s1=s+*this; }
    else if(i==length()) { s1=*this+s; }
    else
    {
        s1  = String(*this,0,i);
        s1 += s;
        s1 += String(*this,i,length());
    }
    *this = s1;
}

int String::findPathSep() const
{
    int pos = findLast("/");
    if (pos < 0)
        pos = findLast("\\");
    return pos;
}

String String::getPath() const
{
    int pos = findPathSep();
    if (pos >= 0)
        return String(*this, 0, pos + 1);
    else
        return String();
}

String String::getFileName() const
{
    int pos = findPathSep();
    if (pos >= 0)
        return String(*this, pos + 1, m_len - 1);
    else
        return String(*this);
}

#if UMBRA_ARCH != UMBRA_SPU
void String::readInputStream(InputStream& in)
{
    UINT8 buf[1024];
    for (;;)
    {
        UINT32 bytes_read = in.read(buf, sizeof(buf) - 1);
        // String terminator.
        buf[bytes_read] = 0;
        *this += (const char*)buf;
        if (bytes_read != sizeof(buf) - 1)
            break;
    }
}

void String::readFile(const String& filename)
{
    FileInputStream in(filename.toCharPtr());
    readInputStream(in);
}
#endif

void String::selfTest   (void)
{
    String  s1("pullo");
    String  s2 = s1 + " " + "karhua";
    s2.replace("karhu","olvi");

    UMBRA_ASSERT(s2 == "pullo olvia");

    String  s3;
    UMBRA_ASSERT(s3.toCharPtr()==NULL);
    s3 = "hox";
    UMBRA_ASSERT(s3.toCharPtr()!=NULL);
    s3 = "";
    UMBRA_ASSERT(s3.toCharPtr()==NULL);     // NOTE: empty string equals NULL string
    s3 = NULL;
    UMBRA_ASSERT(s3.toCharPtr()==NULL);

    s2 += s3;
    UMBRA_ASSERT(s2 == "pullo olvia");

    UMBRA_ASSERT(String("/foobar").getFileName() == String("foobar"));
    UMBRA_ASSERT(String("xxx\\yyy\\foobar").getFileName() == String("foobar"));
    UMBRA_ASSERT(String("foobar").getFileName() == String("foobar"));

    UMBRA_ASSERT(String("/foobar").getPath() == String("/"));
    UMBRA_ASSERT(String("xxx\\yyy\\foobar").getPath() == String("xxx\\yyy\\"));
    UMBRA_ASSERT(String("foobar").getPath() == String(""));
}


OutputStream& operator<<(OutputStream& out, const String& s)
{
    out.write(s.toCharPtr(), s.length());
    return out;
}

}
