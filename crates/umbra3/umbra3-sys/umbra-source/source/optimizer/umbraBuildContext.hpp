#pragma once

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

#include "umbraPrivateDefs.hpp"
#include "optimizer/umbraBuilder.hpp"
#include "umbraStatsHeap.hpp"
#include <standard/Memory.hpp>

namespace Umbra
{

class BuilderHeap: public StatsHeap
{
public:
    void*   allocate     (size_t size, const char* info = NULL);
    void    deallocate   (void* ptr);
};

class BuildContext
{
public:
    BuildContext (const PlatformServices& platform);
    ~BuildContext (void);

    const PlatformServices& getPlatform     (void) const { return m_platform; }
    MemoryManager& getMemory() { return m_memoryManager; }

    BuildContext*   enter   (void);
    void            leave   (void);
    void            deinit  (void);

    static void initServices (PlatformServices& services);

private:

    BuilderHeap         m_statsHeap;
    PlatformServices    m_platform;
    UserMemoryManager   m_memoryManager;
    int                 m_tlsIndex;
    bool                m_allowStore;
    bool                m_validated;
};

class BuilderScope
{
public:
    template<class ImpClass>
    BuilderScope(ImpClass* imp)
    {
        UMBRA_ASSERT(imp);
        m_ctx = imp->getCtx()->enter();
    }

    ~BuilderScope()
    {
        if (m_ctx)
            m_ctx->leave();
    }

    bool isOk() const
    {
        return m_ctx != NULL;
    }

private:
    BuildContext* m_ctx;
};

class BuilderBase
{
public:
    BuildContext* getCtx(void) const { return m_ctx; }
    Allocator* getAllocator(void) const { return m_ctx->getPlatform().allocator; }

protected:
    BuilderBase (BuildContext* ctx): m_ctx(ctx) { UMBRA_ASSERT(ctx); }

private:
    BuildContext* m_ctx;
};

#ifndef UMBRA_COMP_NO_EXCEPTIONS
#   define BUILDER_TRY() try {
#   define BUILDER_CATCH(statement) } catch (OOMException) { statement; }
#else
#   define BUILDER_TRY()
#   define BUILDER_CATCH(statement)
#endif

#define BUILDER_ENTER_GENERIC(imp, ret1, ret2) \
    if (!(imp)) \
        return ret1; \
    BuilderScope builderScope(imp); \
    if (!builderScope.isOk()) \
        return ret2; \
    BUILDER_TRY()

#define BUILDER_EXIT_GENERIC(ret1) \
    BUILDER_CATCH(return ret1)

#define BUILDER_ENTER_ERRORCODE(imp) BUILDER_ENTER_GENERIC(imp, Builder::ERROR_OUT_OF_MEMORY, Builder::ERROR_LICENSE_KEY)
#define BUILDER_EXIT_ERRORCODE() BUILDER_EXIT_GENERIC(Builder::ERROR_OUT_OF_MEMORY)
#define BUILDER_ENTER_VOID(imp) BUILDER_ENTER_GENERIC(imp, UMBRA_EMPTY, UMBRA_EMPTY)
#define BUILDER_EXIT_VOID() BUILDER_EXIT_GENERIC(UMBRA_EMPTY)
#define BUILDER_ENTER_ERRORVALUE(imp, val) BUILDER_ENTER_GENERIC(imp, val, val)
#define BUILDER_EXIT_ERRORVALUE(val) BUILDER_EXIT_GENERIC(val)

}
