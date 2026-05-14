// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRAQUERY_HPP
#define UMBRAQUERY_HPP

/*!
 * \file    umbraQuery.hpp
 * \brief   Umbra query interface
 */

#include "umbraDefs.hpp"

#define UMBRA_QUERY_SIZE_DEFAULT (100*1024)

#if !defined(UMBRA_QUERY_SIZE)
/*! The size of a Query instance, containing the scratch memory needed for executing the query */
#define UMBRA_QUERY_SIZE            UMBRA_QUERY_SIZE_DEFAULT
#endif
/*! The size of a CameraTransform instance */
#define UMBRA_CAMERA_TRANSFORM_SIZE     512
/*! The size of a IndexList instance */
#define UMBRA_INDEX_LIST_SIZE           32
/*! The size of a FloatList instance */
#define UMBRA_FLOAT_LIST_SIZE           32
/*! The size of a OcclusionBuffer instance */
#define UMBRA_OCCLUSION_BUFFER_SIZE     66*1024
/*! The size of a OcclusionBuffer instance */
#define UMBRA_RECEIVER_MASK_BUFFER_BYTE_SIZE 34*1024
/*! The size of a ShadowCuller instance */
#define UMBRA_SHADOW_CULLER_SIZE        52*1024
/*! The size of a IndexList instance */
#define UMBRA_VISIBILITY_SIZE           128
/*! The size of a ObjectDistanceParams instance */
#define UMBRA_OBJECTDISTANCEPARAMS_SIZE 64
/*! The size of a Path instance */
#define UMBRA_PATH_SIZE                 32
/*! The size of a IntersectionQueryResult instance */
#define UMBRA_LINESEGMENTQUERY_SIZE     64
/*! The size of a PortalInfo instance */
#define UMBRA_PORTALINFO_SIZE           160
/*! The maximum number of user clip planes */
#define UMBRA_MAX_USER_CLIP_PLANES      16

namespace Umbra
{

class Tome;
class TomeCollection;

/*!
 * \brief       Frustum class
 *
 */
struct Frustum
{
    /*!
     * \brief       Frustum constructor
     * \note        Initializes all components to zero.
     */
    Frustum (void) :
        left    (0.0f),
        right   (0.0f),
        top     (0.0f),
        bottom  (0.0f),
        zNear   (0.0f),
        zFar    (0.0f),
        type    (Frustum::PERSPECTIVE)
    {
        // nada here
    }

    /*!
     * \brief       Frustum constructor
     * \note        Builds a perspective projection
     *
     * \param       fovY    Field of view in the Y-direction in radians (0 < fovY < PI)
     * \param       aspect  Aspect ratio of the viewport (width/height)
     * \param       inNear  Positive distance to the near plane
     * \param       inFar   Positive distance to the far plane
     *
     */
    Frustum(float fovY, float aspect, float inNear, float inFar);


    /*!
     * \brief       Projection type defined by the frustum
     *
     */
    enum Type
    {
        PERSPECTIVE     = 0,        /*!< the frustum describes a perspective projection */
        ORTHOGRAPHIC    = 1         /*!< the frustum describes an orthographic projection */
    };

    float left;        /*!< frustum left value */
    float right;       /*!< frustum right value */
    float top;         /*!< frustum top value */
    float bottom;      /*!< frustum bottom value */
    float zNear;       /*!< frustum near value in range ]0,zFar[ */
    float zFar;        /*!< frustum far value in range ]zNear,infinity] */
    Type  type;        /*!< PERSPECTIVE (default) or ORTHOGRAPHIC */
};

/*!
 * \brief   A camera view frustum and the associated world transformation
 *
 * Queries that resolve the visibility from a camera viewpoint take an instance of
 * the CameraTransform class as input. The CameraTranform object contains the full
 * transformation matrix from world space to clip space coordinates, as well as any
 * additional clip planes. Helpers are provided for constructing the transformation
 * from the modelview and projection components.
 *
 * \note    Special care needs to be taken to make sure that this transformation matches
 *          that of the rendering pipeline exactly.
 */
class CameraTransform
{
public:

    /*!
     * \brief   The produced depth range of the transformation matrix.
     */
    enum DepthRange
    {
        /*! \brief  The D3D convention, depth range after w division [0,1]. This is the default. */
        DEPTHRANGE_ZERO_TO_ONE = 0,
        /*! \brief  The OpenGL convention, depth range after w division [-1,1] */
        DEPTHRANGE_MINUS_ONE_TO_ONE
    };

    /*!
     *  \brief  Construct an empty CameraTransform object, the empty object is not usable
     *          for anything as such, use set() to populate this object.
     */
    CameraTransform(void);

    /*!
     * \brief   Construct a CameraTransform object with full world to clip transform
     *          and exact camera position.
     *
     * \param   worldToClip     The full transformation matrix. Storage format defined by parameter 'mf'.
     *                          Multiply the modelview matrix with the projection matrix to obtain
     *                          the combined worldToClip matrix.
     * \param   position        The camera position.
     * \param   range           The depth range convention used.
     * \param   mf              The storage format (packed rows or packed columns) of 'worldToClip'.
     */
    CameraTransform(const Matrix4x4& worldToClip, const Vector3& position, DepthRange range = DEPTHRANGE_ZERO_TO_ONE, MatrixFormat mf = MF_COLUMN_MAJOR);

    /*!
     * \brief   Construct a CameraTransform object with an inverse view matrix and a frustum specification.
     *
     * \param   cameraToWorld   The inverse view matrix.
     * \param   frustum         The frustum specification for setting up the camera projection.
     * \param   mf              The storage format (packed rows or packed columns) of 'cameraToWorld'.
     * \deprecated
     */
    CameraTransform(const Matrix4x4& cameraToWorld, const Frustum& frustum, MatrixFormat mf = MF_COLUMN_MAJOR);

    /** Copy constructor */
    CameraTransform(const CameraTransform& rhs);
    /** Assignment operator */
    CameraTransform& operator=(const CameraTransform& rhs);

    /*!
     * \brief   Populate this CameraTransform with a full world to clip transform an an exact camera position.
     *
     * \param   worldToClip     The full transformation matrix. Storage format defined by parameter 'mf'.
     *                          Multiply the modelview matrix with the projection matrix to obtain
     *                          the combined worldToClip matrix.
     * \param   position        The camera position.
     * \param   range           The depth range convention used.
     * \param   mf              The storage format (packed rows or packed columns) of 'worldToClip'.
     */
    void set (const Matrix4x4& worldToClip, const Vector3& position, DepthRange range = DEPTHRANGE_ZERO_TO_ONE, MatrixFormat mf = MF_COLUMN_MAJOR);


    /*!
     * \brief   Get the transformation matrix.
     *
     * \param   worldToClip     The full transformation matrix. Storage format defined by parameter 'mf'.
     *                          Multiply the modelview matrix with the projection matrix to obtain
     *                          the combined worldToClip matrix.
     * \param   range           The depth range convention used.
     * \param   mf              The storage format (packed rows or packed columns) of 'worldToClip'.
     */
    void get(Matrix4x4& outWorldToClip, DepthRange inRange = DEPTHRANGE_ZERO_TO_ONE, MatrixFormat inMatrixFormat = MF_COLUMN_MAJOR) const;

    /*!
     * \brief   Set user clip planes for further culling geometry contained in the Camera frustum.
     *
     * User clip planes cause overhead in occlusion culling queries and should be used only
     * when needed. In particular, never pass in the planes of the camera frustum itself as
     * clipping against the frustum is already handled by the implementation.
     *
     * \param   planes          An array of 4-component plane equations describing the user clip planes.
     *                          Clip planes are given in world space coordinates, and the direction of the plane
     *                          normal points to the 'in' halfspace.
     * \param   planeCount      Numbed of planes in the plane array.
     */
    void setUserClipPlanes (const Vector4* planes, int planeCount);

