# dengjen-tashkeel-capi ships as a prebuilt cdylib + C header in each GitHub
# Release (see https://github.com/ZirekHQ/dengjen-tashkeel/issues/23) -- there
# is no Rust toolchain available in vcpkg's build environment, so this port
# downloads the release archive for the current triplet instead of building
# from source. It ships release binaries only, so skip the debug-variant checks
# -- VCPKG_BUILD_TYPE alone doesn't suppress the post-build binary-count lint
# for a port that never calls vcpkg_cmake_configure/build, hence the policy too.
set(VCPKG_BUILD_TYPE release)
set(VCPKG_POLICY_MISMATCHED_NUMBER_OF_BINARIES enabled)

set(CAPI_VERSION "1.5.3")

# SHA512s below are the actual hashes of the v${CAPI_VERSION} release assets,
# cross-checked against the .sha256 files published alongside them.
if(VCPKG_TARGET_IS_OSX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
    set(CAPI_TARGET_TRIPLE "aarch64-apple-darwin")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "12b58ba7ab4edc383016657e196d247ceaf816cc6d0fd080400c3562890b33c66cac8525f97c195e66a9390c55736df513ac1358910a74a1194d2eca359801da")
elseif(VCPKG_TARGET_IS_LINUX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    set(CAPI_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "a8d5093ca4151bb1c7a132bb744d33ace952070a2010b2c3b00c40876f3c2c7c66a00443213ce2d466eff608de56aae6ad51130c16c8af4f7e1a7036fe7e947e")
elseif(VCPKG_TARGET_IS_WINDOWS AND NOT VCPKG_TARGET_IS_MINGW AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    # The published archive is MSVC-built; vcpkg.json's 'supports' expression
    # already excludes MinGW triplets, but reject them here too in case
    # someone forces an unsupported triplet with --allow-unsupported-port.
    set(CAPI_TARGET_TRIPLE "x86_64-pc-windows-msvc")
    set(CAPI_ARCHIVE_EXT "zip")
    set(CAPI_SHA512 "08714cb000aed92a24ae68932f269bec61098b423c0172ef709eec8bb589c2ab84ce1a6a6c5ad812154c5e1d146a375b1b39545476639dacf5aebe42ee22baf4")
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
