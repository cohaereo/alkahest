#include "umbraFile.hpp"
#include "umbraPrivateDefs.hpp"

#if UMBRA_ARCH != UMBRA_SPU && UMBRA_OS != UMBRA_CAFE

#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <stdio.h>

namespace Umbra
{

File::File(FILE* file) :
    m_handle(file),
    m_pos(0),
    m_size(0),
    m_buffer(NULL),
    m_bufferSize(0),
    m_write(false),
    m_isOpen(false)
{
}

size_t File::read(void* ptr, size_t size, size_t count)
{
    UMBRA_ASSERT(m_handle);
    if (!m_handle)
        return 0;

    return (int)fread(ptr, size, count, (FILE*)m_handle);
}

bool File::open(const char* filename, const char* mode)
{
    m_handle = fopen(filename, mode);

    if (m_handle)
    {
        FILE* file = (FILE*)m_handle;
        fseek(file, 0, SEEK_END);
        m_size = ftell(file);
        fseek(file, 0, SEEK_SET);
        m_isOpen = true;

        return true;
    }

    return false;
}

void File::close()
{
    UMBRA_ASSERT(m_handle);
    if (m_handle)
        fclose((FILE*)m_handle);
    m_isOpen = false;
}

int File::umbraPrintf(const char* format, ... )
{
    UMBRA_ASSERT(m_handle);
    if (!m_handle)
        return 0;

    char out[256] = "";
    va_list args;
    va_start (args, format);
    vsnprintf (out, 256, format, args);
    va_end (args);

    return fprintf((FILE*)m_handle, "%s", out);
}

int File::flush()
{
    UMBRA_ASSERT(m_handle);
    if (!m_handle)
        return EOF;

    return fflush(((FILE*)m_handle));
}

int File::write(const void* ptr, size_t size, size_t count)
{
    UMBRA_ASSERT(m_handle);
    if (!m_handle)
        return 0;

    return (int)fwrite(ptr, size, count, (FILE*)m_handle);
}

int File::seek(long int offset, int origin)
{
    UMBRA_ASSERT(m_handle);
    if (!m_handle)
        return 1;

    return fseek(((FILE*)m_handle), offset, origin);
}

} // namespace Umbra

#endif // UMBRA_OS
