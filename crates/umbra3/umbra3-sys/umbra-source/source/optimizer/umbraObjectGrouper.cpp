// Copyright (c) 2010-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraPrivateDefs.hpp"
#include "optimizer/umbraObjectGrouper.hpp"
#include "umbraImpScene.hpp"
#include "umbraMatrix.hpp"
#include "umbraAABBDatabase.hpp"
#include "umbraHash.hpp"
#include "umbraHeap.hpp"
#include "umbraDeque.hpp"
#include "umbraTimer.hpp"
#include "umbraLogger.hpp"

#include <float.h> // FLT_MAX

#if (UMBRA_OS == UMBRA_WINDOWS)
#pragma warning(disable: 4706) // warning C4706: assignment withing conditional expression
#endif

namespace Umbra
{

//------------------------------------------------------------------------

static const float DefaultObjectCost = 1000.0f;

//------------------------------------------------------------------------

struct Node
{
    Node(void)
    :   m_parent      (NULL)
    ,   m_ID          (-1)
    ,   m_flow        (0.0f)
    ,   m_capacity    (0.0f)
    {
        m_child[0] = m_child[1] = NULL;
    }

    void init(const AABB& aabb, int ID, float objectCost, float capacity)
    {
        m_aabb        = aabb;
        m_objectCost  = objectCost;
        m_ID          = ID;
        m_capacity    = capacity;
    }

    void init(Node* A, Node* B, int ID, float capacity)
    {
        m_child[0]    = A;
        m_child[1]    = B;
        m_ID          = ID;
        m_capacity    = capacity;

        m_aabb = A->m_aabb;
        m_aabb.grow(B->m_aabb);

        m_objectCost = A->m_objectCost + B->m_objectCost;

        A->m_parent = this;
        B->m_parent = this;
    }

    void init(Node* node)
    {
        UMBRA_ASSERT(node);

        m_aabb       = node->m_aabb;
        m_objectCost = node->m_objectCost;
        m_ID         = node->m_ID;
        m_flow       = node->m_flow;
        m_capacity   = node->m_capacity;
    }

    UMBRA_INLINE bool isLeaf(void) const
    {
        return m_child[0] == NULL && m_child[1] == NULL;
    }

    UMBRA_INLINE float getObjectCost(void) const
    {
        return m_objectCost;
    }

    UMBRA_INLINE const AABB& getAABB(void) const
    {
        return m_aabb;
    }

    UMBRA_INLINE float getCapacity(void) const
    {
        return m_capacity;
    }

    UMBRA_INLINE float getFlow(void) const
    {
         return m_flow;
    }

    AABB    m_aabb;
    Node*  m_child[2];
    Node*  m_parent;
    float   m_objectCost;
    int     m_ID;

    float   m_flow;
    float   m_capacity;

    AABBDatabase<int>::Handle m_handle;
};

//------------------------------------------------------------------------

class FCostModel
{
public:

    FCostModel(void)
    :   m_groupCost   (0.0f)
    {
    }

    FCostModel(const AABB& aabb, float groupCost)
    {
        init(aabb, groupCost);
    }

    void init(const AABB& aabb, float groupCost)
    {
        m_groupCost = groupCost;
        m_normalizer  = aabb.getSurfaceArea();
    }

    UMBRA_INLINE float getCost(const AABB& aabb, float objectCost)
    {
        // ###ari Use improved visibility probability

        float P = aabb.getSurfaceArea();
        return (m_normalizer - P)*m_groupCost + P*objectCost;
    }

private:

    float m_normalizer;
    float m_groupCost;
};

//-----------------------------------------------------------------

class TreeBuilder
{
public:

    enum
    {
        MaxNeighbourCount = 32
    };

public:

    TreeBuilder(const PlatformServices& platformServices)
        : m_platformServices(platformServices)
    {
    }

    TreeBuilder(const PlatformServices& platformServices, const AABB* AABBArray, int AABBCount, const ObjectGrouperParams& params, const float* objectCostArray)
        : m_platformServices(platformServices)
    {
        init(AABBArray, AABBCount, params, objectCostArray);
    }

