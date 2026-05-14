#pragma once

#include "umbraPrivateDefs.hpp"
#include "umbraArray.hpp"
#include "umbraVector.hpp"

namespace Umbra
{
    class RectGrouper
    {
    public:
        enum Strategy
        {
            FOUR_QUADRANTS,
            COMBINE_ALL,
            COMBINE_NOTHING,
            GRID_GROUPER
        };

        RectGrouper(Allocator* a, float thresholdArea = 0.f);

        void setAllocator(Allocator* a)
        {
            m_allocator = a;
            m_rects.setAllocator(a);
            m_best.rects.setAllocator(a);
        }

        Allocator* getAllocator() { return m_allocator; }

        void setThreshold(float th) { UMBRA_ASSERT(getResult().getSize() == 0); m_threshold = th; }
        void setStrategy(Strategy s) { m_strategy = s; }
        void addRect(const Vector4& rect) { m_rects.pushBack(rect); }

        void setGridSize(int g) { m_gridSize = g; }

        void execute();

        const Array<Vector4>& getResult() const { return m_best.rects; }

    private:
        struct Solution
        {
            Solution(Allocator* a) : rects(a) {}

            bool isValid() const { return rects.getSize() > 0; }

            Array<Vector4> rects;
        };

        static float& minX(Vector4& r) { return r.x; }
        static float& minY(Vector4& r) { return r.y; }
        static float& maxX(Vector4& r) { return r.z; }
        static float& maxY(Vector4& r) { return r.w; }
        static const float& minX(const Vector4& r) { return r.x; }
        static const float& minY(const Vector4& r) { return r.y; }
        static const float& maxX(const Vector4& r) { return r.z; }
        static const float& maxY(const Vector4& r) { return r.w; }

        static bool splitRectX(const Vector4& r, float x, Vector4& left, Vector4& right);
        static bool splitRectY(const Vector4& r, float y, Vector4& left, Vector4& right);

        void splitX(const Vector4& r, float x, Array<Vector4>& left, Array<Vector4>& right);
        void splitY(const Vector4& r, float y, Array<Vector4>& left, Array<Vector4>& right);

        void fourQuadrants(const Array<Vector4>& rects, Solution& solution);

        Vector4 unionRects(const Array<Vector4>& rects);

        void tryGroupSolution(Solution& s);

        void gridGrouper(Array<Vector4>& out, const Array<Vector4>& in, int s);

        float getSolutionArea(const Solution& s);
        float getArea(const Array<Vector4>& rects, const Vector4& rect, int* indices, int n);

        bool isSolutionBetter(const Solution& ref, const Solution& other);

        Allocator*      m_allocator;
        Strategy        m_strategy;
        float           m_threshold;
        int             m_gridSize;
        Array<Vector4>  m_rects;

        Solution        m_best;
    };
}
