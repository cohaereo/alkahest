// Copyright (c) 2009-2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#pragma once
#ifndef __UMBRAPATCH_H
#define __UMBRAPATCH_H

#include "umbraPrivateDefs.hpp"
#include "umbraPlatform.hpp"

namespace Umbra
{

class PatchWriter
{
public:
    PatchWriter();
    ~PatchWriter();

    void    init(OutputStream& stream, const Umbra::UINT8* output);

    bool    patch(const void* input, int inputIdx, const void* patchValueAddr, const void* patchTargetAddr);
    bool    patch(const void* patchValueAddr, const void* patchTargetAddr);

    bool    finish();

    static size_t computeSize(int patches);

private:

    static const int INPUT_SELF = -1;
    static const int INPUT_END  = -2;

    PatchWriter(const PatchWriter&);
    PatchWriter& operator=(const PatchWriter&);

    OutputStream*        m_stream;
    const Umbra::UINT8*  m_output;
    bool                 m_finished;

    friend class PatchReader;
};

class PatchReader
{
public:
    PatchReader();

    template<typename A, typename B>
    bool init(InputStream& stream, A* output, B** inputs, int nInputs);

private:
    PatchReader(const PatchReader&);
    PatchReader& operator=(const PatchReader&);
};


////////////////////////////////////


struct PointerPatch
{
    int     inputIdx;
    UINT32  offset;
    UINT32  dst;
};

template<typename T>
const UINT8* computePatchAddr(const T* base, Umbra::UINT32 offset)
{
    return (const UINT8*)base + offset;
}

template<typename A, typename B>
bool PatchReader::init(InputStream& stream, A* output, B** inputs, int nInputs)
{
    UMBRA_UNREF(nInputs);

    for (;;)
    {
        PointerPatch patch;
        if (stream.read(&patch, sizeof(PointerPatch)) != sizeof(PointerPatch))
            return false;
        if (patch.inputIdx == PatchWriter::INPUT_END)
            return true;

        UMBRA_ASSERT(patch.inputIdx < nInputs);
        
        if (patch.inputIdx == PatchWriter::INPUT_SELF)
        {
            const void** dst  = (const void**)computePatchAddr<A>(output, patch.dst);
            const void*  addr = computePatchAddr<A>(output, patch.offset);
            *dst = addr;
            continue;
        }

        const void** dst  = (const void**)computePatchAddr<A>(output, patch.dst);
        const void*  addr = computePatchAddr<B>(inputs[patch.inputIdx], patch.offset);
        *dst = addr;            
    }
}

} // namespace Umbra

#endif // __UMBRAPATCH_H