    ~TreeBuilder()
    {
        deinit();
    }

    struct FMove
    {
        FMove(void)
        {
            m_offset[0] = m_offset[1] = -1;
        }

        FMove(int A, int B)
        {
            UMBRA_ASSERT(A != B);
            m_offset[0] = A; m_offset[1] = B;
        }

        UMBRA_INLINE AABB getAABB(Node* basePtr) const
        {
            AABB result = getLeft(basePtr)->getAABB();
            result.grow(getRight(basePtr)->getAABB());
            return result;
        }

        UMBRA_INLINE float getObjectCost(Node* basePtr) const
        {
            return getLeft(basePtr)->getObjectCost() + getRight(basePtr)->getObjectCost();
        }

        UMBRA_INLINE bool isValid(Node* basePtr) const
        {
            return getLeft(basePtr)->m_parent == NULL && getRight(basePtr)->m_parent == NULL;
        }

        UMBRA_INLINE Node* getLeft(Node* basePtr) const
        {
            return &basePtr[m_offset[0]];
        }

        UMBRA_INLINE Node* getRight(Node* basePtr) const
        {
            return &basePtr[m_offset[1]];
        }

        int m_offset[2];
    };

    UMBRA_INLINE int getNextPowerOfTwo(int x)
    {
        x--;
        x = (x>>1)|x;
        x = (x>>2)|x;
        x = (x>>4)|x;
        x = (x>>8)|x;
        x = (x>>16)|x;
        return x+1;
    }