    /*!
     * \brief   Get the user clip planes for further culling geometry contained in the Camera frustum.
     *
     * \param   planes          An array of 4-component plane equations describing the user clip planes.
     *                          Clip planes are given in world space coordinates, and the direction of the plane
     *                          normal points to the 'in' halfspace.
     * \param   planeCount      Numbed of planes in the plane array.
     */
    void getUserClipPlanes(Vector4 outPlanes[UMBRA_MAX_USER_CLIP_PLANES], int& outPlaneCount);

    // deprecated functions for individual setting of view and projection matrices

    void            setMatrixFormat     (MatrixFormat mf);
    MatrixFormat    getMatrixFormat     (void) const;
    void            setCameraToWorld    (const Matrix4x4& matrix);
    void            getCameraToWorld    (Matrix4x4& matrix) const;
    void            setWorldToCamera    (const Matrix4x4& matrix);
    void            getWorldToCamera    (Matrix4x4& matrix) const;
    void            setFrustum          (const Frustum& frustum);
    void            setFrustum          (float fovY, float aspect, float zNear, float zFar);
    void            getFrustum          (Frustum& frustum) const;

    uint8_t m_mem[UMBRA_CAMERA_TRANSFORM_SIZE];
};

/*!
 * \brief   An ordered collection of indices.
 *
 * An IndexList instance represents an array of 32-bit integer indices stored in user
 * supplied memory.
 *
 * IndexList instances are used both as query input and as query output. When used as output,
 * enough capacity needs to be provided to accommodate for the maximum number of elements the
 * query may return. For object indices this equals the total number of objects, likewise for
 * cluster indices. Queries that fill IndexList instance check against the provided bounds and
 * signal an error when the capacity is reached before writing all of the output.
 */
class IndexList
{
public:

    /*!
     * \brief   Populating constructor
     *
     * Construct an IndexList capable of holding 'capacity' indices using storage
     * specified by 'arr'. It is the user's responsibility to provide sufficient storage
     * for up to 'capacity' integers.
     *
     * \param   arr         Pointer to user memory to be used as IndexList storage
     * \param   capacity    Size of the user memory block, in number of 32-bit integers
     * \param   size        Current number of index elements stored in 'arr'. When the
     *                      IndexList is used for output only, this value should be 0.
     */
    IndexList(int* arr, int capacity, int size = 0);

    /*!
     * \brief   Default constructor for creating an uninitialized IndexList
     */
    IndexList(void);

    /** Copy constructor */
    IndexList(const IndexList& rhs);
    /** Assignment operator */
    IndexList& operator=(const IndexList& rhs);

    /** Get a pointer to the underlying storage. */
    int*    getPtr          (void) const;
    /** Set the underlying storage. */
    void    setPtr          (int* arr);
    /** Get the capacity (maximum number of indices) of this IndexList. */
    int     getCapacity     (void) const;
    /** Set the capacity (maximum number of indices) of this IndexList. */
    void    setCapacity     (int capacity);
    /** Get the number of indices stored in this IndexList. */
    int     getSize         (void) const;
    /** Set the number of indices stored in this IndexList. */
    void    setSize         (int size);

    uint8_t m_mem[UMBRA_INDEX_LIST_SIZE];
};

/*!
 * \brief   State vector for user defined gates
 *
 * Umbra visibility and connectivity queries support special user placed portals called 'Gates'
 * the influence of which can be dynamically toggled at runtime.
 *
 * In the 'open' state a Gate does not occlude objects behind it or prevent the connections
 * through the volume taken up by the Gate object, therefore the Gate object for all purposes
 * behaves as if it were not present in the original Scene used in Umbra computation. In the 'closed'
 * state the Gate object behaves as if it was defined as a standard occluder object.
 *
 * The default state of Gates is 'open', reflected both in the initialization of the GateStateVector
 * class and in the behavior of queries when no GateStateVector instance is given.
 *
 * It is the users responsibility to allocate sufficient storage for the state vector, referred to by
 * the 'arr' parameter used in GateStateVector construction. The storage requirement is one bit for
 * each Gate object present in the scene, aligned up to 32-bit boundary. Tome::getGateStateSize() can
 * be used to retrieve this size in number of bytes.
 *
 * A GateStateVector is introduced to a Query object via Query::setGateStates(). Note that it is
 * possible to build several GateStateVectors in advance and toggle the active complete state by
 * pointing the Query to the desired GateStateVector as needed.
 */
class GateStateVector
{
public:

    /*!
     * \brief   Populating constructor
     *
     * Construct a GateStateVector around a block of memory referred to by 'arr' of size 'size' bytes.
     * The size of the memory block needs to be large enough to host the Gate states for all of the
     * gates defined in scene computation. The Gate states are initialized to 'open' by the constructor.
     *
     * \param   arr     The user allocated memory block
     * \param   bytes   Size in bytes of the memory block
     * \param   clear   Boolean indicating whether the memory block should be cleared
     */
    GateStateVector(void* arr, size_t bytes, bool clear = true);

    /*!
     * \brief   Default constructor for creating an uninitialized GateStateVector.
     */
    GateStateVector(void);

    /*!
     * \brief  Set the current state of a Gate.
     *
     * Gates are identified by a running index from 0 to Tome::getGateCount(). See Tome::findGateIndex()
     * and Tome::getGateUserID() for mapping between Gate indices and Gate object user IDs.
     *
     * Toggling a Gate state while a Query is active (another thread is performing a query that holds a
     * reference to the same GateStateVector instance) results in undefined behavior.
     *
     * \param   idx     index of the Gate
     * \param   open    desired state of the Gate; true = open, false = closed
     */
    void        setState    (int idx, bool open);

    /** Get the current open/close state of a Gate */
    bool        getState    (int idx) const;

    /** Get a pointer to the underlying storage */
    void*       getPtr      (void) const;

private:
    void*   m_data;
};

/*!
 * \brief   Debug rendering callbacks.
 *
 * Umbra queries produce visual debugging information when an implementation of the DebugRenderer
 * interface is associated with a Query object via Query::setDebugRenderer(). The visual debugging
 * information can be a valuable tool both during integration and in understanding the quality
 * and performance characteristics of the queries so it is highly recommended to implement support
 * for debug rendering as early as possible.
 *
 * The type of debugging information produced depends on the type of the query and the query
 * parameters, more detailed documentation on the debugging data is provided in the documentation
 * of the Query class.
 *
 * The DebugRenderer class is an abstract class with default implementations that do nothing, the
 * user is expected to inherit this class and implement the member functions to render the passed
 * in elements interleaved with the actual scene geometry. For closer inspection of the debug data
 * it may be useful to either render it to a secondary debug viewport with independent camera
 * positioning controls or to expose controls to freeze the debug data (and the query results) to
 * a given camera transformation.
 */
class DebugRenderer
{
public:
    virtual ~DebugRenderer(void) {};

    /*!
     * \brief   Add a line segment to the debug context.
     *
     * \param   start   The start coordinate of the line, in world space
     * \param   end     The end coordinate of the line, in world space
     * \param   color   The preferred color of the line, where components are in order RGBX
     */
    virtual void addLine    (const Vector3& start, const Vector3& end, const Vector4& color)
    {
        ((void)start);
        ((void)end);
        ((void)color);
    }

    /*!
     * \brief   Add an isolated point to the debug context.
     *
     * \param   pt      The coordinate of the point, in world space
     * \param   color   The preferred color of the point, where components are in order RGBX
     */
    virtual void addPoint   (const Vector3& pt, const Vector4& color)
    {
        addLine(pt, pt, color);
    }

