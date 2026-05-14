#include "umbraRectGrouper.hpp"
#include "umbraVoxelTree.hpp"
#include "umbraSort.hpp"
#include "umbraSet.hpp"

using namespace Umbra;

RectGrouper::RectGrouper(Allocator* a, float th) : m_allocator(a), m_strategy(FOUR_QUADRANTS), m_threshold(th), m_gridSize(0), m_rects(a), m_best(a)
{
    if (m_threshold <= 0.f)
        m_threshold = 0.0001f;
}

struct RectSorter
{
    Vector4 r;

    bool operator<(const RectSorter& b) const
    {
        return memcmp(&r.x, &b.r.x, sizeof(r)) < 0;
    }
    bool operator>(const RectSorter& b) const
    {
        return memcmp(&r.x, &b.r.x, sizeof(r)) > 0;
    }
};

void RectGrouper::execute()
{
    UMBRA_ASSERT(m_rects.getSize() > 0);
    UMBRA_ASSERT(m_best.rects.getSize() == 0);

    if (m_strategy == COMBINE_ALL)
    {
        Vector4 rect = m_rects[0];

        for (int i = 1; i < m_rects.getSize(); i++)
            rect = rectUnion(rect, m_rects[i]);

        m_best.rects.pushBack(rect);
    }
    else if (m_strategy == FOUR_QUADRANTS)
    {
        // Four quadrants splitter is rather slow, so do grid grouping if rect count is huge.

        if (m_rects.getSize() > 128)
        {
            Array<Vector4> out(m_allocator);
            gridGrouper(out, m_rects, 8);
            m_rects = out;
        }

        Array<RectSorter> rects(m_allocator);
        rects.resize(m_rects.getSize());

        for (int i = 0; i < m_rects.getSize(); i++)
        {
            RectSorter rs;
            rs.r = m_rects[i];
            rects[i] = rs;
        }

        quickSort(rects.getPtr(), rects.getSize());

        for (int i = 0; i < rects.getSize(); i++)
            m_rects[i] = rects[i].r;

        m_best.rects.pushBack(unionRects(m_rects));
        UMBRA_ASSERT(m_threshold > 0.f);
        fourQuadrants(m_rects, m_best);
    }
    else if (m_strategy == GRID_GROUPER)
    {
        UMBRA_ASSERT(m_gridSize > 0);
        gridGrouper(m_best.rects, m_rects, m_gridSize);
    }
    else
    {
        UMBRA_ASSERT(m_strategy == COMBINE_NOTHING);

        m_best.rects = m_rects;
    }
}

void RectGrouper::tryGroupSolution(Solution& s)
{
    Solution orig(m_allocator);
    orig = s;

    for (int i = 0; i < orig.rects.getSize(); i++)
        for (int j = i+1; j < orig.rects.getSize(); j++)
        {
            Solution candidate(getAllocator());

            Vector4 r = rectUnion(orig.rects[i], orig.rects[j]);
            candidate.rects.pushBack(r);

            for (int k = 0; k < orig.rects.getSize(); k++)
                if (k != i && k != j)
                    candidate.rects.pushBack(orig.rects[k]);

            if (isSolutionBetter(candidate, s))
                s = candidate;
        }
}

