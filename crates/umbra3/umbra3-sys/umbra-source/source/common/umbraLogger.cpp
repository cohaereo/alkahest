#include "umbraLogger.hpp"

#include "umbraPrivateDefs.hpp"
#if UMBRA_ARCH != UMBRA_SPU

#include "umbraString.hpp"
#include "umbraFileStream.hpp"
#include <time.h>
#include <cstdlib>

#if UMBRA_OS == UMBRA_WINDOWS && defined(UMBRA_DEBUG)
#define NOMINMAX
#include <Windows.h> // OutputDebugStringA
#endif

#define MAX_MESSAGE_LEN 1024

using namespace std;

namespace Umbra
{

static const char* getLogPrefix (Logger::Level level)
{
    switch (level)
    {
    case Logger::LEVEL_DEBUG: return "DEBUG: ";
    case Logger::LEVEL_INFO: return "INFO: ";
    case Logger::LEVEL_WARNING: return "WARNING: ";
    case Logger::LEVEL_ERROR: return "ERROR: ";
    default: return ": ";
    }
}

static int getLogDateString(char* buffer)
{
    time_t timeVal;
    time(&timeVal);
    tm* tmData = localtime(&timeVal);
    return sprintf(buffer, "[%02d:%02d:%02d] ",
        tmData->tm_hour, tmData->tm_min, tmData->tm_sec);
}

static StreamLogger g_defaultLogger(getStdoutStream());

Logger* getDefaultLogger (void)
{
    return &g_defaultLogger;
}


void StreamLogger::log (Logger::Level level, const char* str)
{
    if (!m_out)
        return;
    char date[32];
    int dateLen = getLogDateString(date);
    m_out->write(date, dateLen);
    const char* prefix = getLogPrefix(level);
    m_out->write(prefix, (UINT32)strlen(prefix));
    int len = (UINT32)strlen(str);
    m_out->write(str, len);
    if (str[strlen(str) - 1] != '\n')
        m_out->write("\n", 1);
    m_out->flush();
}

void umbraLogv (Logger* logger, Logger::Level level, const char* fmt, va_list args)
{
    char buf[MAX_MESSAGE_LEN];
    int len = vsnprintf(buf, MAX_MESSAGE_LEN-1, fmt, args);
    UMBRA_UNREF(len);
    if (!logger)
        logger = getDefaultLogger();
    logger->log(level, buf);

#if UMBRA_OS == UMBRA_WINDOWS && defined(UMBRA_DEBUG)
    OutputDebugStringA(buf);
    OutputDebugStringA("\n");
#endif
}

void umbraLog (Logger* logger, Logger::Level level, const char* fmt, ...)
{
    va_list args;
    va_start(args, fmt);
    umbraLogv(logger, level, fmt, args);
    va_end(args);
}

}


#endif // UMBRA_ARCH != UMBRA_SPU