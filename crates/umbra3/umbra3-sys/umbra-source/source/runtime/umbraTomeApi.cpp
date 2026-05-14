/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra runtime data
 *
 */

#include "umbraTomePrivate.hpp"
#include "umbraTomeCollection.hpp"
#include "runtime/umbraTome.hpp"
#include "umbraQueryContext.hpp"
#include "umbraMemory.hpp"
#include "umbraBinStream.hpp"

#include <stdio.h>
#include <stdlib.h>
#if UMBRA_COMPILER == UMBRA_MSC
#include <new.h>
#else
#include <new>
#endif

#undef THIS
#define THIS(x) ((x*)this)

namespace Umbra
{

UMBRA_CT_ASSERT(sizeof(ImpTomeCollection) <= UMBRA_TOMECOLLECTION_SIZE);

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

class TomeUserIdMap
{
public:

    TomeUserIdMap   (void): m_map(NULL), m_numElems(0) {}
    TomeUserIdMap   (const UINT32* map, int numElems): m_map(map), m_numElems(numElems) {}

    UINT32          getSize         (void) const;
    UINT32          getUserId       (int index) const;
    int             getIndex        (UINT32 id) const;

private:

    const UINT32*   m_map;
    int             m_numElems;
};

/*-------------------------------------------------------------------*//*!
 * \internal
 * \brief
 *//*-------------------------------------------------------------------*/

Tome* allocTome (size_t size, Allocator* a)
{
    // room for alignment + HEADER
    size += 20;
    UINT8* buf = a ? (UINT8*)a->allocate(size) : (UINT8*)malloc(size);
    if (!buf)
        return NULL;
    Tome* t = (Tome*)(((UINTPTR)buf + 20) & ~15);
    UINT32* header = ((UINT32*)t) - 1;
    *header = (UINT32)((UINT8*)t - buf);
    return t;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

static Tome::Status checkVersionMagic (UINT32 v)
{
    // magic
    if ((v >> 16) != TOME_MAGIC)
    {
        UINT32 swapped = swapBytes_4(&v);
        if ((swapped >> 16) == TOME_MAGIC)
            return Tome::STATUS_BAD_ENDIAN;

        return Tome::STATUS_CORRUPT;
    }

    // version
    UINT32 version = v & 0xFFFF;
    if (version > TOME_VERSION)
        return Tome::STATUS_NEWER_VERSION;
    if (version < TOME_VERSION)
        return Tome::STATUS_OLDER_VERSION;

    return Tome::STATUS_OK;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Tome::Status checkStatus (const ImpTome* imp, bool checkCRC32)
{
    if (!imp)
        return Tome::STATUS_UNINITIALIZED;

    Tome::Status s = checkVersionMagic(imp->getVersionMagic());
    if (s != Tome::STATUS_OK)
        return s;

    // align
    if (((UINTPTR)imp) & 0xF)
        return Tome::STATUS_BAD_ALIGN;

    if (checkCRC32 && imp->computeCRC32() != imp->getCRC32())
        return Tome::STATUS_CORRUPT;

    return Tome::STATUS_OK;
}


/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 TomeUserIdMap::getSize (void) const
{
    return m_numElems;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 TomeUserIdMap::getUserId (int index) const
{
    UMBRA_ASSERT(index >= 0 && index < m_numElems);
    return m_map[index];
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeUserIdMap::getIndex (Umbra::UINT32 id) const
{
    for (int i = 0; i < m_numElems; i++)
    {
        if (m_map[i] == id)
            return i;
    }
    return -1;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Tome::Status Tome::getStatus (void) const
{
    const ImpTome* imp = THIS(ImpTome);
    return checkStatus(imp, false);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Tome::Status Tome::checkCorruption (void) const
{
    const ImpTome* imp = THIS(ImpTome);
    return checkStatus(imp, true);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getStatistic (Statistic type) const
{
    switch (type)
    {
    case STAT_TOTAL_DATA_SIZE:
        return THIS(ImpTome)->getSize();
    case STAT_PORTAL_DATA_SIZE:
    case STAT_TILE_COMMON_DATA_SIZE:
    case STAT_VIEWTREE_SIZE:
    case STAT_ACCURATE_LOCATION_SIZE:
        {
            int s = 0;
            int numTiles = THIS(ImpTome)->getTreeNodeCount();
            for (int i = 0; i < numTiles; i++)
            {
                const ImpTile* t = THIS(ImpTome)->getTile(i, false);
                if (!t)
                    continue;
                if (type == STAT_PORTAL_DATA_SIZE)
                    s += ImpTile::getPortalDataSize(t);
                else if (type == STAT_VIEWTREE_SIZE)
                    s += ImpTile::getViewTreeDataSize(t);
                else if (type == STAT_ACCURATE_LOCATION_SIZE)
                    s += ImpTile::getAccurateFindDataSize(t);
                else
                {
                    UMBRA_ASSERT(type == STAT_TILE_COMMON_DATA_SIZE);
                    s += t->getSize();
                    s -= ImpTile::getPortalDataSize(t);
                    s -= ImpTile::getViewTreeDataSize(t);
                    s -= ImpTile::getAccurateFindDataSize(t);
                }
            }
            if (type == STAT_PORTAL_DATA_SIZE)
            {
                s += THIS(ImpTome)->getObjectLists().getSizeInBytes();
                s += THIS(ImpTome)->getClusterLists().getSizeInBytes();
            }
            return s;
        }

    case STAT_CLUSTER_GRAPH_DATA_SIZE:
        return THIS(ImpTome)->getClusterNodes().getSizeInBytes() +
               THIS(ImpTome)->getClusterPortals().getSizeInBytes();

    case STAT_OBJECT_DATA_SIZE:
        return THIS(ImpTome)->getObjectBounds().getSizeInBytes() +
               THIS(ImpTome)->getObjectDistances().getSizeInBytes() +
               THIS(ImpTome)->getNumObjects() * sizeof(UINT32); // index to id mapping

    case STAT_PORTAL_GEOMETRY_SIZE:
        return ImpTome::getPrivateStatistic(THIS(ImpTome), PRIVSTAT_PORTAL_GEOMETRY_DATA_SIZE);

    case STAT_MATCHING_DATA_SIZE:
        {
            DataArray matchData = THIS(ImpTome)->getMatchingData();
            DataArray matchTrees = THIS(ImpTome)->getMatchingTrees();
            if (!matchData)
                return 0;
            int s = matchData.getSizeInBytes() + matchTrees.getSizeInBytes();
            int numTiles = THIS(ImpTome)->getTreeNodeCount();
            int leafIdx = 0;
            for (int i = 0; i < numTiles; i++)
            {
                const ImpTile* t = THIS(ImpTome)->getTile(i, false);
                if (!t || !t->isLeaf())
                    continue;

                LeafTileMatchData data;
                matchData.getElem(data, leafIdx++);

                // cell hierarchy map
                s += UMBRA_BITVECTOR_SIZE(data.m_cellLodBitWidth * data.m_cellLodElemWidth * t->getNumCells());

                // matching tree data
                for (int j = 0; j < data.getMatchTreeCount(); j++)
                {
                    SerializedTreeData tree;
                    matchTrees.getElem(tree, data.getMatchTreeOfs() + j);
                    s += KDTree::getDataDwords(tree.getNodeCount()) * sizeof(UINT32) +
                        UMBRA_BITVECTOR_SIZE(((tree.getNodeCount() + 1) / 2) * tree.getMapWidth());
                }
            }

            return s;
        }

    default:
        return 0;
    };
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 Tome::getSize (void) const
{
    const ImpTome* tome = THIS(ImpTome);
    if (tome->getVersionMagic() == TOME_PVSB_MARKER)
        return 0; // can't say
    return tome->getSize();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getVersion (void) const
{
    const ImpTome* tome = THIS(ImpTome);
    if (tome->getVersionMagic() == TOME_PVSB_MARKER)
        return TOME_VERSION_PVSBOOSTER;
    return THIS(ImpTome)->getVersionMagic() & 0xFFFF;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Tome::Status Tome::init (const Tome** tome, const Umbra::UINT8* buf, size_t size)
{
    const Tome* t = (const Tome*)buf;
    Tome::Status status = t->getStatus();
    if ((status == STATUS_OK) && (size < t->getSize()))
        status = STATUS_OUT_OF_MEMORY;
    if ((status == STATUS_OK) && tome)
        *tome = t;
    return status;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void Tome::swapEndianness (Tome* data, size_t size)
{
    UINT32* ptr = (UINT32*)data;
    size_t dwords = size / 4;
    while (dwords--)
    {
        *ptr = swapBytes_4(ptr);
        ptr++;
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

template <typename T>
static bool matchConverter(int srcVersion, int targetVersion)
{
    if (T::getTargetVersion() != targetVersion)
        return false;

    for (int i = 0; i < T::getSourceCount(); i++)
    {
        if (T::getSourceVersion(i) == srcVersion)
            return true;
    }

    return false;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

const Tome* Tome::updateVersion (const Tome* data, Allocator*)
{
    // Note: conversion not supported at the moment
    if (data->getStatus() == Tome::STATUS_OLDER_VERSION)
    {
        return NULL;
    }
    return data;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

bool Tome::testCapability (Umbra::UINT32 caps) const
{
    if ((caps & CAPABILITY_TOMECOLLECTION_INPUT) && !THIS(ImpTome)->getMatchingData())
        return false;
    if ((caps & CAPABILITY_OBJECT_OPTIMIZATIONS) && !THIS(ImpTome)->hasObjectDepthmaps())
        return false;
    // connectivity queries always supported
    if (caps & ~(CAPABILITY_TOMECOLLECTION_INPUT | CAPABILITY_CONNECTIVITY_QUERIES | CAPABILITY_OBJECT_OPTIMIZATIONS))
        return false;

    return true;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getObjectCount(void) const
{
    return THIS(ImpTome)->getNumObjects();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void Tome::getObjectBounds(int objIndex, Vector3& mn, Vector3& mx) const
{
    ObjectBounds o;
    const ImpTome* t = THIS(ImpTome);
    DataArray bounds = t->getObjectBounds();
    bounds.m_base = t;
    bounds.getElem(o, objIndex);
    mn = o.mn;
    mx = o.mx;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getGateCount(void) const
{
    return THIS(ImpTome)->getNumGates();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getObjectUserIDs(int idx, Umbra::UINT32* ids, int max) const
{
    const ImpTome* t = THIS(ImpTome);

    if (t->getUserIDStarts() == 0)
    {
        // Groups were not computed.

        if (idx < 0 || idx >= getObjectCount())
            return 0;

        UINT32 id = getObjectUserID(idx);
        if (ids && max >= 1)
            ids[0] = id;

        return 1;
    }

    int n = t->getUserIDCount(idx);

    if (ids)
        for (int i = 0; i < min2(n, max); i++)
            ids[i] = t->getUserIDs(idx)[i];

    return n;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

size_t Tome::getGateStateSize(void) const
{
    return UMBRA_BITVECTOR_SIZE(getGateCount());
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getClusterCount(void) const
{
    return THIS(ImpTome)->getNumClusters();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getClusterPortalCount(void) const
{
    return THIS(ImpTome)->getNumClusterPortals();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void Tome::getClusterBounds(int idx, Vector3& mn, Vector3& mx) const
{
    THIS(ImpTome)->getClusterBounds(mn, mx, idx);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getCellCount(void) const
{
    return THIS(ImpTome)->getNumCells();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::getTileCount(void) const
{
    return THIS(ImpTome)->getNumTiles();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void Tome::getBounds(Vector3& mn, Vector3& mx) const
{
    const ImpTome* t = THIS(ImpTome);
    mn = t->getTreeMin();
    mx = t->getTreeMax();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 Tome::getObjectUserID(int objIndex) const
{
    if (THIS(ImpTome)->containsGroups())
        return (UINT32)-1;
    return TomeUserIdMap(THIS(ImpTome)->getUserIDs(), THIS(ImpTome)->getNumObjects()).getUserId(objIndex);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 Tome::getGateUserID(int portalIndex) const
{
    return TomeUserIdMap(THIS(ImpTome)->getGateIndexMap(), THIS(ImpTome)->getNumGates()).getUserId(portalIndex);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::findObjectIndex(Umbra::UINT32 id) const
{
    const ImpTome* t = THIS(ImpTome);

    if (t->getUserIDStarts() == 0)
        return TomeUserIdMap(THIS(ImpTome)->getUserIDs(), THIS(ImpTome)->getNumObjects()).getIndex(id);

    for (int i = 0; i < getObjectCount(); i++)
    {
        const UINT32* ids = t->getUserIDs(i);
        int           c   = t->getUserIDCount(i);

        for (int j = 0; j < c; j++)
            if (ids[j] == id)
                return i;
    }

    return -1;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int Tome::findGateIndex(Umbra::UINT32 id) const
{
    return TomeUserIdMap(THIS(ImpTome)->getGateIndexMap(), THIS(ImpTome)->getNumGates()).getIndex(id);
}

/*-------------------------------------------------------------------*//*!
 * \brief   Do whatever you can for TomeLoader loaded tomes
 *//*-------------------------------------------------------------------*/

static const Tome* fixTome (Tome* input, size_t size, Allocator* allocator)
{
    // not possible because we allocated the tome ourselves
    UMBRA_ASSERT(input->getStatus() != Tome::STATUS_BAD_ALIGN);

    // simple things first: swap endianness in-place if necessary
    if (input->getStatus() == Tome::STATUS_BAD_ENDIAN)
        Tome::swapEndianness(input, size);

    // we just fixed this
    UMBRA_ASSERT(input->getStatus() != Tome::STATUS_BAD_ENDIAN);

    // check that size makes sense at this point
    // for very old versions we don't actually now the serialized size
    if (input->getVersion() > TOME_VERSION_PVSBOOSTER && input->getSize() > size)
        return NULL;

    // convert from older version, if possible
    const Tome* updated = Tome::updateVersion(input, allocator);
    return updated ? updated : input;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

const Tome* TomeLoader::load (InputStream& input, Allocator* a)
{
    UINT32 header[3];
    if (input.read(header, sizeof(header)) != sizeof(header))
        return NULL;
    UINT32 versionMagic = header[0];
    UINT32 size = header[2];
    Tome::Status s = checkVersionMagic(versionMagic);
    if (s == Tome::STATUS_BAD_ENDIAN)
    {
        versionMagic = swapBytes_4(&versionMagic);
        size = swapBytes_4(&size);
        s = checkVersionMagic(versionMagic);
    }
    if (s == Tome::STATUS_CORRUPT)
        return NULL;
    Tome* t = allocTome(size, a);
    if (!t)
        return NULL;
    memcpy(t, header, sizeof(header));
    UINT32 readSize = size - sizeof(header);
    if (input.read((UINT8*)t + sizeof(header), readSize) != readSize)
    {
        freeTome(t);
        return NULL;
    }
    const Tome* fixed = fixTome(t, size, a);
    if (fixed != t)
        freeTome(t);
    return fixed;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

const Tome* TomeLoader::loadFromBuffer (const Umbra::UINT8* buffer, size_t bufSize, Allocator* a)
{
    MemInputStream s(buffer, (UINT32)bufSize);
    return load(s, a);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void TomeLoader::freeTome(const Tome* tome, Allocator* a)
{
    if (!tome)
        return;

    UINT32 ofs = *((UINT32*)tome - 1);
    UINT8* buf = (UINT8*)tome - ofs;
    if (a)
        a->deallocate(buf);
    else
        free(buf);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

TomeCollection::TomeCollection(Allocator* a)
{
    UMBRA_ASSERT((void*)this == (void*)m_mem);
    if (!a)
        a = getAllocator();
    new (IMPL(this)) ImpTomeCollection(a);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

TomeCollection::~TomeCollection(void)
{
    IMPL(this)->~ImpTomeCollection();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void TomeCollection::init(Allocator* a)
{
    ImpTomeCollection* imp = IMPL(this);
    if (!a)
        a = getAllocator();
    imp->~ImpTomeCollection();
    new (this) ImpTomeCollection(a);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void TomeCollection::init(void* buf, size_t size)
{
    ImpTomeCollection* imp = IMPL(this);
    imp->~ImpTomeCollection();
    new (this) ImpTomeCollection(buf, size);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

TomeCollection::ErrorCode TomeCollection::build(
    const Tome** tomes, int numTomes, Allocator* scratchAlloc,
    const TomeCollection* previous)
{
    ImpTomeCollection* imp = IMPL(this);
    const ImpTomeCollection* prevImp = IMPL(previous);
    return imp->build((const ImpTome**)tomes, numTomes, AABB(), scratchAlloc, prevImp);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

TomeCollection::ErrorCode TomeCollection::build(
    const Tome** tomes, int numTomes, const Vector3& mn, 
    const Vector3& mx, Allocator* resultAlloc,
    const TomeCollection* previous)
{
    ImpTomeCollection* imp  = IMPL(this);
    const ImpTomeCollection* prevImp = IMPL(previous);
    AABB aabb(mn, mx);
    if (!aabb.isOK())
        return ERROR_INVALID_PARAM;
    return imp->build((const ImpTome**)tomes, numTomes, aabb, resultAlloc, prevImp);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::getNumTomes(void) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (imp == NULL)
        return 0;
    int numContexts = imp->getNumTomeContexts();
    if (numContexts)
        return numContexts;
    return (imp->getResult() && !imp->ownsResult()) ? 1 : 0;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

const Tome* TomeCollection::getTome(int i) const
{
    const ImpTomeCollection* imp = IMPL(this);
    int numContexts = imp->getNumTomeContexts();
    if (numContexts)
    {
        return (const Tome*)((const ImpTome*)imp->getTomeContext(i)->m_tome);
    }
    if ((i == 0) && imp->getResult() && !imp->ownsResult())
        return (const Tome*)imp->getResult();
    return NULL;
}


/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 TomeCollection::getSize(void) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult() || !imp->ownsResult())
        return 0;
    return ((const Tome*)imp->getResult())->getSize();
}


/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::getObjectUserIDs(int objIndex, Umbra::UINT32* ids, int max) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getObjectUserIDs(objIndex, ids, max);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 TomeCollection::getObjectUserID(int objIndex) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getObjectUserID(objIndex);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::findObjectIndex(Umbra::UINT32 userId) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->findObjectIndex(userId);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::getClusterCount(void) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getClusterCount();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::getClusterPortalCount(void) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getClusterPortalCount();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::getObjectCount(void) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getObjectCount();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void TomeCollection::getObjectBounds(int objIndex, Vector3& mn, Vector3& mx) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (imp->getResult())
        ((const Tome*)imp->getResult())->getObjectBounds(objIndex, mn, mx);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::getGateCount(void) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getGateCount();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

Umbra::UINT32 TomeCollection::getGateUserID(int gateIndex) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getGateUserID(gateIndex);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

int TomeCollection::findGateIndex(Umbra::UINT32 userId) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->findGateIndex(userId);
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

size_t TomeCollection::getGateStateSize(void) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return 0;
    return ((const Tome*)imp->getResult())->getGateStateSize();
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void TomeCollection::getBounds(Vector3& mn, Vector3& mx) const
{
    const ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
        return;
    mn = imp->getResult()->getTreeMin();
    mx = imp->getResult()->getTreeMax();
}

static inline int binarySearch(const DataArray& arr, int target, int a, int b)
{
    // binary search
    int start = a;
    int end   = b;
    while (end - start > 0)
    {
        int mid   = (start + end + 1) / 2;
        int value = 0;
        arr.getElem(value, mid);

        if (target >= value)
            start = mid;
        else
            end = mid - 1;
    }

    return start;
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void TomeCollection::findClusterIndex(int globalClusterIdx, int& tomeIdx, int& localIdx)
{
    ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
    {
        tomeIdx = -1;
        localIdx = -1;
        return;
    }
    const ImpTome* result   = imp->getResult();
    DataArray clusterStarts = result->getClusterStarts();
    if (!clusterStarts.m_ofs)
    {
        tomeIdx  = 0;
        localIdx = globalClusterIdx;
        return;
    }

    tomeIdx = binarySearch(clusterStarts, globalClusterIdx, 0, result->getTomeCount() - 1);

    int clusterStart[2];
    clusterStarts.getElem(clusterStart[0], tomeIdx);
    clusterStarts.getElem(clusterStart[1], tomeIdx + 1);
    localIdx = globalClusterIdx - clusterStart[0];

    if (localIdx < 0 || localIdx >= clusterStart[1] - clusterStart[0])
    {
        localIdx = -1;
        tomeIdx = -1;
        return;
    }
}

/*-------------------------------------------------------------------*//*!
 * \brief
 *//*-------------------------------------------------------------------*/

void TomeCollection::findPortalIndex(int globalClusterPortalIdx, int& tomeIdx, int& localIdx)
{
    ImpTomeCollection* imp = IMPL(this);
    if (!imp->getResult())
   {
        tomeIdx = -1;
        localIdx = -1;
        return;
    }
    const ImpTome* result   = imp->getResult();
    DataArray portalStarts = result->getClusterPortalStarts();
    if (!portalStarts.m_ofs)
    {
        tomeIdx  = 0;
        localIdx = globalClusterPortalIdx;
        return;
    }

    tomeIdx = binarySearch(portalStarts, globalClusterPortalIdx, 0, result->getTomeCount() - 1);

    int portalStart[2];
    portalStarts.getElem(portalStart[0], tomeIdx);
    portalStarts.getElem(portalStart[1], tomeIdx + 1);
    localIdx = globalClusterPortalIdx - portalStart[0];

    if (localIdx < 0 || localIdx >= portalStart[1] - portalStart[0])
    {
        localIdx = -1;
        tomeIdx = -1;
        return;
    }
}

TomeCollection::ErrorCode TomeCollection::serialize(OutputStream& stream) const
{
    const ImpTomeCollection* imp = IMPL(this);
    return imp->serialize(stream);
}

TomeCollection::ErrorCode TomeCollection::deserialize(InputStream& stream, const Tome** tomes, int numTomes, Allocator* scratchAllocator)
{
    ImpTomeCollection* imp = IMPL(this);
    return imp->deserialize(stream, (const ImpTome**)tomes, numTomes, scratchAllocator);
}

} // namespace Umbra

#undef THIS
