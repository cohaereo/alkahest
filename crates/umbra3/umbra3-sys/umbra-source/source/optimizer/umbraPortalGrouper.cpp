#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraPortalGrouper.hpp"
#include "umbraLogger.hpp"
#include "umbraWeightedSampler.hpp"
#include "umbraSet.hpp"
#include "umbraTomePrivate.hpp"
#include "umbraUnionFind.hpp"
#include "umbraIntersectExact.hpp"
#include "umbraPair.hpp"
#include "umbraRandom.hpp"
#include "umbraRectGrouper.hpp"
#include <standard/Sampling.hpp>
#include <standard/MigrateFromCommon.hpp>
#include <cmath>

#define LOGE(...) UMBRA_LOG_E(m_platform.logger, __VA_ARGS__)
#define LOGI(...) UMBRA_LOG_I(m_platform.logger, __VA_ARGS__)
#define LOGW(...) UMBRA_LOG_W(m_platform.logger, __VA_ARGS__)
#define LOGD(...) UMBRA_LOG_D(m_platform.logger, __VA_ARGS__)

using namespace Umbra;

namespace Umbra
{

/* \todo [antti 30.11.2012]: improve this, move into umbraintersect.hpp */
UMBRA_FORCE_INLINE static bool testLineSegment(const AxialQuad& aq, const Vector3& a, const Vector3& b)
{
    int axis = aq.face >> 1;
    int axisX = (axis+1) % 3;
    int axisY = (axis+2) % 3;

    if (a[axis] == aq.z || b[axis] == aq.z) // Segments lying on the quad don't intersect.
        return false;

    int cm0 = 0;
    cm0 |= a[axisX] < aq.rect.x ? 1 : a[axisX] > aq.rect.z ? 4 : 0;
    cm0 |= a[axisY] < aq.rect.y ? 2 : a[axisY] > aq.rect.w ? 8 : 0;

    int cm1 = 0;
    cm1 |= b[axisX] < aq.rect.x ? 1 : b[axisX] > aq.rect.z ? 4 : 0;
    cm1 |= b[axisY] < aq.rect.y ? 2 : b[axisY] > aq.rect.w ? 8 : 0;

    if (cm0 == 0 && cm1 == 0)
        return true;

    if ((cm0 & cm1) != 0)
        return false;

    Vector3 o = a;
    Vector3 d = b - a;

    if (d[axis] == 0.f)
        return false;

    float tmin = (aq.z - o[axis]) / d[axis];
    float tmax = (aq.z - o[axis]) / d[axis];

    if (d[axisX] != 0.f)
    {
        float t0 = (aq.rect.x - o[axisX]) / d[axisX];
        float t1 = (aq.rect.z - o[axisX]) / d[axisX];
        if (t1 < t0)
        { float tmp = t0; t0 = t1; t1 = tmp; }
        if (t0 > tmin)
            tmin = t0;
        if (t1 < tmax)
            tmax = t1;
    }

    if (d[axisY] != 0.f)
    {
        float t0 = (aq.rect.y - o[axisY]) / d[axisY];
        float t1 = (aq.rect.w - o[axisY]) / d[axisY];
        if (t1 < t0)
        { float tmp = t0; t0 = t1; t1 = tmp; }
        if (t0 > tmin)
            tmin = t0;
        if (t1 < tmax)
            tmax = t1;
    }

    return tmin <= tmax;
}


class ContractableGraph
{
public:

    struct EndPoints
    {
        bool operator== (const EndPoints& e)
        {
            return a == e.a && b == e.b;
        }

        int a;
        int b;
    };

    struct Node
    {
        void setAllocator (Allocator* heap)
        {
            origCells.setAllocator(heap);
            links.setAllocator(heap);
        }

        Set<int> origCells;
        Set<int> links;
    };

    struct Link
    {
        void setAllocator (Allocator* heap)
        {
            quads.setAllocator(heap);
        }

        int getTarget (int src) const
        {
            if (p.a == src)
                return p.b;
            UMBRA_ASSERT(p.b == src);
            return p.a;
        }

        bool active;
        EndPoints p;
        Array<AxialQuad> quads;

        float weight;
    };

    ContractableGraph (Allocator* a = NULL)
    {
        setAllocator(a);
        clear();
    }

    void setAllocator (Allocator* a)
    {
        m_nodes.setAllocator(a);
        m_links.setAllocator(a);
        m_nodeHash.setAllocator(a);
        m_linkHash.setAllocator(a);
    }

    void clear (void)
    {
        m_nodes.clear();
        m_links.clear();
        m_nodeHash.clear();
        m_linkHash.clear();
        m_totalWeight = 0.f;
        m_activeNodes = 0;
        m_activeLinks = 0;
    }

    ContractableGraph& operator= (const ContractableGraph& o)
    {
        clear();
        m_random = o.m_random;

        // compact arrays on copy

        Array<int> nodeRemap(o.m_nodes.getSize(), m_nodes.getAllocator());
        Array<int> linkRemap(o.m_links.getSize(), m_nodes.getAllocator());
        int nodes = 0;
        for (int i = 0; i < o.m_nodes.getSize(); i++)
        {
            if (!o.m_nodes[i].origCells.getSize())
                nodeRemap[i] = -1;
            else
                nodeRemap[i] = nodes++;
        }
        m_nodes.reset(nodes);
        int links = 0;
        for (int i = 0; i < o.m_links.getSize(); i++)
        {
            if (!o.m_links[i].active)
                linkRemap[i] = -1;
            else
                linkRemap[i] = links++;
        }
        m_links.reset(links);

        for (int i = 0; i < o.m_nodes.getSize(); i++)
        {
            int idx = nodeRemap[i];
            if (idx == -1)
                continue;
            const Node& src = o.m_nodes[i];
            Node& dst = m_nodes[idx];
            dst.origCells = src.origCells;
            dst.links.removeAll(false);
            Set<int>::Iterator linkIter = o.m_nodes[i].links.iterate();
            while (linkIter.next())
                dst.links.insert(linkRemap[linkIter.getValue()]);
        }
        m_nodeHash = o.m_nodeHash;
        Hash<int, int>::Iterator nodeIter = m_nodeHash.iterate();
        while (m_nodeHash.isValid(nodeIter))
        {
            m_nodeHash.getValue(nodeIter) = nodeRemap[m_nodeHash.getValue(nodeIter)];
            m_nodeHash.next(nodeIter);
        }

        for (int i = 0; i < o.m_links.getSize(); i++)
        {
            int idx = linkRemap[i];
            if (idx == -1)
                continue;
            const Link& src = o.m_links[i];
            Link& dst = m_links[idx];
            dst.p.a = nodeRemap[src.p.a];
            dst.p.b = nodeRemap[src.p.b];
            dst.quads = src.quads;
            dst.weight = src.weight;
            dst.active = true;
            m_linkHash.insert(dst.p, idx);
        }

        m_totalWeight = o.m_totalWeight;
        m_activeNodes = o.m_activeNodes;
        m_activeLinks = o.m_activeLinks;

        UMBRA_ASSERT(sanityCheck());

        return *this;
    }

    void init (Random* r, const CellGraph& g, const BitVector& cells)
    {
        m_random = r;
        clear();

        // import graph
        for (int i = 0; i < g.getCellCount(); i++)
        {
            if (!cells.test(i))
                continue;
            int idx = getOrCreateNode(i);

            const CellGraph::Cell& c = g.getCell(i);
            for (int j = 0; j < c.getPortalCount(); j++)
            {
                const CellGraph::Portal& p = c.getPortal(j);
                int origT = p.getTarget();
                if (!cells.test(origT))
                    continue;
                int t = getOrCreateNode(origT);

                // update link
                EndPoints ep;
                ep.a = idx;
                ep.b = t;
                if (idx > t)
                    swap(ep.a, ep.b);
                int linkIdx = getOrCreateLink(ep);
                Link& link = m_links[linkIdx];
                if (!p.isGate())
                {
                    link.quads.pushBack(AxialQuad(p.getRectPortal().getFace(), p.getRectPortal().getZ(), p.getRectPortal().getRect()));
                    float w = rectArea(p.getRectPortal().getRect());
                    UMBRA_ASSERT(w > 0.f);
                    link.weight += w;
                    m_totalWeight += w;
                }
                /* \todo [antti 18.12.2012]: weight for gate portals! */
                m_nodes[idx].links.insert(linkIdx);
                m_nodes[t].links.insert(linkIdx);
            }
        }

        m_activeNodes = m_nodes.getSize();
        m_activeLinks = m_links.getSize();

        UMBRA_ASSERT(sanityCheck());
    }

    int getNumNodes (void) const
    {
        return m_activeNodes;
    }

    int getNumLinks (void) const
    {
        return m_activeLinks;
    }

    void getActiveNodes (Array<const Node*>& out) const
    {
        for (int i = 0; i < m_nodes.getSize(); i++)
        {
            if (m_nodes[i].origCells.getSize())
                out.pushBack(&m_nodes[i]);
        }
    }

    const Node* getNode (int idx) const
    {
        return &m_nodes[idx];
    }

    const Link* getLink (int idx) const
    {
        return &m_links[idx];
    }

    int mapNode (int origIdx) const
    {
        const int* idx = m_nodeHash.get(origIdx);
        UMBRA_ASSERT(idx);
        return *idx;
    }

    void randomContractToSize (int n)
    {
        while (getNumNodes() > n)
            randomContractOne();
    }

#if 0
    void contractLink (int activeIdx)
    {
        int idx = 0;
        for (int i = 0; i < m_links.getSize(); i++)
        {
            const Link& l = m_links[i];
            if (!l.active)
                continue;
            if (idx++ >= activeIdx)
            {
                contract(i);
                return;
            }
        }
    }
#endif

private:


