# No Rust toolchain in vcpkg's build environment, so this port downloads the
# prebuilt release archive for the current triplet instead of building from
# source (see https://github.com/ZirekHQ/dengjen-tashkeel/issues/23).
vcpkg_check_linkage(ONLY_DYNAMIC_LIBRARY)
set(VCPKG_BUILD_TYPE release)
# VCPKG_BUILD_TYPE alone doesn't suppress the post-build binary-count lint
# for a port that never calls vcpkg_cmake_configure/build.
set(VCPKG_POLICY_MISMATCHED_NUMBER_OF_BINARIES enabled)

set(CAPI_VERSION "2.0.0")

# SHA512s below are the actual hashes of the v${CAPI_VERSION} release assets,
# cross-checked against the .sha256 files published alongside them.
if(VCPKG_TARGET_IS_OSX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
    set(CAPI_TARGET_TRIPLE "aarch64-apple-darwin")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "5385b6328bbf154b65efa88343b3696f8eef2b001ed79613e0beb9504615e8678a80e5d9dd9840c193b22571102cea98d4d7eaaffc8c8608d5e928778c433017")
elseif(VCPKG_TARGET_IS_LINUX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    set(CAPI_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "766da811093179fd3561ecdd0e044e6cf59f098a35c537a208cb0c4f58c762433a27250261389eec4ccc58bbf2d5a712893ca36ea78c81277416edc0bc2d47df")
elseif(VCPKG_TARGET_IS_WINDOWS AND NOT VCPKG_TARGET_IS_MINGW AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    # The published archive is MSVC-built; vcpkg.json's 'supports' expression
    # already excludes MinGW triplets, but reject them here too in case
    # someone forces an unsupported triplet with --allow-unsupported-port.
    set(CAPI_TARGET_TRIPLE "x86_64-pc-windows-msvc")
    set(CAPI_ARCHIVE_EXT "zip")
    set(CAPI_SHA512 "d10ba40c03a8a038836761ddbf3549ccb261301d5aefb02668d3627ad904618d17ef28688fbc13f5ec12a33bd83ceb05d55f1a9a88d4dc8a631f06fa446337df")
else()
    message(FATAL_ERROR "dengjen-tashkeel-capi has no prebuilt binary for ${TARGET_TRIPLET}. See vcpkg.json's 'supports' expression for the platforms it ships.")
endif()

set(CAPI_ARCHIVE_STEM "dengjen-tashkeel-capi-${CAPI_TARGET_TRIPLE}")
set(CAPI_ARCHIVE_NAME "${CAPI_ARCHIVE_STEM}.${CAPI_ARCHIVE_EXT}")

vcpkg_download_distfile(CAPI_ARCHIVE
    URLS "https://github.com/ZirekHQ/dengjen-tashkeel/releases/download/v${CAPI_VERSION}/${CAPI_ARCHIVE_NAME}"
    FILENAME "${CAPI_ARCHIVE_NAME}"
    SHA512 "${CAPI_SHA512}"
)

vcpkg_extract_source_archive(
    SOURCE_PATH
    ARCHIVE "${CAPI_ARCHIVE}"
)

if(VCPKG_TARGET_IS_WINDOWS)
    # cargo-dist's archive carries the DLL only; the import library is its own release asset.
    vcpkg_download_distfile(CAPI_IMPLIB
        URLS "https://github.com/ZirekHQ/dengjen-tashkeel/releases/download/v${CAPI_VERSION}/dengjen_tashkeel_capi-${CAPI_TARGET_TRIPLE}.dll.lib"
        FILENAME "dengjen_tashkeel_capi-${CAPI_TARGET_TRIPLE}-${CAPI_VERSION}.dll.lib"
        SHA512 "080381e89262e3366da9dc8ef6e3cda9a652433b7c2a68cb4acd558a122b3e674b4458d1283b5ff7ab85023f0ec91826f9bde8678101193af33c43de54b7f0ac"
    )
    file(GLOB CAPI_DLL "${SOURCE_PATH}/*.dll")
    file(INSTALL ${CAPI_DLL} DESTINATION "${CURRENT_PACKAGES_DIR}/bin")
    file(INSTALL "${CAPI_IMPLIB}" DESTINATION "${CURRENT_PACKAGES_DIR}/lib"
        RENAME "dengjen_tashkeel_capi.dll.lib")
else()
    file(GLOB CAPI_SHARED_LIB "${SOURCE_PATH}/lib*.so" "${SOURCE_PATH}/lib*.dylib")
    file(INSTALL ${CAPI_SHARED_LIB} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
endif()

file(INSTALL "${SOURCE_PATH}/dengjen_tashkeel.h" DESTINATION "${CURRENT_PACKAGES_DIR}/include")
file(INSTALL "${CMAKE_CURRENT_LIST_DIR}/usage" DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}")
vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE-MIT" "${SOURCE_PATH}/LICENSE-APACHE")
