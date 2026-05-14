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
 * \note usage documentation in the hpp file.
 * \todo Error codes.
 * \todo Removal of settings using NULL as value (Maybe done?)
 *
 */

#include "umbraPropertyFile.hpp"

#include "umbraPrivateDefs.hpp"
#if UMBRA_ARCH != UMBRA_SPU

#include "umbraFileStream.hpp"

#include <cstring>
#include <cstdlib>
#include <cstdio>
#include <ctime>

using namespace Umbra;
using namespace std;

namespace
{

struct CString
{
    CString() : ptr(0) {}
    CString(const char* s) : ptr((char*)s) {}

    operator char* () const { return ptr; }
    bool operator==(const CString& other) const { return strcmp(ptr, other.ptr) == 0; }

    char* ptr;
};

template <class Val> inline int testEquals(const Val& a, const Val& b)
{
    return a == b;
}

template <class Key, class Value>
class HashMap
{
public:

    enum
    {
        NIL = -1                                // internal enumeration for representing a NULL link
    };

                                HashMap             (void);
                                ~HashMap            (void);
    void                        insert              (const Key& s, const Value& d);
    void                        remove              (const Key& s);
    void                        remove              (const Key& s, const Value& d);
    void                        removeWithReturn    (const Key& s, const Value& d, Key& retS, Value& retD);
    Value                       get                 (const Key& s) const;
    bool                        get                 (const Key& s, Value& d) const;
    bool                        contains            (const Key& s) const;
    bool                        contains            (const Key& s, const Value& d) const;
    void                        removeAll           (void);
    int                         getSize             (void) const;


    void                        startIteration      (void);
    bool                        hasNext             (void);
    void                        next                (void);
    Value                       getCurrentValue     (void) const;
    Key                         getCurrentKey       (void) const;

private:
                                HashMap             (const HashMap&);   // not allowed
    HashMap&                    operator=           (const HashMap&);   // not allowed
    static inline unsigned int  getHashVal          (const Key& s, const unsigned int hashArraySize);

    void                        rehash              (void);

    struct Entry
    {
        int32       next;                   // next pointer in linked list
        Key         key;                    // key
        Value       value;                  // value
    };

