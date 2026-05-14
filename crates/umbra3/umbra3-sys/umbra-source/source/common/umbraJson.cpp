#include "umbraJson.hpp"
#include "umbraLogger.hpp"
#include <math.h>

using namespace Umbra;

#define CHECK_STATE if (m_error) return NULL

const JsonValue* JsonParser::parse (const char* str, size_t len)
{
    // Reset state
    m_str = m_ptr = str;
    m_len = len;
    m_error = false;

    const JsonValue* root = parseValue();
    if (!root)
        return NULL;

    parseWhiteSpace();
    if (expect(EOF))
        return root;

    UMBRA_DELETE(const_cast<JsonValue*>(root));
    return NULL;
}

const JsonValue* JsonParser::parseValue (void)
{
    CHECK_STATE;
    parseWhiteSpace();

    switch (peek())
    {
        // String
    case '"':
        {
            const String* str = parseString();
            if (!str)
                return NULL;

            return UMBRA_NEW(JsonValue, str);
        }

        // Number
    case '-':
    case '0':
    case '1':
    case '2':
    case '3':
    case '4':
    case '5':
    case '6':
    case '7':
    case '8':
    case '9':
        return parseNumber();

        // Object
    case '{':
        {
            const JsonObject* o = parseObject();
            if (!o)
                return NULL;

            return UMBRA_NEW(JsonValue, o);
        }

        // Array
    case '[':
        {
            const Array<const JsonValue*>* a = parseArray();
            if (!a)
                return NULL;

            return UMBRA_NEW(JsonValue, a);
        }

        // Keyword
    case 't':   // true
    case 'f':   // false
    case 'n':   // null
        return parseKeyword();

    case EOF:
        parseError("Unexpected end of file.");
        return NULL;

    default:
        parseError("Unexpected token.");
        return NULL;
    }
}

const JsonObject* JsonParser::parseObject (void)
{
    CHECK_STATE;

    if (!expect('{'))
        return NULL;

    JsonObject* obj = UMBRA_NEW(JsonObject, getAllocator());

    // Parse members
    while (!m_error)
    {
        parseWhiteSpace();

        switch (peek())
        {
            // Member declaration
        case '"':
            {
                const String* memberName = parseString();
                if (!memberName)
                    break;

                parseWhiteSpace();

                expect(':');
                const JsonValue* memberVal = parseValue();

                if (memberName && memberVal)
                {
                    if (obj->hasMember(*memberName))
                    {
                        String errStr("Object already contains member '");
                        errStr += *memberName + "'";
                        parseError(errStr.toCharPtr());
                        UMBRA_DELETE(const_cast<String*>(memberName));
                        break;
                    }

                    obj->addMember(*memberName, memberVal);

                    parseWhiteSpace();

                    // Either a separator or end of object has to come next
                    int c = peek();
                    if (c != ',' && c != '}')
                    {
                        parseError("Unexpected token. Expected ',' or '}'");
                    }

                    UMBRA_DELETE(const_cast<String*>(memberName));
                }
            }
            break;

            // Member separator
        case ',':
            next();

            // check that there has been at least one member
            // before the separator
            if (!obj->getNumMembers())
                parseError("Unexpected separator ','");
            else
            {
                // check that there's at least one member after
                // the separator
                parseWhiteSpace();
                if (peek() != '"')
                    parseError("Expected a member declaration");
            }
            break;

            // Object ends, return
        case '}':
            next();
            return obj;

        case EOF:
            parseError("Unexpected end-of-file.");
            break;

        default:
            parseError("Unexpected token.");
            break;
        }
    }

    UMBRA_ASSERT(m_error);
    UMBRA_DELETE(obj);
    return NULL;
}

const String* JsonParser::parseString (void)
{
    CHECK_STATE;

    if (!expect('"'))
        return NULL;

    Array<char> tmp;
    while (!m_error)
    {
        int c = next();
        switch (c)
        {
            // End of string
        case '"':
        {
            String* str = UMBRA_NEW(String, getAllocator());
            *str = String(tmp.getPtr(), tmp.getSize());
            return str;
        }

            // Escape character
        case '\\':
            switch (next())
            {
            case '"':   tmp.pushBack('"');     break;
            case '\\':  tmp.pushBack('\\');    break;
            case '/':   tmp.pushBack('/');     break;  // in JSON, '/' can to be escaped for HTML
            case 'b':   tmp.pushBack('\b');    break;
            case 'f':   tmp.pushBack('\f');    break;
            case 'n':   tmp.pushBack('\n');    break;
            case 'r':   tmp.pushBack('\r');    break;
            case 't':   tmp.pushBack('\t');    break;
            case 'u':   parseError("Unicode not supported.");       break;
            default:    parseError("Invalid escape character.");    break;
            }

            break;

            // End of file
        case EOF:
            parseError("Unexpected end of file.");
            break;

            // Regular character
        default:
            UMBRA_ASSERT(0 <= c && c <= 255);
            tmp.pushBack((char)c);
        }
    }

    UMBRA_ASSERT(m_error);
    return NULL;
}


