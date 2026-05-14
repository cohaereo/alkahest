#pragma once

#include "umbraPrivateDefs.hpp"
#include "umbraPlatform.hpp"
#include <stdarg.h>

namespace Umbra
{

class FileOutputStream;

class StreamLogger: public Logger
{
public:
    StreamLogger (FileOutputStream* out = NULL): m_out(out) {}
    void setOutput(FileOutputStream* out) { m_out = out; }
    void log (Level level, const char* str);
private:
    FileOutputStream* m_out;
};

class NullLogger: public Logger
{
public:
    NullLogger(void) {}
    void log(Level, const char*) {}
};

Logger* getDefaultLogger    (void);
void    umbraLog            (Logger* logger, Logger::Level level, const char* fmt, ...);
void    umbraLogv           (Logger* logger, Logger::Level level, const char* fmt, va_list arg);

#if defined(UMBRA_DEBUG)
#   define UMBRA_LOG_D(LOGGER, ...) umbraLog((LOGGER), Logger::LEVEL_DEBUG, __VA_ARGS__)
#else
#   define UMBRA_LOG_D(LOGGER, ...)
#endif
#define UMBRA_LOG_I(LOGGER, ...) umbraLog((LOGGER), Logger::LEVEL_INFO,     __VA_ARGS__)
#define UMBRA_LOG_W(LOGGER, ...) umbraLog((LOGGER), Logger::LEVEL_WARNING,  __VA_ARGS__)
#define UMBRA_LOG_E(LOGGER, ...) umbraLog((LOGGER), Logger::LEVEL_ERROR,    __VA_ARGS__)

} // namespace Umbra
