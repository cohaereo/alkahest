#pragma once

#include <stdio.h>

namespace Umbra
{

class File
{
public:
    File    (void): m_handle(NULL), m_pos(0), m_size(0), m_buffer(NULL), m_bufferSize(0), m_write(false), m_isStdout(false), m_isOpen(false) {}
    File    (FILE* handle);
    ~File   (void) {}

    bool    open    (const char* filename, const char* mode);
    size_t  read    (void* ptr, size_t size, size_t count);
    void    close   (void);
    int     write   (const void* ptr, size_t size, size_t count);
    int     umbraPrintf (const char * format, ... );
    int     flush   (void);
    int     seek    (long int offset, int origin);
    bool    isOpen  (void) { return m_isOpen; }

    size_t  getSize ()  { return m_size; }

private:
    void*   m_handle;
    int     m_pos;
    size_t  m_size;
    void*   m_buffer;
    int     m_bufferSize;
    bool    m_write;
    bool    m_isStdout;
    bool    m_isOpen;
};

} // namespace Umbra
