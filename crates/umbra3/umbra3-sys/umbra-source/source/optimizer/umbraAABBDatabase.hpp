// Copyright (c) 2010-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRAAABBDATABASE_HPP
#define UMBRAAABBDATABASE_HPP

#include "umbraAABB.hpp" 
#include "umbraArray.hpp" 
#include "umbraList.hpp" 
#include "umbraBulkAllocator.hpp" 


namespace Umbra
{ 

//------------------------------------------------------------------------

template <class NodeDataType> 
class AABBDatabase
{
public: 

    enum 
    {
        MaxTreeDepth    = 32,
        StackSize       = 1024, 
        MaxNeighbours   = 32
    }; 

public: 

    struct NodeItem
    {
        UMBRA_INLINE NodeItem(void) 
        {
        } 

        UMBRA_INLINE NodeItem(const AABB& aabb, NodeDataType nodeData) 
        :   m_aabb        (aabb)
        ,   m_nodeData    (nodeData)
        {
        } 

        AABB            m_aabb; 
        NodeDataType    m_nodeData; 
    }; 

    //------------------------------------------------------------------------

    struct Node; 
    struct ChildBlock
    {
        UMBRA_INLINE ChildBlock(void)
        {
        }

        UMBRA_INLINE ChildBlock(const AABB& aabb, float halfSize) 
        {
            for (int I = 0; I < 8; I++) 
            {
                Vector3 mn = aabb.getMin(); 

                if (I&1) mn[0] += halfSize; 
                if (I&2) mn[1] += halfSize; 
                if (I&4) mn[2] += halfSize; 

                Vector3 mx = mn + Vector3(halfSize, halfSize, halfSize); 

                m_child[I] = Node(AABB(mn,mx)); 
            } 
        } 

        Node m_child[8]; 
    }; 

    //------------------------------------------------------------------------

    struct Node
    {
        UMBRA_INLINE Node(void) 
        {
        } 

        UMBRA_INLINE Node(const AABB& aabb)
        :   m_aabb            (aabb) 
        ,   m_nodeItemList    (NULL)
        ,   m_childBlock      (NULL)
        {
        }

        UMBRA_INLINE float getSize(void) const   
        { 
            return m_aabb.getAxisLength(0); 
        } 

        UMBRA_INLINE bool isLeaf(void) const   
        { 
            return m_childBlock == NULL;
        } 

        UMBRA_INLINE Node* getChild(int index) const 
        {
            UMBRA_ASSERT(m_childBlock); 
            return &m_childBlock->m_child[index]; 
        } 

        AABB            m_aabb; 
        List<NodeItem>* m_nodeItemList; 
        ChildBlock*     m_childBlock; 
    }; 

    //------------------------------------------------------------------------

    struct Handle
    {
        UMBRA_INLINE Handle(void)
        :   m_node        (NULL)
        ,   m_nodeItem    (NULL) 
        {
        } 

        UMBRA_INLINE Handle(Node* node, List<NodeItem>* nodeItem) 
        :   m_node      (node)
        ,   m_nodeItem  (nodeItem)
        {
        } 

        Node*               m_node; 
        List<NodeItem>*     m_nodeItem; 
    }; 

    //------------------------------------------------------------------------

    class WorkingSet
    {
    private: 

        struct Entry 
        {
            UMBRA_INLINE Entry(void) 
            {
            } 

            UMBRA_INLINE Entry(const NodeItem& nodeItem, float cost) 
            :   m_nodeItem  (nodeItem)
            ,   m_cost      (cost)
            {
            }

            NodeItem    m_nodeItem; 
            float       m_cost; 
        }; 

    public: 

        WorkingSet(int maxSize) 
        {
            init(maxSize);
        } 

        void init(int maxSize) 
        {
            UMBRA_ASSERT(maxSize <= MaxNeighbours); 

            m_maxSize = maxSize;
            m_size = 0; 
            for (int I = MaxNeighbours-maxSize; I < MaxNeighbours; I++) 
                m_entryArray[I].m_cost = FLT_MAX; 
        } 

        UMBRA_INLINE float getMaxCost() const 
        {
            int index = MaxNeighbours-m_maxSize; 
            return m_entryArray[index].m_cost; 
        }

        UMBRA_INLINE void add(const NodeItem& nodeItem, float cost) 
        {
            if (cost > getMaxCost()) 
                return; 

            int index = MaxNeighbours-m_maxSize; 
            m_entryArray[index] = Entry(nodeItem, cost); 
            if (m_size < m_maxSize) 
                m_size++; 

            while (index < MaxNeighbours - 1 && cost < m_entryArray[index+1].m_cost) 
            {
                swap(m_entryArray[index], m_entryArray[index+1]); 
                index++; 
            } 
        } 

