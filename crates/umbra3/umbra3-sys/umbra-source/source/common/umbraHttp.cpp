#include "umbraHttp.hpp"

#if 0 // UMBRA_OS == UMBRA_WINDOWS || UMBRA_OS == UMBRA_LINUX || UMBRA_OS ==
      // UMBRA_OSX

#include "umbraPlatform.hpp"
#include "umbraRandom.hpp"
#include "umbraString.hpp"

#define CURL_STATICLIB
#include <curl/curl.h>

const Umbra::String Umbra::Http::StriHash::s_empty("");

#if UMBRA_OS == UMBRA_WINDOWS

#pragma comment(lib, "ws2_32")
#pragma comment(lib, "wldap32")

#endif

namespace Umbra
{

class ImpHttp : public Base
{
public:
    ImpHttp (Allocator* a) : Base(a), m_curl(NULL)
    {
        m_curl = curl_easy_init();
    }
    ~ImpHttp ()
    {
        if (m_curl)
            curl_easy_cleanup(m_curl);
    }

    Http::ErrorCode request (Http::Verb verb, const String& url, Http::Response& response, const Http::RequestParams* params, Http::ProgressListener* progress)
    {
        curl_easy_reset(m_curl);

        if (curl_easy_setopt(m_curl, CURLOPT_URL, url.toCharPtr()) != CURLE_OK)
            return Http::ERR_INIT;

        if (curl_easy_setopt(m_curl, CURLOPT_READFUNCTION, sendCallback) != CURLE_OK)
            return Http::ERR_INIT;
        if (curl_easy_setopt(m_curl, CURLOPT_READDATA, this) != CURLE_OK)
            return Http::ERR_INIT;
        if (curl_easy_setopt(m_curl, CURLOPT_WRITEFUNCTION, receiveCallback) != CURLE_OK)
            return Http::ERR_INIT;
        if (curl_easy_setopt(m_curl, CURLOPT_WRITEDATA, this) != CURLE_OK)
            return Http::ERR_INIT;
        if (curl_easy_setopt(m_curl, CURLOPT_HEADERFUNCTION, headerCallback) != CURLE_OK)
            return Http::ERR_INIT;
        if (curl_easy_setopt(m_curl, CURLOPT_HEADERDATA, this) != CURLE_OK)
            return Http::ERR_INIT;
#if LIBCURL_VERSION_MINOR >= 32
        if (curl_easy_setopt(m_curl, CURLOPT_XFERINFOFUNCTION, progressCallback) != CURLE_OK)
            return Http::ERR_INIT;
        if (curl_easy_setopt(m_curl, CURLOPT_XFERINFODATA, progress) != CURLE_OK)
            return Http::ERR_INIT;
#else
        if (curl_easy_setopt(m_curl, CURLOPT_PROGRESSFUNCTION, progressCallback) != CURLE_OK)
            return Http::ERR_INIT;
        if (curl_easy_setopt(m_curl, CURLOPT_PROGRESSDATA, progress) != CURLE_OK)
            return Http::ERR_INIT;
#endif
        if (curl_easy_setopt(m_curl, CURLOPT_NOPROGRESS, 0L) != CURLE_OK) // for xferinfo to be called
            return Http::ERR_INIT;

#define _STRFY(x) #x
#define STRFY(x) _STRFY(x)

        const char* userAgent = "Umbra Native HTTP client " STRFY(UMBRA_VERSION_MAJOR) "." STRFY(UMBRA_VERSION_MINOR) "." STRFY(UMBRA_VERSION_REVISION) "." STRFY(UMBRA_VERSION_STATUS);
        if (curl_easy_setopt(m_curl, CURLOPT_USERAGENT, userAgent) != CURLE_OK)
            return Http::ERR_INIT;

#undef _STRFY
#undef STRFY

        if (verb == Http::PUT)
            curl_easy_setopt(m_curl, CURLOPT_PUT, 1L);
        else if (verb == Http::POST)
        {
            curl_easy_setopt(m_curl, CURLOPT_POST, 1L);
            curl_easy_setopt(m_curl, CURLOPT_POSTFIELDSIZE, params ? params->data.getSize() : 0L);
        }

        struct curl_slist* slist = NULL;
        if (params)
        {
            if (params->data.getSize())
            {
                if (curl_easy_setopt(m_curl, CURLOPT_INFILESIZE_LARGE, params->data.getSize()) != CURLE_OK)
                    return Http::ERR_INIT;

                m_sendPtr       = params->data.getPtr();
                m_sendDataSize  = params->data.getSize();
            }

            // User authentication
            if (params->username.length())
                curl_easy_setopt(m_curl, CURLOPT_USERNAME, params->username.toCharPtr());
            if (params->password.length())
                curl_easy_setopt(m_curl, CURLOPT_PASSWORD, params->password.toCharPtr());

            // Headers
            const Hash<String, String>& headers = params->headers.m_hash;
            Hash<String, String>::Iterator i;
            String headerStr;
            for (i = headers.iterate(); headers.isValid(i); headers.next(i))
            {
                const String& k = headers.getKey(i);
                const String& v = headers.getValue(i);
                slist = curl_slist_append(slist, (k + ": " + v).toCharPtr());
            }
            if (slist)
                curl_easy_setopt(m_curl, CURLOPT_HTTPHEADER, slist);
        }

        m_response = &response;
        response.data.clear();
        response.headers.m_hash.clear();

        CURLcode res = curl_easy_perform(m_curl);

        curl_easy_getinfo(m_curl, CURLINFO_HTTP_CODE, &response.code);

        curl_slist_free_all(slist);
        // \todo detect errors
        return res == CURLE_OK ? Http::ERR_OK : Http::ERR_UNKNOWN;
    }

private:
    static size_t receiveCallback (void* buffer, size_t size, size_t nItems, void* userPtr)
    {
        ImpHttp* req = (ImpHttp*)userPtr;
        return req->receiveHandler(buffer, size, nItems);
    }
    size_t receiveHandler (void* buffer, size_t size, size_t nItems)
    {
        m_response->data.append((char*)buffer, (int)(size*nItems));
        return size*nItems;
    }

