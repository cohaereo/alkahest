#include "umbraSubdivisionTree.hpp"

using namespace Umbra;

void SubdivisionTree::setRoot(Node* node)
{
    m_root = node;
}

SubdivisionTree::LeafNode* SubdivisionTree::newLeaf()
{
    LeafNode* leaf = UMBRA_HEAP_NEW(&m_leaves, LeafNode);
    memset(leaf, 0, sizeof(*leaf));
    leaf->m_data = (int)LEAF;
    return leaf;
}

SubdivisionTree::MedianNode* SubdivisionTree::newMedian()
{
    MedianNode* median = UMBRA_HEAP_NEW(&m_medians, MedianNode);
    memset(median, 0, sizeof(*median));
    median->m_data = (int)MEDIAN;
    median->m_right = 0;
    return median;
}

SubdivisionTree::AxialNode* SubdivisionTree::newAxial()
{
    AxialNode* axial = UMBRA_HEAP_NEW(&m_axials, AxialNode);
    memset(axial, 0, sizeof(*axial));
    axial->m_data = (int)AXIAL;
    axial->m_right = 0;
    axial->m_pos = 0.f;
    return axial;
}

SubdivisionTree::PlaneNode* SubdivisionTree::newPlane()
{
    PlaneNode* plane = UMBRA_HEAP_NEW(&m_planes, PlaneNode);
    memset(plane, 0, sizeof(*plane));
    plane->m_data = (int)PLANE;
    plane->m_right = 0;
    plane->m_pleq = Vector4();
    return plane;
}

void SubdivisionTree::deleteNode(Node* node)
{
    switch (node->getType())
    {
    case SubdivisionTree::LEAF:
        m_leaves.deallocate(node);
        break;
    case SubdivisionTree::MEDIAN:
        m_medians.deallocate(node);
        break;
    case SubdivisionTree::AXIAL:
        m_axials.deallocate(node);
        break;
    case SubdivisionTree::PLANE:
        m_planes.deallocate(node);
        break;
    default:
        UMBRA_ASSERT(0);
    }
}

void SubdivisionTree::deleteTree(Node* node)
{
    if (node->isInner())
    {
        deleteTree(node->getInner()->getLeft());
        deleteTree(node->getInner()->getRight());
    }
    deleteNode(node);
}