        UMBRA_INLINE int getSize(void) const 
        {
            return m_size; 
        } 

        UMBRA_INLINE const NodeItem& getNodeItem(int index) const 
        {
            return m_entryArray[MaxNeighbours - index - 1].m_nodeItem; 
        } 

    private: 
        int     m_size; 
        int     m_maxSize; 
        Entry   m_entryArray[MaxNeighbours]; 
    };  
    
    //------------------------------------------------------------------------

public: 

    AABBDatabase(Allocator* allocator = NULL) 
    :   m_nodeAllocator         (allocator)
    ,   m_listAllocator         (allocator)
    ,   m_childBlockAllocator   (allocator)
    ,   m_allocator             (allocator)
    {
        if (!m_allocator)
            m_allocator = getAllocator(); 
    } 

    ~AABBDatabase(void) 
    { 
        deinit(); 
    } 

    void init(AABB aabb) 
    {
        float maxAxisLen    = aabb.getMaxAxisLength(); 
        int maxAxis         = aabb.getMaxAxis(); 

        for (int I = 0; I < 3; I++) 
        {
            if (I == maxAxis) 
                continue; 

            Vector3 V(aabb.getMin());  
            V[I] += maxAxisLen; 

            aabb.grow(V); 
        } 

        m_root = Node(aabb); 
    } 

    void deinit()
    {
        m_nodeAllocator.releaseAll(); 
        m_listAllocator.releaseAll(); 
        m_root.m_childBlock = NULL;  
    } 

    Handle insert(const AABB& aabb, NodeDataType nodeData) 
    {
        Vector3 center  = aabb.getCenter(); 
        float radius    = aabb.getDiagonalLength()*0.5f; 
        return insert(&m_root, aabb, nodeData, center, radius, 0);
    }

    void remove(Handle handle) 
    {
        handle.m_node->m_nodeItemList = unlink(handle.m_node->m_nodeItemList, handle.m_nodeItem); 
        m_listAllocator.release(handle.m_nodeItem); 
    } 

    const AABB& getAABB(void) const 
    {
        return m_root.aabb; 
    } 

    struct FChild
    {
        FChild(void) 
        {
        } 

        FChild(Node* node, float cost) 
        :   m_node    (node)
        ,   m_cost    (cost) 
        {
        } 

        Node*   m_node; 
        float   m_cost; 
    }; 

    void findKNearestNeighbours(int K, const AABB& aabb, Array<NodeDataType>& result) 
    {
        WorkingSet workingSet(K); 
        Node* stack[StackSize]; 

        int stackPointer        = 0; 
        stack[stackPointer++]   = &m_root; 

        while (stackPointer)
        {
            Node* node = stack[--stackPointer]; 

            List<NodeItem>* list = node->m_nodeItemList; 
            while (list) 
            { 
                float cost = AABB(aabb, list->Get().m_aabb).getSurfaceArea(); 
                workingSet.add(list->Get(), cost); 
                list = list->m_next; 
            } 

            if (node->isLeaf())
                continue; 

            FChild childArray[8]; 
            int childCount = 0; 

            for (int I = 0; I < 8; I++) 
            {
                // Prune subtree if we can't get better result 

                AABB minBound = getMinBounds(aabb, node->getChild(I)->m_aabb); 
                float minCost = minBound.getSurfaceArea(); 

                if (minCost > workingSet.getMaxCost()) 
                    continue; 

                // Sort children based on min cost 

                int J = childCount; 
                childArray[childCount++] = FChild(node->getChild(I), minCost); 

                while (J >= 1 && minCost > childArray[J-1].m_cost) 
                {
                    swap(childArray[J], childArray[J-1]); 
                    J--; 
                } 
            } 

            for (int I = 0; I < childCount; I++) 
                stack[stackPointer++] = childArray[I].m_node; 
        }
         
        for (int I = 0; I < workingSet.getSize(); I++)
            result.pushBack(workingSet.getNodeItem(I).m_nodeData); 
    } 

