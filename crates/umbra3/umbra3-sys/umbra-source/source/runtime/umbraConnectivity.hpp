#pragma once
#ifndef __UMBRACONNECTIVITY_H
#define __UMBRACONNECTIVITY_H

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
 * \brief   Umbra runtime intersection tests
 *
 */

#include "umbraQueryContext.hpp"
#include "umbraQueryArgs.hpp"
#include "umbraStaticHeap.hpp"
#include "umbraPortalTraversal.hpp"

#define UMBRA_OPENSET_LIMIT 2000
#define UMBRA_MAX_CLUSTERSTACK_SIZE 10000

namespace Umbra
{

template<class NODE>
class ConnectTraversal
{
public:

    ConnectTraversal(QueryContext* query, int numNodes): m_stack(query->getAllocator()), m_numNodes(numNodes)
    {
        UMBRA_ASSERT(numNodes < 0xFFFF);
        m_open = UMBRA_HEAP_NEW_ARRAY(m_stack, UINT32, UMBRA_BITVECTOR_DWORDS(numNodes)*2);
        if (!m_open)
        {
            query->setError(Query::ERROR_OUT_OF_MEMORY);
            return;
        }
        m_closed = m_open + UMBRA_BITVECTOR_DWORDS(numNodes);
        reset();
    }

    ~ConnectTraversal(void)
    {
        UMBRA_HEAP_DELETE_ARRAY(m_stack, m_open);
    }

    static void estimateSize(StatsAlloc* s, int numNodes)
    {
        // open + closed sets
        UMBRA_HEAP_NEW_ARRAY_NOINIT(s, UINT32, UMBRA_BITVECTOR_DWORDS(numNodes)*2);
    }

    int     getNumNodes     (void) const { return m_numNodes; }
    void    reset           (void) { memset(m_open, 0, UMBRA_BITVECTOR_SIZE(m_numNodes) * 2); }
    int     getSize         (void) const { return m_openSet.getSize(); }
    bool    isEmpty         (void) const { return m_openSet.getSize() == 0; }
    bool    isNodeClosed    (NODE n) const { return testBit(m_closed, n.getIndex()); }
    bool    updateNode      (NODE n, float dist);
    NODE    removeFirst     (float& distance);

private:
    bool    isNodeOpen      (NODE n) const { return testBit(m_open, n.getIndex()); }

    StackAlloc*     m_stack;
    UINT32*         m_open;
    UINT32*         m_closed;
    int             m_numNodes;
    StaticHeap<float, NODE, UMBRA_OPENSET_LIMIT> m_openSet;
};

struct ClusterPathNode
{
    int               index;
    float             modifier;

    ClusterPathNode(void): index(-1), modifier(0.f) {}
    ClusterPathNode(int index, float modifier): index(index), modifier(modifier) {}

    bool operator== (const ClusterPathNode& o) { return index == o.index; }

    int getIndex(void) const { return index; }
};

class RegionFinder
{
public:
    RegionFinder(QueryContext* q, int cluster, const Vector3& point, float limit, bool skipFirstDistance);
    ~RegionFinder(void);

    void execute (UserList<int>* clusters, UserList<float>* clusterPathDistances, UserList<float>* clusterPathModifier, UserList<int>* clusterEntryPortals);

private:

    ArrayMapper m_clusters;
    ArrayMapper m_extClusters;
    ArrayMapper m_clusterPortals;
    ArrayMapper m_extClusterPortals;
    QueryContext* m_query;
    ConnectTraversal<ClusterPathNode> m_traverse;
    int m_startCluster;
    Vector3 m_startPoint;
    int m_startNodeIdx;
    float m_limit;
    UINT32* m_clusterBV;
    bool    m_skipFirstDistance;
};

class PathFinder : public QueryRunner
{
public:
    PathFinder(QueryContext& q);
    ~PathFinder(void);

    void find (ImpPath& p, const Vector3& start, const Vector3& end);

    static size_t getMemoryRequirement(const ImpTome* t);

private:

    enum
    {
        INVALID_NODE = 0xFFFF
    };

    void reversePath (void);
    void outputPath (ImpPath& p, float modifier, const Vector3& start, const Vector3& end);

