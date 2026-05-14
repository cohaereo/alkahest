#pragma once

#include "umbraString.hpp"
#include "umbraHash.hpp"

namespace Umbra
{

// \todo [jasin] should use a pool allocator
/* \todo [antti 5.9.2012]: should use stream API */

class JsonValue;

class JsonObject : public Base
{
public:
    JsonObject (Allocator* a = NULL) : Base(a), m_members(a) {}
    ~JsonObject ();

    JsonObject (const JsonObject& o) { *this = o; }

    bool                hasMember       (const String& name)    const               { return m_members.contains(name); }
    int                 getNumMembers   (void)                  const               { return m_members.getNumKeys(); }
    void                getMemberNames  (Array<String>& names)  const               { m_members.getKeyArray(names); }
    const JsonValue*    getMemberValue  (const String& name)    const               { UMBRA_ASSERT(hasMember(name)); return *m_members.get(name); }
    void                addMember       (const String& name, const JsonValue* val)  { UMBRA_ASSERT(!m_members.contains(name)); m_members.insert(name, val); }

    // Templatized parsing helpers for primitive types
    template<typename T> bool getMember (const char* name, T& dst) const;

    JsonObject& operator= (const JsonObject& o) { UMBRA_ASSERT(getAllocator() == o.getAllocator()); m_members = o.m_members; return *this; }

private:
    Hash<String, const JsonValue*>  m_members;
};

typedef Array<const JsonValue*> JsonArray;

class JsonValue
{
public:
    enum Type
    {
        JSON_TYPE_STRING = 0,
        JSON_TYPE_INTEGER,       // Internally distinguish between
        JSON_TYPE_FLOAT,         // integer and floating point types
        JSON_TYPE_OBJECT,
        JSON_TYPE_ARRAY,
        JSON_TYPE_BOOL,
        JSON_TYPE_NULL,

        JSON_TYPE_MAX
    };
    // Map native primitive types
    template<typename T> JsonValue::Type getJsonType (void) const;

    JsonValue (const String* s)     : m_type(JSON_TYPE_STRING)   { m_val.s = s; }
    JsonValue (INT64 i)             : m_type(JSON_TYPE_INTEGER)  { m_val.i = i; }
    JsonValue (double f)            : m_type(JSON_TYPE_FLOAT)    { m_val.f = f; }
    JsonValue (const JsonObject* o) : m_type(JSON_TYPE_OBJECT)   { m_val.o = o; }
    JsonValue (const JsonArray* a)  : m_type(JSON_TYPE_ARRAY)    { m_val.a = a; }
    JsonValue (bool b)              : m_type(JSON_TYPE_BOOL)     { m_val.b = b; }
    JsonValue (void)                : m_type(JSON_TYPE_NULL)     { }

    ~JsonValue (void);

    // \todo make these private or protected!
    Type              getType     (void) const { return m_type; }
    const String*     getString   (void) const { UMBRA_ASSERT(m_type == JSON_TYPE_STRING);  return m_val.s; }
    INT64             getInteger  (void) const { UMBRA_ASSERT(m_type == JSON_TYPE_INTEGER); return m_val.i; }
    double            getFloat    (void) const { UMBRA_ASSERT(m_type == JSON_TYPE_FLOAT);   return m_val.f; }
    const JsonObject* getObject   (void) const { UMBRA_ASSERT(m_type == JSON_TYPE_OBJECT);  return m_val.o; }
    const JsonArray*  getArray    (void) const { UMBRA_ASSERT(m_type == JSON_TYPE_ARRAY);   return m_val.a; }
    bool              getBool     (void) const { UMBRA_ASSERT(m_type == JSON_TYPE_BOOL);    return m_val.b; }

    // Templatized parsing helpers for primitive types
    // \todo consider overloading cast operators
    template<typename T> bool get (T& dst) const
    {
        if (getType() != getJsonType<T>())
            return false;
        assignVal(dst);
        return true;
    }

private:
    template<typename T> void assignVal(T& dst) const;

