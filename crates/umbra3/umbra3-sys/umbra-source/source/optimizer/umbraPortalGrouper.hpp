#pragma once

#include "umbraCellGraph.hpp"
#include "umbraExtCellGraph.hpp"
#include "umbraUnionFind.hpp"

#define DEFAULT_OCCLUSION_SAMPLES 200
// minimum number of visible objects in graph to consider object occlusion
#define GRAPHCUT_MIN_TARGETS_THRESHOLD 5
// how much is object occlusion allowed to get worse due to grouping
#define GRAPHCUT_OBJECT_OCCLUSION_FACTOR 0.2f

namespace Umbra
{

struct AxialQuad
{
    int     face;
    float   z;
    Vector4 rect;

    AxialQuad() : face(-1), z(0.f) {}
    AxialQuad(int f, float z, const Vector4& r) : face(f), z(z), rect(r) {}

    int     getFace() const { return face; }
    float   getZ() const { return z; }
    Vector4 getRect() const { return rect; }

    // Note that on the plane is not in front. (same thing for back testing)
    bool isFront (const Vector3& p) const
    {
        if ((face & 1) == 0)
            return p[face>>1] < z;
        else
            return p[face>>1] > z;
    }

    bool isBack (const Vector3& p) const
    {
        if ((face & 1) == 0)
            return p[face>>1] > z;
        else
            return p[face>>1] < z;
    }

    bool sameDir (const Vector3& dir) const
    {
        return (dir[face>>1] < 0.f) == ((face & 1) == 0);
    }

    float getArea() const
    {
        return (rect.z - rect.x) * (rect.w - rect.y);
    }

    Vector3 getPoint(float x, float y) const
    {
        UMBRA_ASSERT(x >= 0.f && x <= 1.f);
        UMBRA_ASSERT(y >= 0.f && y <= 1.f);
        Vector3 p;
        p[face >> 1] = z;
        p[((face >> 1) + 1) % 3] = rect.x + (rect.z - rect.x) * x;
        p[((face >> 1) + 2) % 3] = rect.y + (rect.w - rect.y) * y;
        return p;
    }

    AABB getAABB() const
    {
        int axis = face >> 1;
        AABB aabb;
        aabb.setMin(axis, z);
        aabb.setMin((axis+1)%3, rect.x);
        aabb.setMin((axis+2)%3, rect.y);
        aabb.setMax(axis, z);
        aabb.setMax((axis+1)%3, rect.z);
        aabb.setMax((axis+2)%3, rect.w);
        return aabb;
    }

    bool operator== (const AxialQuad& q) const
    {
        return face == q.face && z == q.z && rect == q.rect;
    }

    bool operator!= (const AxialQuad& q) const
    {
        return !operator==(q);
    }
};

struct SourceQuad
{
    AxialQuad   aq;
    int         src;
};

struct PortalGrouperParams
{
    enum Strategy
    {
        NONE, BOX, COLLAPSE, AGGRESSIVE, GRAPHCUT_EXTERNAL, CLUSTER,
        OCCLUSION_SPLITTER, COLLAPSE_EXTERNALS
    };

    PortalGrouperParams() : featureSize(0.f), debug(false), strategy(NONE), planeAxis(-1) {}

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, featureSize);
        stream(op, debug);
        stream(op, strategy);
        stream(op, viewVolume);
        stream(op, planeAxis);
        stream(op, planeZ);
    }

    float    featureSize;
    bool     debug;
    Strategy strategy;
    AABB     viewVolume;
    int      planeAxis; // For collapse.
    float    planeZ;
};

class ContractableGraph;

class PortalGrouper
{
public:
    PortalGrouper(const PlatformServices& platform, CellGraph& out, const CellGraph&, const PortalGrouperParams&);
    ~PortalGrouper();

    void perform (ExternalCellGraph* ext = NULL);

    const Array<int>& getGrouping(void) const { return m_cellToGroup; }

    CellGraph&  getCellGraph(void) { return m_cellGraph; }

    Allocator* getAllocator(void) const { return m_platform.allocator; }

    double getOcclusionValue() { UMBRA_ASSERT(m_params.strategy == PortalGrouperParams::OCCLUSION_SPLITTER); return m_occlusionValue; }

private:

    void    collectExternalPortals  (const SubdivisionTree::Node* node, const AABB& aabb, int face);
    void    groupCells              (int vc);
    void    groupPortals            (void);
    void    collectSources          (const BitVector& bv, Array<SourceQuad>& sources);
    float   estimateOcclusion       (const BitVector& cells, const Array<SourceQuad>& sources, int samples = DEFAULT_OCCLUSION_SAMPLES);
    float   estimatePortals         (int vc0, int vc1);
    bool    testLineSegment         (int ac, const Vector3& a, int bc, const Vector3& b, const BitVector& cell);
    void    collect                 (int vc, int openFace, const AABB& aabb, BitVector& dst);
    void    cellAABB                (int vc, AABB& aabb);
    int     cluster                 (Array<int>& grouping);
    void    floodFill               (int c, BitVector& out);

    struct Group;
    struct GroupPortal;

    void         computeOccludedRays     (bool cheap);
    void         split                   (Array<Group*>& out, const Group* in, const UnionFind<int>& uf);
    void         splitByPlane            (Array<Group*>& out, const Group* in, int axis, float pos);
    void         splitNonConnected       (Array<Group*>& out, const Group* in);
    void         disconnectGroup         (Group* g);
    GroupPortal* computeGroupPortal      (const Group* ga, const Group* gb);
    bool         testRay                 (Set<Group*>& visited, Group* ga, Group* gb, int rayIdx, GroupPortal* collapsedPortal = 0);
    int          shootRays               (const Array<Group*>& in, bool record = false, GroupPortal* collapsedPortal = 0);
    void         occlusionSplitter       (Array<Group*>& out, const Group* in);
    int          findBestSplit           (int& axis, float& pos, const Group* g);

    // Input.

    PlatformServices            m_platform;
    const CellGraph&            m_input;
    const PortalGrouperParams&  m_params;

    // Output.

    CellGraph&                  m_cellGraph;

    // Misc.

    Array<Array<AxialQuad> >    m_extPortals;
    Array<int>                  m_cellToGroup;
    int                         m_cellGroups;

    Array<float>                m_volumes;
    Array<float>                m_importances;

    Array<int>                  m_stack;
    BitVector                   m_used;

    BitVector                   m_cells1;
    BitVector                   m_cells2;
    Array<int>                  m_cellStack;
    BitVector                   m_cellVisited;

    // Occlusion splitter.

    struct OccludedRay
    {
        int     a, b;
        Vector3 s, e;
    };

    struct GroupPortal
    {
        GroupPortal(Allocator* a) : ga(0), gb(0), quads(a) {}

        Group* getOpposite(const Group* g) const
        {
            UMBRA_ASSERT(g == ga || g == gb);
            return g == ga ? gb : ga;
        }

        void connect()
        {
            ga->portals.pushBack(this);
            gb->portals.pushBack(this);
        }

        Group*           ga;
        Group*           gb;
        Array<AxialQuad> quads;
    };

    struct Group
    {
        Group(Allocator* a) : cells(a), portals(a), rays(a) {}

        Array<int>          cells;
        Array<GroupPortal*> portals;
        Array<int>          rays;
    };

    Array<OccludedRay>          m_occludedRays;
    double                      m_occlusionPerRay;
    int                         m_occludedRaysThreshold;
    Array<Group*>               m_groupStack;
    double                      m_occlusionValue;

    PortalGrouper& operator=(const PortalGrouper&) { return *this; } // deny
};

}
