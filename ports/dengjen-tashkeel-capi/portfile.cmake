# No Rust toolchain in vcpkg's build environment, so this port downloads the
# prebuilt release archive for the current triplet instead of building from
# source (see https://github.com/ZirekHQ/dengjen-tashkeel/issues/23).
vcpkg_check_linkage(ONLY_DYNAMIC_LIBRARY)
set(VCPKG_BUILD_TYPE release)
# VCPKG_BUILD_TYPE alone doesn't suppress the post-build binary-count lint
# for a port that never calls vcpkg_cmake_configure/build.
set(VCPKG_POLICY_MISMATCHED_NUMBER_OF_BINARIES enabled)

set(CAPI_VERSION "1.6.6")

# SHA512s below are the actual hashes of the v${CAPI_VERSION} release assets,
# cross-checked against the .sha256 files published alongside them.
if(VCPKG_TARGET_IS_OSX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
    set(CAPI_TARGET_TRIPLE "aarch64-apple-darwin")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "82fc97349a7929f53d61ba28a1be7d2c98aa59c15ed14f7cfeeb7963aafc733ec4eea200a1d9e34242cbef96df1b895bfd7aa9746a1b4f5c480598b52b579928")
elseif(VCPKG_TARGET_IS_LINUX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    set(CAPI_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "944f63e34063a29744e6777955a44dbb396fba9c4ad553072f91787d9f32a550379d4a9148a790c4e350bc5faea056d3c2b2edb57c7789053b09790d20d89ba4")
elseif(VCPKG_TARGET_IS_WINDOWS AND NOT VCPKG_TARGET_IS_MINGW AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    # The published archive is MSVC-built; vcpkg.json's 'supports' expression
    # already excludes MinGW triplets, but reject them here too in case
    # someone forces an unsupported triplet with --allow-unsupported-port.
    set(CAPI_TARGET_TRIPLE "x86_64-pc-windows-msvc")
    set(CAPI_ARCHIVE_EXT "zip")
    set(CAPI_SHA512 "402e4e31bed932c7c6bba2cf47ac5431fba5fa30f26b699952ddcd9a5e6330f9ef4869974d78488ff94d8f7ac71ce5ffed4a2fc5fb75955807e77540f645957d")
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
    file(GLOB CAPI_DLL "${SOURCE_PATH}/*.dll")
    file(GLOB CAPI_IMPLIB "${SOURCE_PATH}/*.dll.lib")
    file(INSTALL ${CAPI_DLL} DESTINATION "${CURRENT_PACKAGES_DIR}/bin")
    file(INSTALL ${CAPI_IMPLIB} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
else()
    file(GLOB CAPI_SHARED_LIB "${SOURCE_PATH}/lib*.so" "${SOURCE_PATH}/lib*.dylib")
    file(INSTALL ${CAPI_SHARED_LIB} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
endif()

file(INSTALL "${SOURCE_PATH}/dengjen_tashkeel.h" DESTINATION "${CURRENT_PACKAGES_DIR}/include")
file(INSTALL "${CMAKE_CURRENT_LIST_DIR}/usage" DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}")
vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE-MIT" "${SOURCE_PATH}/LICENSE-APACHE")
