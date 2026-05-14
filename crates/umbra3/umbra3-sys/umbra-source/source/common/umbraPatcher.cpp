// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraPatcher.hpp"
#include "umbraPlatform.hpp"

using namespace Umbra;

PatchWriter::PatchWriter()
    : m_stream(NULL)
    , m_output(NULL)
    , m_finished(false)
{
}

PatchWriter::~PatchWriter()
{
    if (!m_finished)
        finish();
}

void PatchWriter::init(OutputStream& stream, const Umbra::UINT8* output)
{
    m_stream = &stream;
    m_output  = output;
}

bool PatchWriter::patch(const void* input, int inputIdx, const void* patchValueAddr, const void* patchTargetAddr)
{
    UMBRA_ASSERT((const Umbra::UINT8*)patchValueAddr  >= (const Umbra::UINT8*)input);
    UMBRA_ASSERT((const Umbra::UINT8*)patchTargetAddr >= m_output);

    PointerPatch patch;
    patch.inputIdx = inputIdx;
    patch.offset   = (UINT32)((const Umbra::UINT8*)patchValueAddr - (const Umbra::UINT8*)input);
    patch.dst      = (UINT32)((const Umbra::UINT8*)patchTargetAddr - m_output);
    return m_stream->write(&patch, sizeof(PointerPatch)) == sizeof(PointerPatch);
}

bool PatchWriter::patch(const void* patchValueAddr, const void* patchTargetAddr)
{
    UMBRA_ASSERT((const Umbra::UINT8*)patchValueAddr  >= m_output);
    UMBRA_ASSERT((const Umbra::UINT8*)patchTargetAddr >= m_output);

    PointerPatch patch;
    patch.inputIdx = INPUT_SELF;
    patch.offset   = (UINT32)((const Umbra::UINT8*)patchValueAddr  - m_output);
    patch.dst      = (UINT32)((const Umbra::UINT8*)patchTargetAddr - m_output);
    return m_stream->write(&patch, sizeof(PointerPatch)) == sizeof(PointerPatch);
}

bool PatchWriter::finish()
{
    m_finished = true;
    PointerPatch patch;
    patch.inputIdx = INPUT_END;
    return m_stream->write(&patch, sizeof(PointerPatch)) == sizeof(PointerPatch);
}

size_t PatchWriter::computeSize(int patches)
{
    return (patches + 1) * sizeof(PointerPatch);
}

PatchReader::PatchReader()
{
}
