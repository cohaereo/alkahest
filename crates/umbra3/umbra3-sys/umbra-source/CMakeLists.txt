cmake_minimum_required(VERSION 3.10)
project(Umbra3 CXX)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Platform detection
if(WIN32)
    set(UMBRA_PLATFORM_WINDOWS ON)
elseif(APPLE)
    set(UMBRA_PLATFORM_MACOSX ON)
elseif(UNIX AND NOT APPLE)
    set(UMBRA_PLATFORM_LINUX ON)
endif()

# Add common library
add_library(umbra_common STATIC)

target_sources(umbra_common PRIVATE
    # Common sources
    source/common/umbraAABB.cpp
    source/common/umbraBinStream.cpp
    source/common/umbraBitMath.cpp
    source/common/umbraBitOps.cpp
    source/common/umbraCRCStream.cpp
    source/common/umbraChecksum.cpp
    source/common/umbraCmdlineApp.cpp
    source/common/umbraDPVSShared.cpp
    source/common/umbraDirScan.cpp
    source/common/umbraDouble.cpp
    source/common/umbraFileStream.cpp
    source/common/umbraFile_ansic.cpp
    source/common/umbraFloat.cpp
    source/common/umbraHttp.cpp
    source/common/umbraIntersect.cpp
    source/common/umbraJson.cpp
    source/common/umbraLogger.cpp
    source/common/umbraMatrix.cpp
    source/common/umbraPatcher.cpp
    source/common/umbraPoolAllocator.cpp
    source/common/umbraPrivateVersion.cpp
    source/common/umbraPropertyFile.cpp
    source/common/umbraRandom.cpp
    source/common/umbraSerializer.cpp
    source/common/umbraStatsHeap.cpp
    source/common/umbraString.cpp
    source/common/umbraTimer.cpp
    source/common/umbra_stub_allocator.cpp
)

# Platform-specific common sources
if(UMBRA_PLATFORM_WINDOWS)
    target_sources(umbra_common PRIVATE
        source/common/windows/umbraOsWin32.cpp
        source/common/windows/umbraProcessWin32.cpp
    )
else()
    # For non-Windows platforms, we use the portable implementations
    # The code includes platform detection via preprocessor directives
    # in the header files (umbraOs.hpp, umbraProcess.hpp, etc.)
    # These are in the generic common/*.cpp files above
endif()

target_include_directories(umbra_common PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/interface
    ${CMAKE_CURRENT_SOURCE_DIR}/source/common
    ${CMAKE_CURRENT_SOURCE_DIR}/source/standard
    ${CMAKE_CURRENT_SOURCE_DIR}/source
)

target_compile_definitions(umbra_common PRIVATE
    _CRT_SECURE_NO_WARNINGS
    _CRT_NONSTDC_NO_WARNINGS
)

if(MSVC)
    target_compile_options(umbra_common PRIVATE /W4)
else()
    target_compile_options(umbra_common PRIVATE -Wall -Wextra -Wno-unused-parameter)
endif()

# Add runtime library
add_library(umbra_runtime STATIC)

target_sources(umbra_runtime PRIVATE
    source/runtime/umbraBSPTree.cpp
    source/runtime/umbraConnectivity.cpp
    source/runtime/umbraCubemap.cpp
    source/runtime/umbraDPVS.cpp
    source/runtime/umbraPortalCull.cpp
    source/runtime/umbraPortalRaster.cpp
    source/runtime/umbraPortalRayTracer.cpp
    source/runtime/umbraQueryApi.cpp
    source/runtime/umbraQueryArgs.cpp
    source/runtime/umbraQueryContext.cpp
    source/runtime/umbraRuntimeInfoString.cpp
    source/runtime/umbraRuntimeTomeGenerator.cpp
    source/runtime/umbraShadows.cpp
    source/runtime/umbraTome.cpp
    source/runtime/umbraTomeApi.cpp
    source/runtime/umbraTomeCollection.cpp
    source/runtime/umbraTransformer.cpp
)

target_include_directories(umbra_runtime PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/interface
    ${CMAKE_CURRENT_SOURCE_DIR}/interface/runtime
    ${CMAKE_CURRENT_SOURCE_DIR}/source/common
    ${CMAKE_CURRENT_SOURCE_DIR}/source/runtime
    ${CMAKE_CURRENT_SOURCE_DIR}/source/standard
)

target_link_libraries(umbra_runtime PUBLIC umbra_common)

target_compile_definitions(umbra_runtime PRIVATE
    _CRT_SECURE_NO_WARNINGS
    _CRT_NONSTDC_NO_WARNINGS
)

if(MSVC)
    target_compile_options(umbra_runtime PRIVATE /W4)
else()
    target_compile_options(umbra_runtime PRIVATE -Wall -Wextra -Wno-unused-parameter)
endif()