    ArrayMapper             m_clusters;
    ArrayMapper             m_extClusters;
    ArrayMapper             m_clusterPortals;
    ArrayMapper             m_extClusterPortals;
    ConnectTraversal<ClusterPathNode> m_traverse;
    UINT16*                 m_sources;
    UINT16                  m_startNodeIdx;
    UINT16                  m_endNodeIdx;
};

class DepthFirstRegionFinder
{
public:
    DepthFirstRegionFinder(QueryContext* q, UINT32 flags)
        :   m_query(q),
            m_flags(flags),
            m_clusters(q, sizeof(ClusterNode)),
            m_extClusters(q, sizeof(ExtClusterNode)),
            m_portals(q->getAllocator(), q->getTagManager()),
            m_visited(NULL)
    {
        m_visited = UMBRA_HEAP_NEW_ARRAY(m_query->getAllocator(), UINT32,
            UMBRA_BITVECTOR_DWORDS(m_query->getState()->getRootTome()->getNumClusters()));

        if (!m_visited ||
            !m_clusters.isInitialized() ||
            !m_extClusters.isInitialized() ||
            !m_portals.isInitialized())
        {
            m_query->setError(Query::ERROR_OUT_OF_MEMORY);
        }
    }

    ~DepthFirstRegionFinder ()
    {
        UMBRA_HEAP_DELETE_ARRAY(m_query->getAllocator(), m_visited);
    }

    bool execute (UserList<int>* clustersOut, const UINT32* clustersToFind,
        int startCluster, const Vector3& center, float radius, float* confidenceBound);

private:

    struct StackEntry
    {
        int tome;
        int cluster;
    };

    QueryContext*   m_query;
    UINT32          m_flags;
    ArrayMapper     m_clusters;
    ArrayMapper     m_extClusters;
    PortalIterator  m_portals;
    StackEntry      m_stack[UMBRA_MAX_CLUSTERSTACK_SIZE];
    UINT32*         m_visited;
};

#define UMBRA_LINESEGMENT_DEBUG				    0 // debugging features for viewer
#define UMBRA_LINESEGMENT_ALWAYS_FIND_END       0 // always find end cell immediately, faster success condition
#define UMBRA_LINESEGMENT_CALCULATE_MINMAX      0 // calculate intersection range as [0, 1] floats

class LineSegmentFinder
{
public:
	LineSegmentFinder(QueryContext* q) :
        m_query(q), 
        m_foundObjects(NULL),
        m_cellNodeMap(m_query, sizeof(CellNode)),
        m_extCellNodeMap(m_query, sizeof(ExtCellNode)),
        m_objectIter(m_query->getAllocator(), m_query->getTagManager()),
        m_cellStartMap(m_query, m_query->getTome()->getCellStarts())
    {
        int numCells = m_query->getTome()->getNumCells();
        m_visitedCells = (UINT32*)m_query->allocWorkMem(UMBRA_BITVECTOR_SIZE(numCells), true);
    }

	~LineSegmentFinder()
    {
        m_query->freeWorkMem(m_visitedCells);
    }

	void				execute		        (ImpLineSegmentQuery* queries, int count);

private:

    template<bool findObjects>
	bool				queryInternal		(const Vector3& start, const Vector3& end, Cell& startCell, Cell& endCell, UserList<int>* objects);

	inline bool			intersectRayAARect	(const Vector3& mn, const Vector3& mx, Face normal);


	struct StackItem
    {
		PortalNode	m_node;
        #if UMBRA_LINESEGMENT_CALCULATE_MINMAX
        Vector3     m_enterMin, m_enterMax;
        #endif
    };

	static const int	    g_stackCapacity = 256;	 // must be power of two
	QueryContext*		    m_query;
	StackItem               m_stack[g_stackCapacity];
	UINT32				    m_stackStart;
	UINT32				    m_stackEnd;

    static const int        g_historySize = 256;
    int                     m_history[g_historySize];
    UINT32                  m_historyStart;
    UINT32                  m_historyPos;
    float                   m_portalExpand;

	Vector3				    m_start;
	Vector3				    m_end;

    UINT32*	                m_foundObjects;

    UINT32*                 m_visitedCells;
    ArrayMapper             m_cellNodeMap;
    ArrayMapper             m_extCellNodeMap;
    RangeIterator           m_objectIter;
    ArrayMapper             m_cellStartMap;


	inline bool			    intersectRayAABB    (const SIMDRegister& mn, const SIMDRegister& mx);
    inline void			    findHitPoints       (const Vector4& mn, const Vector4& mx, SIMDRegister& min, SIMDRegister& max);

	SIMDRegister		    m_startSIMD;
	SIMDRegister		    m_endSIMD;
	SIMDRegister		    m_startSIMDLocal;
	SIMDRegister		    m_endSIMDLocal;
	SIMDRegister		    m_slotMinSIMD;
	SIMDRegister		    m_slotMaxSIMD;
    SIMDRegister		    m_localScale;
    SIMDRegister            m_portalExpandSIMD;

	SIMDRegister		    m_invdir;
    #if UMBRA_LINESEGMENT_CALCULATE_MINMAX
	SIMDRegister		    m_hitMin;
	SIMDRegister		    m_hitMax;
    #endif

};

} // namespace Umbra

#endif