    void init(const AABB* AABBArray, int AABBCount, const ObjectGrouperParams& params, const float* objectCostArray)
    {
        if (AABBCount == 0)
            return;

        m_leafNodeArray.setAllocator(m_platformServices.allocator);
        m_tree.setAllocator(m_platformServices.allocator);

        // Compute scene AABB

        AABB sceneAABB(AABBArray[0]);
        for (int I = 0; I < AABBCount; I++)
            sceneAABB.grow(AABBArray[I]);

        AABB worldAABB = sceneAABB;
        if (params.isWorldSizeValid())
        {
            worldAABB.set(Vector3(0,0,0), Vector3(params.worldSizeX, params.worldSizeY, params.worldSizeZ));

            Vector3 worldSize = worldAABB.getDimensions();
            Vector3 sceneSize = sceneAABB.getDimensions();

            if (worldSize.x < sceneSize.x ||
                worldSize.y < sceneSize.y ||
                worldSize.z < sceneSize.z)
            {
                worldAABB = sceneAABB;
                UMBRA_LOG_W(m_platformServices.logger, "Reference world size is smaller than scene. Using scene bounds instead.");
            }
        }

        FCostModel costModel(worldAABB, params.clusterCost);

        AABBDatabase<int> database(m_platformServices.allocator);
        database.init(sceneAABB);

        // Initialize open nodes

        Array<Node> stackAllocator(2*getNextPowerOfTwo(AABBCount), m_platformServices.allocator); // Complete binary tree has at most 2*N nodes

        int stackIndex = 0;
        Node* basePtr = stackAllocator.getPtr();

        int groupID = AABBCount;
        m_leafNodeArray.reset(AABBCount);
        Array<int> openNodes(m_platformServices.allocator);
        openNodes.reset(AABBCount);

        BulkAllocator<FMove, 4096> moveAllocator(m_platformServices.allocator);

        for (int I = 0; I < AABBCount; I++)
        {
            Node* node         = &stackAllocator[stackIndex++];
            float objectCost    = objectCostArray ? objectCostArray[I] : DefaultObjectCost;
            float cost          = costModel.getCost(AABBArray[I], objectCost);

            node->init(AABBArray[I], I, objectCost, cost);
            node->m_handle = database.insert(node->m_aabb, stackIndex-1);

            openNodes[I]        = stackIndex-1;
            m_leafNodeArray[I]  = node;
        }

        // Build tree

        Heap<float, FMove*> heap(m_platformServices.allocator);
        Array<int> tmpArray(m_platformServices.allocator);
        Array<int> neighborArray(m_platformServices.allocator);

        while (openNodes.getSize() > 1)
        {
            for (int I = 0; I < openNodes.getSize(); I++)
            {
                neighborArray.clear();
                Node* node = &stackAllocator[openNodes[I]];
                database.findKNearestNeighbours(MaxNeighbourCount, node->getAABB(), neighborArray);

                for (int J = 0; J < neighborArray.getSize(); J++)
                {
                    if (neighborArray[J] == openNodes[I])
                        continue;
                    FMove* Move = new(moveAllocator.allocate()) FMove(openNodes[I], neighborArray[J]);
                    float cost = costModel.getCost(Move->getAABB(basePtr), Move->getObjectCost(basePtr));
                    heap.insert(cost, Move);
                }
            }

            while (heap.getSize())
            {
                FMove* Move = heap.getValue(0);
                float cost = heap.getKey(0);
                heap.removeFirst();

                if (!Move->isValid(basePtr))
                    continue;

                Node* node = &stackAllocator[stackIndex++];
                node->init(Move->getLeft(basePtr), Move->getRight(basePtr), groupID++, cost);
                openNodes.pushBack(stackIndex-1);

                database.remove(Move->getLeft(basePtr)->m_handle);
                database.remove(Move->getRight(basePtr)->m_handle);

                neighborArray.clear();
                database.findKNearestNeighbours(MaxNeighbourCount, node->getAABB(), neighborArray);
                node->m_handle = database.insert(node->getAABB(), stackIndex-1);

                moveAllocator.release(Move);

                for (int J = 0; J < neighborArray.getSize(); J++)
                {
                    if (stackAllocator[neighborArray[J]].m_parent)
                        continue;
                    FMove* Move = new(moveAllocator.allocate()) FMove(stackIndex-1, neighborArray[J]);
                    float cost = costModel.getCost(Move->getAABB(basePtr), Move->getObjectCost(basePtr));
                    heap.insert(cost, Move);
                }
            }

            // Collect orphans

            tmpArray.clear();
            for (int I = 0; I < openNodes.getSize(); I++)
            {
                if (stackAllocator[openNodes[I]].m_parent)
                    continue;
                tmpArray.pushBack(openNodes[I]);
            }

            openNodes.reset(tmpArray.getSize());
            for (int I = 0; I < tmpArray.getSize(); I++)
                openNodes[I] = tmpArray[I];
        }

        UMBRA_ASSERT(openNodes.getSize() == 1);
        Node* root = &stackAllocator[openNodes[0]];
        buildLinearLayout(root);
    }

    void deinit(void)
    {
    }

    Node* getRoot(void)
    {
        return &m_tree[0];
    }

    int getLeafNodeCount(void) const
    {
        return m_leafNodeArray.getSize();
    }

    Node* getLeafNode(int index)
    {
        return m_leafNodeArray[index];
    }

    int getNodeCount(Node* node)
    {
        if (!node)
            return 0;
        return getNodeCount(node->m_child[0]) + getNodeCount(node->m_child[1]) + 1;
    }

    struct FItem
    {
        FItem(void)
        :   m_node        (NULL)
        ,   m_newParent   (NULL)
        ,   m_flags       (0)
        {
        }

        FItem(Node* node, Node* parent, int flags)
        :   m_node        (node)
        ,   m_newParent   (parent)
        ,   m_flags       (flags)
        {
        }

        Node*   m_node;
        Node*   m_newParent;
        int     m_flags;
    };

