#ifndef UMBRAPROPERTYFILE_HPP
#define UMBRAPROPERTYFILE_HPP

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
 * \brief   Umbra Property File
 *
 */

namespace Umbra
{

/*----------------------------------------------------------------------*//*!
 * \brief   PropertyFile
 *
 *          PropertyFile class gives an easy access to files containing text
 *          formed in key-value pairs separated by equality signs.
 *          Example file could look like this:
 *
 *          # foobarrabbiini
 *          foo=bar
 *                  bar     =    bar
 *          numfoo = 5
 *          floatfoo = 0.456
 *          bad =food # comment
 *          new = val u e
 *          ### LATEST WRITE: Tue Jun 04 13:26:25 2002
 *
 *          # is the quote character. Quote continues to the end of the line.
 *          White space characters (space and tab) are removed from the both sides
 *          of the key or the value. (e.g. tabbed and spaced new = val u e pair yields
 *          key "new" and value "val u e", without spacing in the ends, but mid string
 *          spacing is preserved.
 *  \todo   BUG: Does not use first character of the key!
 *//*----------------------------------------------------------------------*/

class PropertyFile
{
public:


                    PropertyFile    (const char* filename, bool readOnly);
                    ~PropertyFile   (void);

    bool            load            (void);
    bool            save            (void) const;

    bool            containsKey     (const char* key) const;

    bool            getBool         (const char* key, bool& value) const { int foo = 0; if (getInt(key,foo)) { value = foo?true:false; return true; } else return false; }
    bool            getInt          (const char* key, int& value) const;
    bool            getFloat        (const char* key, float& value) const;
    const char*     getValue        (const char* key) const;
    void            setValue        (const char* key, const char* value);
    void            setInt          (const char* key, int value);
    void            setFloat        (const char* key, float value);
    void            setStrArray     (const char* key, const char ** value, char limiter, int nStrings);

    static void     selfTest        (bool removeFile);

private:
                    PropertyFile    (const PropertyFile&);      // not allowed!
    PropertyFile&   operator=       (const PropertyFile&);      // not allowed!

    struct ImpData;

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------

    char*           m_filename;
    ImpData*        m_data;
    bool            m_readOnly;
};


/*-----------------------------------------------------------------------*//*!
 * \brief   StringTokenizer
 *
 *          This helps processing lists contained in the property files.
 *          If user gives list like "red.blue,green", StringTokenizer
 *          helps to separate components.
 * \note    You can use String::trimWhiteSpace() remove extra white space
 *          from the tokens.
 *//*----------------------------------------------------------------------*/

class StringTokenizer
{
public:
                        StringTokenizer     (const char* string, char limiter);
                        ~StringTokenizer    (void);

    bool                hasNext             (void) const;
    const char*         nextToken           (void);

    static void         selfTest            (void);

private:
                        StringTokenizer     (const StringTokenizer&);       // not allowed!
    StringTokenizer&    operator=           (const StringTokenizer&);       // not allowed!

    void                seekNext            (void);

    //--------------------------------------------------------------------
    // member variables
    //--------------------------------------------------------------------
    char*               m_string;
    int                 m_strLen;
    char*               m_current;
    char*               m_nextStart;
    int                 m_nextLen;
    char                m_limiter;
};

} // namespace Umbra

#endif // UMBRAPROPERTYFILE_HPP