    void moveLink (int i, int src, int dst)
    {
        UMBRA_ASSERT(!m_nodes[dst].links.contains(i));
        Link& l = m_links[i];
        UMBRA_ASSERT(l.active);
        int otherIdx;
        if (l.p.a == src)
        {
            otherIdx = l.p.b;
            l.p.a = dst;
        }
        else
        {
            UMBRA_ASSERT(l.p.b == src);
            otherIdx = l.p.a;
            l.p.b = dst;
        }
        UMBRA_ASSERT(l.p.a != l.p.b);
        if (l.p.a > l.p.b)
            swap(l.p.a, l.p.b);
        int* cidx = m_linkHash.get(l.p);
        if (cidx)
        {
            // link already exists, merge
            Link& newLink = m_links[*cidx];
            UMBRA_ASSERT(newLink.active);
            newLink.weight += l.weight;
            newLink.quads.append(l.quads);
            l.active = false;
            l.quads.clear();
            m_activeLinks--;
            UMBRA_ASSERT(m_nodes[otherIdx].links.contains(*cidx));
            UMBRA_ASSERT(m_nodes[dst].links.contains(*cidx));
            m_nodes[otherIdx].links.remove(i);
        }
        else
        {
            // retarget link
            m_linkHash.insert(l.p, i);
            UMBRA_ASSERT(m_nodes[otherIdx].links.contains(i));
            m_nodes[dst].links.insert(i);
        }
    }

    bool sanityCheck (void) const
    {
        if (getNumNodes() < 1)
        {
            UMBRA_ASSERT(!"bad number of nodes");
            return false;
        }

        UnionFind<int> uf(m_nodes.getAllocator());
        int n = 0;
        for (int i = 0; i < m_nodes.getSize(); i++)
        {
            if (m_nodes[i].origCells.getSize())
            {
                n++;
                Set<int>::Iterator iter = m_nodes[i].links.iterate();
                while (iter.next())
                {
                    int link = iter.getValue();
                    if (!m_links[link].active)
                    {
                        UMBRA_ASSERT(!"inactive link");
                        return false;
                    }
                    int t = m_links[link].getTarget(i);
                    if (m_nodes[t].origCells.getSize() == 0)
                    {
                        UMBRA_ASSERT(!"invalid node");
                        return false;
                    }
                    uf.unionSets(i, t);
                }
            }
        }
        if (n != m_activeNodes)
        {
            UMBRA_ASSERT(!"invalid number of nodes");
            return false;
        }
        Set<int> unions(m_nodes.getAllocator());
        for (int i = 0; i < m_nodes.getSize(); i++)
        {
            if (m_nodes[i].origCells.getSize())
                unions.insert(uf.findSet(i));
        }
        if (unions.getSize() != 1)
        {
            UMBRA_ASSERT(!"disconnected cells");
            return false;
        }
        return true;
    }

    bool contract (int link)
    {
        Link& l = m_links[link];

        int keepIdx = l.p.a;
        int removeIdx = l.p.b;

        if (keepIdx > removeIdx)
            swap(keepIdx, removeIdx);

        Node& keep = m_nodes[keepIdx];
        Node& remove = m_nodes[removeIdx];

        // move content from remove to keep, update hash
        keep.origCells |= remove.origCells;
        Set<int>::Iterator cells = remove.origCells.iterate();
        while (cells.next())
            *m_nodeHash.get(cells.getValue()) = keepIdx;
        remove.origCells.removeAll(false);

        // update links
        keep.links.remove(link);
        Set<int>::Iterator links = remove.links.iterate();
        while (links.next())
        {
            int i = links.getValue();
            if (i == link)
                continue;
            moveLink(i, removeIdx, keepIdx);
        }
        m_activeNodes--;
        m_totalWeight -= l.weight;
        l.active = false;
        m_activeLinks--;

        return true;
    }

    void randomContractOne (void)
    {
        float c = m_random->get() * m_totalWeight;
        float cum = 0;
        int nodes = getNumNodes();

        UMBRA_ASSERT(nodes >= 2);

        for (int i = 0; i < m_links.getSize(); i++)
        {
            const Link& l = m_links[i];
            if (!l.active)
                continue;
            cum += l.weight;
            if (cum >= c)
            {
                contract(i);
                break;
            }
        }

        UMBRA_ASSERT(getNumNodes() == nodes - 1);
        UMBRA_UNREF(nodes);
    }

    int getOrCreateNode (int origIdx)
    {
        int* idx = m_nodeHash.get(origIdx);
        if (idx)
            return *idx;

        int last = m_nodes.getSize();
        m_nodes.resize(last + 1);
        Node& n = m_nodes[last];
        n.links.removeAll(false);
        n.origCells.removeAll(false);
        n.origCells.insert(origIdx);
        m_nodeHash.insert(origIdx, last);
        return last;
    }

    int getOrCreateLink (const EndPoints& ep)
    {
        int* idx = m_linkHash.get(ep);
        if (idx)
            return *idx;

        int last = m_links.getSize();
        m_links.resize(last + 1);
        Link& l = m_links[last];
        l.p = ep;
        l.weight = 0.f;
        l.active = true;
        m_linkHash.insert(ep, last);
        return last;
    }

