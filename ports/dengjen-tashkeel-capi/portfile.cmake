# dengjen-tashkeel-capi ships as a prebuilt cdylib + C header in each GitHub
# Release (see https://github.com/ZirekHQ/dengjen-tashkeel/issues/23) -- there
# is no Rust toolchain available in vcpkg's build environment, so this port
# downloads the release archive for the current triplet instead of building
# from source. It ships release binaries only, so skip the debug-variant checks
# -- VCPKG_BUILD_TYPE alone doesn't suppress the post-build binary-count lint
# for a port that never calls vcpkg_cmake_configure/build, hence the policy too.
set(VCPKG_BUILD_TYPE release)
set(VCPKG_POLICY_MISMATCHED_NUMBER_OF_BINARIES enabled)

set(CAPI_VERSION "1.5.2")

# SHA512s below are the actual hashes of the v${CAPI_VERSION} release assets,
# cross-checked against the .sha256 files published alongside them.
if(VCPKG_TARGET_IS_OSX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
    set(CAPI_TARGET_TRIPLE "aarch64-apple-darwin")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "e4f94cfca5cd7e5fd650b365a1d87b68410f97f31b35fe9f8c6621ade98bf04dcd454327c55b5ac47f73ca0c762d41e79b1eef43e9525a06e04e9019eb709e37")
elseif(VCPKG_TARGET_IS_LINUX AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    set(CAPI_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
    set(CAPI_ARCHIVE_EXT "tar.xz")
    set(CAPI_SHA512 "153fec1a58b650a4bdb726618a951c16c94f1553886e8b05a125d8474e36a437723c0c2adf5ef774464e426ca40b3777f224064ffb52b5b818be345cad4f7d54")
elseif(VCPKG_TARGET_IS_WINDOWS AND NOT VCPKG_TARGET_IS_MINGW AND VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    # The published archive is MSVC-built; vcpkg.json's 'supports' expression
    # already excludes MinGW triplets, but reject them here too in case
    # someone forces an unsupported triplet with --allow-unsupported-port.
    set(CAPI_TARGET_TRIPLE "x86_64-pc-windows-msvc")
    set(CAPI_ARCHIVE_EXT "zip")
    set(CAPI_SHA512 "7fb26aa682b1483b6ac8f743b9ed7379aa6e61b7176c6358fd31c689edbd7308500e761d229cf47fa273ed45f5ac2df25d2a3a4d36e3742153dd26c7c024c2db")
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