    static size_t sendCallback (char* buffer, size_t size, size_t nItems, void* userPtr)
    {
        ImpHttp* req = (ImpHttp*)userPtr;
        return req->sendHandler(buffer, size, nItems);
    }

    size_t sendHandler (char* buffer, size_t size, size_t nItems)
    {
        size_t bytesSent = min2(m_sendDataSize, size*nItems);
        memcpy(buffer, m_sendPtr, bytesSent);
        m_sendPtr += bytesSent;
        m_sendDataSize -= bytesSent;
        return bytesSent;
    }

    static size_t headerCallback (char* buffer, size_t size, size_t nItems, void* userPtr)
    {
        ImpHttp* req = (ImpHttp*)userPtr;
        return req->headerHandler(buffer, size, nItems);
    }

    size_t headerHandler (char* buffer, size_t size, size_t nItems)
    {
        String hdr(buffer, (int)(size*nItems));

        Array<String> toks = hdr.split(":", false);
        int numToks = toks.getSize();
        String k, v;
        if (numToks < 2)
            return size*nItems;   // Only interested in key-value pairs

        k = toks[0];
        v = toks[1];

        for (int i = 2; i < toks.getSize(); i++)
        {
            v += ":" + toks[i];
        }

        k.trimWhiteSpace();
        v.removeFromTail('\n');
        v.removeFromTail('\r');
        v.trimWhiteSpace();

        m_response->headers.set(k, v);
        return size*nItems;
    }

#if LIBCURL_VERSION_MINOR >= 32
    static int progressCallback (void *userPtr, curl_off_t dltotal, curl_off_t dlnow, curl_off_t ultotal, curl_off_t ulnow)
#else
    static int progressCallback (void *userPtr, double dltotal, double dlnow, double ultotal, double ulnow)
#endif
    {
        Http::ProgressListener* progress = (Http::ProgressListener*)userPtr;
        bool success = true;
        if (progress)
            success = progress->transferProgress((size_t)dlnow, (size_t)dltotal, (size_t)ulnow, (size_t)ultotal);
        return !success; // non-zero return value aborts transfer
    }

    CURL*           m_curl;
    char*           m_sendPtr;
    size_t          m_sendDataSize;
    size_t          m_sendOffs;
    Http::Response* m_response;
};

Http::Http (const PlatformServices& platform)
{
    Allocator* a = platform.allocator;
    if (!a)
        a = Umbra::getAllocator();
    m_imp = UMBRA_HEAP_NEW(a, ImpHttp, a);
}

Http::~Http ()
{
    if (m_imp)
    {
        Allocator* a = m_imp->getAllocator();
        UMBRA_HEAP_DELETE(a, m_imp);
        }
}

Http::ErrorCode Http::request (Http::Verb verb, const String& url, Http::Response& response, const Http::RequestParams* params, ProgressListener* progress)
{
    UMBRA_ASSERT(m_imp);
    if (!m_imp)
        return ERR_UNKNOWN;
    return m_imp->request(verb, url, response, params, progress);
}

} // namespace Umbra
#else

namespace Umbra {

Http::Http(const PlatformServices &) { UMBRA_ASSERT(!"Not implemented."); }

Http::~Http() {}

Http::ErrorCode Http::request(Http::Verb, const String &, Http::Response &,
                              const Http::RequestParams *, ProgressListener *) {
  UMBRA_ASSERT(!"Not implemented.");
  return Http::ERR_NOT_IMPLEMENTED;
}

} // namespace Umbra

#endif // windows, linux or osx