const JsonValue* JsonParser::parseNumber (void)
{
    CHECK_STATE;

    // Parse integer part
    INT64 i;
    if (!parseInt(i))
        return NULL;

    int c = peek();

    // Parse fraction, if any
    double frac = 0.0;
    bool isFloat = false;
    if (c == '.')
    {
        if (!parseFrac(frac))
            return NULL;
        isFloat = true;
    }

    // Parse exponent, if any
    INT64 exp = 0;
    c = peek();
    if (c == 'e' || c == 'E')
    {
        if (!parseExp(exp))
            return NULL;
        isFloat = true;
    }

    // Return integer value if no fraction or exponent found
    if (!isFloat)
        return UMBRA_NEW(JsonValue, i);

    // Construct final floating point value
    double val = (double)i;
    if (frac != 0.0)
        val += frac;

    if (exp != 0)
    {
        double e = pow(10.0, (double)exp);
        val *= e;
    }

    return UMBRA_NEW(JsonValue, val);
}

bool JsonParser::parseInt (Umbra::INT64& i)
{
    int c = next();

    // Parse sign, if any
    bool sign = false;
    if (c == '-')
    {
        sign = true;
        c = next();
    }

    // Parse first digit
    if (!isDigit(c))
    {
        parseError("Invalid numeric value.");
        return false;
    }

    // Check for leading zero
    if (c == '0')
    {
        if (isDigit(peek()))
        {
            parseError("Octals not supported in JSON");
            return false;
        }
    }

    // Parse rest of the digits
    i = c - '0';
    while (isDigit(peek()))
    {
        c = next();
        i *= 10;
        i += c - '0';
    }

    // flip sign if there was one
    if (sign)
        i = -i;

    return true;
}

bool JsonParser::parseFrac (double& frac)
{
    if (!expect('.'))
        return false;

    // Parse first digit
    int c = next();
    if (!isDigit(c))
    {
        parseError("Invalid fraction.");
        return false;
    }

    double exp = .1;
    frac = exp * (double)(c - '0');

    // Parse rest of the decimals
    while (isDigit(peek()))
    {
        c = next();
        exp *= .1;  // move decimal point
        double dec = (double)(c - '0') * exp;
        frac += dec;
    }

    return true;
}

bool JsonParser::parseExp (Umbra::INT64& e)
{
    int c = next();
    if (c != 'e' && c != 'E')
    {
        parseError("Invalid exponent.");
        return false;
    }

    c = peek();
    bool sign = false;
    if (c == '-' || c == '+')
    {
        sign = (c == '-');
        next();
    }

    c = next();

    // Parse first digit
    if (!isDigit(c))
    {
        parseError("Invalid exponent.");
        return false;
    }
    e = c - '0';

    while (isDigit(peek()))
    {
        c = next();
        e *= 10;
        e += c - '0';
    }

    if (sign)
        e = -e;

    return true;
}

const Array<const JsonValue*>* JsonParser::parseArray (void)
{
    if (!expect('['))
        return NULL;

    Array<const JsonValue*>* a = UMBRA_NEW(Array<const JsonValue*>, getAllocator());

    while (!m_error)
    {
        parseWhiteSpace();

        int c = peek();

        // Handle separator
        if (c == ',')
        {
            if (!a->getSize())
            {
                parseError("Unexpected separator.");
                break;
            }

            next();
            parseWhiteSpace();
            c = peek();

            if (c == ']')
            {
                parseError("Expected array element.");
                break;
            }
        }

        // End of array
        if (c == ']')
        {
            next();
            return a;
        }

        // Parse array element
        const JsonValue* val = parseValue();
        if (!val)
            break;
        // Add to array
        a->pushBack(val);

        parseWhiteSpace();
        // Either a separator or end of array has to come next
        c = peek();
        if (c != ',' && c != ']')
        {
            parseError("Unexpected token.");
            break;
        }
    }

    UMBRA_ASSERT(m_error);
    UMBRA_DELETE(const_cast<Array<const JsonValue*>*>(a));
    return NULL;
}

const JsonValue* JsonParser::parseKeyword (void)
{
    switch (next())
    {
        // true
    case 't':
        if (next() != 'r') { parseError("Invalid keyword."); return NULL; }
        if (next() != 'u') { parseError("Invalid keyword."); return NULL; }
        if (next() != 'e') { parseError("Invalid keyword."); return NULL; }
        return UMBRA_NEW(JsonValue, true);

        // false
    case 'f':
        if (next() != 'a') { parseError("Invalid keyword."); return NULL; }
        if (next() != 'l') { parseError("Invalid keyword."); return NULL; }
        if (next() != 's') { parseError("Invalid keyword."); return NULL; }
        if (next() != 'e') { parseError("Invalid keyword."); return NULL; }
        return UMBRA_NEW(JsonValue, false);

        // null
    case 'n':
        if (next() != 'u') { parseError("Invalid keyword."); return NULL; }
        if (next() != 'l') { parseError("Invalid keyword."); return NULL; }
        if (next() != 'l') { parseError("Invalid keyword."); return NULL; }
        return UMBRA_NEW(JsonValue);
    }

    parseError("Invalid keyword."); // Actually ICE if we end up here
    return NULL;
}