    void buildLinearLayout(Node* root)
    {
        enum
        {
            UpdateLeftChild     = 1,
            UpdateRightChild    = 2
        };

        Deque<FItem> queue;
        queue.setAllocator(m_platformServices.allocator);
        queue.addLast(FItem(root, 0, 0));

        m_tree.resize(getNodeCount(root));
        int freeIndex = 0;

        m_leafNodeArray.clear();

        while (queue.getSize())
        {
            FItem item = queue.getFirst();
            queue.removeFirst();

            Node* newNode = &m_tree[freeIndex++];
            newNode->init(item.m_node);
            newNode->m_parent = item.m_newParent;

            if (item.m_flags & UpdateLeftChild)
                newNode->m_parent->m_child[0] = newNode;
            if (item.m_flags & UpdateRightChild)
                newNode->m_parent->m_child[1] = newNode;

            if (item.m_node->isLeaf())
            {
                m_leafNodeArray.pushBack(newNode);
                continue;
            }

            queue.addLast(FItem(item.m_node->m_child[0], newNode, UpdateLeftChild));
            queue.addLast(FItem(item.m_node->m_child[1], newNode, UpdateRightChild));
        }
    }

private:
    PlatformServices      m_platformServices;
    Array<Node*>          m_leafNodeArray;
    Array<Node>           m_tree;
};

//-----------------------------------------------------------------

class GroupOptimizer
{
public:

    enum
    {
        StackSize = 1024
    };

public:

    GroupOptimizer(const PlatformServices& platformServices)
        : m_platformServices(platformServices)
    {
    }

    GroupOptimizer(const PlatformServices& platformServices, Node* root, int objectCount)
        : m_platformServices(platformServices)
    {
        init(root, objectCount);
    }

    float findBottleneck(Node* node)
    {
        UMBRA_ASSERT(node->isLeaf());

        float minCapacity = FLT_MAX;

        while (node->m_parent)
        {
            float capacity = node->getCapacity() - node->getFlow();
            if (capacity < minCapacity)
                minCapacity = capacity;
            node = node->m_parent;
        }

        return minCapacity;
    }

    void addFlow(Node* node, float flow)
    {
        UMBRA_ASSERT(node->isLeaf());

        while (node->m_parent)
        {
            node->m_flow += flow;
            node = node->m_parent;
        }
    }

    void findMinCut(Node* root, Array<Node*>& minCut)
    {
        Node* stack[StackSize];
        int stackPointer = 0;
        stack[stackPointer++] = root;

        const float Epsilon = 0.001f;

        while (stackPointer)
        {
            Node* node = stack[--stackPointer];

            if (node->isLeaf())
            {
                minCut.pushBack(node);
                continue;
            }

            if (fabs(node->getFlow() - node->getCapacity()) < Epsilon &&
                (node->m_child[0]->isLeaf() || node->m_child[1]->isLeaf() ||
                fabs(node->m_child[0]->getFlow() - node->m_child[0]->getCapacity()) > Epsilon ||
                (fabs(node->m_child[1]->getFlow() - node->m_child[1]->getCapacity()) > Epsilon)))
            {
                minCut.pushBack(node);
                continue;
            }

            UMBRA_ASSERT(stackPointer < StackSize - 2);

            stack[stackPointer++] = node->m_child[0];
            stack[stackPointer++] = node->m_child[1];
        }
    }

    void init(Node* root, int objectCount)
    {
        m_groupIDToIndex.setAllocator(m_platformServices.allocator);
        m_groupArray.setAllocator(m_platformServices.allocator);
        m_objectToGroup.setAllocator(m_platformServices.allocator);
        m_objectToGroupRoot.setAllocator(m_platformServices.allocator);

        Node* leaf = NULL;
        while ((leaf = findAugmentingPath(root)))
        {
            float flow = findBottleneck(leaf);
            addFlow(leaf, flow);
        }

        findMinCut(root, m_groupArray);

        // Build object to group mapping

        m_groupIDToIndex.clear();
        m_objectToGroup.reset(objectCount);
        m_objectToGroupRoot.reset(objectCount);

        for (int I = 0; I < m_objectToGroup.getSize(); I++)
        {
            m_objectToGroup[I] = -1;
            m_objectToGroupRoot[I] = NULL;
        }

        Node* stack[StackSize];

        for (int I = 0; I < m_groupArray.getSize(); I++)
        {
            int stackPointer = 0;
            stack[stackPointer++] = m_groupArray[I];
            int groupID = m_groupArray[I]->m_ID;

            m_groupIDToIndex.insert(groupID, I);

            while (stackPointer)
            {
                Node* node = stack[--stackPointer];

                if (node->isLeaf())
                {
                    m_objectToGroup[node->m_ID] = groupID;
                    m_objectToGroupRoot[node->m_ID] = m_groupArray[I];
                    continue;
                }

                UMBRA_ASSERT(stackPointer < StackSize - 2);

                stack[stackPointer++] = node->m_child[0];
                stack[stackPointer++] = node->m_child[1];
            }
        }

        return;
    }

