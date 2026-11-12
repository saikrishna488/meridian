cmake_minimum_required(VERSION 3.22)
project(MarsOSNative LANGUAGES C CXX)
file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/../Cargo.toml" VERSION_LINE REGEX "^version = ")
string(REGEX MATCH "[0-9]+\\.[0-9]+\\.[0-9]+" MERIDIAN_VERSION "${VERSION_LINE}")
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
option(MERIDIAN_SANITIZERS "Enable address and undefined behavior sanitizers" OFF)
if(MERIDIAN_SANITIZERS)
    add_compile_options(-fsanitize=address,undefined -fno-omit-frame-pointer)
    add_link_options(-fsanitize=address,undefined)
endif()
find_package(PkgConfig REQUIRED)
pkg_check_modules(GIO REQUIRED IMPORTED_TARGET gio-unix-2.0>=2.78)
pkg_check_modules(SYSTEMD REQUIRED IMPORTED_TARGET libsystemd>=257)
find_package(Threads REQUIRED)
add_library(meridian-native STATIC
    src/services/catalog.cpp
    src/protocol/policy.cpp
    src/core/json.cpp
    src/core/io.cpp
    src/core/process.cpp
    src/protocol/validation.cpp
    src/protocol/backend.cpp
    src/protocol/dispatch.cpp
    src/services/stores.cpp
    src/services/applications.cpp
    src/services/fuzzy.cpp
    src/services/startup.cpp
    src/services/system.cpp
    src/services/files.cpp)
target_sources(meridian-native PRIVATE src/services/transfers.cpp src/services/trash.cpp src/services/audio.cpp)
target_compile_features(meridian-native PUBLIC cxx_std_20)
target_include_directories(meridian-native PUBLIC include)
target_link_libraries(meridian-native PUBLIC PkgConfig::GIO PkgConfig::SYSTEMD Threads::Threads)
target_include_directories(meridian-native PRIVATE src)
target_compile_options(meridian-native PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_compile_definitions(meridian-native PRIVATE MERIDIAN_VERSION="${MERIDIAN_VERSION}")
option(MERIDIAN_BUILD_SURFACES "Build GTK/Wayland surface integration" ON)
if(MERIDIAN_BUILD_SURFACES)
    pkg_check_modules(GTK REQUIRED IMPORTED_TARGET gtk4>=4.18)
    pkg_check_modules(LAYER REQUIRED IMPORTED_TARGET gtk4-layer-shell-0>=1.2)
    add_library(meridian-surfaces STATIC src/platform/surface.cpp)
    target_compile_features(meridian-surfaces PUBLIC cxx_std_20)
    target_include_directories(meridian-surfaces PUBLIC include)
    target_link_libraries(meridian-surfaces PUBLIC PkgConfig::GTK PkgConfig::LAYER)
    target_compile_options(meridian-surfaces PRIVATE -Wall -Wextra -Wpedantic -Werror)
endif()
include(CTest)
if(BUILD_TESTING)
    foreach(contract validation config backend fuzzy startup applications process system transfers trash audio)
        add_executable(${contract}-test tests/${contract}.cpp)
        target_include_directories(${contract}-test PRIVATE src)
        target_link_libraries(${contract}-test PRIVATE meridian-native)
        add_test(NAME ${contract} COMMAND ${contract}-test)
    endforeach()
    if(MERIDIAN_BUILD_SURFACES)
        add_executable(surface-test tests/surface.cpp)
        target_link_libraries(surface-test PRIVATE meridian-surfaces)
        add_test(NAME surface COMMAND surface-test)
        set_tests_properties(surface PROPERTIES SKIP_RETURN_CODE 77)
    endif()
    add_executable(catalog-test tests/catalog.cpp)
    target_link_libraries(catalog-test PRIVATE meridian-native)
    add_test(NAME catalog COMMAND catalog-test)
    add_executable(policy-test tests/policy.cpp)
    target_link_libraries(policy-test PRIVATE meridian-native)
    add_test(NAME policy COMMAND policy-test)
    add_executable(c-abi-test tests/abi.c)
    target_link_libraries(c-abi-test PRIVATE meridian-native)
    add_test(NAME c-abi COMMAND c-abi-test)
endif()
