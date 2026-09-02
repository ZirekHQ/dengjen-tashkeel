# dengjen-tashkeel-capi ships as a prebuilt cdylib + C header in each GitHub
# Release (see https://github.com/ZirekHQ/dengjen-tashkeel/issues/23) -- there
# is no Rust toolchain available in vcpkg's build environment, so this port
# downloads the release archive for the current triplet instead of building
# from source. It ships release binaries only, so skip the debug-variant checks.
set(VCPKG_BUILD_TYPE release)

set(CAPI_VERSION "1.5.2")

# TODO(release): every SHA512 below is a placeholder. Replace them with the
# real hashes of the published assets once v${CAPI_VERSION} ships capi
# artifacts -- see packaging/README.md for the exact steps.
if(VCPKG_TARGET_IS_OSX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
    set(CAPI_TARGET_TRIPLE "aarch64-apple-darwin")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
elseif(VCPKG_TARGET_IS_LINUX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    set(CAPI_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
elseif(VCPKG_TARGET_IS_WINDOWS AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    set(CAPI_TARGET_TRIPLE "x86_64-pc-windows-msvc")
    set(CAPI_ARCHIVE_EXT "zip")
    set(CAPI_SHA512 "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
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
vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE")
