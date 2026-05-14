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
 * \brief   Main builder frontend, implementation of umbraBuilder.hpp
 *
 */

#include "umbraTileProcessor.hpp"
#include "umbraLogger.hpp"
#include "umbraTomeGenerator.hpp"
#include "umbraCRCStream.hpp"

using namespace Umbra;

namespace Umbra
{
    class ImpBuilder
    {
    public:
        ImpBuilder (const PlatformServices& platform): m_ctx(platform)
        {
        }

        BuildContext*       getCtx          (void)                          { return &m_ctx; }
        Logger*             getLogger       (void)                          { return m_ctx.getPlatform().logger; }
        Allocator*          getAllocator    (void)                          { return m_ctx.getPlatform().allocator; }
    private:

        BuildContext      m_ctx;
    };
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Builder (const PlatformServices& platform_): m_imp(NULL)
{
    init(platform_);
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Builder (void): m_imp(NULL)
{
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void Builder::init (const PlatformServices& platform_)
{
    PlatformServices platform(platform_);
    BuildContext::initServices(platform);
    UMBRA_LOG_D(platform.logger, "Umbra::Builder init");
    BUILDER_TRY()
    m_imp = UMBRA_HEAP_NEW(platform.allocator, ImpBuilder, platform);
    BUILDER_CATCH(UMBRA_EMPTY)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::~Builder (void)
{
    BUILDER_TRY()
    m_imp->getCtx()->deinit();
    BUILDER_CATCH(UMBRA_EMPTY)
    PlatformServices platform(m_imp->getCtx()->getPlatform());
    UMBRA_HEAP_DELETE(platform.allocator, m_imp);
    m_imp = NULL;
    UMBRA_LOG_D(platform.logger, "Umbra::Builder destructor");
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

static bool getTileSizeAndGeometryDistance(const ComputationParams& p, float& ts, float& geom)
{
    float so;
    p.getParam(ComputationParams::SMALLEST_OCCLUDER, so);
    if (so <= 0.f)
        return false;

    p.getParam(ComputationParams::CLUSTER_SIZE, ts);
    if (ts <= 0.f)
        ts = so;

    float bfLimit;
    p.getParam(ComputationParams::BACKFACE_LIMIT, bfLimit);
    if (bfLimit < 100.f)
        geom = so * 1.6f;
    else
    {
        p.getParam(ComputationParams::SMALLEST_HOLE, geom);
        geom *= 2.f;
    }

    return true;
}

static bool isTomeAligned(const ComputationParams& p, const Vector3& tomeMn, const Vector3& tomeMx)
{
    float ts, geom;
    if (!getTileSizeAndGeometryDistance(p, ts, geom))
        return false;

    for (int i = 0; i < 3; i++)
    {
        float mn = floorf(tomeMn[i] / ts) * ts;
        if (mn != tomeMn[i])
            return false;

        float mx = floorf(tomeMx[i] / ts) * ts;
        if (mx != tomeMx[i])
            return false;

        if (tomeMn[i] >= tomeMx[i])
            return false;
    }

    return true;
}

Builder::Error Builder::getGeometryBounds(Vector3& geomMn, Vector3& geomMx, const ComputationParams& p, const Vector3& tomeMn, const Vector3& tomeMx)
{
    float ts, geom;
    if (!getTileSizeAndGeometryDistance(p, ts, geom))
        return ERROR_PARAM;

    geomMn.x = tomeMn.x - geom;
    geomMn.y = tomeMn.y - geom;
    geomMn.z = tomeMn.z - geom;
    geomMx.x = tomeMx.x + geom;
    geomMx.y = tomeMx.y + geom;
    geomMx.z = tomeMx.z + geom;

    return SUCCESS;
}

/*---------------------------------------------------------------*//*!
 * \brief   Build initialization
 *//*---------------------------------------------------------------*/

Builder::Error Builder::split(
    TileInputSet& res,
    Scene* scene,
    const ComputationParams& params)
{
    BUILDER_ENTER_ERRORCODE(m_imp)

    UMBRA_HEAP_DELETE(m_imp->getAllocator(), res.m_imp);
    res.m_imp = UMBRA_HEAP_NEW(m_imp->getAllocator(), ImpTileInputSet, m_imp->getCtx());
    return res.m_imp->init(scene, params, AABB());

    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief   Build initialization
 *//*---------------------------------------------------------------*/

Builder::Error Builder::split(
    TileInputSet& res,
    Scene* scene,
    const ComputationParams& params,
    const Vector3& aabbMn,
    const Vector3& aabbMx)
{
    BUILDER_ENTER_ERRORCODE(m_imp)

    if (!isTomeAligned(params, aabbMn, aabbMx))
        return ERROR_PARAM;

    ImpTileInputSet* imp = UMBRA_HEAP_NEW(m_imp->getAllocator(), ImpTileInputSet, m_imp->getCtx());
    Error err = imp->init(scene, params, AABB(aabbMn, aabbMx));
    if (err != Builder::SUCCESS)
    {
        UMBRA_HEAP_DELETE(m_imp->getAllocator(), imp);
    }
    else
    {
        if (res.m_imp)
            UMBRA_HEAP_DELETE(m_imp->getAllocator(), res.m_imp);
        res.m_imp = imp;
    }

    return err;

    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief   Tile computation
 *//*---------------------------------------------------------------*/

Builder::Error Builder::computeTile(
    TileResult& out,
    const TileInput& in)
{
    BUILDER_ENTER_ERRORCODE(m_imp)

    // note: we delete old result content even on error
    UMBRA_HEAP_DELETE(m_imp->getAllocator(), out.m_imp);
    out.m_imp = TileProcessor(m_imp->getCtx()).execute(*in.m_imp);
    return out.m_imp ? SUCCESS : ERROR_OUT_OF_MEMORY;

    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error Builder::join(TomeGenerator& gen, const ComputationParams& params)
{
    BUILDER_ENTER_ERRORCODE(m_imp)

    // note: we delete old result content even on error
    UMBRA_HEAP_DELETE(m_imp->getAllocator(), gen.m_imp);
    gen.m_imp = UMBRA_HEAP_NEW(m_imp->getAllocator(), ImpTomeGenerator, m_imp->getCtx(), params, AABB());
    return gen.m_imp ? SUCCESS : ERROR_OUT_OF_MEMORY;

    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error Builder::join(TomeGenerator& gen, const ComputationParams& params, const Vector3& mn, const Vector3& mx)
{
    BUILDER_ENTER_ERRORCODE(m_imp)

    // note: we delete old result content even on error
    UMBRA_HEAP_DELETE(m_imp->getAllocator(), gen.m_imp);
    gen.m_imp = UMBRA_HEAP_NEW(m_imp->getAllocator(), ImpTomeGenerator, m_imp->getCtx(), params, AABB(mn, mx));
    return gen.m_imp ? SUCCESS : ERROR_OUT_OF_MEMORY;

    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error Builder::loadTileInput (TileInput& elem, InputStream& in)
{
    BUILDER_ENTER_ERRORCODE(m_imp)

    UMBRA_HEAP_DELETE(m_imp->getAllocator(), elem.m_imp);
    elem.m_imp = UMBRA_HEAP_NEW(m_imp->getAllocator(), ImpTileInput, m_imp->getCtx());
    CRCInputStream in2(m_imp->getAllocator(), in);
    Deserializer loader(&in2);
    stream(loader, *elem.m_imp);
    return (loader.isOk() && in2.isOk()) ? SUCCESS : ERROR_PARAM;

    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error Builder::loadTileResult (TileResult& elem, InputStream& in)
{
    BUILDER_ENTER_ERRORCODE(m_imp)

    UMBRA_HEAP_DELETE(m_imp->getAllocator(), elem.m_imp);
    elem.m_imp = UMBRA_HEAP_NEW(m_imp->getAllocator(), ImpTileResult, m_imp->getCtx());
    CRCInputStream in2(m_imp->getAllocator(), in);
    Deserializer loader(&in2);
    stream(loader, *elem.m_imp);
    return (loader.isOk() && in2.isOk()) ? SUCCESS : ERROR_PARAM;

    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TomeGenerator::TomeGenerator (void): m_imp(NULL)
{
    // m_imp is allocated in Builder::generateTome
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TomeGenerator::~TomeGenerator (void)
{
    BUILDER_ENTER_VOID(m_imp)
    UMBRA_HEAP_DELETE(m_imp->getAllocator(), m_imp);
    m_imp = NULL;
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TomeGenerator::setNumThreadsExt(int n)
{
    BUILDER_ENTER_VOID(m_imp)
    m_imp->setNumThreads(n);
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/


void TomeGenerator::setCachePathExt(const char* path)
{
    BUILDER_ENTER_VOID(m_imp)
    m_imp->setCachePath(path);
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error TomeGenerator::addTileResult (const TileResult& tile)
{
    BUILDER_ENTER_ERRORCODE(m_imp)
    return m_imp->addTile(tile.m_imp);
    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

float TomeGenerator::getProgress () const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, 0.f)
    return m_imp->getProgress();
    BUILDER_EXIT_ERRORVALUE(0.f)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error TomeGenerator::getTomeSize (Umbra::UINT32& size) const
{
    BUILDER_ENTER_ERRORCODE(m_imp)
    return m_imp->getTomeSize(size);
    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

const Tome* TomeGenerator::getTome (Umbra::UINT8* buf, Umbra::UINT32 bufSize) const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, NULL)
    return m_imp->getTome(buf, bufSize);
    BUILDER_EXIT_ERRORVALUE(NULL)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TomeGenerator::visualizeState(DebugRenderer* debug) const
{
    BUILDER_ENTER_VOID(m_imp)
    m_imp->visualize(debug);
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TileInput::TileInput (void): m_imp(NULL) {}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TileInput::~TileInput (void)
{
    BUILDER_ENTER_VOID(m_imp)
    UMBRA_HEAP_DELETE(m_imp->getCtx()->getPlatform().allocator, m_imp);
    m_imp = NULL;
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TileInput::serialize (OutputStream& out) const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, false)
    return m_imp->serialize(out);
    BUILDER_EXIT_ERRORVALUE(false)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TileInput::equals (const TileInput& other) const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, false)

    // \todo Compare both concurrently or serialize only the other
    // and implement an OutputStream that compares the one against
    // the other without actually allocating another buf
    // ...Or just implement ImpTileInput::operator==
    MemOutputStream aStream(m_imp->getAllocator());
    MemOutputStream bStream(m_imp->getAllocator());

    serialize(aStream);
    other.serialize(bStream);
    if (aStream.getSize() == bStream.getSize() &&
        !memcmp(aStream.getPtr(), bStream.getPtr(), aStream.getSize()))
        return true;

    return false;

    BUILDER_EXIT_ERRORVALUE(false)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

void TileInput::getAABB (Vector3& mn, Vector3& mx) const
{
    BUILDER_ENTER_VOID(m_imp)
    AABB aabb = m_imp->getAABB();
    mn = aabb.getMin();
    mx = aabb.getMax();
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TileInput::isEmpty (void) const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, false)
    return m_imp->isEmpty();
    BUILDER_EXIT_ERRORVALUE(false)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

const char* TileInput::getHashValue (void) const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, NULL)
    return m_imp->getHash();
    BUILDER_EXIT_ERRORVALUE(NULL)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TileInputSet::TileInputSet (void) : m_imp(NULL) {}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TileInputSet::~TileInputSet (void)
{
    BUILDER_ENTER_VOID(m_imp)
    UMBRA_HEAP_DELETE(m_imp->getAllocator(), m_imp);
    m_imp = NULL;
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

int TileInputSet::size (void) const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, -1)
    return m_imp->size();
    BUILDER_EXIT_ERRORVALUE(-1)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

Builder::Error TileInputSet::get (TileInput& tile, int idx) const
{
    BUILDER_ENTER_ERRORCODE(m_imp)
    Builder::Error err = m_imp->get(&tile.m_imp, idx) ? Builder::SUCCESS : Builder::ERROR_EMPTY_ITERATOR;
    return err;
    BUILDER_EXIT_ERRORCODE()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TileResult::TileResult (void): m_imp(NULL) {}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

TileResult::~TileResult (void)
{
    BUILDER_ENTER_VOID(m_imp)
    UMBRA_HEAP_DELETE(m_imp->getAllocator(), m_imp);
    m_imp = NULL;
    BUILDER_EXIT_VOID()
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TileResult::serialize (OutputStream& out) const
{
    BUILDER_ENTER_ERRORVALUE(m_imp, false)
    CRCOutputStream out2(out);
    Serializer serializer(&out2);
    stream(serializer, *m_imp);
    return serializer.isOk() && out2.flush();
    BUILDER_EXIT_ERRORVALUE(false)
}

/*---------------------------------------------------------------*//*!
 * \brief
 *//*---------------------------------------------------------------*/

bool TileResult::stripObjects (const Umbra::UINT32* ids, int len)
{
    BUILDER_ENTER_ERRORVALUE(m_imp, false)
    Set<UINT32> idSet(ids, len, m_imp->getAllocator());
    m_imp->m_cellGraph.removeTargetObjectsById(idSet);
    return true;
    BUILDER_EXIT_ERRORVALUE(false)
}


#endif // !defined(UMBRA_EXCLUDE_COMPUTATION)