    void findAABBsIntersectingAABB(Array<AABB>& result, const AABB& aabb)
    {
        Node* stack[StackSize]; 
        int stackPointer        = 0; 
        stack[stackPointer++]   = m_root; 

        while (stackPointer) 
        {
            Node* node = stack[--stackPointer]; 

            if (aabb.contains(node->aabb))
            {
                collectSubtree(result, node); 
                continue; 
            } 

            List<AABB>* list = node->AABBList; 
            while (list) 
            { 
                if (aabb.intersects(list->Get()))
                    result.pushBack(list->Get()); 
                list = list->m_next; 
            } 

            if (node->isLeaf())
                continue; 

            for (int I = 0; I < 8; I++) 
            {
                float distanceSqr = aabb.getDistanceSqr(node->getChild(I)->aabb); 
                float thresholdSqr = 0.25f*node->getChild(I)->getSize()*node->getChild(I)->getSize(); 

                if (distanceSqr < thresholdSqr)
                    stack[stackPointer++] = node->getChild(I); 
            } 
        }  
    } 

    void optimize()
    {
        optimize(&m_root); 
    } 

private: 

    Handle insert(Node* node, const AABB& aabb, NodeDataType nodeData, const Vector3& center, float radius, int depth)
    {
        UMBRA_ASSERT(node); 
        UMBRA_ASSERT(node->m_aabb.contains(center)); 

        float halfSize = node->getSize() * 0.5f; 
        if ((depth >= MaxTreeDepth) || (radius >= halfSize))
        {
            List<NodeItem>* nodeItem = new (m_listAllocator.allocate()) List<NodeItem>(NodeItem(aabb, nodeData)); 
            node->m_nodeItemList = link(node->m_nodeItemList, nodeItem); 
            return Handle(node, nodeItem); 
        } 

        if (node->isLeaf()) 
            node->m_childBlock = new (m_childBlockAllocator.allocate()) ChildBlock(node->m_aabb, halfSize); 

        for (int I = 0; I < 8; I++) 
        {
            if (node->getChild(I)->m_aabb.contains(center))
                return insert(node->getChild(I), aabb, nodeData, center, radius, depth + 1);
        } 

        // Safeguard against floating point errors 

        List<NodeItem>* nodeItem = new (m_listAllocator.allocate()) List<NodeItem>(NodeItem(aabb, nodeData)); 
        node->m_nodeItemList = link(node->m_nodeItemList, nodeItem); 
        return Handle(node, nodeItem); 
    } 

    AABB getMinBounds(const AABB& source, const AABB& target) 
    {
        Vector3 mn = source.getMin();
        Vector3 mx = source.getMax();  

        for (int I = 0; I < 3; I++)
        {
            if (source.getMax()[I] < target.getMin()[I])
                mx[I] = target.getMin()[I]; 
            if (target.getMax()[I] < source.getMin()[I]) 
                mn[I] = target.getMax()[I]; 
        } 

        return AABB(mn, mx); 
    } 

    AABB getMaxBounds(const AABB& source, const AABB& target) 
    {
        float halfSize = 0.5f*target.getAxisLength(0); 
        Vector3 extrusion(halfSize, halfSize, halfSize); 
        AABB extrudedTargetAABB(target.getMin() - extrusion, target.getMax() + extrusion); 
        return AABB(source, extrudedTargetAABB); 
    } 

    void collectSubtree(Array<NodeDataType>& result, Node* node) 
    {
        Node* stack[StackSize]; 
        int stackPointer        = 0; 
        stack[stackPointer++]   = node; 

        while (stackPointer)
        {
            Node* node = stack[--stackPointer]; 

            List<NodeItem>* list = node->m_nodeItemList; 
            while (list) 
            { 
                result.add(list->Get().m_nodeData); 
                list = list->Next; 
            } 

            for (int I = 0; I < 8; I++) 
            {
                stack[stackPointer++] = node->getChild(I); 
            } 
        }
    } 

    bool collapseChildren(Node* node) 
    {
        // Check that all children all empty leaves 


        for (int I = 0; I < 8; I++) 
        {
            if ((!node->getChild(I)->isLeaf() || node->getChild(I)->m_nodeItemList))
                return false; 
        } 

        m_childBlockAllocator.release(node->m_childBlock); 
        node->m_childBlock = NULL; 

        return true; 
    } 

    void optimize(Node* node) 
    {
        if (!node) 
            return;
        
        if (node->isLeaf()) 
            return; 

        if (collapseChildren(node)) 
            return; 

        for (int I = 0; I < 8; I++) 
            optimize(node->getChild(I)); 
    } 

private: 

    Node                                m_root; 
    BulkAllocator<Node, 4096>           m_nodeAllocator; 
    BulkAllocator<List<NodeItem>, 4096> m_listAllocator; 
    BulkAllocator<ChildBlock, 4096>     m_childBlockAllocator; 
    Allocator*                          m_allocator; 
}; 

//------------------------------------------------------------------------

} // namespace Umbra

#endif // UMBRAAABBDATABASE_HPP