    Type m_type;
    union Value
    {
        const String*     s;
        INT64             i;
        double            f;
        const JsonObject* o;
        const JsonArray*  a;
        bool              b;
    } m_val;
};

template<typename T>
inline bool JsonObject::getMember (const char* name, T& dst) const
{
    if (hasMember(name))
        return getMemberValue(name)->get(dst);
    return false;
}

inline JsonObject::~JsonObject ()
{
    Array<String> names;
    getMemberNames(names);
    int num = names.getSize();
    for (int i = 0; i < num; i++)
    {
        const JsonValue* member = *m_members.get(names[i]);
        UMBRA_DELETE(const_cast<JsonValue*>(member));
    }
}

template<> inline JsonValue::Type JsonValue::getJsonType<String>      (void) const { return JsonValue::JSON_TYPE_STRING;    }
template<> inline JsonValue::Type JsonValue::getJsonType<INT64>       (void) const { return JsonValue::JSON_TYPE_INTEGER;   }
template<> inline JsonValue::Type JsonValue::getJsonType<double>      (void) const { return JsonValue::JSON_TYPE_FLOAT;     }
template<> inline JsonValue::Type JsonValue::getJsonType<bool>        (void) const { return JsonValue::JSON_TYPE_BOOL;      }
template<> inline JsonValue::Type JsonValue::getJsonType<JsonObject>  (void) const { return JsonValue::JSON_TYPE_OBJECT;    }

template<> inline void JsonValue::assignVal(String&       dst) const { dst = *getString(); }
template<> inline void JsonValue::assignVal(INT64&        dst) const { dst = getInteger(); }
template<> inline void JsonValue::assignVal(double&       dst) const { dst = getFloat(); }
template<> inline void JsonValue::assignVal(bool&         dst) const { dst = getBool(); }
template<> inline void JsonValue::assignVal(JsonObject&   dst) const { dst = *getObject(); }
template<> inline void JsonValue::assignVal(JsonArray&    dst) const { dst = *getArray(); }

class JsonParser : public Base
{
public:
    JsonParser  (Allocator* a, Logger* l) : Base(a), m_log(l) {}
    ~JsonParser () {}

    const JsonValue* parse (const char* str, size_t len);

private:
    const JsonValue*  parseValue      (void);
    const String*     parseString     (void);
    const JsonValue*  parseNumber     (void);
    bool              parseInt        (INT64& i);
    bool              parseFrac       (double& frac);
    bool              parseExp        (INT64& exp);
    const JsonObject* parseObject     (void);
    const JsonArray*  parseArray      (void);
    const JsonValue*  parseKeyword    (void);
    void              parseWhiteSpace (void);

    // Utils and helpers

    static bool isWhiteSpace (int c)
    {
        if (c == ' '    ||
            c == '\t'   ||
            c == '\r'   ||
            c == '\n'   ||
            c == '\b'   ||
            c == '\f')
            return true;
        return false;
    }

    static bool isDigit (int c)
    {
        if (c >= '0' && c <= '9')
            return true;
        return false;
    }

    int peek (void) const
    {
        if ((size_t)(m_ptr - m_str) >= m_len)
            return EOF;  // EOF
        return *m_ptr;
    }

    int next (void)
    {
        int c = peek();
        m_ptr++;
        return c;
    }

    bool expect (int c)
    {
        if (next() == c)
            return true;

        String err("Expected: ");
        if (c == EOF)
            err += "EOF";
        else
            err += "'" + String((char)c) + "'";

        parseError(err.toCharPtr());
        return false;
    }

    int  getCurrentLine (void);
    void parseError (const char* err);

    Logger*     m_log;

    const char* m_str;
    const char* m_ptr;
    size_t      m_len;
    bool        m_error;
};

class JsonPrinter
{
public:
    static void print       (String& dst, const JsonValue* obj);
    static void prettyPrint (String& dst, const JsonValue* obj);

private:
    static void print       (String& dst, const String* s);
    static void print       (String& dst, INT64 i);
    static void print       (String& dst, double f);
    static void print       (String& dst, const JsonObject* o);
    static void print       (String& dst, const Array<const JsonValue*>* a);
    static void print       (String& dst, bool b);
    static void printNull   (String& dst);

    Allocator* m_alloc;
};

}