    /*!
     * \brief   Add an AABB to the debug context.
     *
     * The supplied default implementation renders the AABB as lines using addLine().
     *
     * \param   mn      Minimum coordinates of the AABB, in world space
     * \param   mx      Maximum coordinates of the AABB, in world space
     * \param   color   The preferred color of the AABB, where components are in order RGBX
     */
    virtual void addAABB (const Vector3& mn, const Vector3& mx, const Vector4& color)
    {
        addAABBLines(this, mn, mx, color);
    }

    /*!
     * \brief   Add an quad to the debug context
     * \param   x0y0    The min-x, min-y corner coordinates of the quad
     * \param   x0y1    The min-x, max-y corner coordinates of the quad
     * \param   x1y1    The max-x, max-y corner coordinates of the quad
     * \param   x1y0    The max-x, min-y corner coordinates of the quad
     * \param   color   The preferred color of the quad, where components are in order RGBX
     *
     * The supplied default implementation renders the quad as lines using addLine().
     */
    virtual void addQuad    (const Vector3& x0y0, const Vector3& x0y1, const Vector3& x1y1, const Vector3& x1y0, const Vector4& color)
    {
        addLine(x0y0, x0y1, color);
        addLine(x0y1, x1y1, color);
        addLine(x1y1, x1y0, color);
        addLine(x1y0, x0y0, color);
    }

    /*!
     * \brief   Process query statistic
     *
     * When query statistics output is enabled via Query::DEBUGFLAG_STATISTICS the Umbra
     * queries report statistics of their internal operation through this callback.
     * The produced statistics can provide valuable information to debugging performance
     * or quality issues.
     *
     * \param   stat    A description of the statistic, zero terminated
     * \param   val     Integer value for the statistic
     */
    virtual void addStat    (const char* stat, int val)
    {
        ((void)stat);
        ((void)val);
    }

    /*!
     * \brief   Utility function for adding debug lines for AABB edges
     */
    static void addAABBLines(DebugRenderer* dbg, const Vector3& mn, const Vector3& mx, const Vector4& color);
};

/*!
 * \brief   A cached representation of the visibility from a given frustum, point or region source.
 *
 * When requested, the visibility queries populate a OcclusionBuffer instance that can be used to
 * determine the visibility of dynamic objects or other scene elements that were not introduced to
 * the computation as target objects.
 *
 * Creating an OcclusionBuffer instance during a visibility query will increase the running time of
 * the query slightly compared to only returning the list of visible static objects, but the time
 * taken for the combined query is still considerably faster than doing separate queries for each.
 *
 * It is possible to also test for the visibility of static objects using the OcclusionBuffer
 * mechanism and never request the visible static object list from the visibility query. This will,
 * however, produce lower quality culling for the static objects and therefore it is advisable to
 * process static objects via the visible object list mechanism.
 *
 * Portal visibility queries can produce an OcclusionBuffer. The portal visibility query generates
 * the occlusion buffer as a byproduct of the portal traversal and rasterization.
 *
 * Individual isAABBVisible() and testAABBVisibility() tests against a OcclusionBuffer object are
 * fast, the intention is that thousands of tests can be executed within the budget of a frame.
 *
 * When the OcclusionBuffer has been generated from a Portal visibility query, it is also possible
 * to obtain the occlusion buffer depth value data via getBufferDesc() and getBuffer().
 */
class OcclusionBuffer
{
public:
    enum ErrorCode
    {
        /*! \brief   Operation succeeded */
        ERROR_OK = 0,
        /*! \brief   Invalid dimensions requested */
        ERROR_INVALID_DIMENSIONS,
        /*! \brief   Invalid buffer stride requested */
        ERROR_INVALID_STRIDE,
        /*! \brief   Invalid format requested */
        ERROR_INVALID_FORMAT,
        /*! \brief   Invalid output pointer */
        ERROR_INVALID_POINTER,
        /*! \brief   Occlusion buffer has no data */
        ERROR_EMPTY_BUFFER
    };

    enum Format
    {
        FORMAT_HISTOGRAM_8BPP   = (0 << 8) | 8,  /**< Single channel 8BPP with histogram equalization */
        FORMAT_NDC_FLOAT        = (1 << 8) | 32, /**< Linear floating point buffer of normalized device coordinates (z/w) */
        FORMAT_FORCE_32BIT      = 0x7fffffff
    };

#define UMBRA_FORMAT_BPP(x) ((x) & 0xff)     /**< Get bits per pixel for a format */

    /** Description of the layout of a pixel buffer */
    struct BufferDesc
    {
        /** the width of the buffer, in pixels */
        int     width;
        /** the height of the buffer, in pixels */
        int     height;
        /** the distance between successive scanline, in bytes */
        int     stride;
        /** the pixel format of the buffer */
        Format  format;
    };

    /** Possible return values for occlusion testing */
    enum VisibilityTestResult
    {
        /** \brief Target volume is completely occluded */
        OCCLUDED      = 0x0,
        /** \brief Target volume may be at least partially visible */
        VISIBLE       = 0x1,
        /** \brief Target volume is probably fully visible */
        FULLY_VISIBLE = 0x3
    };

    enum VisibilityTestFlags
    {
        /** \brief Test for full visibility */
        TEST_FULL_VISIBILITY = 0x1
    };

    /*!
     * \brief   Default constructor.
     *
     * Note that the occlusion buffer instance is relatively large so it is generally a good idea
     * to allocate the object from the heap.
     */
    OcclusionBuffer     (void);

    /** Copy constructor */
    OcclusionBuffer     (const OcclusionBuffer& rhs);
    /** Assignment operator */
    OcclusionBuffer&    operator=(const OcclusionBuffer& rhs);

    /*!
     * \deprecated This function is superseded by testAABBVisibility(mn, mx)
     *
     * \brief      Test for visibility of an axis-aligned box against the occlusion buffer.
     *
     * This is the legacy entrypoint for dynamic object testing. Always produces conservatively correct
     * results, when this function returns false then the AABB is completely occluded by the static
     * occluder geometry; or outside of the view frustum. The converse is not true: an AABB may be
     * occluded even when this function returns true.
     *
     * Note that the implementation has no prior knowledge of what the passed in AABB represents. The
     * most common use is to test for the visibility of dynamic objects by passing in the axis aligned
     * bounds of an object, but this function could be used for querying the visibility of arbitrary
     * volumes: one possible use is to test for the visibility of the nodes of a spatial hierarchy in
     * a recursively deepening fashion.
     *
     * \param   mn      The min 3D coordinate of the axis aligned bounding box of the object to test
     * \param   mx      The max 3D coordinate of the axis aligned bounding box of the object to test
     * \return          'true' if the object may be visible, 'false' if it is definitely hidden
     */
    bool isAABBVisible  (const Vector3& mn, const Vector3& mx) const
    {
        return testAABBVisibility(mn, mx) != OCCLUDED;
    }