    int32*                      hash;                       // hash pointers
    Entry*                      table;                      // allocation table
    int32                       first;                      // handle of first free entry
    int                         size;                       // size of hash table
    int32                       iterCurEntry;
    int                         iterIndex;
};


//------------------------------------------------------------------------
// Iterator implementation - completely thread unsafe. If you try to
// remove/insert while you are iterating, the results are unpredictable
//
// \review
//------------------------------------------------------------------------

template <class Key, class Value> inline void HashMap<Key,Value>::startIteration(void)
{
    iterCurEntry = NIL;
    iterIndex = 0;
}

template <class Key, class Value> inline bool HashMap<Key,Value>::hasNext(void)
{
    if (iterIndex == size) return false;
    if (iterCurEntry == NIL || table[iterCurEntry].next == NIL)
    {
        while (iterIndex < size && hash[iterIndex] == NIL) iterIndex++;
        return iterIndex < size;
    }
    else
    {
        return true;
    }
}

template <class Key, class Value> inline void HashMap<Key,Value>::next(void)
{
    UMBRA_ASSERT(iterIndex < size);
    if (iterCurEntry == NIL || table[iterCurEntry].next == NIL)
    {
        iterCurEntry = hash[iterIndex];
        iterIndex++;
    }
    else
    {
        iterCurEntry = table[iterCurEntry].next;
    }
}

template <class Key, class Value> inline Value HashMap<Key,Value>::getCurrentValue(void) const
{
    return table[iterCurEntry].value;
}

template <class Key, class Value> inline Key HashMap<Key,Value>::getCurrentKey(void) const
{
    return table[iterCurEntry].key;
}


//------------------------------------------------------------------------
// Implementation
//------------------------------------------------------------------------

template <class Key, class Value> inline unsigned int HashMap<Key,Value>::getHashVal    (const Key& s, const unsigned int hashArraySize)
{
    return Umbra::getHashValue(s) & (hashArraySize-1);
}

template <class Key, class Value> inline void HashMap<Key,Value>::insert    (const Key& s, const Value& d)
{
    if (first == NIL)                   // need to re-alloc and re-hash tables
        rehash();
    int32 h = first;
    first   = table[first].next;

    unsigned int hval   = getHashVal(s,size);
    table[h].key    = s;
    table[h].value  = d;
    table[h].next   = hash[hval];
    hash[hval]      = h;
}

template <class Key, class Value> inline int HashMap<Key,Value>::getSize (void) const
{
    return size;
}

template <class Key, class Value> inline void HashMap<Key,Value>::removeAll (void)
{
    for (int i = 0; i < size; i++)
    {
        int32 f = hash[i];
        if (f!=NIL)
        {
            int32 h = f;
            while (table[h].next != NIL)
                h = table[h].next;
            table[h].next = first;      // link to first free
            first = f;                  // link to beginning
            hash[i] = NIL;              // kill entry in hash
        }
    }
}

template <class Key, class Value> inline void HashMap<Key,Value>::remove    (const Key& s)
{
    unsigned int hval = getHashVal(s,size);
    int32  prev = NIL;
    int32   h    = hash[hval];

    while (h != NIL)
    {
        if (testEquals(table[h].key,s))
        {
            if (prev!=NIL)
                table[prev].next = table[h].next;
            else
                hash[hval] = table[h].next;
            table[h].next = first;
            first = h;
            return;
        }
        prev = h;
        h    = table[h].next;
    }
}

template <class Key, class Value> inline void HashMap<Key,Value>::remove (const Key& s, const Value& d)
{
    unsigned int hval = getHashVal(s,size);
    int32  prev = NIL;
    int32   h    = hash[hval];

    while (h != NIL)
    {
        if (testEquals(table[h].key,s) && testEquals(table[h].value,d))
        {
            if (prev!=NIL)
                table[prev].next = table[h].next;
            else
                hash[hval] = table[h].next;
            table[h].next = first;
            first = h;
            return;
        }
        prev = h;
        h    = table[h].next;
    }
}

template <class Key, class Value> inline void HashMap<Key,Value>::removeWithReturn (const Key& s, const Value& d, Key& retS, Value& retD)
{
    unsigned int hval = getHashVal(s,size);
    int32  prev = NIL;
    int32   h    = hash[hval];

    while (h != NIL)
    {
        if (testEquals(table[h].key,s) && testEquals(table[h].value,d))
        {
            retS = table[h].key;
            retD = table[h].value;
            if (prev!=NIL)
                table[prev].next = table[h].next;
            else
                hash[hval] = table[h].next;
            table[h].next = first;
            first = h;
            return;
        }
        prev = h;
        h    = table[h].next;
    }
}

template <class Key, class Value> inline Value HashMap<Key,Value>::get (const Key& s) const
{
    int32  h = hash[getHashVal(s,size)];
    while (h!=NIL)
    {
        if (testEquals(table[h].key,s))
            return table[h].value;
        h = table[h].next;
    }
    return (Value)(NULL);
}

template <class Key, class Value> inline bool HashMap<Key,Value>::get (const Key& s, Value& d) const
{
    int32  h = hash[getHashVal(s,size)];
    while (h!=NIL)
    {
        if (testEquals(table[h].key,s))
        {
            d = table[h].value;
            return true;
        }
        h = table[h].next;
    }
    return false;
}

template <class Key, class Value> inline bool HashMap<Key,Value>::contains (const Key& s) const
{
    int32  h = hash[getHashVal(s,size)];
    while (h!=NIL)
    {
        if (testEquals(table[h].key,s))
            return true;
        h = table[h].next;
    }
    return false;
}

template <class Key, class Value> inline bool HashMap<Key,Value>::contains (const Key& s, const Value& d) const
{
    int32  h = hash[getHashVal(s,size)];
    while (h!=NIL)
    {
        if (testEquals(table[h].key,s) && testEquals(table[h].value,d))
            return true;
        h = table[h].next;
    }
    return false;
}

template <class Key, class Value> inline void HashMap<Key,Value>::rehash    (void)
{
    size_t newSize = size*2;
    if (newSize < 4)
        newSize = 4;

    Entry  *newTable = UMBRA_NEW_ARRAY(Entry, newSize);
    int32 *newHash   = UMBRA_NEW_ARRAY(int32, newSize);

    int cnt = 0;
    int i;

    for (i = 0; i < (int)newSize; i++)
    {
        newTable[i].next    = NIL;
        newHash[i]          = NIL;
    }

    if (size)                                           // if we have existing data, it needs to be rehashed
    {
        for (i = 0; i < (int)size; i++)                     // step through each old hash set
        {
            int32   h = hash[i];
            while (h != NIL)
            {
                unsigned int hVal       = getHashVal(table[h].key, (unsigned int)newSize);
                newTable[cnt].key   = table[h].key;
                newTable[cnt].value = table[h].value;
                newTable[cnt].next  = newHash[hVal];
                newHash[hVal]       = cnt;
                cnt++;
                h = table[h].next;
            }
        }
        UMBRA_DELETE_ARRAY(hash);
        UMBRA_DELETE_ARRAY(table);
    }

    for (i = cnt; i < (int)newSize; i++)
        newTable[i].next = i+1;
    newTable[newSize-1].next = NIL;

    first   = cnt;
    hash    = newHash;
    table   = newTable;
    size    = (int)newSize;
}

template <class Key, class Value> inline HashMap<Key,Value>::HashMap () : hash(0),table(0),first(NIL),size(0),iterCurEntry(NIL),iterIndex(0)
{
    rehash ();
}

template <class Key, class Value> inline HashMap<Key,Value>::~HashMap ()
{
    UMBRA_DELETE_ARRAY(hash);
    UMBRA_DELETE_ARRAY(table);
}

} // Empty namespace