    Node* findAugmentingPath(Node* root)
    {
        if (root->isLeaf())
            return NULL;

        Node* leaf = NULL;
        int stackPointer = 0;
        Node* stack[1024];
        stack[stackPointer++] = root;

        while (stackPointer)
        {
            Node* node = stack[--stackPointer];
            float capacity = node->getCapacity() - node->getFlow();

            if (capacity <= 0.0f)
                continue;

            if (node->isLeaf())
            {
                leaf = node;
                break;
            }

            if (node->m_child[0])
                stack[stackPointer++] = node->m_child[0];

            if (node->m_child[1])
                stack[stackPointer++] = node->m_child[1];
        }

        return leaf;
    }

    int getObjectCount(void) const
    {
        return m_objectToGroup.getSize();
    }

    int getGroupCount(void) const
    {
        return m_groupArray.getSize();
    }

    Node* getGroupRoot(int I)
    {
        return m_groupArray[I];
    }

    int getGroupId(int objectIndex)
    {
        return m_objectToGroup[objectIndex];
    }

    const AABB& getObjectGroupAABB(int objectIndex)
    {
        return m_objectToGroupRoot[objectIndex]->getAABB();
    }

    const AABB& getGroupAABB(int groupId)
    {
        return m_groupArray[*m_groupIDToIndex.get(groupId)]->m_aabb;
    }

private:
    PlatformServices m_platformServices;
    Hash<int, int>   m_groupIDToIndex;
    Array<Node*>     m_groupArray;
    Array<int>       m_objectToGroup;
    Array<Node*>     m_objectToGroupRoot;
};

//-----------------------------------------------------------------

class ImpObjectGrouperInput
{
    struct Object
    {
        Object(void)
        {
        }

        Object(int id, const AABB& aabb, float cost)
        :   m_id            (id)
        ,   m_cost          (cost)
        ,   m_aabb          (aabb)
        {
        }

        int     m_id;
        float   m_cost;
        AABB    m_aabb;
    };

public:

    void init(const PlatformServices& platformServices)
    {
        m_platformServices = platformServices;
        m_objectToLinear.setAllocator(platformServices.allocator);
        m_objectArray.setAllocator(platformServices.allocator);
        m_objectToLinear.clear();
    }

    void add(const Scene& scene)
    {
        if (scene.getObjectCount() == 0)
            return;

        for (int I = 0; I < scene.getObjectCount(); I++)
        {
            const SceneObject* sceneObject = scene.getObject(I);
            const SceneModel* sceneModel = sceneObject->getModel();

            if (sceneModel->getTriangleCount() == 0 || sceneModel->getVertexCount() == 0)
                continue;

            UMBRA_ASSERT(sceneModel->getTriangleCount());
            UMBRA_ASSERT(sceneModel->getVertexCount());

            Matrix4x4 transform;
            sceneObject->getMatrix(transform, MF_ROW_MAJOR);

            Vector3 mn(FLT_MAX,FLT_MAX,FLT_MAX);
            Vector3 mx(-FLT_MAX,-FLT_MAX,-FLT_MAX);

            for (int J = 0; J < sceneModel->getVertexCount(); J++)
            {
                Vector3 v = sceneModel->getVertices()[J];
                Vector4 w = transform.transform(Vector4(v.x,v.y,v.z,1.0f));
                Vector3 u = Vector3(w.x, w.y, w.z);

                if (u.x > mx.x) mx.x = u.x;
                if (u.y > mx.y) mx.y = u.y;
                if (u.z > mx.z) mx.z = u.z;
                if (u.x < mn.x) mn.x = u.x;
                if (u.y < mn.y) mn.y = u.y;
                if (u.z < mn.z) mn.z = u.z;
            }

            float cost = ImpScene::getImplementation(sceneModel)->getCost();
            add(sceneObject->getID(), AABB(mn, mx), cost);
        }
    }