void RectGrouper::fourQuadrants(const Array<Vector4>& rects, Solution& solution)
{
    Vector4 full = rects[0];
    for (int i = 1; i < rects.getSize(); i++)
        full = rectUnion(full, rects[i]);

    Set<Vector2> samplePositions(m_allocator);

    for (int i = 0; i < rects.getSize(); i++)
    {
        const Vector4& rect = rects[i];

        samplePositions.insert(Vector2(minX(rect), minY(rect)));
        samplePositions.insert(Vector2(maxX(rect), minY(rect)));
        samplePositions.insert(Vector2(minX(rect), maxY(rect)));
        samplePositions.insert(Vector2(maxX(rect), maxY(rect)));
    }

    Array<Vector4> left(m_allocator);
    Array<Vector4> right(m_allocator);

    Array<Vector4> leftBottom(m_allocator);
    Array<Vector4> rightBottom(m_allocator);
    Array<Vector4> leftTop(m_allocator);
    Array<Vector4> rightTop(m_allocator);

    Solution s(m_allocator);

    Set<Vector2>::Iterator iter = samplePositions.iterate();
    while (iter.next())
    {
        left.resize(0);
        right.resize(0);
        leftBottom.resize(0);
        rightBottom.resize(0);
        leftTop.resize(0);
        rightTop.resize(0);
        s.rects.resize(0);

        Vector2 p = iter.getValue();

        for (int i = 0; i < rects.getSize(); i++)
            splitX(rects[i], p.x, left, right);

        for (int i =0 ; i < left.getSize(); i++)
            splitY(left[i], p.y, leftBottom, leftTop);

        for (int i =0 ; i < right.getSize(); i++)
            splitY(right[i], p.y, rightBottom, rightTop);

        if (leftBottom.getSize())
            s.rects.pushBack(unionRects(leftBottom));
        if (rightBottom.getSize())
            s.rects.pushBack(unionRects(rightBottom));
        if (leftTop.getSize())
            s.rects.pushBack(unionRects(leftTop));
        if (rightTop.getSize())
            s.rects.pushBack(unionRects(rightTop));

        if (isSolutionBetter(s, solution))
            solution = s;
    }
}

void RectGrouper::gridGrouper(Array<Vector4>& out, const Array<Vector4>& in, int size)
{
    Vector4 rect = in[0];
    for (int i = 1; i < in.getSize(); i++)
        rect = rectUnion(rect, in[i]);

    Array<Vector4> grid(size*size, m_allocator);

    for (int i = 0; i < grid.getSize(); i++)
        grid[i] = rectInvalid();

    float w = rect.z - rect.x;
    float h = rect.w - rect.y;

    for (int i = 0; i < in.getSize(); i++)
    {
        int x0 = (int)floorf((in[i].x - rect.x) * size / w);
        int y0 = (int)floorf((in[i].y - rect.y) * size / h);
        int x1 = (int)ceilf((in[i].z - rect.x) * size / w);
        int y1 = (int)ceilf((in[i].w - rect.y) * size / h);

        for (int y = y0; y < y1; y++)
            for (int x = x0; x < x1; x++)
            {
                Vector4 gr;
                gr.x = rect.x + x * w / size;
                gr.y = rect.y + y * h / size;
                gr.z = rect.x + (x+1) * w / size;
                gr.w = rect.y + (y+1) * h / size;

                Vector4 g = rectIntersection(gr, in[i]);
                if (rectIsValid(g))
                    grid[y*size+x] = rectUnion(grid[y*size+x], g);
            }
    }

    for (int y = 0; y < size; y++)
        for (int x = 0; x < size; x++)
        {
            if (!rectIsValid(grid[y*size+x]))
                continue;

            Vector4 gr;
            gr.x = rect.x + x * w / size;
            gr.y = rect.y + y * h / size;
            gr.z = rect.x + (x+1) * w / size;
            gr.w = rect.y + (y+1) * h / size;

            UMBRA_ASSERT(rectUnion(gr, grid[y*size+x]) == gr);
        }

    for (int i = 0; i < grid.getSize(); i++)
        if (rectIsValid(grid[i]))
            out.pushBack(grid[i]);
}

Vector4 RectGrouper::unionRects(const Array<Vector4>& rects)
{
    UMBRA_ASSERT(rects.getSize());

    Vector4 full = rects[0];

    for (int i = 1; i < rects.getSize(); i++)
        full = rectUnion(full, rects[i]);

    return full;
}