    /*!
     * \brief   Test for visibility of an axis-aligned box against the occlusion buffer.
     *
     * This is the main entrypoint for dynamic object testing. Always produces conservatively correct
     * results, when this function returns false then the AABB is completely occluded by the static
     * occluder geometry; or outside of the view frustum. The converse is not true: an AABB may be
     * occluded even when this function returns true.
     *
     * Full visibility defined so that if an AABB is fully visible, all AABBs inside it would always
     * return visibility if queried separately. As described above, the AABBs may be actually occluded.
     *
     * Note that the implementation has no prior knowledge of what the passed in AABB represents. The
     * most common use is to test for the visibility of dynamic objects by passing in the axis aligned
     * bounds of an object, but this function could be used for querying the visibility of arbitrary
     * volumes: one possible use is to test for the visibility of the nodes of a spatial hierarchy in
     * a recursively deepening fashion.
     *
     * The full visibility check is implemented in order to provide an early exit when recursing
     * through a spatial hierarchy.
     *
     * \param   mn      The min 3D coordinate of the axis aligned bounding box of the object to test
     * \param   mx      The max 3D coordinate of the axis aligned bounding box of the object to test
     * \param   flags   A bitmask of or'ed VisibilityTestFlags values
     * \return          'OCCLUDED' if rectangle is fully occluded, 'VISIBLE' if rectangle may be visible,
     *                  'FULLY_VISIBLE' if TEST_FULL_VISIBILITY flag is specified and all volumes inside the
     *                  argument bounding box would return 'VISIBLE'
     */
    VisibilityTestResult testAABBVisibility(const Vector3& mn, const Vector3& mx, uint32_t flags = 0, float* contribution = NULL) const;

    /*!
     * \deprecated      This function is superseded by testAARectVisibility(mn, mx, z)
     * \brief   Test for visibility of an axis-aligned rectangle with given depth against the occlusion buffer.
     *
     * \param   mn      The min clip space x, y coordinates of the rectangle to test
     * \param   mx      The max clip space x, y coordinates of the rectangle to test
     * \param   z       The z coordinate of the rectangle to test in normalized device coordinates (z/w)
     * \return          'true' if the AABB may be visible, 'false' if it is definitely hidden
     */
    bool isAARectVisible(const Vector2& mn, const Vector2& mx, float z) const
    {
        return testAARectVisibility(mn, mx, z) != OCCLUDED;
    }

    /*!
     * \brief Test for full visibility of an axis-aligned rectangle with given depth against the
     * occlusion buffer.
     *
     * \param   mn      The min clip space x, y coordinates of the rectangle to test
     * \param   mx      The max clip space x, y coordinates of the rectangle to test
     * \param   z       The z coordinate of the rectangle to test in normalized device coordinates (z/w)
     * \param   flags   A bitmask of or'ed VisibilityTestFlags values
     * \return          'OCCLUDED' if rectangle is fully occluded, 'VISIBLE' if rectangle may be visible,
     *                  'FULLY_VISIBLE' if TEST_FULL_VISIBILITY flag is specified and all areas inside the
     *                  argument rectangle would return 'VISIBLE'
     */
    VisibilityTestResult testAARectVisibility(const Vector2& mn, const Vector2& mx, float z, uint32_t flags = 0) const;

    /*!
     * \brief   Width of the depth buffer in pixels.
     */
    int getWidth (void) const;

    /*!
     * \brief   Height of the depth buffer in pixels.
     */
    int getHeight (void) const;

    /*!
     * \brief   Copy out the occlusion buffer depth values.
     *
     * Generates a depth buffer representing the occlusion into user provided memory. Useful for
     * visualizing the occlusion or using the occlusion information for more elaborate purposes.
     *
     * Optional BufferDesc describes the requested format. Currently width and height must
     * match getWidth() and getHeight();
     *
     * \param   data    A pointer to the memory the depth values will be written to
     * \param   desc    Describes the required format.
     * \return  Error indicating result of operation.
     */
    ErrorCode getBuffer (void* data, const BufferDesc* desc = NULL) const;

    /*!
     * \brief   Get a default description for of the occlusion depth buffer.
     *
     * This function is intended to be used prior to getBuffer() for retrieving information on
     * the depth buffer that will be generated.
     *
     * The amount of memory that needs to be allocated for the buffer is equal to desc.stride * desc.height.
     *
     * \param   desc    A BufferDesc element to be filled with the buffer description
     * \deprecated
     */
    void getBufferDesc (BufferDesc& desc) const
    {
        desc.width  = getWidth();
        desc.height = getHeight();
        desc.format = FORMAT_HISTOGRAM_8BPP;
        desc.stride = desc.width * (UMBRA_FORMAT_BPP(desc.format) / 8);
    }

    /*!
     * \brief   Combine another occlusion buffer into this one.
     *
     * This is intended to be used for combining visibility results of multiple visibility query
     * jobs.
     */
    void combine (const OcclusionBuffer& other);

    /*!
     * \brief   Reset the OcclusionBuffer to the state it is in after construction
     */
    void clear (void);

    /*!
     * \brief   Override internal camera transform that is used for AABB tests.
     *
     * This is intended to be used if a world transformation was combined with visibility query
     * input transformation.
     */
    void setCameraTransform(const CameraTransform& src);

    uint8_t m_mem[UMBRA_OCCLUSION_BUFFER_SIZE];
};

/*!
 * \brief   A container for visiblity query inputs and outputs
 *
 * The visibility queries can produce different types of visibility information depending on the
 * user needs. The Visibility class is a simple container for input and output data pointers for
 * a visibility query, and the presence of the member elements is used to determine whether a
 * given type of visibility data should be produced.
 *
 * The Visibility object itself, as well as the data referred by it, is only accessed by the Query
 * context during the execution of the visibility query and can therefore be deleted immediately
 * after the query has returned.
 *
 * Three different kinds of visibility output are currently supported:
 * \li  <em>object list</em> List of visibile static target objects
 * \li  <em>output buffer</em> Occlusion buffer for dynamic object visibility
 * \li  <em>cluster list</em> List of visible clusters
 *
 * All methods and all source types for visibility queries support producing these three.
 */
class Visibility
{
public:

    /** Default constructor for creating a completely empty Visibility instance */
    Visibility (void);

    /*!
     * \brief   Constructor for the common case of object list and output buffer
     *
     * \param   outputObjects   output object list pointer
     * \param   outputBuffer    output occlusion buffer pointer
     */
    Visibility (IndexList* outputObjects, OcclusionBuffer* outputBuffer);

    /** Copy constructor */
    Visibility(const Visibility& rhs);
    /** Assignment operator */
    Visibility& operator=(const Visibility& rhs);

    /** Set output object list */
    void                setOutputObjects        (IndexList* objects);
    /* Get output object list */
    IndexList*          getOutputObjects        (void) const;

    /** Set output occlusion buffer */
    void                setOutputBuffer         (OcclusionBuffer* buffer);
    /** Get output occlusion buffer */
    OcclusionBuffer*    getOutputBuffer         (void) const;

    /** Set output cluster list */
    void                setOutputClusters       (IndexList* clusters);
    /** Get output cluster list */
    IndexList*          getOutputClusters       (void) const;

    /*!
     * \brief   Set input object filter list.
     *
     * Visibility queries also support pre-filtering of the static objects that
     * are considered for visibility testing. This can be useful when it is known
     * in advance that a given object or set of objects will not be rendered even
     * if it turned out to be visible based on the static occluder geometry.
     *
     * Narrowing down the set of objects processed by the visibility query reduces
     * the amount of work that the query needs to do and simplifies the processing
     * of the results.
     *
     * The input object IndexList instance is allowed to be the same instance that
     * is used for query output. This can be used to filter down a list of objects
     * in place.
     *
     * Input object filtering can also be used to chain up visibility queries.
     *
     * A value of NULL (the default) means that no filtering will be done:
     * visibility is determined for all static target objects.
     *
     * \param   objectMask  A list of objects to query visibility for
     */
    void                setInputObjects         (const IndexList* objectMask);

    /** Get the current input object filter list */
    const IndexList*    getInputObjects         (void) const;

    /** Set input occlusion buffer */
    void                setInputBuffer          (const OcclusionBuffer* buffer);
    /** Get input occlusion buffer */
    const OcclusionBuffer* getInputBuffer       (void) const;