namespace Umbra
{
template <> inline UINT32 getHashValue (const CString& cs)
{
    UINT32 val = 0;
    const char* ptr = cs;
    while (*ptr)
        val = (val>>13)^(val<<19)^(*(ptr++));
    return val;
}
}

//------------------------------------------------------------------------
// Implementation data structure. Used to hide some changes in the
// implementation.
//------------------------------------------------------------------------

struct PropertyFile::ImpData
{
public:
    ::HashMap<CString,CString> pairs;
    ImpData(void): pairs()              {};
private:
    ImpData(const ImpData&);
    ImpData& operator=(const ImpData&);
};


/*----------------------------------------------------------------------*//*!
 * \brief   PropertyFile constructor
 *
 *          Creates an instance coupled with specified file.
 *
 * \param   filename    name of the file that is used to load and save the
 *                      data.
 * \param   readOnly    if true, properties cannot be written to disk.
 * \note
 *//*-------------------------------------------------------------------*/

PropertyFile::PropertyFile(const char* filename, bool readOnly):
m_readOnly(readOnly)
{
    UMBRA_ASSERT(filename != NULL);
    m_filename = UMBRA_NEW_ARRAY(char, strlen(filename)+1);
    strcpy(m_filename, filename);
    m_data = UMBRA_NEW(ImpData);
}

PropertyFile::~PropertyFile(void)
{
    m_data->pairs.startIteration();
    while (m_data->pairs.hasNext())
    {
        m_data->pairs.next();
        UMBRA_DELETE_ARRAY(m_data->pairs.getCurrentValue().ptr);
        UMBRA_DELETE_ARRAY(m_data->pairs.getCurrentKey().ptr);
    }
    UMBRA_DELETE_ARRAY(m_filename);
    UMBRA_DELETE(m_data);
}

static bool isWhiteSpace(char f)
{
    return ( f == ' ' || f == '\t');
}


