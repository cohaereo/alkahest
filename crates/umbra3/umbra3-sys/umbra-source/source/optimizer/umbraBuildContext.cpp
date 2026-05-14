#if !defined(UMBRA_EXCLUDE_COMPUTATION)

/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Build context
 *
 */

#include "umbraBuildContext.hpp"
#include "umbraMemory.hpp"
#include "umbraString.hpp"
#include "umbraLogger.hpp"
#include "umbraThread.hpp"
#include "umbraLicense.hpp"

#if UMBRA_OS == UMBRA_WINDOWS
#   include <Windows.h> // MessageBox
#endif

using namespace Umbra;

void* BuilderHeap::allocate (size_t size, const char* info)
{
    bool old = allowDefaultAllocator(true);
    void* ptr = StatsHeap::allocate(size + 16, info);
    allowDefaultAllocator(old);

    if (!ptr)
        return 0;

    // Fix alignment to 16 bytes
    int8* alignedPtr = align128((int8*)ptr + 1);

    // store offset for freeing the original pointer
    int8 alignOffs = (int8)((UINTPTR)alignedPtr - (UINTPTR)ptr);
    UMBRA_ASSERT(1 <= alignOffs && alignOffs <= 16);
    alignedPtr[-1] = alignOffs;

    return alignedPtr;
}

void BuilderHeap::deallocate (void* ptr)
{
    void* origPtr = NULL;
    if (ptr)
    {
        UMBRA_ASSERT(is128Aligned(ptr));
        int8 alignOffs = ((int8*)ptr)[-1];
        UMBRA_ASSERT(1 <= alignOffs && alignOffs <= 16);

        origPtr = (void*)((UINTPTR)ptr - alignOffs);
    }

    bool old = allowDefaultAllocator(true);
    StatsHeap::deallocate(origPtr);
    allowDefaultAllocator(old);
}

BuildContext::BuildContext (const PlatformServices& platform): m_memoryManager(platform.allocator)
{
    UMBRA_ASSERT(platform.allocator);
    m_statsHeap.setAllocator(platform.allocator);
    m_platform.allocator = &m_statsHeap;
    m_platform.logger = platform.logger;
    m_tlsIndex = Thread::allocTls();

    char licenseKey[128];
    memset(licenseKey, 0, sizeof(licenseKey));
    if (platform.licenseKey)
        platform.licenseKey->readKey(licenseKey);
    m_validated = License::validate(licenseKey);
    if (!m_validated)
    {
        static const char* MESSAGE_EXPIRED =
            "Your Umbra 3 license has expired. Please contact\n"
            "sales@umbrasoftware.com to obtain a valid license or to\n"
            "extend your evaluation period. In case of techical problems\n"
            "please contact support@umbrasoftware.com.\n\n"
            "If you have a valid license key, copy umbra_license.txt\n"
            "to application's working directory.";

        UMBRA_LOG_E(platform.logger, MESSAGE_EXPIRED);
#if UMBRA_OS == UMBRA_WINDOWS
        static bool alertShown = false;
        if (!alertShown && !getenv("UMBRA_SUPPRESS_LICENSE_DIALOG"))
        {
            alertShown = true;
            MessageBox(GetActiveWindow(), MESSAGE_EXPIRED, "Umbra 3 Optimizer Failed", MB_DEFBUTTON1 | MB_ICONEXCLAMATION);
        }
#endif
    }
}

BuildContext::~BuildContext (void)
{
    m_platform.allocator = m_statsHeap.getAllocator();
    Thread::freeTls(m_tlsIndex);
}

BuildContext* BuildContext::enter (void)
{
    if (!m_validated)
        return NULL;

    // TODO: this is now true during migration to umbra standard
    bool old = allowDefaultAllocator(true);
    Thread::setTlsValue(m_tlsIndex, old ? 1 : 0);
    return this;
}

void BuildContext::leave (void)
{
    bool old = (Thread::getTlsValue(m_tlsIndex) != 0);
    allowDefaultAllocator(old);
}

void BuildContext::deinit (void)
{
    // unwrap actual allocator and deinit stats heap
    m_platform.allocator = m_statsHeap.getAllocator();
    m_statsHeap.dump(m_platform.logger);
    m_statsHeap.reset();
}

void BuildContext::initServices (PlatformServices& platform)
{
    if (!platform.allocator)
        platform.allocator = getAllocator();
    if (!platform.logger)
        platform.logger = getDefaultLogger();
};

#endif
