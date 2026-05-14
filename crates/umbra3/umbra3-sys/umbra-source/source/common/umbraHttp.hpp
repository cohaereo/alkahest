#pragma once

#include <standard/Base.hpp>
#include "umbraPlatform.hpp"
#include "umbraArray.hpp"
#include "umbraHash.hpp"
#include "umbraString.hpp"

namespace Umbra
{
class String;
class ImpHttp;

class Http
{
public:

    // interface to monitor download/upload progress
    class ProgressListener
    {
    public:
        // return false if you want to abort
        virtual bool transferProgress (size_t dlCur, size_t dlTotal, size_t ulCur, size_t ulTotal) = 0;
    protected:
        virtual ~ProgressListener() {}
    };

     // Hash with array operator overloads and a limited public interface
    // keys are case-insensitive (lowercase), values are not

    class StriHash
    {
    public:
        const String& operator[] (const String& k) const
        {
            String kstr(k);
            kstr.lower();
            const String* r = m_hash.get(kstr);
            if (!r)
                return s_empty;
            return *r;
        }
        // add/replace
        void set (const String& k, const String& v)
        {
            String kstr(k);
            kstr.lower();
            if (m_hash.contains(kstr))
                *m_hash.get(kstr) = v;
            else
                m_hash.insert(kstr, v);
        }
        /*
        String& operator[] (const String& k)
        {
            String kstr(k);
            kstr.lower();
            if (!m_hash.contains(kstr))
            {
                m_hash.insert(kstr, s_empty);
            }
            return *m_hash.get(kstr);
        }
        */
    private:

        friend class ImpHttp;
        static const String s_empty;
        Hash<String, String> m_hash;
    };

    struct RequestParams
    {
        StriHash    headers;            // Content-Types etc
        String      username;           // HTTP basic auth username
        String      password;           // HTTP basic auth password
        Array<char> data;               // optional request data for e.g. POST
    };

    struct Response
    {
        enum
        {
            STATUS_OK                       = 200,
            STATUS_UNAUTHORIZED             = 401,
            STATUS_NOT_FOUND                = 404,
            STATUS_INTERNAL_SERVER_ERROR    = 500
        };

        int         code;
        Array<char> data;
        StriHash    headers;

        Response (void) : code(-1) {}

    protected:
        Response (int c, const Array<char>& d, const StriHash& h)
            : code(c), data(d), headers(h)
        {}
    };

    enum ErrorCode
    {
        ERR_OK = 0,
        ERR_INIT,
        ERR_INVALID_URL,
        ERR_CANNOT_CONNECT,
        ERR_CONNECTION_TERMINATED,
        ERR_TIMEOUT,
        ERR_NO_DATA,

        ERR_NOT_IMPLEMENTED,
        ERR_UNKNOWN
    };

    // \todo allocators, logging and whatnot
    Http (const PlatformServices& platform);
    ~Http();

    enum Verb
    {
        GET,
        PUT,
        POST
        // \todo others as required
    };

    ErrorCode request (Verb verb, const String& url, Response& response, const Http::RequestParams* params = NULL, ProgressListener* progress = NULL);

private:

    ImpHttp* m_imp;
};

} // namespace Umbra