static char* newString(const char* old, int nChars)
{
    UMBRA_ASSERT(nChars >= 0);
    UMBRA_ASSERT(old != NULL);
    char* res = UMBRA_NEW_ARRAY(char, nChars+1);
    strncpy(res,old,nChars);
    res[nChars] = '\0';
    return res;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Loads properties.
 *
 *          Loads properties from the file specified at construction.
 *
 * \return  true if succeeded, false otherwise.
 * \note
 *//*----------------------------------------------------------------------*/

bool PropertyFile::load(void)
{
    FileInputStream propFileIn(m_filename);
    StreamReader propFile(&propFileIn);

    Array<char> readBuffer;
    int         index=0;

    if (!propFileIn.isOpen()) return false;

    while (propFile.readLine(readBuffer))
    {
        // length without terminating '0'
        int len = readBuffer.getSize() - 1;
        index = 0;

        //----------------------------------------------------------------
        // Remove newlines
        //----------------------------------------------------------------
        while (len && (readBuffer[len-1] == '\n' || readBuffer[len-1] == '\r'))
        {
            readBuffer[--len] = '\0';
        }

        //----------------------------------------------------------------
        // Read white space away.
        //----------------------------------------------------------------
        while (index < len && isWhiteSpace(readBuffer[index])) index++;

        UMBRA_ASSERT(!isWhiteSpace(readBuffer[index]));

        //----------------------------------------------------------------
        // Check for comments
        //----------------------------------------------------------------
        if (readBuffer[index] != '#' && readBuffer[index] != '\0')
        {
            UMBRA_ASSERT(readBuffer[index] != '\0');
            //------------------------------------------------------------
            // Find he key.
            //------------------------------------------------------------

            int lastNonWhiteSpace   = -1;
            int keyStart            = index;

            while (index < len && readBuffer[index] != '=')
            {
                UMBRA_ASSERT(readBuffer[index] != '#');
                if (!isWhiteSpace(readBuffer[index])) lastNonWhiteSpace = index;
                index++;
            }
            if (index == len)
                return false;

            UMBRA_ASSERT(index > keyStart);
            UMBRA_ASSERT(readBuffer[index] == '=');
            UMBRA_ASSERT(keyStart <= lastNonWhiteSpace);

            char* key = newString(&readBuffer[keyStart],lastNonWhiteSpace-keyStart+1);

            index++;

            //------------------------------------------------------------
            // Find the value
            //------------------------------------------------------------

            int valueBegin = -1;
            lastNonWhiteSpace = valueBegin - 1;
            while (index < len && readBuffer[index] != '#')
            {
                if (!isWhiteSpace(readBuffer[index]))
                {
                    //----------------------------------------------------
                    // Remove white space at the start.
                    //----------------------------------------------------
                    if (valueBegin == -1) valueBegin = index;
                    lastNonWhiteSpace = index;
                }
                index++;
            }
            UMBRA_ASSERT(readBuffer[index] == '\0' || readBuffer[index] == '#');
            if (lastNonWhiteSpace < valueBegin)
                return false;

            UMBRA_ASSERT(lastNonWhiteSpace >= valueBegin);
            UMBRA_ASSERT(readBuffer[valueBegin] != '=');

            char* value = newString(&readBuffer[valueBegin],lastNonWhiteSpace-valueBegin+1);

            m_data->pairs.insert(key,value);
        }
    }
    return true;
}

/*----------------------------------------------------------------------*//*!
 * \brief   Gets value of the given key.
 *
 *          Gets value of the given key. Key compare is case sensitive.
 *
 * \param   key String giving the desired key.
 * \return  value in the property file. NULL if cannot be found.
 * \note    At the moment non-NULL response does not guarantee that string
 *          is otherwise non-empty.
 *//*------------------------------------------------------------------------*/

const char* PropertyFile::getValue(const char* key) const
{
    UMBRA_ASSERT(key != NULL);
    return m_data->pairs.get((char* const&) key);
}

/*----------------------------------------------------------------------*//*!
 * \brief   Gets int.
 *
 *          Gets a value and formats it to an integer. If value for the given
 *          key cannot be found it is interpreted as failure.
 *
 * \param   key     Name of the key of an integer property
 * \param   success Is set true if succeeded, false otherwise. Can be NULL.
 * \return  int constructed from value, 0 on failure
 * \note    This uses atoi, which returns 0 in the case of failure. This method
 *          checks whether the value is exactly "0" or not. This desides whether
 *          the result was failure or not.
 *//*------------------------------------------------------------------------*/

bool PropertyFile::getInt(const char* key, int& value) const
{
    UMBRA_ASSERT(key != NULL);
    char* val = m_data->pairs.get((char* const&)key);
    if (val == NULL)
    {
        return false;
    }
    else
    {
        int res = atoi(val);;
        if (res == 0)
        {
            bool success = (strcmp("0",val) == 0) ? true : false;
            if (success)
            {
                value = res;
            }
            return success;
        }
        value = res;
        return true;
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief   Gets float.
 *          Gets a value and formats it to a float. If value cannot be found
 *          for the given key it is considered as failure.
 * \param   key     Name of the key of a float property
 * \param   success Is set true if succeeded, false otherwise. Can be NULL
 * \return  float constructed from value, 0 on failure
 * \note    This uses atof, which returns 0 in the case of failure. This method
 *          checks wether the value is exactly "0" or "0.0" or not. This desides wether
 *          the result was failure or not.
 *//*----------------------------------------------------------------------*/

bool PropertyFile::getFloat(const char* key, float& value) const
{
    UMBRA_ASSERT(key != NULL);
    char* val = m_data->pairs.get((char* const&) key);
    if (val == NULL)
    {
        return false;
    }
    else
    {
        char * endPoint;
        float res = (float) strtod(val, &endPoint);

        //----------------------------------------------------------------
        // Check that the whole string was OK.
        //----------------------------------------------------------------
        if (endPoint != NULL && (unsigned)(endPoint-val) != strlen(val))
            return false;
        value = res;
        return true;
    }
}


/*----------------------------------------------------------------------*//*!
 * \brief   Sets a value
 *          Sets the value of the given property. This overwrites the possible
 *          old value. To remove a key value pair, set the value of the key to
 *          NULL.
 * \param   key     Name of the property or NULL to remove the key.
 * \param   value   Value of the property.
 * \note    Setting is not reflected on the disk, you must separately save
 *          the property file.
 *//*----------------------------------------------------------------------*/

void PropertyFile::setValue(const char* key, const char* value)
{
    UMBRA_ASSERT(key != NULL);
    char* actKey = NULL;
    char* actVal = NULL;
    if (m_data->pairs.contains((char* const&)key))
    {
        const char* val = m_data->pairs.get((char* const&) key);
        CString key2;
        CString value2;
        m_data->pairs.removeWithReturn((char* const&)key, (char* const&)val, key2, value2);
        actKey = key2;
        actVal = value2;
        UMBRA_DELETE_ARRAY(actVal);
        if (value == NULL)
            UMBRA_DELETE_ARRAY(actKey);
    }
    else
    {
        if (value != NULL)
            actKey = newString(key,(int)strlen(key));
    }
    if (value != NULL)
    {
        actVal = newString(value,(int)strlen(value));
        m_data->pairs.insert(actKey, actVal);
    }
}

/*----------------------------------------------------------------------*//*!
 * \brief   Saves the property file.
 *          Writes properties to disk. This tries to conserve all comments
 *          and file layout. If you align comments at the same line with
 *          key-value pairs with tabs, they will not be preserved. Appends
 *          also lastest write time to the file.
 * \return  true if succeeded, false otherwise.
 * \note
 *//*----------------------------------------------------------------------*/

bool PropertyFile::save(void) const
{
    if (m_readOnly) return false;

    UMBRA_ASSERT(!m_readOnly);

    HashMap<CString,int> doneCheck;
    Array<char> readBuffer;

    FileInputStream propFile(m_filename);
    MemOutputStream tempFileBuf;

    //--------------------------------------------------------------------
    // If we can find the old file, try to preserve it as well as we can.
    //--------------------------------------------------------------------
    if (propFile.isOpen())
    {
        StreamReader r(&propFile);
        StreamWriter tempFile(&tempFileBuf);

        //-------------------------------------------------------------------
        // First find possible old key positions.
        //--------------------------------------------------------------------
        while (r.readLine(readBuffer))
        {
            // length without terminating '0'
            int len = readBuffer.getSize() - 1;
            int index = 0;

            //----------------------------------------------------------------
            // Remove newlines
            //----------------------------------------------------------------
            while (len && (readBuffer[len-1] == '\n' || readBuffer[len-1] == '\r'))
            {
                readBuffer[--len] = '\0';
            };

            //------------------------------------------------------------
            // Transfer whitespace
            //------------------------------------------------------------
            while(index < len && isWhiteSpace(readBuffer[index]))
            {
                tempFile.put(readBuffer[index]);
                index++;
            }

            UMBRA_ASSERT(!isWhiteSpace(readBuffer[index]) || (index==len));
            //------------------------------------------------------------
            // If it is a full comment line, copy it.
            //------------------------------------------------------------
            if (readBuffer[index] == '#')
            {

                //--------------------------------------------------------
                // ... but exclude our fine little latest write message.
                //--------------------------------------------------------
                if (strncmp(&(readBuffer[index]),"### LATEST WRITE:", strlen("### LATEST WRITE:")) != 0)
                {
                    tempFile.putStr(&readBuffer[index]);
                }
            }
            else if (len > 1)
            {
                //--------------------------------------------------------
                // Process key-value pair.
                //--------------------------------------------------------
                int keyStart = index;
                int keyStop = keyStart-1;
                while (index < len && readBuffer[index] != '=')
                {
                    if (!isWhiteSpace(readBuffer[index])) keyStop = index;
                    index++;
                }
                UMBRA_ASSERT(index < len-1);
                UMBRA_ASSERT(index > keyStart);
                //tempFile.writeChar('=');

                char* key = newString(&readBuffer[keyStart], keyStop-keyStart+1);
                //--------------------------------------------------------
                // If we have a value for the key, replace. Otherwise just
                // copy.
                //--------------------------------------------------------
                if (m_data->pairs.contains((char* const&) key))
                {
                    doneCheck.insert(key, 1);
                    const char* value = m_data->pairs.get((char* const&) key);
                    if (value != NULL)
                    {
                        tempFile.putStr(key);
                        tempFile.putStr(" = ");
                        tempFile.putStr(value);

                        //----------------------------------------------------
                        // Check that value is not postfixed with comment.
                        // If it is, copy the comment.
                        //----------------------------------------------------
                        char* commentStart = strchr(readBuffer.getPtr() ,'#');
                        if (commentStart != NULL)
                        {
                            tempFile.putStr(" ");
                            tempFile.putStr(commentStart);
                        }

                        tempFile.putStr("\n");
                    }

                    //----------------------------------------------------
                    // Will not delete key, as it is inserted in the Hash.
                    //----------------------------------------------------

                }
                else
                {

                    //----------------------------------------------------
                    // We do not need the key anymore.
                    //----------------------------------------------------
                    UMBRA_DELETE_ARRAY(key);
                    /*while (index < len-1)
                    {
                        tempFile.writeChar(readBuffer[index]);
                        index++;
                    }*/
                }
            }
            //--------------------------------------------------------
            // Remember to add that linefeed!
            //--------------------------------------------------------
        }
        propFile.close();
    }

    FileOutputStream newPropFile(m_filename);
    if (!newPropFile.isOpen())
        return false;
    StreamWriter outFile(&newPropFile);

    //--------------------------------------------------------------------
    // Transfer temporary file to the correct property file.
    //--------------------------------------------------------------------

    if (tempFileBuf.getSize())
        outFile.put((const char*)tempFileBuf.getPtr(), tempFileBuf.getSize());


    //--------------------------------------------------------------------
    // Now propFile is at the position where we can append rest
    // of the stuff.
    // We must iterate through all key-value pairs and write those that
    // are not already written.
    //--------------------------------------------------------------------
    m_data->pairs.startIteration();
    while (m_data->pairs.hasNext())
    {
        m_data->pairs.next();
        if (!doneCheck.contains(m_data->pairs.getCurrentKey()))
        {
            outFile.putStr(m_data->pairs.getCurrentKey());
            outFile.putStr(" = ");
            outFile.putStr(m_data->pairs.getCurrentValue());
            outFile.putStr("\n");
        }
    }


    //--------------------------------------------------------------------
    // Delete string pointers from the doneCheck
    //--------------------------------------------------------------------
    doneCheck.startIteration();
    while(doneCheck.hasNext())
    {
        doneCheck.next();
        UMBRA_DELETE_ARRAY(doneCheck.getCurrentKey().ptr);
    }


    time_t clock;
    time(&clock);

    //--------------------------------------------------------------------
    // localtime uses static buffer, so this does not leak!
    //--------------------------------------------------------------------
    struct tm* fulltime = localtime(&clock);

    //--------------------------------------------------------------------
    // asctime uses static buffer, so this does not leak!
    //--------------------------------------------------------------------
    outFile.putStr("### LATEST WRITE: ");
    outFile.putStr(asctime(fulltime));
    outFile.putStr("\n");

    return true;
}

void PropertyFile::setInt (const char* key, int value)
{
    char tmp[32];
    sprintf (tmp,"%d",value);
    setValue(key,tmp);
}

void PropertyFile::setFloat (const char* key, float value)
{
    char tmp[32];
    sprintf (tmp,"%g",value);
    setValue(key,tmp);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Sets a string array to a single value using given delimiter.
 *          Sets a string array to a single value using given delimiter.
 *          Then the value is easy to split in the reading phase using
 *          the StringTokenizer class.
 * \param   key         Key to receive the value.
 * \param   value       New value array.
 * \param   limiter     Limiter character inserted between values.
 * \param   nStrings    Number of strings in the array.
 * \note
 *//*-------------------------------------------------------------------*/

void PropertyFile::setStrArray(const char * key, const char ** value, char limiter, int nStrings)
{
    UMBRA_ASSERT(nStrings > 0);
    UMBRA_ASSERT(value != NULL);
    UMBRA_ASSERT(key != NULL);
    UMBRA_ASSERT(limiter != '\0');
    UMBRA_ASSERT(value[0] != NULL);

    Array<char> tempStr(nStrings*((int)strlen(value[0])));
    int index = 0;

    for (int i = 0; i < nStrings; i++)
    {
        UMBRA_ASSERT(value[i] != NULL);
        int strLen = (int)strlen(value[i]);
        tempStr.resize(strLen + 2 + index);
        strcpy(&tempStr[index], value[i]);
        index += strLen;
        tempStr[index++] = limiter;
        tempStr[index] = '\0';
    }

    //--------------------------------------------------------------------
    // Remove the last (redundant) limiter.
    //--------------------------------------------------------------------

    tempStr[index-1] = '\0';

    setValue(key, tempStr.getPtr());

}

/*-------------------------------------------------------------------*//*!
 * \brief   Tests wether this property file contains this key or not.
 * \param   key     Key to be tested.
 * \return  True if key is in the property file, false otherwise.
 *//*-------------------------------------------------------------------*/

bool PropertyFile::containsKey(const char* key) const
{
    return m_data->pairs.contains((char* const&)key);
}

/*----------------------------------------------------------------------*//*!
 * \brief   Self Test
 *          Runs a series of diagnostic tests on the PropertyFile. You can
 *          leave the file produced by the tests to the disk, so that you
 *          can further analyze it.
 * \param   removeFile  Should the property file used in the tests be removed, if true
 *                      the file is removed.
 * \note
 * \todo Should also check that comments are really there.
 *//*----------------------------------------------------------------------*/
void PropertyFile::selfTest(bool removeFile)
{

    const char * strs[3] = {"foo","bar","baabaa"};

    //--------------------------------------------------------------------
    // First test HashMap iteration.
    //--------------------------------------------------------------------

    HashMap<int,int> testMap;
    testMap.insert(1,4);
    testMap.insert(2,5);
    testMap.insert(3,6);
    UMBRA_ASSERT(testMap.get(1) == 4);
    UMBRA_ASSERT(testMap.get(2) == 5);
    UMBRA_ASSERT(testMap.get(3) == 6);
    testMap.startIteration();
    int count=0;
    while (testMap.hasNext())
    {
        testMap.next();
        UMBRA_ASSERT(testMap.getCurrentKey() >= 1 && testMap.getCurrentKey() <= 3);
        UMBRA_ASSERT(testMap.getCurrentValue() >= 4 && testMap.getCurrentValue() <= 6);
        UMBRA_ASSERT(testMap.getCurrentKey() + 3 == testMap.getCurrentValue());
        count++;
    }
    UMBRA_ASSERT(count == 3);


    //--------------------------------------------------------------------
    // Prepare test
    //--------------------------------------------------------------------
    FILE* testCreate = fopen("test.foobar", "w");
    UMBRA_ASSERT(testCreate != NULL);
    fprintf(testCreate, "# foobarrabbiini\n");
    fprintf(testCreate, "foo=bar\n");
    fprintf(testCreate, "\t \tbar \t=  \t foo\t \t\n");
    fprintf(testCreate, "numfoo = 5\n");
    fprintf(testCreate, "floatfoo = 0.456\n");
    fprintf(testCreate, "invalidFloat = abs.87987\n");
    fprintf(testCreate, "weirdFloat = 0.000000\n");
    fprintf(testCreate, "bad = food # comment\n");
    fprintf(testCreate, "spaceTest = space \t test me\n");
    fprintf(testCreate, "arrayTest = foo,bar,buuBuu\n");
    fprintf(testCreate, "\n");
    fflush(testCreate);
    fclose(testCreate);


    //--------------------------------------------------------------------
    // Init prop. file.
    //--------------------------------------------------------------------

    PropertyFile test("test.foobar", false);
    UMBRA_ASSERT(test.load());


    //--------------------------------------------------------------------
    // Test getters
    //--------------------------------------------------------------------

    UMBRA_ASSERT(!test.containsKey("Notherekey"));
    UMBRA_ASSERT(test.containsKey("foo"));
    UMBRA_ASSERT(test.getValue("foo") != NULL);
    UMBRA_ASSERT(strcmp(test.getValue("foo"),"bar") == 0);
    UMBRA_ASSERT(test.getValue("bar") != NULL);
    UMBRA_ASSERT(strcmp(test.getValue("bar"), "foo") == 0);
    int valInt=0;
    UMBRA_ASSERT(test.getInt("numfoo", valInt));
    UMBRA_ASSERT(valInt == 5);
    UMBRA_UNREF(valInt);
    float valFloat = 0.0f;
    UMBRA_UNREF(valFloat);
    UMBRA_ASSERT(test.getFloat("floatfoo", valFloat));
    UMBRA_ASSERT(valFloat == 0.456f);
    UMBRA_ASSERT(test.getValue("bad") != NULL);
    UMBRA_ASSERT(strcmp(test.getValue("bad"), "food") == 0);
    UMBRA_ASSERT(test.getValue("spaceTest") != NULL);
    UMBRA_ASSERT(strcmp(test.getValue("spaceTest"), "space \t test me") == 0);
    UMBRA_ASSERT(!test.getFloat("foo", valFloat));
    UMBRA_ASSERT(!test.getInt("foo",valInt));
    UMBRA_ASSERT(test.getValue("arrayTest") != NULL);
    UMBRA_ASSERT(!test.getFloat("invalidFloat", valFloat));
    valFloat = 100.f;
    UMBRA_ASSERT(test.getFloat("weirdFloat", valFloat));
    UMBRA_ASSERT(valFloat == 0.f);
    StringTokenizer tokens(test.getValue("arrayTest"), ',');
    int amount = 0;
    while (tokens.hasNext())
    {
        switch(amount)
        {
        case 0:
            UMBRA_ASSERT(strcmp(tokens.nextToken(),"foo") == 0);
            break;
        case 1:
            UMBRA_ASSERT(strcmp(tokens.nextToken(),"bar") == 0);
            break;
        case 2:
            UMBRA_ASSERT(strcmp(tokens.nextToken(),"buuBuu") == 0);
            break;
        default:
            UMBRA_ASSERT(false);
            break;
        }
        amount++;
    }
    UMBRA_ASSERT(amount == 3);
    //--------------------------------------------------------------------
    // Test set, unset and save.
    //--------------------------------------------------------------------

    test.setValue("bar", "bar");
    test.setValue("new", "value");
    test.setValue("floatfoo", NULL);
    test.setStrArray("arrayTest", &strs[0],',',3);
    test.setFloat("zerotest", 0);
    bool res = test.save();
    UMBRA_UNREF(res);
    UMBRA_ASSERT(res);
    PropertyFile test2("test.foobar", true);
    res = test2.load();
    UMBRA_ASSERT(res);
    UMBRA_ASSERT(!test2.save());
    UMBRA_ASSERT(strcmp(test2.getValue("bar"),"bar") == 0);
    UMBRA_ASSERT(test2.getValue("new") != NULL);
    UMBRA_ASSERT(strcmp(test2.getValue("new"), "value") == 0);

    //--------------------------------------------------------------------
    // Test getters again
    //--------------------------------------------------------------------

    UMBRA_ASSERT(test2.getValue("foo") != NULL);
    UMBRA_ASSERT(strcmp(test2.getValue("foo"),"bar") == 0);
    valInt=0;
    UMBRA_ASSERT(test.getInt("numfoo", valInt));
    UMBRA_ASSERT(valInt == 5);
    valFloat = 0.0f;
    UMBRA_ASSERT(test2.getValue("floatFoo") == NULL);
    UMBRA_ASSERT(!test2.containsKey("floatFoo"));
    UMBRA_ASSERT(!test2.getFloat("floatfoo", valFloat));
    UMBRA_ASSERT(valFloat == 0.0f);
    UMBRA_ASSERT(test2.getValue("bad") != NULL);
    UMBRA_ASSERT(strcmp(test2.getValue("bad"), "food") == 0);
    UMBRA_ASSERT(test2.getValue("spaceTest") != NULL);
    UMBRA_ASSERT(strcmp(test2.getValue("spaceTest"), "space \t test me") == 0);
    UMBRA_ASSERT(!test2.getFloat("foo",valFloat));
    UMBRA_ASSERT(!test2.getInt("foo", valInt));
    valFloat = 10.0f;
    UMBRA_ASSERT(test2.getFloat("zerotest", valFloat));
    UMBRA_ASSERT(valFloat == 0.f);

    StringTokenizer tokens2(test2.getValue("arrayTest"), ',');
    amount = 0;
    while (tokens2.hasNext())
    {
        switch(amount)
        {
        case 0:
            UMBRA_ASSERT(strcmp(tokens2.nextToken(),"foo") == 0);
            break;
        case 1:
            UMBRA_ASSERT(strcmp(tokens2.nextToken(),"bar") == 0);
            break;
        case 2:
            UMBRA_ASSERT(strcmp(tokens2.nextToken(),"baabaa") == 0);
            break;
        default:
            UMBRA_ASSERT(false);
            tokens2.nextToken(); // this is here so that the release build can run the self test without hanging.
            break;
        }
        amount++;
    }
    UMBRA_ASSERT(amount == 3);


    //--------------------------------------------------------------------
    // Do load save cycle for n times
    //--------------------------------------------------------------------

    PropertyFile* loopTest = NULL;

    for (int i = 0; i < 15; i++)
    {
        loopTest = UMBRA_NEW(PropertyFile, "test.foobar", false);
        loopTest->load();
        loopTest->save();
        UMBRA_DELETE(loopTest);
    }

    if (removeFile)
    {
        remove("test.foobar");
    }

}

StringTokenizer::StringTokenizer(const char* string, char limiter):
m_nextStart(NULL),

//------------------------------------------------------------------------
// This -1 is needed in the seekNext!
//------------------------------------------------------------------------
m_nextLen(-1),
m_limiter(limiter)
{
    if (string != NULL)
    {
        m_strLen = (int)strlen(string);
        m_string = newString(string, m_strLen);
        m_current = NULL;
        m_nextStart = m_string;
        seekNext();
    }
    else
    {
        m_string = NULL;
        m_current = NULL;
    }
}

StringTokenizer::~StringTokenizer(void)
{
    UMBRA_DELETE_ARRAY(m_string);
    UMBRA_DELETE_ARRAY(m_current);
}

bool StringTokenizer::hasNext(void) const
{
    return m_nextStart != NULL;
}

const char* StringTokenizer::nextToken(void)
{
    UMBRA_ASSERT(m_nextStart != NULL);
    UMBRA_ASSERT(m_nextLen >= 0 && m_nextLen <= m_strLen-(m_string-m_nextStart));
    UMBRA_DELETE_ARRAY(m_current);
    m_current = newString(m_nextStart, m_nextLen);
    seekNext();
    return m_current;
}

void StringTokenizer::seekNext(void)
{
    //----------------------------------------------------------------
    // Hop over the previous and seek next.
    //----------------------------------------------------------------
    m_nextStart += m_nextLen + 1;
    UMBRA_ASSERT((m_nextStart == m_string && m_nextLen == -1) || (m_nextStart != m_string && m_nextLen >= 0));
    char * nextToken = NULL;
    if (m_nextStart-m_string < m_strLen)
        nextToken = strchr(m_nextStart, m_limiter);
    if (nextToken == NULL)
    {
        m_nextLen = m_strLen-(int)(m_nextStart-m_string);
    }
    else
    {
        m_nextLen = (int)(nextToken-m_nextStart);
    }
    if (m_nextLen == -1)
    {
        m_nextStart = NULL;
        m_nextLen = 0;
    }
    UMBRA_ASSERT(m_nextLen >= 0 && (m_nextLen <= m_strLen-(m_string-m_nextStart) || m_nextStart == NULL));
}

void StringTokenizer::selfTest(void)
{
    StringTokenizer testee(",foo,bar,,rumbah", ',');
    StringTokenizer testee2("foo, bar, ,rumbah,", ',');
    StringTokenizer nullString(NULL, ',');
    StringTokenizer oneToken("foobar ", ',');

    UMBRA_ASSERT(!nullString.hasNext());
    UMBRA_ASSERT(oneToken.hasNext());
    UMBRA_ASSERT(strcmp(oneToken.nextToken(), "foobar ") == 0);
    UMBRA_ASSERT(!oneToken.hasNext());

    for (int i = 1; i <= 5; i++)
    {
        UMBRA_ASSERT(testee.hasNext());
        UMBRA_ASSERT(testee2.hasNext());
        switch (i)
        {
        case 1:
            UMBRA_ASSERT(strcmp(testee.nextToken(),"") == 0);
            UMBRA_ASSERT(strcmp(testee2.nextToken(),"foo") == 0);
            break;
        case 2:
            UMBRA_ASSERT(strcmp(testee.nextToken(),"foo") == 0);
            UMBRA_ASSERT(strcmp(testee2.nextToken()," bar") == 0);
            break;
        case 3:
            UMBRA_ASSERT(strcmp(testee.nextToken(),"bar") == 0);
            UMBRA_ASSERT(strcmp(testee2.nextToken()," ") == 0);
            break;
        case 4:
            UMBRA_ASSERT(strcmp(testee.nextToken(),"") == 0);
            UMBRA_ASSERT(strcmp(testee2.nextToken(),"rumbah") == 0);
            break;
        case 5:
            UMBRA_ASSERT(strcmp(testee.nextToken(),"rumbah") == 0);
            UMBRA_ASSERT(strcmp(testee2.nextToken(),"") == 0);
            break;
        default:
            UMBRA_ASSERT(false);
        }
    }
    UMBRA_ASSERT(!testee.hasNext());
    UMBRA_ASSERT(!testee2.hasNext());
}

#endif // UMBRA_ARCH != UMBRA_SPU