    /*!
     * \brief   Set array for output object distances
     *
     * Umbra visibility queries can optionally return approximate shortest
     * distance values to the visible objects.
     *
     * This function is used to pass in memory in the form of a float array that
     * the queries will write distance values into. After a successful query,
     * the array will contain squared distance values (distance * distance) for
     * visible objects at the corresponding position in the output index list.
     *
     * The distance is computed exactly as the distance for distance culling purposes,
     * including honoring the 'distanceBounds' parameter for Scene::insertObject().
     *
     * \param   dist    An array for populating object distance values into,
     *                  must be at least as large as the capacity of the output objects
     *                  list.
     */
    void                setOutputObjectDistances (float* dist);

    /** Get the current output object distances array */
    float*              getOutputObjectDistances (void) const;

    /*!
     * \brief   Set array for object contributions
     *
     * Umbra visibility queries can optionally generate approximate object pixel
     * contributions on screen. A contribution is measured as object's relative
     * area on screen. For example, a contribution of 100 pixels in 1000x1000
     * viewport would be 100 / (1000 * 1000) = 0.0001.
     *
     * This function is used to pass in memory in the form of a float array that
     * the queries will write contribution values into. After a successful query,
     * the array will contain contribution values for visible objects at the
     * corresponding position in the output index list.
     *
     * \param   dist    An array for populating object contribution values into,
     *                  must be at least as large as the capacity of the output objects
     *                  list.
     */
    void                setOutputContributions (float* dist);

    /** Get the current object contributions array */
    float*              getOutputContributions (void) const;

    /** Set output object bit mask array */
    void                setOutputObjectMask(uint32_t* objects);
    /* Get output object bit mask array */
    uint32_t*           getOutputObjectMask(void) const;

    uint8_t m_mem[UMBRA_VISIBILITY_SIZE];
};

/*!
 * \brief   Modifiers for object distance range calculations
 *
 * Parameters for object distance range culling computations for visibility 
 * queries that process static objects.
 */
class ObjectDistanceParams
{
public:

    ObjectDistanceParams (void);
    ObjectDistanceParams(const ObjectDistanceParams& rhs);
    ObjectDistanceParams& operator=(const ObjectDistanceParams& rhs);

    ObjectDistanceParams (const Vector3* referencePt, float distanceScale);

    /*!
     * \brief  Set distance culling reference position. 
     * 
     * The world space position to which distance calculations are done for the 
     * purposes of object distance range culling. Using this feature will 
     * significantly increase the query time and is therefore highly discouraged!
     */
    void setReferencePoint (const Vector3* referencePt);
    bool getReferencePoint (Vector3& referencePtOut) const;

    /*!
     * \brief  Set distance culling multiplier.
     * 
     * Camera distance multiplier in range ]0,1] for distance range culling 
     * for implementing zooming.
     */
    void  setDistanceScale (float scale);
    float getDistanceScale (void) const;

    /*!
     * \brief  Set minimum relative contribution for contribution culling.
     *
     * Objects with smaller contribution are culled. The contribution value
     * is measured as object's relative area on screen. 0 means no contribution,
     * 1 means object covers the whole screen. The value does not measure
     * unoccluded area, but object's size on screen.
     */
    void  setMinRelativeContribution (float contribution);
    float getMinRelativeContribution (void) const;

    uint8_t m_mem[UMBRA_OBJECTDISTANCEPARAMS_SIZE];
};

/*!
 * \brief   The Umbra query context
 *
 * A Query object is a lightweight context for executing Umbra queries. The Query object itself contains enough
 * memory to perform the query operations, to eliminate the need for dynamic allocations.
 *
 * The Query object contains minimal state and the initialization of the object performs no heavy operations.
 * This allows, for example, the Query object to be re-allocated each time a query operation is needed in a
 * job/thread/process specific context.
 *
 * A Query object is not thread safe, invoking simultaneous queries on the same Query object from multiple
 * threads will produce undefined behavior. It is recommended to allocate a separate Query object for each
 * thread that does Umbra queries.
 */
class Query
{
public:

    /*!
     * \brief   Query return values
     */
    enum ErrorCode
    {
        /*! \brief   Query succeeded */
        ERROR_OK = 0,
        /*! \brief   Something completely unexpected happened */
        ERROR_GENERIC_ERROR = 1,
        /*! \brief   Not enough memory was available in the Query context to perform the operation
         *
         * As the queries operate in fixed memory this should never happen with the recommended
         * size for the Query object
         */
        ERROR_OUT_OF_MEMORY = 2,
        /*! \brief   An invalid value was passed in */
        ERROR_INVALID_ARGUMENT = 3,
        /*! \brief   A tile required to complete the Query was not present in the tome */
        ERROR_SLOTDATA_UNAVAILABLE = 4,
        /*! \brief   A query location was found to be outside of the scene boundaries */
        ERROR_OUTSIDE_SCENE = 5,
        /*! \brief   No data was given to the Query */
        ERROR_NO_TOME = 6,
        /*! \brief   Operation not supported */
        ERROR_UNSUPPORTED_OPERATION = 7,
        /*! \brief   Path does not exist */
        ERROR_NO_PATH = 8
    };

    /*!
     * \brief   Flags for visibility queries
     */
    enum VisibilityQueryFlags
    {
        /*! \brief  Do not use the camera position for visibility determination
         *
         * The culling algorithm will use the near plane quad object instead of
         * the position coordinate to locate the camera in the visibility data.
         * Set this flag for queries under perspective projection when it is
         * possible for the position to be contained occluder geometry. This
         * will implicitly be set for orthographic projection matrices.
         */
        QUERYFLAG_IGNORE_CAMERA_POSITION        = (1 << 1),
        /*! \brief  Use the reference implementation (slow) */
        QUERYFLAG_REFERENCE                     = (1 << 2),
        /*! \brief  Ignore per-object optimizations
         *
         * Ignores per-object data optimizations enabled using Optimizer
         * setting DATA_OBJECT_OPTIMIZATIONS, if present in data. */
        QUERYFLAG_IGNORE_OBJECT_OPTIMIZATIONS   = (1 << 3),
        /*! \brief  Produce debug lines for current view cell */
        DEBUGFLAG_VIEWCELL                      = (1 << 4),
        /*! \brief  Produce debug lines for portals traversed in query */
        DEBUGFLAG_PORTALS                       = (1 << 5),
        /*! \brief  Produce debug lines from view location to visible objects */
        DEBUGFLAG_VISIBILITY_LINES              = (1 << 6),
        /*! \brief  Produce debug lines for bounds of visible objects */
        DEBUGFLAG_OBJECT_BOUNDS                 = (1 << 7),
        /*! \brief  Produce debug lines for volume PVS */
        DEBUGFLAG_VISIBLE_VOLUME                = (1 << 8),
        /*! \brief  Produce debug lines for view frustum */
        DEBUGFLAG_VIEW_FRUSTUM                  = (1 << 9),
        /*! \brief  Produce statistics for queries */
        DEBUGFLAG_STATISTICS                    = (1 << 10)
    };

    /*!
     * \brief   SPU usage
     */
    enum SpuUsage
    {
        /* Do not dispatch queries to SPU */
        SPU_USAGE_NONE = 0,
        /* Use spurs context of thread 0 (default) */
        SPU_USAGE_SPURS_THREAD0,
        /* Use spurs context of thread 1 */
        SPU_USAGE_SPURS_THREAD1
    };

    /*!
     * \brief   Set up reference to the computed data.
     *
     * Umbra queries need access to the data generated by the Umbra optimizer for the
     * scene.
     *
     * All data pointers passed in to Query need to be 16-byte aligned and in the
     * native endianness of the execution environment. The TomeLoader utility takes
     * care of this automatically.
     *
     * A query can be reinitialized with different data as many times as required.
     *
     * \param   tome    The full computed data contained in a Tome.
     *
     * \note    The query context only reads the data during query execution and no
     *          state that depends on the data is cached by the context.
     *
     * \param   tome    A pointer to the Umbra tome
     */
    void    init    (const Tome* tome);
    void    init    (const TomeCollection* tomes);

