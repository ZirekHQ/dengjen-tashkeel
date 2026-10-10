import os

from conan import ConanFile
from conan.errors import ConanInvalidConfiguration
from conan.tools.files import copy, get


_RELEASE_ASSETS = {
    ("Macos", "armv8"): (
        "aarch64-apple-darwin",
        "tar.xz",
        "936a40bf4c81d3877bf7e390bd3eb64cd04a00777ec77007c40eb46f7f2ef284",
    ),
    ("Linux", "x86_64"): (
        "x86_64-unknown-linux-gnu",
        "tar.xz",
        "3c89228a67db4ad19d3fa91dbcdb08837073021cd06b9cbb409fde46fbe22014",
    ),
    ("Windows", "x86_64"): (
        "x86_64-pc-windows-msvc",
        "zip",
        "598a55e5a3905ea9e5ef195de73b4d3a0b5788d4361aa1ef8355e2d4c554021a",
    ),
}


_PACKAGE_LAYOUT = (
    ("*dengjen_tashkeel.h", "include"),
    ("*.so", "lib"),
    ("*.dylib", "lib"),
    ("*.dll", "bin"),
    ("*.dll.lib", "lib"),
    ("*LICENSE-MIT", "licenses"),
    ("*LICENSE-APACHE", "licenses"),
)


class DengjenTashkeelCapiConan(ConanFile):
    name = "dengjen-tashkeel-capi"
    version = "1.6.6"
    description = "Arabic-text diacritic restoration using neural networks (C API)"
    homepage = "https://github.com/ZirekHQ/dengjen-tashkeel"
    license = "MIT OR Apache-2.0"
    package_type = "shared-library"
    settings = "os", "arch", "compiler"

    def validate(self) -> None:
        if (str(self.settings.os), str(self.settings.arch)) not in _RELEASE_ASSETS:
            raise ConanInvalidConfiguration(
                f"dengjen-tashkeel-capi has no prebuilt binary for "
                f"{self.settings.os}/{self.settings.arch}"
            )
        if self.settings.os == "Windows" and self.settings.compiler != "msvc":
            raise ConanInvalidConfiguration(
                "dengjen-tashkeel-capi's Windows binary is built with MSVC; "
                f"compiler={self.settings.compiler} is not supported."
            )

    def package_id(self) -> None:
        del self.info.settings.compiler

    def build(self) -> None:
        target_triple, ext, sha256 = _RELEASE_ASSETS[
            (str(self.settings.os), str(self.settings.arch))
        ]
        archive = f"dengjen-tashkeel-capi-{target_triple}.{ext}"
        base_url = "https://github.com/ZirekHQ/dengjen-tashkeel/releases/download"
        get(
            self,
            f"{base_url}/v{self.version}/{archive}",
            sha256=sha256,
            destination=self.build_folder,
        )

    def package(self) -> None:
        for pattern, subdir in _PACKAGE_LAYOUT:
            copy(self, pattern, src=self.build_folder,
                 dst=os.path.join(self.package_folder, subdir), keep_path=False)

    def package_info(self) -> None:
        # Conan appends .lib, so the import library <crate>.dll.lib is referenced as <crate>.dll.
        is_windows = self.settings.os == "Windows"
        self.cpp_info.libs = [
            "dengjen_tashkeel_capi.dll" if is_windows else "dengjen_tashkeel_capi"
        ]
        self.cpp_info.set_property("cmake_file_name", "dengjen-tashkeel-capi")
        self.cpp_info.set_property(
            "cmake_target_name", "dengjen-tashkeel-capi::dengjen-tashkeel-capi"
        )
        self.cpp_info.set_property("pkg_config_name", "dengjen-tashkeel-capi")