    void add(UINT32 objectId, const AABB& bounds, float objectCost)
    {
        m_objectArray.pushBack(Object(objectId, bounds, objectCost));

        UMBRA_ASSERT(!m_objectToLinear.contains(objectId));

        int linearIndex = m_objectArray.getSize() - 1;
        m_objectToLinear.insert(objectId, linearIndex);

        m_aabb.grow(bounds);
    }

    void deinit(void)
    {
        m_objectArray.clear();
        m_objectToLinear.clear();
    }

    int getObjectID(int index) const
    {
        return m_objectArray[index].m_id;
    }

    const Hash<int,int>& getLinearIndexMap(void) const
    {
        return m_objectToLinear;
    }

    int getLinearIndex(int objectIndex) const
    {
        // Safeguard against objects with zero polygons (Umbra::Scene allows this)

        if (!m_objectToLinear.contains(objectIndex))
            return 0;

        UMBRA_ASSERT(m_objectToLinear.contains(objectIndex));
        return *m_objectToLinear.get(objectIndex);
    }

    int getObjectCount(void) const
    {
        return m_objectArray.getSize();
    }

    const AABB& getAABB(void) const
    {
        return m_aabb;
    }

    const AABB& getObjectAABB(int index) const
    {
        return m_objectArray[index].m_aabb;
    }

    float getObjectCost(int index) const
    {
        return m_objectArray[index].m_cost;
    }

    const PlatformServices& getPlatformServices(void) const
    {
        return m_platformServices;
    }

private:

    Array<Object>       m_objectArray;
    AABB                m_aabb;
    Hash<int,int>       m_objectToLinear;
    PlatformServices    m_platformServices;
};

//-----------------------------------------------------------------

class LinearIndex
{
public:

    ~LinearIndex(void)
    {
        deinit();
    }

    void init(const PlatformServices& services, const ImpObjectGrouperInput& input, GroupOptimizer& groupOptimizer)
    {
        m_objectToGroupArray.setAllocator(services.allocator);
        m_groupAABBArray.setAllocator(services.allocator);
        m_objectToLinear.setAllocator(services.allocator);

        Hash<int, int> IDMap(services.allocator);
        int linearID = 0;

        m_objectToGroupArray.reset(groupOptimizer.getObjectCount());
        m_groupAABBArray.reset(groupOptimizer.getGroupCount());

        for (int I = 0; I < m_objectToGroupArray.getSize(); I++)
        {
            int groupID = groupOptimizer.getGroupId(I);

            int ID = 0;
            if (IDMap.contains(groupID))
            {
                ID = *IDMap.get(groupID);
            }
            else
            {
                IDMap.insert(groupID, linearID);
                ID = linearID;
                linearID++;
            }

            m_objectToGroupArray[I]  = ID;
            m_groupAABBArray[ID] = groupOptimizer.getGroupAABB(groupID);
        }

        m_objectToLinear = input.getLinearIndexMap();
        //m_groupAABBArray.resize(linearID);
    }

    void deinit(void)
    {
        m_objectToGroupArray.clear();
        m_groupAABBArray.clear();
    }

    int getGroupIndex(UINT32 objectIndex) const
    {
        int linearIndex = 0;
        if (m_objectToLinear.contains(objectIndex))
            linearIndex = *m_objectToLinear.get(objectIndex);

        if (m_objectToGroupArray.getSize() <= linearIndex)
            return -1;

        return m_objectToGroupArray[linearIndex];
    }