    /*!
     * \brief   Deinitializes the Query object, resets all state to default.
     *
     * \note    As the Query does no resource allocations it is not mandatory
     *          to deinitialize a Query object when it is no longer used.
     */
    void    deinit  (void);

    /*!
     * \brief   Deinitializes all the SPU resources Umbra has initialized
     */
    static void deinitSpu   (void);

    /*!
     * \brief   Empty constructor, must call init() before Query can be used
     */
    Query   (void);

    /*!
     * \brief   Initialize query for the given Tome
     * \sa      Query::init()
     */
    Query   (const Tome* tome);
    Query   (const TomeCollection* tomes);

    /*!
     * \brief   Destructor
     */
    ~Query  (void);

    /** Copy constructor */
    Query(const Query& rhs);
    /** Assignment operator */
    Query& operator= (const Query& rhs);

    /*!
     * \brief   Sets a thread Id for threadsafe SPURS usage
     *          (deprecated, use setSpuUsage instead)
     */
    void        setThreadId                 (uint32_t threadId);

    /*!
     * \brief   Set SPU usage policy
     */
    void        setSpuUsage                 (SpuUsage usage);

    /*!
     * \brief   Set a DebugRenderer to receive query debug visualization data
     *
     * \param   debug   A DebugRender implementation that will receive callbacks
     *                  for debug rendering events during the visibility queries
     *                  based on the debug rendering flags.
     */
    void        setDebugRenderer            (DebugRenderer* debug);

    /*!
     * \brief   Set bit vector open/closed state for user defined gates.
     *
     * This function does not take a copy of the GateStateVector but simply keeps
     * a reference to it. It is therefore not necessary to call this function
     * between every query.
     *
     * \param   gateStates      A GateStateVector pointer containing the state of
     *                          the gate objects.
     */
    void        setGateStates               (const GateStateVector* gateStates);

    /*!
     * \brief   Set gate path cost vector.
     *
     * In addition to the 1-bit open/close toggle, gate objects can have a floating
     * point "path cost" value associated to them. The effect of the path cost is that
     * any path, as seen by queryShortestPath() or queryConnectedRegion() with a path
     * distance based radius, traversing through a gate will either have its length
     * multiplied by the gate path cost value (default behavior) or will have the gate
     * path cost value added to the path length (additive mode).
     *
     * This function does not take a copy of the gate cost vector but simply keeps
     * a reference to it. It is therefore not necessary to call this function
     * between every query.
     *
     * \param   gateCosts       A float array pointer containing the gate path cost
     *                          value per gate object index.
     * \param   additive        Treat path costs as additive to path length
     */
    void        setGatePathCosts            (const float* gateCosts, bool additive);

    /*!
     * \brief   Query portal-based visibility from a camera view
     *
     * \param   flags       A bitmask of or'ed VisibilityQueryFlags values
     * \param   visibility  A Visibility instance for defining visibility
     *                      inputs and outputs
     * \param   src         The camera transform for which visibility is to be queried
     * \param   distance    The amount of uncertainty in the camera translation
     * \param   accurateOcclusionThreshold  The distance to which accurate occlusion
     *                      information is gathered, negative value means that
     *                      the value is deduced automatically
     * \param   objDist     Parameters for object distance culling calculations. 
     *                      Passing in NULL causes defaults to be used.
     * \param   jobIndex    The index in range [0,numJobs[ of the subquery to execute
     * \param   numJobs     The number of jobs this visibility query is split into
     * \param   gridWidth   (hint) The width of the grid subdivision, when non-zero numJobs must be a multiple of this
     */

    ErrorCode   queryPortalVisibility       (uint32_t flags, const Visibility& visibility, const CameraTransform& src,
                                             float distance = 0.f, float accurateOcclusionThreshold = -1.f,
                                             const ObjectDistanceParams* objDist = NULL, int jobIndex = 0, 
                                             int numJobs = 1, int gridWidth = 0);


    /**
     * \brief       Performs view frustum culling without occlusion culling
     *
     * \param   flags       A bitmask of or'ed VisibilityQueryFlags values
     * \param   visibility  A Visibility instance for defining visibility
     *                      inputs and outputs
     * \param   src         The camera transform for which visibility is to be queried
     * \param   distance    The amount of uncertainty in the camera translation
     * \param   objDist     Parameters for object distance culling calculations. 
     *                      Passing in NULL causes defaults to be used.
     * \param   jobIndex    The index in range [0,numJobs[ of the subquery to execute
     * \param   numJobs     The number of jobs this visibility query is split into
     */

    ErrorCode   queryFrustumVisibility    (uint32_t flags, const Visibility& visibility, const CameraTransform& src,
                                           float distance = 0.f, const ObjectDistanceParams* objDist = NULL,
                                           int jobIndex = 0, int numJobs = 1);

    uint8_t m_mem[UMBRA_QUERY_SIZE];
};

// experimental code begins
// following interface is still in flux

/*!
 * \brief   Description of a portal
 */
class PortalInfo
{
public:

    PortalInfo(void);
    PortalInfo(const PortalInfo& rhs);
    PortalInfo& operator=(const PortalInfo& rhs);

    void        getCenter           (Vector3& center) const;
    int         getNumGateIndices   (void) const;
    int         getGateIndex        (int i) const;
    int         getTargetCluster    (void) const;
    void        getAABB             (Vector3& mn, Vector3& mx) const;
    int         getNumTriangles     (void) const;
    void        getTriangle         (int idx, Vector3& a, Vector3& b, Vector3& c) const;

    // Portal representation as vertices of convex hull
    int         getNumHullVertices  (void) const;
    void		getHullVertex       (int idx, Vector3& coord) const;
    void        getBoundingCircles  (Vector3& center, float& minRadius, float& maxRadius, Vector4 &planeEq) const;

    uint8_t m_mem[UMBRA_PORTALINFO_SIZE];
};

/*!
 * \brief   A path consisting of straight line segments
 */
class Path
{
public:
    struct Node
    {
        Node(void) {}

        /* world space coordinate of this path node */
        Vector3 coord;
        /* the portal index that this node is associated with,
         * -1 if not a portal node. QueryExt::getPortalInfo() can be
         * used to retrieve data on the portal. */
        int portalIndex;
        /* accumulated path distance from the start point */
        float distanceFromStart;
        /* padding */
        int reserved1;
        int reserved2;
        int reserved3;
    };

    Path(void);
    Path(Node* arr, int capacity);
    Path(const Path& rhs);
    Path& operator=(const Path& rhs);

    float       getLength   (void) const;
    int         getNumNodes (void) const;
    int         getCapacity (void) const;
    Node*       getNodes    (void) const;

    uint8_t m_mem[UMBRA_PATH_SIZE];
};

/*!
 * \brief   Holds input and output for a line segment query
 *              (QueryExt::queryLineSegment).
 *
 *          The ErrorCode indicates the result of the query.
 *          The following values are possible:
 *          \li RESULT_NO_INTERSECTION  no intersection
 *          \li RESULT_INTERSECTION     intersection with occluder geometry
 *          \li RESULT_OUTSIDE_SCENE    starting point is outside the scene
 *
 * The IndexList receives indices of potentially intersecting objects. It is
 * optional. Several queries can share the same IndexList if they are
 * consecutive entries in the QueryExt::queryLineSegment parameter array.
 */
struct LineSegmentQuery
{
    /** Indicates result of the query */
    enum ResultCode
    {
        RESULT_INTERSECTION,    /**< Intersection with occluder geometry */
        RESULT_NO_INTERSECTION, /**< No itersection */
        RESULT_OUTSIDE_SCENE    /**< Starting point is outside the scene */
    };