bool RectGrouper::splitRectX(const Vector4& r, float x, Vector4& left, Vector4& right)
{
    if (x <= minX(r) || x >= maxX(r))
        return false;

    left = r;
    maxX(left) = x;
    right = r;
    minX(right) = x;

    return true;
}

bool RectGrouper::splitRectY(const Vector4& r, float y, Vector4& left, Vector4& right)
{
    if (y <= minY(r) || y >= maxY(r))
        return false;

    left = r;
    maxY(left) = y;
    right = r;
    minY(right) = y;

    return true;
}

void RectGrouper::splitX(const Vector4& r, float x, Array<Vector4>& left, Array<Vector4>& right)
{
    if (maxX(r) <= x)
        left.pushBack(r);
    else if (minX(r) >= x)
        right.pushBack(r);
    else
    {
        Vector4 rl = r;
        maxX(rl) = x;
        left.pushBack(rl);

        Vector4 rr = r;
        minX(rl) = x;
        right.pushBack(rr);
    }
}

void RectGrouper::splitY(const Vector4& r, float y, Array<Vector4>& left, Array<Vector4>& right)
{
    if (maxY(r) <= y)
        left.pushBack(r);
    else if (minY(r) >= y)
        right.pushBack(r);
    else
    {
        Vector4 rl = r;
        maxY(rl) = y;
        left.pushBack(rl);

        Vector4 rr = r;
        minY(rl) = y;
        right.pushBack(rr);
    }
}

float RectGrouper::getArea(const Array<Vector4>& rects, const Vector4& rect, int* indices, int n2)
{
    // Filter intersecting triangles.

    int n = 0;
    for (int i = 0; i < n2; i++)
        if (rectIntersects(rects[indices[i]], rect))
            swap2(indices[n++], indices[i]);

    if (n == 0)
        return 0.f;

    // Check if the whole area is filled.

    int j;
    for (j = 0; j < n; j++)
        if (rectIntersection(rects[indices[j]], rect) != rect)
            break;
    if (j == n)
        return rectArea(rect);

    // Try split.

    for (int i = 0; i < n; i++)
    {
        Vector4 rl, rr;

        if (splitRectX(rect, minX(rects[indices[i]]), rl, rr))
            return getArea(rects, rl, indices, n) +
                   getArea(rects, rr, indices, n);

        if (splitRectX(rect, maxX(rects[indices[i]]), rl, rr))
            return getArea(rects, rl, indices, n) +
                   getArea(rects, rr, indices, n);

        if (splitRectY(rect, minY(rects[indices[i]]), rl, rr))
            return getArea(rects, rl, indices, n) +
                   getArea(rects, rr, indices, n);

        if (splitRectY(rect, maxY(rects[indices[i]]), rl, rr))
            return getArea(rects, rl, indices, n) +
                   getArea(rects, rr, indices, n);
    }

    UMBRA_ASSERT(0);
    return rectArea(rect);
}

float RectGrouper::getSolutionArea(const Solution& s)
{
    Vector4 full = s.rects[0];

    float area = 0.f;
    for (int i = 0; i < s.rects.getSize(); i++)
    {
        full = rectUnion(full, s.rects[i]);
        area += rectArea(s.rects[i]);
    }

#if 1

    return area;

#else

    Array<int> indices(m_allocator);
    for (int i = 0; i < s.rects.getSize(); i++)
        indices.pushBack(i);

    float r = getArea(s.rects, full, indices.getPtr(), indices.getSize());

    UMBRA_ASSERT(r <= area);

    return r;
#endif
}

bool RectGrouper::isSolutionBetter(const Solution& ref, const Solution& other)
{
    UMBRA_ASSERT(ref.isValid() && other.isValid());

#if 1
    return getSolutionArea(ref) - getSolutionArea(other) <
           (other.rects.getSize() - ref.rects.getSize()) * m_threshold;
#elif 0
    return getSolutionArea(ref) < getSolutionArea(other);
#else
    return ref.rects.getSize() < other.rects.getSize();
#endif
}