    int getGroupCount(void) const
    {
        return m_groupAABBArray.getSize();
    }

    void getGroupAABB(Vector3& outMin, Vector3& outMax, int groupIndex) const
    {
        outMin = m_groupAABBArray[groupIndex].getMin();
        outMax = m_groupAABBArray[groupIndex].getMax();
    }

private:

    Array<int>          m_objectToGroupArray;
    Array<AABB>         m_groupAABBArray;
    Hash<int,int>       m_objectToLinear;
};

//------------------------------------------------------------------------

class ImpObjectGrouper
{
public:

    ImpObjectGrouper(void)
    {
    }

    ImpObjectGrouper(const PlatformServices& platformServices, const ObjectGrouperInput& input, const ObjectGrouperParams& params)
    {
        init(platformServices, input, params);
    }

    ~ImpObjectGrouper(void)
    {
        deinit();
    }

    void init(const PlatformServices& platformServices, const ObjectGrouperInput& inInput, const ObjectGrouperParams& params)
    {
        deinit();

        m_platformServices = platformServices;
        ImpObjectGrouperInput& input = *inInput.m_imp;

        if (input.getObjectCount() <= 0)
            return;

        Array<AABB> AABBArray(m_platformServices.allocator);
        Array<float> objectCostArray(m_platformServices.allocator);

        AABBArray.reset(input.getObjectCount());
        objectCostArray.reset(input.getObjectCount());

        for (int I = 0; I < input.getObjectCount(); I++)
        {
            AABBArray[I] = input.getObjectAABB(I);
            objectCostArray[I] = input.getObjectCost(I);
        }

        Timer timer(m_platformServices.allocator);

        timer.resetTimer("TreeBuilder");
        timer.startTimer("TreeBuilder");
        TreeBuilder treeBuilder(m_platformServices, AABBArray.getPtr(), AABBArray.getSize(), params, objectCostArray.getPtr());
        timer.stopTimer("TreeBuilder");
        UMBRA_LOG_I(m_platformServices.logger, "Built object group tree in %.3f ms\n", float(1000.0*timer.getTimerValue("TreeBuilder")));

        timer.resetTimer("GroupOptimizer");
        timer.startTimer("GroupOptimizer");
        GroupOptimizer groupOptimizer(m_platformServices, treeBuilder.getRoot(), input.getObjectCount());
        timer.stopTimer("GroupOptimizer");
        UMBRA_LOG_I(m_platformServices.logger, "Optimized object groups in %.3f ms\n", float(1000.0*timer.getTimerValue("GroupOptimizer")));
        UMBRA_LOG_I(m_platformServices.logger, "Grouped %d objects into %d groups\n", input.getObjectCount(), groupOptimizer.getGroupCount());

        m_linearIndex.init(platformServices, input, groupOptimizer);
    }

    void deinit(void)
    {
        m_linearIndex.deinit();
    }

    UMBRA_FORCE_INLINE int getGroupIndex(UINT32 objectID) const
    {
        return m_linearIndex.getGroupIndex(objectID);
    }

    UMBRA_FORCE_INLINE int getGroupCount(void) const
    {
        return m_linearIndex.getGroupCount();
    }

    UMBRA_FORCE_INLINE void getGroupAABB(Vector3& outMin, Vector3& outMax, int groupIndex) const
    {
        m_linearIndex.getGroupAABB(outMin, outMax, groupIndex);
    }

    PlatformServices& getPlatformServices(void)
    {
        return m_platformServices;
    }

private:

    PlatformServices    m_platformServices;
    LinearIndex         m_linearIndex;
};

//------------------------------------------------------------------------

ObjectGrouperInput::ObjectGrouperInput(void)
:   m_imp   (NULL)
{
}

//------------------------------------------------------------------------

ObjectGrouperInput::ObjectGrouperInput(const PlatformServices& platformServices, const Scene& scene)
:   m_imp (NULL)
{
    init(platformServices);
    add(scene);
}

//------------------------------------------------------------------------

ObjectGrouperInput::ObjectGrouperInput(const PlatformServices& platformServices)
:   m_imp (NULL)
{
    init(platformServices);
}

//------------------------------------------------------------------------

ObjectGrouperInput::~ObjectGrouperInput(void)
{
    deinit();
}

//------------------------------------------------------------------------

void ObjectGrouperInput::init(const PlatformServices& platformServices)
{
    deinit();

    PlatformServices services = platformServices;

    if (!services.allocator)
        services.allocator = getAllocator();

    m_imp = UMBRA_HEAP_NEW(services.allocator, ImpObjectGrouperInput);
    m_imp->init(services);
}

//------------------------------------------------------------------------

void ObjectGrouperInput::add(const Scene& scene)
{
    m_imp->add(scene);
}

//------------------------------------------------------------------------

void ObjectGrouperInput::add(UINT32 objectId, const Vector3& mn, const Vector3& mx, float objectCost)
{
    m_imp->add(objectId, AABB(mn,mx), objectCost);
}

//------------------------------------------------------------------------

int ObjectGrouperInput::getObjectCount(void) const
{
    return m_imp->getObjectCount();
}

//------------------------------------------------------------------------

Umbra::UINT32 ObjectGrouperInput::getObjectId(int idx) const
{
    return m_imp->getObjectID(idx);
}

//------------------------------------------------------------------------

void ObjectGrouperInput::deinit(void)
{
    if (!m_imp)
        return;
    m_imp->deinit();
    UMBRA_HEAP_DELETE(m_imp->getPlatformServices().allocator, m_imp);
    m_imp = NULL;
}

//-----------------------------------------------------------------

ObjectGrouper::ObjectGrouper(void)
:   m_imp   (NULL)
{
}

//------------------------------------------------------------------------

ObjectGrouper::ObjectGrouper(
    const PlatformServices&     platformServices,
    const ObjectGrouperInput&   input,
    const ObjectGrouperParams&  params)
:   m_imp   (NULL)
{
    init(platformServices, input, params);
}


//------------------------------------------------------------------------

ObjectGrouper::~ObjectGrouper(void)
{
    deinit();
}

//------------------------------------------------------------------------

void ObjectGrouper::init(
    const PlatformServices&     platformServices,
    const ObjectGrouperInput&   input,
    const ObjectGrouperParams&  params)
{
    deinit();

    PlatformServices services = platformServices;
    if (!services.allocator)
        services.allocator = getAllocator();

    m_imp = UMBRA_HEAP_NEW(services.allocator, ImpObjectGrouper);
    m_imp->init(services, input, params);
}

//------------------------------------------------------------------------

void ObjectGrouper::deinit(void)
{
    if (!m_imp)
        return;
    UMBRA_HEAP_DELETE(m_imp->getPlatformServices().allocator, m_imp);
    m_imp = NULL;
}

//------------------------------------------------------------------------

int ObjectGrouper::getGroupIndex(UINT32 objectID) const
{
    if (!m_imp)
        return -1;
    return m_imp->getGroupIndex(objectID);
}

//------------------------------------------------------------------------

int ObjectGrouper::getGroupCount(void) const
{
    if (!m_imp)
        return 0;
    return m_imp->getGroupCount();
}

//------------------------------------------------------------------------

void ObjectGrouper::getGroupAABB(
    Vector3&    outMin,
    Vector3&    outMax,
    int         groupIndex) const
{
    if (!m_imp)
        return;
    return m_imp->getGroupAABB(outMin, outMax, groupIndex);
}

//------------------------------------------------------------------------

bool ObjectGrouperParams::isWorldSizeValid(void) const
{
    return (worldSizeX > 0.0f && worldSizeX < FLT_MAX &&
            worldSizeY > 0.0f && worldSizeY < FLT_MAX &&
            worldSizeZ > 0.0f && worldSizeZ < FLT_MAX);
}

//------------------------------------------------------------------------

} // namespace Umbra