    /** Constructor */
    LineSegmentQuery(void);
    /** Constructor with start and end coordinates */
    LineSegmentQuery(const Vector3& start, const Vector3& end, IndexList* objectSet = NULL);

    LineSegmentQuery(const LineSegmentQuery& rhs);
    LineSegmentQuery& operator=(const LineSegmentQuery& rhs);

    // Inputs
    void                setStart        (const Vector3& start); /**< Input: set start coordinate */
    const Vector3&      getStart        (void) const;           /**< Input: get start coordinate */
    void                setEnd          (const Vector3& end);   /**< Input: set end coordinate */
    const Vector3&      getEnd          (void) const;           /**< Input: get end coordinate */

    // Outputs
    ResultCode          getResult       (void) const;           /**< Output: get result */
    void                setObjectSet    (IndexList* objectSet); /**< Output: (optional) Set IndexList to receive intersecting objects */
    IndexList*          getObjectSet    (void) const;           /**< Output: (optional) Get IndexList */

    uint8_t m_mem[UMBRA_LINESEGMENTQUERY_SIZE];
};

/*!
 * \brief Development features
 */

class ReceiverMaskBuffer
{
public:

    ReceiverMaskBuffer(void);
    ReceiverMaskBuffer(const ReceiverMaskBuffer& rhs);
    ReceiverMaskBuffer& operator=(const ReceiverMaskBuffer& rhs);

    const CameraTransform&  getCameraTransform  (void) const;
    float                   getDepth            (int x, int y) const;
    int                     getWidth            (void) const;
    int                     getHeight           (void) const;

    uint8_t m_mem[UMBRA_RECEIVER_MASK_BUFFER_BYTE_SIZE];
};

class ShadowCullerExt
{
public:
    ShadowCullerExt (void);
    ShadowCullerExt (const ShadowCullerExt& rhs);
    ShadowCullerExt& operator=(const ShadowCullerExt& rhs);

    /*!
     * \brief   Test if a dynamic shadow caster is active or if it can be culled

     * \param   mn  Shadow caster AABB min coordinate
     * \param   mx  Shadow caster AABB max coordinate
     *
     * \return  false if culled, true otherwise
    **/
    bool                isAABBActive            (const Vector3& mn, const Vector3& mx) const;

    /*!
     * \brief   Test if a dynamic shadow caster is active or if it can be culled

     * \param   mn            Shadow caster AABB min coordinate
     * \param   mx            Shadow caster AABB max coordinate
     * \param   cascadeMasks  Bitmask indicating which cascades the AABB belongs in
     *                        (if cascades were provided when building culler)
     *
     * \return  false if culled, true otherwise
    **/
    bool                isAABBActive            (const Vector3& mn, const Vector3& mx, UINT32& cascadeMask) const;

    /** Get the receiver mask buffer for debugging and visualizations */
    Query::ErrorCode    getReceiverMaskBuffer   (ReceiverMaskBuffer& out) const;

    uint8_t m_mem[UMBRA_SHADOW_CULLER_SIZE];
};


class FloatList
{
public:

    FloatList(float* arr, int capacity, int size = 0);
    FloatList(void);
    FloatList(const FloatList& rhs);
    FloatList& operator= (const FloatList& rhs);

    float*  getPtr          (void) const;
    void    setPtr          (float* arr);
    int     getCapacity     (void) const;
    void    setCapacity     (int capacity);
    int     getSize         (void) const;
    void    setSize         (int size);

    uint8_t m_mem[UMBRA_FLOAT_LIST_SIZE];
};

struct SphereLight
{
    Vector3 center;
    float   radius;
};

class QueryExt : public Query
{
public:

    /** Query type enumeration */
    enum QueryType
    {
        /** Shortest path query (queryShortestPath) */
        QUERYTYPE_SHORTEST_PATH
    };

    /** Flags for connectivity queries affecting the mode of operation */
    enum QueryFlagsExt
    {
        /** Connectivity query uses path distance instead of euclidean distance */
        QUERYFLAG_PATH_DISTANCE                     = 1<<0,
        /** Connectivity query (path variant) ignores initial point-to-first-portal distance */
        QUERYFLAG_DISTANCE_FROM_CLUSTER             = 1<<1,
        /** Confidence bound is calculated from intersecting portals only */
        QUERYFLAG_CONFIDENCE_INTERSECTING           = 1<<2,
        /** Confidence bound is calculated from non-intersecting portals only */
        QUERYFLAG_CONFIDENCE_NONINTERSECTING        = 1<<3,
        /** Only cascades until the first fully containing cascade are reported in a cascade mask
            (queryStaticShadowCasters and isAABBActive): a fully containing cascade excludes rest
            of the cascades. Casacdes should be be passed in smallest first. */
        QUERYFLAG_EXCLUSIVE_CASCADES                = 1<<4
    };

    QueryExt   (void);
    QueryExt   (const Tome* tome);
    QueryExt   (const TomeCollection* tomes);

    /*!
     * \brief   Query the connectivity database to find an approximation of the
     *          shortest path between two points.
     *
     * \param   flags       QueryFlagsExt to pass to the query. Only 0 is supported.
     * \param   p           Output path parameter
     * \param   start       Input parameter: start point for the path
     * \param   end         Input parameter: end point for the path
     *
     * \return  Error code indicating the result of the query. The following values are possible:
     *          \li ERROR_OK if the query is succesful
     *          \li ERROR_OUTSIDE_SCENE if either the start or end point is outside the scene
     *          \li ERROR_NO_PATH if there is no path between the points
     *          \li ERROR_UNSUPPORTED_OPERATION will be returned if this query is not supported on the platform
     *          where it is executed.
     */
    ErrorCode   queryShortestPath           (uint32_t flags, Path& p, const Vector3& start, const Vector3& end);

    /*!
     * \brief   Get cluster ID for point
     */

    ErrorCode   queryClusterForPoint        (const Vector3& point, int& clusterOut);

    /*!
     * \brief   Query the connectivity database to define a region reachable from
     *          a point with a given connected path distance limit.
     *
     * \param   flags       Umbra::QueryFlagsExt that determines modifiers for the
     *                      operation of the query.
     * \param   clustersOut The clusters within this connected region
     * \param   cluster     The starting cluster for this query, pass in -1 to use
     *                      the center point for determining the cluster.
     * \param   pt          The center point of the sphere that defines the volume
     *                      to be included.
     * \param   distance    Sphere radius that defines the volume to be included.
     *                      A negative distance defines a region that consists of
     *                      the complete reachable region from the point.
     * \param   confidenceBound Query result is guaranteed to be valid if the query point stays withing 
     *                          this distance from the original query point.
     */
    ErrorCode   queryConnectedRegion        (uint32_t flags, IndexList& clustersOut, int cluster, const Vector3& pt, float distance, float* confidenceBound = NULL,
                                             FloatList* clusterPathDistances = NULL, FloatList* clusterPathModifiers = NULL, IndexList* clusterEntryPortals = NULL);

    /*!
     * \brief   Retrieve indices of outbound portals for cluster
     */
    ErrorCode   clusterPortals              (IndexList& portals, int cluster);

    /*!
     * \brief   Get information on portal by index
     */
    ErrorCode   getPortalInfo               (PortalInfo& portal, int portalIndex);