void JsonParser::parseWhiteSpace (void)
{
    while (isWhiteSpace(peek())) next();
}

void JsonParser::parseError (const char* err)
{
    m_error = true;
    UMBRA_LOG_E(m_log, "Parse error on line %d: %s\n", getCurrentLine()+1 , err);
}

int JsonParser::getCurrentLine (void)
{
    int line = 0;
    for (const char* p = m_str; p <= m_ptr; p++)
    {
        if (*p == '\n')
            line++;
    }

    return line;
}




void JsonPrinter::print (String& dst, const JsonValue* val)
{
    UMBRA_ASSERT(val);
    if (val)
    {
        switch (val->getType())
        {
        case JsonValue::JSON_TYPE_STRING:    print(dst, val->getString());   break;
        case JsonValue::JSON_TYPE_INTEGER:   print(dst, val->getInteger());  break;
        case JsonValue::JSON_TYPE_FLOAT:     print(dst, val->getFloat());    break;
        case JsonValue::JSON_TYPE_OBJECT:    print(dst, val->getObject());   break;
        case JsonValue::JSON_TYPE_ARRAY:     print(dst, val->getArray());    break;
        case JsonValue::JSON_TYPE_BOOL:      print(dst, val->getBool());     break;
        case JsonValue::JSON_TYPE_NULL:      printNull(dst);                 break;
        default:
            UMBRA_ASSERT(!"Invalid value type.");
        }
    }
}

// Print string
void JsonPrinter::print (String& dst, const String* s)
{
    dst += '"';

    int len = s->length();
    for (int i = 0; i < len; i++)
    {
        char c = (*s)[i];

        // Escape characters
        switch (c)
        {
        case '\"':  dst += "\\\"";
        case '\\':  dst += "\\\\";
        case '/':   dst += "\\/";
        case '\b':  dst += "\\b";
        case '\f':  dst += "\\f";
        case '\n':  dst += "\\n";
        case '\r':  dst += "\\r";
        case '\t':  dst += "\\t";
        default:    dst += c;
        }
    }
    dst += '"';
}

// Print integer
void JsonPrinter::print (String& dst, Umbra::INT64 i)
{
    char buf[64];
#if UMBRA_OS == UMBRA_WINDOWS
    sprintf(buf, "%I64d", i);
#else
    sprintf(buf, "%lld", (long long)i);
#endif
    dst += buf;
}

// Print double
 void JsonPrinter::print (String& dst, double f)
 {
     char buf[64];
     sprintf(buf, "%g", f);
     dst += buf;
 }

 // Print object
void JsonPrinter::print (String& dst, const JsonObject* o)
{
    dst += '{';
    int numMembers = o->getNumMembers();
    if (numMembers)
    {
        Array<String> names;
        o->getMemberNames(names);

        for (int i = 0; i < numMembers; i++)
        {
            const JsonValue* val = o->getMemberValue(names[i]);

            print(dst, &names[i]);
            dst += ':';
            print(dst, val);
            if (i < (numMembers-1))
                dst += ',';
        }
    }
    dst += '}';
}

// Print array
void JsonPrinter::print (String& dst, const Array<const JsonValue*>* a)
{
    dst += '[';
    int numElems = a->getSize();
    for (int i = 0; i < numElems; i++)
    {
        const JsonValue* val = (*a)[i];
        print(dst, val);
        if (i < (numElems-1))
            dst += ',';
    }
    dst += ']';
}
void JsonPrinter::print (String& dst, bool b)
{
    dst += b ? "true" : "false";
}

void JsonPrinter::printNull (String& dst)
{
    dst += "null";
}

JsonValue::~JsonValue (void)
{
    if (m_type == JSON_TYPE_ARRAY)
    {
        Array<const JsonValue*>* arr = const_cast<Array<const JsonValue*>*>(getArray());
        Allocator* alloc             = arr->getAllocator();
        int num                      = arr->getSize();
        for (int i = 0; i < num; i++)
            UMBRA_HEAP_DELETE(alloc, const_cast<JsonValue*>((*arr)[i]));
        UMBRA_HEAP_DELETE(alloc, arr);
    }
    else if (m_type == JSON_TYPE_OBJECT)
    {
        JsonObject* obj     = const_cast<JsonObject*>(getObject());
        Allocator* alloc    = obj->getAllocator();
        UMBRA_HEAP_DELETE(alloc, obj);
    }
    else if (m_type == JSON_TYPE_STRING)
    {
        String*     str     = const_cast<String*>(getString());
        Allocator*  alloc   = str->getAllocator();
        UMBRA_HEAP_DELETE(alloc, str);
    }
}