    Random* m_random;
    Array<Node> m_nodes;
    Array<Link> m_links;
    Hash<int, int> m_nodeHash;
    Hash<EndPoints, int> m_linkHash;
    float m_totalWeight;
    int m_activeNodes;
    int m_activeLinks;
};

static inline void copyHeap (ContractableGraph::Node* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

static inline void copyHeap (ContractableGraph::Link* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

static inline void copyHeap (ContractableGraph* elem, Allocator* heap)
{
    elem->setAllocator(heap);
}

template <> inline unsigned int getHashValue (const ContractableGraph::EndPoints& e)
{
    return getHashValue(e.a) + getHashValue(e.b);
}

class GraphCutFinder
{
public:

    GraphCutFinder(const CellGraph& g, int numSamples = DEFAULT_OCCLUSION_SAMPLES)
        : m_input(g), m_samples(numSamples, g.getAllocator()),
          m_graphStack(g.getAllocator()), m_sampleStack(g.getAllocator()),
          m_cellVisited(g.getCellCount(), g.getAllocator()),
          m_cellObjectsTagged(g.getCellCount(), g.getAllocator()),
          m_targetVisible(g.getTargetObjectCount(), g.getAllocator())
    {
    }

    int find (Array<int>& output, int outOffset, float featureSize,
        const BitVector& cells, const Array<SourceQuad>& sources)
    {
        m_random.reset(0);

        initSamples(sources);

        m_potentialTargetObjs = 0;
        m_targetVisible.clearAll();
        Array<int> objArray(m_input.getAllocator());
        for (int i = 0; i < m_input.getCellCount(); i++)
        {
            if (!cells.test(i))
                continue;
            const CellGraph::Cell& c = m_input.getCell(i);
            c.getObjects(objArray);
            for (int j = 0; j < objArray.getSize(); j++)
            {
                int idx = objArray[j];
                if (!m_targetVisible.test(idx))
                {
                    m_targetVisible.set(idx);
                    m_potentialTargetObjs++;
                }
            }
        }

        if (m_potentialTargetObjs < GRAPHCUT_MIN_TARGETS_THRESHOLD)
            m_potentialTargetObjs = 0;

        m_graphStack.reset(1);
        m_graphStack[0].graph.init(&m_random, m_input, cells);
        m_graphStack[0].tries = highestBitSet(m_graphStack[0].graph.getNumNodes());

        Fitness baseline = estimateFitness(m_graphStack[0].graph);
        if (baseline.externalOcclusion > 0.f)
            baseline.externalOcclusion *= min2(featureSize * featureSize / (baseline.externalOcclusion * m_sourceArea), 1.f);
        if (baseline.targetsOccluded > 0.f)
            baseline.targetsOccluded *= GRAPHCUT_OBJECT_OCCLUSION_FACTOR;

        m_bestCutNodes = updateCut(output, outOffset, m_graphStack[0].graph);

        while (m_graphStack.getSize() && (m_bestCutNodes > 1))
        {
            int head = m_graphStack.getSize();

            {
                GraphStackItem& cur = m_graphStack[head - 1];
                if (cur.tries-- == 0)
                {
                    m_graphStack.resize(head - 1);
                    continue;
                }
            }

            m_graphStack.resize(head + 1);
            GraphStackItem& item = m_graphStack[head];
            item.tries = 0;
            item.graph = m_graphStack[head-1].graph;
            int targetNodes = (int)(item.graph.getNumNodes() / sqrt(2.f));
            item.graph.randomContractToSize(targetNodes);

            // is quality acceptable?
            Fitness f = estimateFitness(item.graph);
            if (f.externalOcclusion < baseline.externalOcclusion ||
                f.targetsOccluded < baseline.targetsOccluded)
                continue;

            if (targetNodes < m_bestCutNodes)
                m_bestCutNodes = updateCut(output, outOffset, item.graph);
            if (targetNodes > 1)
                item.tries = highestBitSet(targetNodes);
        }

        return m_bestCutNodes;
    }

    struct Fitness
    {
        Fitness(void): targetsOccluded(-1.f), externalOcclusion(-1.f) {}
        Fitness(float t, float e): targetsOccluded(t), externalOcclusion(e) {}

        float combined (void) const
        {
            if (targetsOccluded < 0.f)
                return externalOcclusion;
            if (externalOcclusion < 0.f)
                return targetsOccluded;
            return targetsOccluded * externalOcclusion;
        }

        bool operator== (const Fitness& f) const
        {
            return f.targetsOccluded == targetsOccluded &&
                f.externalOcclusion == externalOcclusion;
        }

        bool operator> (const Fitness& f) const
        {
            return combined() > f.combined();
        }

        float               targetsOccluded;
        float               externalOcclusion;
    };

    struct GraphStackItem
    {
        ContractableGraph   graph;
        int                 tries;
    };

private:

    struct SampleRay
    {
        int     startCell;
        int     endCell;
        Vector3 a;
        Vector3 b;
    };

    void initSamples (const Array<SourceQuad>& src)
    {
        WeightedSampler ws(src.getAllocator());
        ws.resize(src.getSize());

        m_sourceArea = 0.f;
        for (int i = 0; i < src.getSize(); i++)
        {
            float a = src[i].aq.getArea();
            ws.setWeight(i, a);
            m_sourceArea += a;
        }

        int rndSeed = 1;
        int dirRnd1 = 1;
        int dirRnd2 = 1;

        m_numExtRays = 0;

        int i = 0;
        while (i < m_samples.getSize())
        {
            SampleRay& ray = m_samples[i];
            int rnd = rndSeed++;
            const SourceQuad& sq = src[ws.pickSample(haltonf<11>(rnd))];
            ray.startCell = sq.src;
            ray.a = sq.aq.getPoint(haltonf<5>(rnd), haltonf<7>(rnd));
            ray.endCell = -1;

            /* \todo [antti 10.12.2012]: fix this! */
            UMBRA_ASSERT(!"that sampler is probably broken");
            Vector3 dir;
            do
            {
                dir = migrate(uniformPointOnSphere(migrate(Vector2(haltonf<2>(dirRnd1++), haltonf<3>(dirRnd2++)))));
            } while (!sq.aq.sameDir(dir));
            ray.b = ray.a + dir * m_input.getAABB().getDiagonalLength();

            // find end cell (if any)
            for (int j = 0; j < src.getSize(); j++)
            {
                if (::testLineSegment(src[j].aq, ray.a, ray.b))
                {
                    UMBRA_ASSERT(src[j].aq != sq.aq);
                    ray.endCell = src[j].src;
                    m_numExtRays++;
                    break;
                }
            }

            // try a bit harder to find ext rays
            if ((ray.endCell == -1) && (m_numExtRays < m_samples.getSize() * 0.2f) && (rndSeed < m_samples.getSize()*20))
                continue;
            i++;
        }
    }

    Fitness estimateFitness (const ContractableGraph& g)
    {
        Array<int> objArray(m_input.getAllocator());
        int numTargetsVisible = 0;
        int extRaysHit = 0;
        m_targetVisible.clearAll();
        m_cellObjectsTagged.clearAll();

        for (int i = 0; i < m_samples.getSize(); i++)
        {
            const SampleRay& ray = m_samples[i];
            m_cellVisited.clearAll();
            int start = g.mapNode(ray.startCell);
            int end = (ray.endCell == -1) ? -1 : g.mapNode(ray.endCell);
            m_sampleStack.pushBack(start);
            bool hit = true;

            while (m_sampleStack.getSize())
            {
                int cell = m_sampleStack.popBack();
                if (m_cellVisited.test(cell))
                    continue;
                m_cellVisited.set(cell);
                const ContractableGraph::Node* n = g.getNode(cell);

                // tag visible objects
                if (!m_cellObjectsTagged.test(cell))
                {
                    Set<int>::Iterator iter = n->origCells.iterate();
                    while (iter.next())
                    {
                        const CellGraph::Cell& orig = m_input.getCell(iter.getValue());
                        orig.getObjects(objArray);
                        for (int j = 0; j < objArray.getSize(); j++)
                        {
                            if (!m_targetVisible.test(objArray[j]))
                            {
                                m_targetVisible.set(objArray[j]);
                                numTargetsVisible++;
                            }
                        }
                    }
                    m_cellObjectsTagged.set(cell);
                }

                // terminate
                if (cell == end)
                {
                    hit = false;
                    m_sampleStack.clear();
                    break;
                }

                // traverse
                Set<int>::Iterator links = n->links.iterate();
                while (links.next())
                {
                    const ContractableGraph::Link* l = g.getLink(links.getValue());
                    int target = l->getTarget(cell);
                    if (m_cellVisited.test(target))
                        continue;
                    for (int j = 0; j < l->quads.getSize(); j++)
                    {
                        const AxialQuad& q = l->quads[j];
                        if (!q.isFront(ray.b) || !q.isBack(ray.a))
                            continue;
                        if (::testLineSegment(l->quads[j], ray.a, ray.b))
                        {
                            m_sampleStack.pushBack(target);
                            break;
                        }
                    }
                }
            }

            if (hit && (ray.endCell != -1))
                extRaysHit++;
        }

        Fitness f;
        f.targetsOccluded = (m_potentialTargetObjs == 0) ? -1.f :
            (float)(m_potentialTargetObjs - numTargetsVisible) / m_potentialTargetObjs;
        f.externalOcclusion = (m_numExtRays == 0) ? -1.f : (float)extRaysHit / m_numExtRays;
        return f;
    }

    static int updateCut (Array<int>& out, int ofs, const ContractableGraph& g)
    {
        Array<const ContractableGraph::Node*> n(out.getAllocator());
        g.getActiveNodes(n);
        for (int i = 0; i < n.getSize(); i++)
        {
            Set<int>::Iterator iter = n[i]->origCells.iterate();
            while (iter.next())
            {
                out[iter.getValue()] = ofs + i;
            }
        }
        return g.getNumNodes();
    }

    const CellGraph&           m_input;
    Random                     m_random;
    int                        m_potentialTargetObjs;
    int                        m_numExtRays;
    float                      m_sourceArea;
    Array<SampleRay>           m_samples;
    Array<GraphStackItem>      m_graphStack;
    Array<int>                 m_sampleStack;
    BitVector                  m_cellVisited;
    BitVector                  m_cellObjectsTagged;
    BitVector                  m_targetVisible;
    int                        m_bestCutNodes;

    GraphCutFinder& operator=(const GraphCutFinder&) { return *this; } // deny
};

static inline void copyHeap (GraphCutFinder::GraphStackItem* elem, Allocator* heap)
{
    elem->graph.setAllocator(heap);
}

}

PortalGrouper::PortalGrouper(const PlatformServices& platform, CellGraph& out, const CellGraph& cg, const PortalGrouperParams& p)
:   m_platform(platform),
    m_input(cg),
    m_params(p),
    m_cellGraph(out),
    m_extPortals(platform.allocator),
    m_cellToGroup(platform.allocator),
    m_volumes(platform.allocator),
    m_importances(platform.allocator),
    m_stack(platform.allocator),
    m_used(cg.getCellCount(), platform.allocator),
    m_cells1(cg.getCellCount(), platform.allocator),
    m_cells2(cg.getCellCount(), platform.allocator),
    m_cellStack(platform.allocator),
    m_cellVisited(cg.getCellCount(), platform.allocator),
    m_occludedRays(platform.allocator),
    m_groupStack(platform.allocator)
{
    UMBRA_ASSERT(&m_cellGraph != &m_input);
}

PortalGrouper::~PortalGrouper()
{
}

void PortalGrouper::perform(ExternalCellGraph* ext)
{
    LOGD("feature size %f => occlusion area limit %f", m_params.featureSize, m_params.featureSize*m_params.featureSize);

    m_cellGraph = CellGraph(m_cellGraph.getAllocator());
    m_cellGraph.setAABB(m_input.getAABB());
    m_cellGraph.setPortalExpand(m_input.getPortalExpand());

    // Collect external portals.

    if (!ext)
    {
        m_extPortals.resize(m_input.getCellCount());
        for (int face = 0; face < 6; face++)
            if (!m_input.getMatchingTree(face).isEmpty())
            {
                SubdivisionTree st(getAllocator());
                m_input.getMatchingTree(face).deserialize(st);

                collectExternalPortals(st.getRoot(), st.getAABB(), face);
            }
    }

    // Group cells.

    m_cellToGroup.resize(m_input.getCellCount());
    for (int i = 0; i < m_input.getCellCount(); i++)
        m_cellToGroup[i] = -1;
    m_cellGroups = 0;

    if (m_params.strategy == PortalGrouperParams::BOX)
    {
        m_used.clearAll();

        for (int i = 0; i < m_input.getCellCount(); i++)
        {
            if (m_used.test(i))
                continue;
            groupCells(i);
        }

        LOGD("box grouped cells from %d to %d", m_input.getCellCount(), m_cellGroups);
    }
    else if (m_params.strategy == PortalGrouperParams::COLLAPSE)
    {
        m_used.clearAll();

        for (int i = 0; i < m_input.getCellCount(); i++)
        {
            if (m_used.test(i))
                continue;

            const CellGraph::Cell& cell = m_input.getCell(i);

            m_cellToGroup[i] = m_cellGroups;

            for (int j = 0; j < cell.getRectPortalCount(); j++)
            {
                int target = cell.getRectPortal(j).getTarget();
                if (m_used.test(target))
                    continue;

                if (m_params.planeAxis >= 0 && (cell.getRectPortal(j).getAxis() != m_params.planeAxis || cell.getRectPortal(j).getZ() != m_params.planeZ))
                    continue;

                float est = estimatePortals(i, target);

                if (est < m_params.featureSize*m_params.featureSize)
                {
                    m_cellToGroup[target] = m_cellGroups;
                    m_used.set(target);
                    break;
                }
            }

            m_cellGroups++;
            m_used.set(i);
        }

        LOGD("collapsed cells from %d to %d", m_input.getCellCount(), m_cellGroups);
    }
    else if (m_params.strategy == PortalGrouperParams::CLUSTER)
    {
        int numClusters = 0;
        for (int i = 0; i < m_input.getCellCount(); i++)
        {
            UMBRA_ASSERT(m_input.getCell(i).getClusters().getSize() < 2);
            if (m_input.getCell(i).getClusters().getSize() == 1)
                numClusters = max2(numClusters, m_input.getCell(i).getClusters()[0] + 1);
        }

        m_cellGroups = numClusters;
        for (int i = 0; i < m_input.getCellCount(); i++)
        {
            if (m_input.getCell(i).getClusters().getSize() == 1)
                m_cellToGroup[i] = m_input.getCell(i).getClusters()[0];
            else
                m_cellToGroup[i] = -1;
        }

        LOGD("clustered cells from %d to %d", m_input.getCellCount(), m_cellGroups);
    }
    else if (m_params.strategy == PortalGrouperParams::AGGRESSIVE)
    {
        // first build cell clusters
        Array<int> clusters(getAllocator());
        int numClusters = cluster(clusters);

        for (int i = 0; i < numClusters; i++)
        {
            int numExtConnected = 0;
            for (int j = 0; j < m_input.getCellCount(); j++)
            {
                if (clusters[j] != i)
                    continue;
                if (ext->getCell(j).getPortalCount())
                    numExtConnected++;
            }
            int group = !numExtConnected ? -1 : m_cellGroups++;
            for (int j = 0; j < m_input.getCellCount(); j++)
            {
                if (clusters[j] != i)
                    continue;
                m_cellToGroup[j] = group;
            }
        }

        LOGD("collapsed cells from %d to %d", m_input.getCellCount(), m_cellGroups);
    }
    else if (m_params.strategy == PortalGrouperParams::OCCLUSION_SPLITTER)
    {
        bool cheap = false;

        cheap = !(m_params.viewVolume.isOK() && m_input.getAABB().intersectsWithVolume(m_params.viewVolume));

        computeOccludedRays(cheap);

        //printf("Total occlusion %d (rays), threshold %d, cells %d\n", m_occludedRays.getSize(), m_occludedRaysThreshold, m_input.getCellCount());

        Group* g = UMBRA_NEW(Group, getAllocator());

        for (int i = 0; i < m_input.getCellCount(); i++)
            if (m_cells2.test(i))
                g->cells.pushBack(i);

        Array<Group*> out(getAllocator());

        occlusionSplitter(out, g);

        int numVisibleRays = shootRays(out, false, 0);
        int numOccludedRays = m_occludedRays.getSize() - numVisibleRays;

        m_occlusionValue = numOccludedRays * m_occlusionPerRay;

        for (int i = 0; i < out.getSize(); i++)
            for (int j = 0; j < out[i]->cells.getSize(); j++)
                m_cellToGroup[out[i]->cells[j]] = i;

        m_cellGroups = out.getSize();

        UMBRA_DELETE(g);

        for (int i = 0; i < out.getSize(); i++)
        {
            Array<GroupPortal*> portals(getAllocator());
            portals = out[i]->portals;
            disconnectGroup(out[i]);
            for (int j = 0; j < portals.getSize(); j++)
                UMBRA_DELETE(portals[j]);
        }

        for (int i = 0; i < out.getSize(); i++)
            UMBRA_DELETE(out[i]);
    }
    else if (m_params.strategy == PortalGrouperParams::COLLAPSE_EXTERNALS)
    {
        m_cellToGroup.resize(m_input.getCellCount());
        for (int i = 0; i < m_input.getCellCount(); i++)
            m_cellToGroup[i] = i;

        m_used.clearAll();

        for (int i = 0; i < m_input.getCellCount(); i++)
        {
            if (m_used.test(i))
                continue;

            m_cellToGroup[i] = m_cellGroups;
            m_used.set(i);

            if (m_input.getCell(i).getPortalCount() > 0)
            {
                m_cellGroups++;
                continue;
            }

            AABB aabb;
            int faceMask = 0;
            for (int j = 0; j < m_extPortals[i].getSize(); j++)
            {
                aabb.grow(m_extPortals[i][j].getAABB());
                faceMask |= 1 << m_extPortals[i][j].getFace();
            }

            for (int j = 0; j < m_input.getCellCount(); j++)
            {
                if (m_used.test(j) || m_input.getCell(j).getPortalCount() > 0)
                    continue;

                // Same cluster ids. Note that at this point TomeGenerator has calculated neighoring cluster ids for each cell.

#if 1
                {
                    int numClusters = m_input.getCell(i).getClusters().getSize();

                    if (numClusters != m_input.getCell(j).getClusters().getSize())
                        continue;

                    int k;
                    for (k = 0; k < numClusters; k++)
                        if (m_input.getCell(i).getClusters()[k] != m_input.getCell(j).getClusters()[k])
                            break;

                    if (k != numClusters)
                        continue;
                }
#endif

                // Same objects.

                if (m_input.getCell(i).getObjectCount() != m_input.getCell(j).getObjectCount())
                    continue;

                if (m_input.getCell(i).getObjectCount() > 0)
                {
                    Array<int> objs(getAllocator()), objs2(getAllocator());

                    m_input.getCell(i).getObjects(objs);
                    m_input.getCell(j).getObjects(objs2);

                    quickSort(objs.getPtr(), objs.getSize());
                    quickSort(objs2.getPtr(), objs2.getSize());

                    int k;
                    for (k = 0; k < objs.getSize(); k++)
                        if (objs[k] != objs2[k])
                            break;
                    if (k < objs.getSize())
                        continue;
                }

                // Same face mask.

                AABB aabb2;
                int faceMask2 = 0;
                for (int k = 0; k < m_extPortals[j].getSize(); k++)
                {
                    aabb2.grow(m_extPortals[j][k].getAABB());
                    faceMask2 |= 1 << m_extPortals[j][k].getFace();
                }

                if (faceMask != faceMask2)
                    continue;

                AABB aabb3 = aabb;
                aabb3.grow(aabb2);

                // Collapse cells if external portal surface area doesn't grow too much.
                // \todo [Hannu] fix snowball effect of aabb

                float area = aabb3.getSurfaceArea();

                area -= aabb.getSurfaceArea();
                area -= aabb2.getSurfaceArea();

                // intersected area must be added back
                aabb2.clamp(aabb);
                if (aabb2.isOK())
                    area += aabb2.getSurfaceArea();

                area /= 2.f;

                if (area < m_params.featureSize*m_params.featureSize)
                {
                    aabb = aabb3;
                    faceMask |= faceMask2;
                    m_cellToGroup[j] = m_cellGroups;
                    m_used.set(j);
                    continue;
                }
            }

            m_cellGroups++;
        }
    }
    else if (m_params.strategy == PortalGrouperParams::NONE)
    {
        m_cellToGroup.resize(m_input.getCellCount());
        for (int i = 0; i < m_input.getCellCount(); i++)
            m_cellToGroup[i] = i;

        m_cellGroups = m_input.getCellCount();
    }
    else
    {
        UMBRA_ASSERT(!"Unsupported strategy");
        return;
    }

    // Collect cells.

    if (m_cellGroups)
        m_cellGraph.addCell(m_cellGroups);
    Array<Set<int> > clustersPerCell(m_cellGroups, m_input.getAllocator());

    for (int i = 0; i < m_input.getCellCount(); i++)
    {
        int dstIdx = m_cellToGroup[i];
        if (dstIdx < 0)
            continue;

        const CellGraph::Cell& src = m_input.getCell(i);
        CellGraph::Cell& dst = m_cellGraph.getCell(dstIdx);

        const Array<int>& srcClusters = src.getClusters();
        for (int j = 0; j < srcClusters.getSize(); j++)
        {
            int cluster = srcClusters[j];
            if (!clustersPerCell[dstIdx].contains(cluster))
            {
                clustersPerCell[dstIdx].insert(cluster);
                dst.addClusterId(cluster);
            }
        }

        AABB aabb = dst.getAABB();
        aabb.grow(src.getAABB());
        dst.setAABB(aabb);

        Array<int> srcObjs(m_platform.allocator);
        Array<AABB> srcObjBounds(m_platform.allocator);

        src.getObjects(srcObjs);
        src.getObjectBounds(srcObjBounds);

        for (int j = 0; j < srcObjs.getSize(); j++)
            dst.addObject(srcObjs[j], srcObjBounds[j]);

        if (!src.isOutside())
            dst.setOutside(false);

        if (src.isReachable())
            dst.setReachable(true);

        if (src.isForceReachable())
            dst.setForceReachable(true);
    }

    // Group portals.

    groupPortals();

    // Remap view tree and matching trees

    if (!m_input.getViewTree().isEmpty())
    {
        SubdivisionTree st(getAllocator());
        m_input.getViewTree().deserialize(st);
        SubdivisionTreeUtils::remapLeafIndices(st, m_cellToGroup);
        m_cellGraph.getViewTree().serialize(st);
    }

    for (int i = 0; i < 6; i++)
        if (!m_input.getMatchingTree(i).isEmpty())
        {
            SubdivisionTree st(getAllocator());
            m_input.getMatchingTree(i).deserialize(st);
            SubdivisionTreeUtils::remapLeafIndices(st, m_cellToGroup);
            m_cellGraph.getMatchingTree(i).serialize(st);
        }

    LOGD("cells %d", m_cellGraph.getCellCount());

    // copy target objects

    for (int i = 0; i < m_input.getTargetObjectCount(); i++)
        m_cellGraph.addTargetObject(m_input.getTargetObject(i));

    // simplify portal triangles

    m_cellGraph.simplifyPortalHulls();

    m_cellGraph.checkConsistency(
        (m_params.strategy == PortalGrouperParams::BOX ||
         m_params.strategy == PortalGrouperParams::COLLAPSE) ? CellGraph::BIDI : 0);
}

void PortalGrouper::floodFill (int start, BitVector& out)
{
    UMBRA_ASSERT(m_cellStack.getSize() == 0);
    m_cellStack.pushBack(start);

    while (m_cellStack.getSize())
    {
        int idx = m_cellStack.popBack();
        if (out.test(idx))
            continue;
        out.set(idx);
        const CellGraph::Cell& c = m_input.getCell(idx);
        for (int i = 0; i < c.getPortalCount(); i++)
        {
            const CellGraph::Portal& p = c.getPortal(i);
            if (!p.isGate())
                m_cellStack.pushBack(p.getTarget());
        }
    }
}

int PortalGrouper::cluster (Array<int>& cellToCluster)
{
    UnionFind<int> clusters(getAllocator());

    for (int i = 0; i < m_input.getCellCount(); i++)
    {
        const CellGraph::Cell& cell = m_input.getCell(i);
        UMBRA_ASSERT(cell.isReachable());
        for (int j = 0; j < cell.getPortalCount(); j++)
            clusters.unionSets(i, cell.getPortal(j).getTarget());
    }

    cellToCluster.reset(m_input.getCellCount());
    int numClusters = 0;
    Hash<int, int> idToIdx(getAllocator());
    for (int i = 0; i < m_input.getCellCount(); i++)
    {
        int id = clusters.findSet(i);
        int* idx = idToIdx.get(id);
        if (!idx)
            idx = idToIdx.insert(id, numClusters++);
        cellToCluster[i] = *idx;
    }

    return numClusters;
}

void PortalGrouper::collectExternalPortals(const SubdivisionTree::Node* node, const AABB& aabb, int face)
{
    if (node->isLeaf())
    {
        int vc = node->getLeaf()->getIndex();
        if (vc < 0)
            return;

        AxialQuad aq(face, aabb.getFaceDist(face), aabb.getFaceRect(face));

        Array<AxialQuad>& aqs = m_extPortals[vc];

        if (!aqs.getSize() || aqs[aqs.getSize()-1].face != face)
            aqs.pushBack(aq);
        else
        {
            AxialQuad& aq2 = aqs[aqs.getSize()-1];
            aq2.rect.x = min2(aq2.rect.x, aq.rect.x);
            aq2.rect.y = min2(aq2.rect.y, aq.rect.y);
            aq2.rect.z = max2(aq2.rect.z, aq.rect.z);
            aq2.rect.w = max2(aq2.rect.w, aq.rect.w);
        }
    }
    else
    {
        AABB left, right;
        SubdivisionTreeUtils::splitBounds(node, aabb, left, right);

        collectExternalPortals(node->getInner()->getLeft(), left, face);
        collectExternalPortals(node->getInner()->getRight(), right, face);
    }
}

void PortalGrouper::groupCells(int vc)
{
    UMBRA_ASSERT(!m_used.test(vc));

    AABB aabb;
    cellAABB(vc, aabb);

    m_cells1.clearAll();
    collect(vc, -1, aabb, m_cells1);
    UMBRA_ASSERT(m_cells1.countOnes() == 1);

    Array<SourceQuad> sources(getAllocator());
    collectSources(m_cells1, sources);

    float est = estimateOcclusion(m_cells1, sources);
    UMBRA_UNREF(est);
    UMBRA_ASSERT(est == 0.f);

    bool done = false;
    int iter = 0;

    // Don't group cells with user portals. Visibility leaks too easily from those as they don't have any geometry for frustum culling.

#if 0
    {
        const CellGraph::Cell& cell = m_input.getCell(vc);
        for (int i = 0; i < cell.getPortalCount(); i++)
        {
            const CellGraph::Portal& portal = cell.getPortal(i);
            if (portal.isGate())
                done = true;
        }
    }
#endif

    // Iteratively grow box to each direction.

    while (!done && iter < 100)
    {
        int numCells = m_cells1.countOnes();

        for (int face = 0; face < 6; face++)
        {
            m_cells2.clearAll();
            collect(vc, face, aabb, m_cells2);
            collectSources(m_cells2, sources);

            float est2 = estimateOcclusion(m_cells2, sources);

            if (est2 <= m_params.featureSize*m_params.featureSize)
            {
                AABB aabb2;

                for (int j = 0; j < m_input.getCellCount(); j++)
                    if (m_cells1.test(j))
                        cellAABB(j, aabb2);

                // New AABB has to be close enough to a cube.

#if 0
                float mn = aabb2.getMinAxisLength();
                float mx = aabb2.getMaxAxisLength();

                if (mn > 0.f && mx / mn > 8.f)
                    continue;
#endif
                // Accept.

                aabb = aabb2;
                m_cells1._or(m_cells2);

                est = est2;
            }
        }

        done = numCells == m_cells1.countOnes();
        iter++;
    }

    for (int j = 0; j < m_input.getCellCount(); j++)
    {
        if (!m_cells1.test(j))
            continue;

        UMBRA_ASSERT(!m_used.test(j));
        UMBRA_ASSERT(m_cellToGroup[j] == -1);

        m_cellToGroup[j] = m_cellGroups;
        m_used.set(j);
    }

    m_cellGroups++;
}

void PortalGrouper::groupPortals()
{
    Array<Array<CellGraph::RectPortal> > rectPortals(m_cellGroups, m_platform.allocator);
    Array<Array<CellGraph::GatePortal> > gatePortals(m_cellGroups, m_platform.allocator);

    for (int i = 0; i < m_input.getCellCount(); i++)
    {
        const CellGraph::Cell& cell = m_input.getCell(i);
        int src = m_cellToGroup[i];
        if (src < 0)
            continue;

        for (int j = 0; j < cell.getRectPortalCount(); j++)
        {
            const CellGraph::RectPortal& portal = cell.getRectPortal(j);
            int target = m_cellToGroup[portal.getTarget()];
            if (src == target || target < 0)
                continue;

            CellGraph::RectPortal p = portal;
            p.setTarget(target);
            rectPortals[src].pushBack(p);
        }

        for (int j = 0; j < cell.getGatePortalCount(); j++)
        {
            const CellGraph::GatePortal& portal = cell.getGatePortal(j);
            int target = m_cellToGroup[portal.getTarget()];
            if (src == target || target < 0)
                continue;

            CellGraph::GatePortal p = portal;
            p.setTarget(target);
            gatePortals[src].pushBack(p);
        }
    }

    for (int i = 0; i < m_cellGroups; i++)
    {
        Array<CellGraph::RectPortal>& portals = rectPortals[i];

        BitVector bv(portals.getSize(), m_platform.allocator);
        CellGraph::Cell& dst = m_cellGraph.getCell(i);

        bv.clearAll();

        for (int j = 0; j < portals.getSize(); j++)
        {
            if (bv.test(j))
                continue;

            CellGraph::RectPortal p = portals[j];
            Array<Vector4> rects(m_platform.allocator);

            for (int k = j; k < portals.getSize(); k++)
            {
                if (bv.test(k))
                    continue;

                const CellGraph::RectPortal& p2 = portals[k];

                if (p.getTarget() != p2.getTarget())
                    continue;

                if (p.getRectPortal().getFace() == p2.getRectPortal().getFace() && p.getRectPortal().getZ() == p2.getRectPortal().getZ())
                {
                    rects.pushBack(p2.getRectPortal().getRect());
                    bv.set(k);
                    continue;
                }
            }

            // Group rectangles.

            RectGrouper rg(getAllocator(), m_params.featureSize * m_params.featureSize * .5f);

            for (int i = 0; i < rects.getSize(); i++)
                rg.addRect(rects[i]);

            rg.execute();

            for (int i = 0; i < rg.getResult().getSize(); i++)
            {
                CellGraph::RectPortal p2 = p.getRectPortal();
                p2.setRect(rg.getResult()[i]);
                dst.addRectPortal(p2);
            }
        }
    }

    for (int i = 0; i < m_cellGroups; i++)
    {
        Array<CellGraph::GatePortal>& portals = gatePortals[i];

        BitVector bv(portals.getSize(), m_platform.allocator);
        CellGraph::Cell& dst = m_cellGraph.getCell(i);

        bv.clearAll();

        for (int j = 0; j < portals.getSize(); j++)
        {
            if (bv.test(j))
                continue;

            CellGraph::GatePortal p = portals[j];
            Array<Vector4> rects(m_platform.allocator);

            for (int k = j; k < portals.getSize(); k++)
            {
                if (bv.test(k))
                    continue;

                const CellGraph::GatePortal& p2 = portals[k];

                if (p.getTarget() != p2.getTarget())
                    continue;

                if (p.getGatePortal().getGateIDs() == p2.getGatePortal().getGateIDs())
                {
                    bv.set(k);
                    if (j != k)
                    {
                        const Hash<Vector4, CellGraph::PortalHull>& hulls = p2.getGatePortal().getPortalHulls();
                        Hash<Vector4, CellGraph::PortalHull>::Iterator iter = hulls.iterate();
                        while (hulls.isValid(iter))
                        {
                            p.getGatePortal().addHullVertices(hulls.getKey(iter), hulls.getValue(iter).getVertices());
                            hulls.next(iter);
                        }
                    }
                }
            }

            dst.addGatePortal(p);
        }
    }
}

void PortalGrouper::collect(int vc, int openFace, const AABB& aabb, BitVector& dst)
{
    UMBRA_ASSERT(!m_used.test(vc));
    UMBRA_ASSERT(m_stack.getSize() == 0);

    dst.set(vc);
    m_stack.pushBack(vc);

    while (m_stack.getSize() != 0)
    {
        int cur = m_stack.popBack();

        const CellGraph::Cell& cell = m_input.getCell(cur);
        for (int i = 0; i < cell.getRectPortalCount(); i++)
        {
            const CellGraph::RectPortal& portal = cell.getRectPortal(i);

            if (m_used.test(portal.getTarget()))
                continue;

            if (dst.test(portal.getTarget()))
                continue;

            if (!aabb.intersectsWithArea(portal.getAABB()))
                continue;

            // Portal must not only-touch, unless the face is the open face.

            if (portal.getFace() != openFace)
            {
                if (aabb.getFaceDist(portal.getFace()) == portal.getZ())
                    continue;
            }

            dst.set(portal.getTarget());
            m_stack.pushBack(portal.getTarget());
        }
    }
}

void PortalGrouper::cellAABB(int vc, AABB& aabb)
{
    const CellGraph::Cell& cell = m_input.getCell(vc);

    for (int i = 0; i < cell.getRectPortalCount(); i++)
    {
        const CellGraph::RectPortal& portal = cell.getRectPortal(i);
        aabb.grow(portal.getAABB());
    }
}

void PortalGrouper::collectSources(const BitVector& bv, Array<SourceQuad>& sources)
{
    sources.clear();

    for (int i = 0; i < m_input.getCellCount(); i++)
    {
        if (!bv.test(i))
            continue;

        for (int j = 0; j < m_extPortals[i].getSize(); j++)
        {
            SourceQuad sq;
            sq.aq = m_extPortals[i][j];
            sq.aq.face ^= 1;
            sq.src = i;
            sources.pushBack(sq);
        }

        const CellGraph::Cell& cell = m_input.getCell(i);
        for (int j = 0; j < cell.getRectPortalCount(); j++)
        {
            const CellGraph::RectPortal& portal = cell.getRectPortal(j);
            if (bv.test(portal.getTarget()))
                continue;

            SourceQuad sq;
            sq.aq.face = portal.getFace() ^ 1;
            sq.aq.z = portal.getZ();
            sq.aq.rect = portal.getRect();
            sq.src = i;
            sources.pushBack(sq);
        }
    }
}

float PortalGrouper::estimateOcclusion(const BitVector& bv, const Array<SourceQuad>& src, int samples)
{
    if (!src.getSize())
        return 0.f; // \todo [Hannu] what to do without sources?

    WeightedSampler ws(m_platform.allocator);
    ws.resize(src.getSize());
    float totalArea = 0.f;

    for (int i = 0; i < ws.getSize(); i++)
    {
        UMBRA_ASSERT(bv.test(src[i].src));
        float area = src[i].aq.getArea();
        ws.setWeight(i, area);
        totalArea += area;
    }

    int hit = 0, miss = 0;

    for (int i = 0; i < samples; i++)
    {
        float t0 = (i + .5f) / samples;
        float t1 = haltonf<2>(i+1);
        float t2 = haltonf<3>(i+1);
        float t3 = haltonf<5>(i+1);
        float t4 = haltonf<7>(i+1);
        float t5 = haltonf<11>(i+1);

        int idx0 = ws.pickSample(t0);
        int idx1 = ws.pickSample(t1);
        if (idx0 == idx1)
            continue;

        const SourceQuad& s0 = src[idx0];
        const SourceQuad& s1 = src[idx1];

        if (s0.aq.face == s1.aq.face && s0.aq.z == s1.aq.z)
            continue;

        Vector3 a = s0.aq.getPoint(t2, t3);
        Vector3 b = s1.aq.getPoint(t4, t5);

        if (!s0.aq.isFront(b) || !s1.aq.isFront(a))
        {
            miss++;
            continue;
        }

        bool t = testLineSegment(s0.src, a, s1.src, b, bv);

        if (t)
        {
            hit++;
        }
        else
        {
            miss++;
        }
    }

    float occlusion = (hit+miss) ? miss / float(hit+miss) : 0.f;
    return (totalArea * occlusion);
}

float PortalGrouper::estimatePortals(int vc0, int vc1)
{
    const CellGraph::Cell& cl = m_input.getCell(vc0);
    const CellGraph::Cell& cr = m_input.getCell(vc1);

    // Collect left, right and testee portals.

    Array<AxialQuad> leftSources(m_platform.allocator);
    Array<AxialQuad> rightSources(m_platform.allocator);
    Array<AxialQuad> testee(m_platform.allocator);

    for (int j = 0; j < m_extPortals[vc0].getSize(); j++)
    {
        AxialQuad aq;
        aq = m_extPortals[vc0][j];
        aq.face ^= 1;
        leftSources.pushBack(aq);
    }

    for (int j = 0; j < m_extPortals[vc1].getSize(); j++)
    {
        AxialQuad aq;
        aq = m_extPortals[vc1][j];
        aq.face ^= 1;
        rightSources.pushBack(aq);
    }

    for (int i = 0; i < cl.getRectPortalCount(); i++)
    {
        const CellGraph::RectPortal& portal = cl.getRectPortal(i);

        AxialQuad aq;
        aq.face = portal.getFace() ^ 1;
        aq.z    = portal.getZ();
        aq.rect = portal.getRect();

        if (cl.getRectPortal(i).getTarget() == vc1)
            testee.pushBack(aq);
        else
            leftSources.pushBack(aq);
    }

    for (int i = 0; i < cr.getRectPortalCount(); i++)
    {
        const CellGraph::RectPortal& portal = cr.getRectPortal(i);

        AxialQuad aq;
        aq.face = portal.getFace() ^ 1;
        aq.z    = portal.getZ();
        aq.rect = portal.getRect();

        if (cr.getRectPortal(i).getTarget() != vc0)
            rightSources.pushBack(aq);
    }

    // Random sampling.

    if (leftSources.getSize() == 0 || rightSources.getSize() == 0)
        return 0.f;

    WeightedSampler lws(m_platform.allocator);
    lws.resize(leftSources.getSize());
    for (int i = 0; i < lws.getSize(); i++)
        lws.setWeight(i, leftSources[i].getArea());

    WeightedSampler rws(m_platform.allocator);
    rws.resize(rightSources.getSize());
    for (int i = 0; i < rws.getSize(); i++)
        rws.setWeight(i, rightSources[i].getArea());

    const int SAMPLES = 1000;
    int hit = 0, miss = 0;

    for (int i = 0; i < SAMPLES; i++)
    {
        float t0 = (i + .5f) / SAMPLES;
        float t1 = haltonf<2>(i+1);
        float t2 = haltonf<3>(i+1);
        float t3 = haltonf<5>(i+1);
        float t4 = haltonf<7>(i+1);
        float t5 = haltonf<11>(i+1);

        const AxialQuad& aq0 = leftSources[lws.pickSample(t0)];
        const AxialQuad& aq1 = rightSources[rws.pickSample(t1)];

        if (aq0.face == aq1.face && aq0.z == aq1.z)
            continue;

        Vector3 a = aq0.getPoint(t2, t3);
        Vector3 b = aq1.getPoint(t4, t5);

        int j;
        for (j = 0; j < testee.getSize(); j++)
            if (::testLineSegment(testee[j], a, b))
                break;

        if (j == testee.getSize())
        {
            miss++;
        }
        else
        {
            hit++;
        }
    }

    if (hit + miss == 0)
        return 0.f;

    float area = 0.f;
    for (int i = 0; i < leftSources.getSize(); i++)
        area += leftSources[i].getArea();
    for (int i = 0; i < rightSources.getSize(); i++)
        area += rightSources[i].getArea();

    float occlusion = area * miss / float(hit + miss);

    return occlusion;
}

bool PortalGrouper::testLineSegment(int ac, const Vector3& a, int bc, const Vector3& b, const BitVector& bv)
{
    UMBRA_ASSERT(bv.test(ac));
    UMBRA_ASSERT(bc < 0 || bv.test(bc));

    if (ac == bc)
        return true;

    m_cellVisited.clearAll();
    UMBRA_ASSERT(m_cellStack.getSize() == 0);
    m_cellStack.pushBack(ac);
    m_cellVisited.set(ac);
    while (m_cellStack.getSize() != 0)
    {
        int cur = m_cellStack.popBack();
        const CellGraph::Cell& cell = m_input.getCell(cur);
        for (int i = 0; i < cell.getPortalCount(); i++)
        {
            const CellGraph::Portal& portal = cell.getPortal(i);
            int tgt = portal.getTarget();

            if (!bv.test(tgt) ||
                m_cellVisited.test(tgt) ||
                (!portal.isGate() && !::testLineSegment(AxialQuad(portal.getRectPortal().getFace(), portal.getRectPortal().getZ(), portal.getRectPortal().getRect()), a, b)))
                continue;

            if (tgt == bc)
            {
                m_cellStack.clear();
                return true;
            }

            m_cellStack.pushBack(tgt);
            m_cellVisited.set(tgt);
        }
    }

    return false;
}

static int getCubeFace(const AABB& aabb, Vector3 p)
{
    p -= aabb.getCenter();
    p.x /= aabb.getDimensions().x / 2.f;
    p.y /= aabb.getDimensions().y / 2.f;
    p.z /= aabb.getDimensions().z / 2.f;

    int axis = getLongestAxis(Vector3f(fabsf(p.x), fabsf(p.y), fabsf(p.z)));
    return (axis << 1) | (p[axis] < 0.f ? 0 : 1);
}

static Vector3 clampToAABB(const AABB& aabb, Vector3 p)
{
    p.x = max2(p.x, aabb.getMin().x);
    p.y = max2(p.y, aabb.getMin().y);
    p.z = max2(p.z, aabb.getMin().z);
    p.x = min2(p.x, aabb.getMax().x);
    p.y = min2(p.y, aabb.getMax().y);
    p.z = min2(p.z, aabb.getMax().z);
    return p;
}

void PortalGrouper::computeOccludedRays(bool cheap)
{
    if (m_params.featureSize <= 0.f)
    {
        m_cells2.setAll();
        m_occludedRays.clear();
        m_occludedRaysThreshold = 1;
        return;
    }

    SubdivisionTree* matchingTrees[6];

    for (int i = 0; i < 6; i++)
    {
        if (m_input.getMatchingTree(i).isEmpty())
            matchingTrees[i] = 0;
        else
        {
            matchingTrees[i] = UMBRA_NEW(SubdivisionTree, getAllocator());
            m_input.getMatchingTree(i).deserialize(*matchingTrees[i]);
        }
    }

    Vector3 center = m_input.getAABB().getCenter();
    float   radius = m_input.getAABB().getDiagonalLength() * 1.0001f / 2.f;

    int targetRayCount = cheap ? 20 : 100;

    // Aim for certain ray threshold.
    int SAMPLES = (int)ceilf((4.f * 3.14159265f * radius * radius) / (m_params.featureSize * m_params.featureSize) * targetRayCount);
    SAMPLES = max2(SAMPLES, 10000);
    SAMPLES = min2(SAMPLES, 1000000);

    m_cells1.setAll();

    if (cheap)
        m_cells2.setAll();
    else
        m_cells2.clearAll();

    for (int i = 0; i < 6; i++)
        if (matchingTrees[i])
        {
            SubdivisionTree::LeafIterator iter;
            for (matchingTrees[i]->iterate(iter); !iter.end(); iter.next())
                if (iter.leaf()->getIndex() >= 0)
                    m_cells2.set(iter.leaf()->getIndex());
        }

    for (int i = 0; i < SAMPLES; i++)
    {
        float t0 = (i + .5f) / SAMPLES;
        float t1 = haltonf<2>(i+1);
        float t2 = haltonf<3>(i+1);
        float t3 = haltonf<5>(i+1);

        t0 = t0 * 2.f - 1.f;
        t1 = t1 * 3.14159265f * 2.0f;

        Vector3 d;
        d.z = t0;
        float l = sqrtf(1.f - t0*t0);
        d.x = (float)std::cos(t1) * l;
        d.y = (float)std::sin(t1) * l;

        t2 = t2 * 3.14159265f * 2.0f;
        t3 = sqrtf(t3);

        float discX = (float)std::cos(t2) * t3;
        float discY = (float)std::sin(t2) * t3;

        Vector3 p = MatrixFactory::orthonormalBasis(d).transform(Vector3f(discX, discY, 0.f)).xyz();

        Vector3 a = p - d;
        Vector3 b = p + d;

        a = center + a * radius;
        b = center + b * radius;

        if (!intersectAABBLineSegment_Fast(m_input.getAABB(), a, b))
            continue;

        {
            // TODO: actual bounds!
            Vector3 aa = a - (b - a) * 1000.f;
            Vector3 bb = b + (b - a) * 1000.f;

            if (m_params.viewVolume.isOK() && !intersectAABBLineSegment_Fast(m_params.viewVolume, aa, bb))
                continue;
        }

        Vector3 p0, p1;
        if (!intersectAABBLineSegment(m_input.getAABB(), a, b, p0))
            continue;
        if (!intersectAABBLineSegment(m_input.getAABB(), b, a, p1))
            continue;

        p0 = clampToAABB(m_input.getAABB(), p0);
        p1 = clampToAABB(m_input.getAABB(), p1);

        int faceA = getCubeFace(m_input.getAABB(), p0);
        int faceB = getCubeFace(m_input.getAABB(), p1);

        p0 = clampToAABB(matchingTrees[faceA]->getAABB(), p0);
        p1 = clampToAABB(matchingTrees[faceB]->getAABB(), p1);

        int cellA = -1, cellB = -1;

        cellA = SubdivisionTreeUtils(*matchingTrees[faceA]).findLeafIndex(p0);
        cellB = SubdivisionTreeUtils(*matchingTrees[faceB]).findLeafIndex(p1);

        if (cellA == cellB)
            continue;

        if (cellA < 0)
        {
            UMBRA_ASSERT(cellB >= 0);
            swap2(cellA, cellB);
            swap2(p0, p1);
        }

        bool connected = testLineSegment(cellA, p0, cellB, p1, m_cells1);
        m_cells2._or(m_cellVisited);

        if (connected || cellB < 0)
            continue;

        OccludedRay ray;
        ray.a = cellA;
        ray.b = cellB;
        ray.s = p0;
        ray.e = p1;
        m_occludedRays.pushBack(ray);
    }

    m_occludedRaysThreshold = int(ceilf(m_params.featureSize * m_params.featureSize / (4.f * 3.14159265f * radius * radius) * float(SAMPLES))) + 1;
    //m_occludedRaysThreshold = int(ceilf((4.f * 3.14159265f * radius * radius) / m_params.featureSize / m_params.featureSize / float(SAMPLES))) + 1;

    m_occlusionPerRay = 4.0 * 3.14159265 * double(radius) * double(radius) / double(SAMPLES);

    //printf("Occlusion %f\n", 4.f * 3.14159265f * radius * radius * m_occludedRays.getSize() / float(SAMPLES));

    for (int i = 0; i < 6; i++)
        UMBRA_DELETE(matchingTrees[i]);
}

void PortalGrouper::split(Array<Group*>& out, const Group* g, const UnionFind<int>& uf)
{
    Hash<int, Group*> idToGroup(getAllocator());

    for (int i = 0; i < g->cells.getSize(); i++)
    {
        int id = uf.findSet(g->cells[i]);

        Group** gr = idToGroup.get(id);

        if (gr == 0)
        {
            gr = idToGroup.insert(id, UMBRA_NEW(Group, getAllocator()));
            out.pushBack(*gr);
        }

        (*gr)->cells.pushBack(g->cells[i]);
    }
}

void PortalGrouper::splitNonConnected(Array<Group*>& out, const Group* g)
{
    Set<int> cellSet(getAllocator());
    for (int i = 0; i < g->cells.getSize(); i++)
        cellSet.insert(g->cells[i]);

    // Split non-connected components in the group.

    UnionFind<int> uf(getAllocator());

    for (int i = 0; i < g->cells.getSize(); i++)
    {
        const CellGraph::Cell& cell = m_input.getCell(g->cells[i]);

        for (int j = 0; j < cell.getPortalCount(); j++)
        {
            const CellGraph::Portal& portal = cell.getPortal(j);

            if (portal.isGate())
            {
                uf.unionSets(g->cells[i], portal.getTarget());
                continue;
            }

            if (!cellSet.contains(portal.getTarget()))
                continue;

            uf.unionSets(g->cells[i], portal.getTarget());
        }
    }

    split(out, g, uf);
}

void PortalGrouper::splitByPlane(Array<Group*>& out, const Group* g, int axis, float pos)
{
    Set<int> cellSet(getAllocator());
    for (int i = 0; i < g->cells.getSize(); i++)
        cellSet.insert(g->cells[i]);

    AABB intersectionAABB;
    for (int i = 0; i < g->cells.getSize(); i++)
    {
        AABB aabb;
        cellAABB(g->cells[i], aabb);
        if (aabb.isOK() && pos > aabb.getMin()[axis] && pos < aabb.getMax()[axis])
            intersectionAABB.grow(aabb);
    }

    // Find if this is a valid split.

    UnionFind<int> uf(getAllocator());

    for (int i = 0; i < g->cells.getSize(); i++)
    {
        const CellGraph::Cell& cell = m_input.getCell(g->cells[i]);

        if (!cellSet.contains(g->cells[i]))
            continue;

        for (int j = 0; j < cell.getRectPortalCount(); j++)
        {
            const CellGraph::RectPortal& portal = cell.getRectPortal(j);

            if (portal.getAxis() == axis && portal.getZ() == pos)
                continue;

            if (intersectionAABB.isOK() &&
                (intersectionAABB.getMin()[portal.getAxis()] == portal.getZ() ||
                 intersectionAABB.getMax()[portal.getAxis()] == portal.getZ()))
            {
                continue;
            }

            if (!cellSet.contains(portal.getTarget()))
                continue;

            uf.unionSets(g->cells[i], portal.getTarget());
        }
    }

    int firstId = uf.findSet(g->cells[0]);
    bool ok = false;

    for (int k = 1; k < g->cells.getSize(); k++)
    {
        if (uf.findSet(g->cells[k]) != firstId)
        {
            ok = true;
            break;
        }
    }

    if (!ok)
        return;

    split(out, g, uf);
}

PortalGrouper::GroupPortal* PortalGrouper::computeGroupPortal(const Group* ga, const Group* gb)
{
    Set<int> cellSet(getAllocator());
    for (int i = 0; i < gb->cells.getSize(); i++)
        cellSet.insert(gb->cells[i]);

    Array<AxialQuad> quads(getAllocator());

    for (int i = 0; i < ga->cells.getSize(); i++)
    {
        const CellGraph::Cell& cell = m_input.getCell(ga->cells[i]);

        for (int j = 0; j < cell.getRectPortalCount(); j++)
        {
            const CellGraph::RectPortal& portal = cell.getRectPortal(j);

            if (!cellSet.contains(portal.getTarget()))
                continue;

            AxialQuad aq(portal.getFace(), portal.getZ(), portal.getRect());
            quads.pushBack(aq);
        }
    }

    if (quads.getSize() == 0)
        return 0;

    GroupPortal* p = UMBRA_NEW(GroupPortal, getAllocator());
    p->ga = (Group*)ga;
    p->gb = (Group*)gb;

    Set<Pair<int, float> > groupedPlanes(getAllocator());

    for (int i = 0; i < quads.getSize(); i++)
    {
        Pair<int, float> plane(quads[i].getFace(), quads[i].getZ());

        if (groupedPlanes.contains(plane))
            continue;
        groupedPlanes.insert(plane);

        RectGrouper rg(getAllocator(), m_params.featureSize * m_params.featureSize * .5f);

        int n = 0;
        for (int j = i; j < quads.getSize(); j++)
            if (quads[j].getFace() == plane.a && quads[j].getZ() == plane.b)
            {
                rg.addRect(quads[j].getRect());
                n++;
            }

        if (n > 128)
            rg.setStrategy(RectGrouper::COMBINE_ALL);

        rg.execute();

        for (int j = 0; j < rg.getResult().getSize(); j++)
        {
            AxialQuad aq(plane.a, plane.b, rg.getResult()[j]);
            p->quads.pushBack(aq);
        }
    }

    return p;
}

bool PortalGrouper::testRay(Set<Group*>& visited, Group* ga, Group* gb, int rayIdx, GroupPortal* collapsedPortal)
{
    UMBRA_ASSERT(!visited.contains(ga) && !visited.contains(gb));

    visited.insert(ga);
    visited.insert(gb);
    UMBRA_ASSERT(m_groupStack.getSize() == 0);
    m_groupStack.pushBack(ga);

    bool hit = false;

    while (m_groupStack.getSize())
    {
        Group* g = m_groupStack.popBack();
        if (g == gb)
            hit = true;

        for (int i = 0; i < g->portals.getSize(); i++)
        {
            Group* g2 = g->portals[i]->getOpposite(g);
            if (visited.contains(g2))
                continue;

            if (g->portals[i] == collapsedPortal)
            {
                visited.insert(g2);
                m_groupStack.pushBack(g2);
                break;
            }

            for (int j = 0; j < g->portals[i]->quads.getSize(); j++)
                if (::testLineSegment(g->portals[i]->quads[j], m_occludedRays[rayIdx].s, m_occludedRays[rayIdx].e))
                {
                    visited.insert(g2);
                    m_groupStack.pushBack(g2);
                    break;
                }
        }
    }

    return hit;
}

int PortalGrouper::shootRays(const Array<Group*>& in, bool record, GroupPortal* collapsedPortal)
{
    Hash<int, Group*> cellToGroup(getAllocator());

    for (int i = 0; i < in.getSize(); i++)
        for (int j = 0; j < in[i]->cells.getSize(); j++)
            cellToGroup.insert(in[i]->cells[j], in[i]);

    if (record)
        for (int i = 0; i < in.getSize(); i++)
            in[i]->rays.clear();

    int numRays = 0;

    for (int i = 0; i < m_occludedRays.getSize(); i++)
    {
        Group* ga = *cellToGroup.get(m_occludedRays[i].a);
        Group* gb = *cellToGroup.get(m_occludedRays[i].b);

        Set<Group*> visited(getAllocator());
        if (testRay(visited, ga, gb, i, collapsedPortal))
        {
            if (record)
            {
                Set<Group*>::Iterator iter = visited.iterate();
                while (iter.next())
                    iter.getValue()->rays.pushBack(i);
            }

            numRays++;
        }
    }

    return numRays;
}

void PortalGrouper::occlusionSplitter(Array<Group*>& out, const Group* g)
{
    Array<Group*> current(getAllocator());

    current.pushBack((Group*)g);

    for (int i = 0; i < current.getSize(); i++)
        splitNonConnected(out, current[i]);

    if (out.getSize() == 0)
        return;

    bool done;

    do
    {
        done = true;
        current = out;
        out.clear();

        // Shoot rays and record non-occluded rays to Groups.

        Hash<int, Group*> cellToGroup(getAllocator());

        for (int i = 0; i < current.getSize(); i++)
            for (int j = 0; j < current[i]->cells.getSize(); j++)
                cellToGroup.insert(current[i]->cells[j], current[i]);

        for (int i = 0; i < current.getSize(); i++)
            current[i]->rays.clear();

        int numRays = 0;

        for (int i = 0; i < m_occludedRays.getSize(); i++)
        {
            Group* ga = *cellToGroup.get(m_occludedRays[i].a);
            Group* gb = *cellToGroup.get(m_occludedRays[i].b);

            Set<Group*> visited(getAllocator());
            if (testRay(visited, ga, gb, i))
            {
                Set<Group*>::Iterator iter = visited.iterate();
                while (iter.next())
                    iter.getValue()->rays.pushBack(i);

                numRays++;
            }
        }

        // Sort groups according to possible occlusion in that group. This
        // reduces unnecessary split tests.

        UMBRA_ASSERT(current.getSize());

        Array<Pair<int, int> > sortedGroups(current.getSize(), getAllocator());

        for (int i = 0; i < sortedGroups.getSize(); i++)
            sortedGroups[i] = Pair<int, int>(-current[i]->rays.getSize(), i);

        quickSort(sortedGroups.getPtr(), sortedGroups.getSize());

        Array<Group*> oldCurrent = current;

        for (int i = 0; i < sortedGroups.getSize(); i++)
            current[i] = oldCurrent[sortedGroups[i].b];

        // Find best split.

        int    bestOcclusion = m_occludedRaysThreshold-1;
        Group* bestGroup     = 0;
        int    bestAxis      = -1;
        float  bestPos       = 0.f;

        for (int i = 0; i < current.getSize(); i++)
        {
            Group* g = current[i];

            if (g->rays.getSize() < bestOcclusion)
                continue;

            Array<GroupPortal*> originalPortals(getAllocator());
            originalPortals = g->portals;
            disconnectGroup(g);

            Set<Pair<int, float> > testedSplits(getAllocator());

            for (int i = 0; i < g->cells.getSize(); i++)
            {
                const CellGraph::Cell& cell = m_input.getCell(g->cells[i]);

                for (int j = 0; j < cell.getRectPortalCount(); j++)
                {
                    const CellGraph::RectPortal& portal = cell.getRectPortal(j);

                    int   axis = portal.getFace() >> 1;
                    float pos  = portal.getZ();

                    if (testedSplits.contains(Pair<int, float>(axis, pos)))
                        continue;

                    testedSplits.insert(Pair<int, float>(axis, pos));

                    // When there are huge amount of cells, don't test all
                    // portals. Union finding and stuff gets really slow with
                    // huge cell counts.

                    if (g->cells.getSize() > 2000)
                    {
                        if ((testedSplits.getSize() % 10) != 0)
                            continue;
                    }

                    Array<Group*> splitted(getAllocator());
                    splitByPlane(splitted, g, axis, pos);

                    if (!splitted.getSize())
                        continue;

                    // Connect new groups to each other.

                    for (int i = 0; i < splitted.getSize(); i++)
                        for (int j = i+1; j < splitted.getSize(); j++)
                        {
                            GroupPortal* p = computeGroupPortal(splitted[i], splitted[j]);
                            if (p)
                                p->connect();
                        }

                    // Connect new groups to other groups.

                    for (int i = 0; i < splitted.getSize(); i++)
                        for (int j = 0; j < originalPortals.getSize(); j++)
                        {
                            Group* g2 = originalPortals[j]->getOpposite(g);
                            GroupPortal* p = computeGroupPortal(splitted[i], g2);
                            if (p)
                                p->connect();
                        }

                    // Test new rays that were occluded.

                    Hash<int, Group*> newCellToGroup(getAllocator());

                    for (int i = 0; i < splitted.getSize(); i++)
                        for (int j = 0; j < splitted[i]->cells.getSize(); j++)
                            newCellToGroup.insert(splitted[i]->cells[j], splitted[i]);

                    int occludedRays = 0;

                    for (int i = 0; i < g->rays.getSize(); i++)
                    {
                        // Early exit if this group cannot reach best occlusion anymore.

                        if (occludedRays + (g->rays.getSize()-i) <= bestOcclusion)
                            break;

                        Group** ga = newCellToGroup.get(m_occludedRays[g->rays[i]].a);
                        if (!ga)
                            ga = cellToGroup.get(m_occludedRays[g->rays[i]].a);

                        Group** gb = newCellToGroup.get(m_occludedRays[g->rays[i]].b);
                        if (!gb)
                            gb = cellToGroup.get(m_occludedRays[g->rays[i]].b);

                        Set<Group*> vis(getAllocator());
                        if (!testRay(vis, *ga, *gb, g->rays[i]))
                            occludedRays++;
                    }

                    if (occludedRays > bestOcclusion)
                    {
                        bestOcclusion = occludedRays;
                        bestGroup = g;
                        bestAxis = axis;
                        bestPos = pos;
                    }

                    // Clean up.

                    for (int i = 0; i < splitted.getSize(); i++)
                    {
                        Array<GroupPortal*> portals(getAllocator());
                        portals = splitted[i]->portals;

                        disconnectGroup(splitted[i]);

                        for (int j = 0; j < portals.getSize(); j++)
                            UMBRA_DELETE(portals[j]);

                        UMBRA_DELETE(splitted[i]);
                    }
                }
            }

            for (int i = 0; i < originalPortals.getSize(); i++)
                originalPortals[i]->connect();
        }

        // Split best group.

        if (bestGroup)
        {
            Array<GroupPortal*> originalPortals(getAllocator());
            originalPortals = bestGroup->portals;
            disconnectGroup(bestGroup);

            Array<Group*> splitted(getAllocator());
            splitByPlane(splitted, bestGroup, bestAxis, bestPos);

            // Connect new groups to each other.

            for (int i = 0; i < splitted.getSize(); i++)
                for (int j = i+1; j < splitted.getSize(); j++)
                {
                    GroupPortal* p = computeGroupPortal(splitted[i], splitted[j]);
                    if (p)
                        p->connect();
                }

            // Connect new groups to other groups.

            for (int i = 0; i < splitted.getSize(); i++)
                for (int j = 0; j < originalPortals.getSize(); j++)
                {
                    Group* g2 = originalPortals[j]->getOpposite(bestGroup);
                    GroupPortal* p = computeGroupPortal(splitted[i], g2);
                    if (p)
                        p->connect();
                }

            out.append(splitted);

            for (int i = 0; i < originalPortals.getSize(); i++)
                UMBRA_DELETE(originalPortals[i]);

            UMBRA_DELETE(bestGroup);

            done = false;
        }

        for (int i = 0; i < current.getSize(); i++)
            if (current[i] != bestGroup)
                out.pushBack(current[i]);

        UMBRA_ASSERT(current.getSize());

        // Try to collapse cells.

#if 0
        current = out;
        out.clear();

        {
            int currentOcclusion = shootRays(current, false);

            GroupPortal* worstPortal    = 0;
            int          worstOcclusion = INT_MAX;

            for (int i = 0; i < current.getSize(); i++)
            {
                for (int j = 0; j < current[i]->portals.getSize(); j++)
                {
                    int occlusion = shootRays(current, false, current[i]->portals[j]);

                    printf("%d => %d\n", currentOcclusion, occlusion);

                }
            }

            if (worstPortal)
            {
                Group* ga = worstPortal->ga;
                Group* gb = worstPortal->gb;

                Array<GroupPortal*> originalA = ga->portals;
                disconnectGroup(ga);

                Array<GroupPortal*> originalB = gb->portals;
                disconnectGroup(gb);

                Group* newGroup = UMBRA_NEW(Group, getAllocator());
                newGroup->cells = ga->cells;
                newGroup->cells.append(gb->cells);

                Set<Group*> connected(getAllocator());

                for (int i = 0; i < originalA.getSize(); i++)
                {
                    GroupPortal* p = computeGroupPortal(newGroup, originalA[i]->getOpposite(ga));
                    UMBRA_ASSERT(p);
                    p->connect();
                    connected.insert(originalA[i]->getOpposite(ga));
                }

                for (int i = 0; i < originalB.getSize(); i++)
                {
                    if (connected.contains(originalB[i]->getOpposite(gb)))
                        continue;

                    GroupPortal* p = computeGroupPortal(newGroup, originalB[i]->getOpposite(gb));
                    UMBRA_ASSERT(p);
                    p->connect();
                }

                out.pushBack(newGroup);

                for (int i = 0; i < current.getSize(); i++)
                    if (current[i] != worstPortal->ga && current[i] != worstPortal->gb)
                        out.pushBack(current[i]);

                done = false;
            }
            else
                out = current;
        }
#endif

    } while (!done);
}

void PortalGrouper::disconnectGroup(Group* g)
{
    for (int i = 0; i < g->portals.getSize(); i++)
    {
        Group* g2 = g->portals[i]->getOpposite(g);

        bool found = false;

        for (int j = 0; j < g2->portals.getSize(); j++)
            if (g2->portals[j] == g->portals[i])
            {
                g2->portals[j] = g2->portals[g2->portals.getSize()-1];
                g2->portals.popBack();
                found = true;
                break;
            }

        UMBRA_UNREF(found);
        UMBRA_ASSERT(found);
    }

    g->portals.clear();
}

#endif