    /**
    * \brief    Queries line segment intersection against occluder geometry.
    *           Performs experimental raycast-like point-to-point visibility
    *           queries.
    *
    * \param    queries    array of LineSegmentQuery objects
    * \param    count      number of queries to perform (number of items in the
    *                      queries array)
    *
    *           The result is conservatively correct: inaccuracy produces
    *           false positives.
    *
    *           Each LineSegmentQuery-object in queries array holds input and output
    *           for a single line segment query. LineSegmentQuery::setStart and
    *           LineSegmentQuery::setEnd define the line segment.
    *
    *           Optional IndexList (LineSegmentQuery::setObjectSet) in
    *           LineSegmentQuery receives a list of potentially intersecting
    *           objects until the first intersection. Several queries can share
    *           the same IndexList if they are consecutive in the array.
    *
    *           ResultCode in LineSegmentQuery indicates result of the
    *           query. The following values are possible:
    *           \li RESULT_NO_INTERSECTION	no intersection
    *           \li RESULT_INTERSECTION     intersection with occluder geometry
    *           \li RESULT_OUTSIDE_SCENE    starting point is outside the scene
    *
    * \note     Intersection result has the accuracy of Tome computation
    *           parameters. Intersecting objects are reported in AABB accuracy.
    *
    */
    ErrorCode   queryLineSegment            (LineSegmentQuery* queries, int count);

    /*!
     * \brief   Allows increasing the amount of memory available for query
     *          execution. Calling this is normally not required.
     * \sa      QueryExt::getMemoryRequirement
     *
     *          By default queries use the memory buffer embedded in Umbra::Query.
     *          Setting additional work memory is necessary only in special cases
     *          explained in documentation.
     *
     * \param   workMem     Additional memory area for query execution, NULL to disable.
     * \param   workMemSize Size of memory area in bytes pointed by workMem.
     *
     * \return  Error code indicating the result of the query. The following values are possible:
     *          \li ERROR_OK                operation is successful
     */
    ErrorCode   setWorkMem  (uint8_t* workMem, size_t workMemSize);

    /*!
     * \brief   Get amount of work memory required for executing the
     *          given query with the given tome.
     * \sa      QueryExt::setWorkMem
     *
     *          Currently executing the shortest path query might need
     *          more memory than is by default available. Function
     *          getMemoryRequirement can be used to query the required amount
     *          of additional memory. This additional memory can be added
     *          using setWorkMem.
     *
     * \param   type    Query type
     * \param   tome    A tome file for which the query is going to
     *                  be executed.
     * \return  Amount of additional memory to be set using setWorkMem.
     *
     */
    static size_t UMBRACALL getMemoryRequirement (QueryType type, const Tome* tome);

    /*!
     * \brief   Build a ShadowCuller to be used for receiver mask shadow caster culling
     *
     * \param   culler              The shadow caster culler to be built
     * \param   vis                 The input Visibility instance that contains the results
     *                              of the visibility query, most importantly the occlusion buffer
     * \param   lightDir            The light direction vector
     * \param   dynBounds           A list of AABBs (Vector3-pairs, mn,mx) of visible dynamic shadow receivers
     * \param   numDynBounds        Number of visible dynamic receiver AABBs
     * \param   farPlaneDistance    (OPTIONAL) Custom far-plane distance
     * \param   flags               (OPTIONAL) QUERYFLAG_EXCLUSIVE_CASCADES or 0
     * \param   cascades            (OPTIONAL) Array of cascade transforms. If provided, 
     *                              the culler can assign objects into cascades.
     * \param   numCascades         (OPTIONAL) Number of entries in cascades array.
     */

    ErrorCode   buildMaskShadowCuller (ShadowCullerExt&        culler,
                                       const Visibility&       vis,
                                       const Vector3&          lightDir,
                                       const Vector3*          dynBounds,
                                       int                     numDynBounds,
                                       float*                  farPlaneDistance = NULL,
                                       UINT32                  flags = 0,
                                       const CameraTransform** cascades = 0, 
                                       int                     numCascades = 0);

    /*!
     * \brief   Build a ShadowCuller to be used for plane shadow caster culling
     *
     * \param   culler              The shadow caster culler to be built
     * \param   camera              The main camera (note: NOT the shadow camera)
     * \param   lightDir            The light direction vector
     * \param   farPlaneDistance    (OPTIONAL) Custom far-plane distance
     * \param   flags               (OPTIONAL) QUERYFLAG_EXCLUSIVE_CASCADES or 0
     * \param   cascades            (OPTIONAL) Array of cascade transforms. If provided, 
     *                              the culler can assign objects into cascades.
     * \param   numCascades         (OPTIONAL) Number of entries in cascades array.
     */

    ErrorCode   buildPlaneShadowCuller (ShadowCullerExt&        culler,
                                        const CameraTransform&  camera,
                                        const Vector3&          lightDir,
                                        float*                  farPlaneDistance = NULL,
                                        UINT32                  flags = 0,
                                        const CameraTransform** cascades = 0, 
                                        int                     numCascades = 0);
    /*!
     * \brief   Query potential static shadow casters using a pre-generated ShadowCullerExt
     *
     * \note    OPTIONAL parameters can be NULL or 0.
     *
     * \param   culler                      The pre-generated shadow caster culler     *
     * \param   out                         List of potential static shadow casters.
     * \param   inObjDistParams             (OPTIONAL) Parameters for object distance culling calculations.
     *                                      Passing in NULL causes defaults to be used.
     * \param   jobIdx                      (OPTIONAL) If the query is to be split across multiple jobs, this is the
     *                                      current job index
     * \param   numJobs                     (OPTIONAL) If the query is to be split across multiple jobs, this is the
     *                                      total number of jobs
     * \param   cascadeMasks                (OPTIONAL) If cascades were provided for buildMaskShadowCuller/buildPlaneShadowCuller,
     *                                      bitmasks indicating object assignment into cascades can be outputted. The order corresponds
     *                                      to out IndexList.
     */

    ErrorCode   queryStaticShadowCasters  (const ShadowCullerExt& culler, IndexList& out, const ObjectDistanceParams* inObjDistParams = NULL, int jobIdx = 0, int numJobs = 1, IndexList* cascadeMasks = NULL);

    /**
     * \brief   Query the connectivity database to find which dynamic sphere lights may be visible given a set of visible clusters.
     *
     * This is a helper utility that runs queryConnectedRegion() queries for each of the sphere lights and finds if there is
     * an intersection between the sphere light connected region cluster list and the visible cluster list.
     *
     * It is recommended first test the visibility of the whole sphere against the camera visibility results by calls
     * to OcclusionBuffer::testAABBVisibility() and only do the local light queries for lights that are not fully occluded and
     * not fully visible.
     *
     * \param   outVisibleLights   The result set of lights that may be visible. The indices
     *                             in outVisibleLights match the ones in lightOrigins and lightRadii
     *                             arrays.
     * \param   flags              Additional flags to pass to the query, currently does nothing.
     * \param   lightCenters       An array of points representing the centers of
     *                             the dynamic sphere lights. The array must have at least
     *                             lightCount elements.
     * \param   lightRadii         An array of scalars representing the radii of the dynamic
     *                             sphere lights. The array must have at least lightCount elements.
     * \param   lightCount         The number of lights to process from the input arrays.
     * \param   visibleClusters    The set of visible clusters from which all of the resulting
     *                             dynamic lights must be visible.
     * \param   visibleLightFilter Optional filter for pre-culled lights (can point to outVisibleLights)     
     *
     * \return  Error code indicating the result of the query.
     */
    ErrorCode   queryLocalLights    (IndexList&                 outVisibleLights,
                                     uint32_t                   flags,
                                     const SphereLight*         sphereLights,
                                     int                        lightCount,
                                     const IndexList&           visibleClusters,
                                     const IndexList*           visibleLightFilter = NULL);
};

} // namespace Umbra
#endif // UMBRAQUERY_HPP
