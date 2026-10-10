import os

from conan import ConanFile
from conan.errors import ConanInvalidConfiguration
from conan.tools.files import copy, get


_RELEASE_ASSETS = {
    ("Macos", "armv8"): (
        "aarch64-apple-darwin",
        "tar.xz",
        "fddcb88329f5dd19b6038d637c077f7160e41f110316f80d7720ac48d17a2696",
    ),
    ("Linux", "x86_64"): (
        "x86_64-unknown-linux-gnu",
        "tar.xz",
        "28213d52d3d78ee58fff95f29d22c0e517b885e6318f0a571a7c263560545997",
    ),
    ("Windows", "x86_64"): (
        "x86_64-pc-windows-msvc",
        "zip",
        "cfdef273138332cf10e4f9a9799b5d2edf32221e266fcd64666d68864418552f",
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
    version = "1.5.3"
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
        # Rust's MSVC cdylib import library is named <crate>.dll.lib.
        is_windows = self.settings.os == "Windows"
        self.cpp_info.libs = [
            "dengjen_tashkeel_capi.dll" if is_windows else "dengjen_tashkeel_capi"
        ]
        self.cpp_info.set_property("cmake_file_name", "dengjen-tashkeel-capi")
        self.cpp_info.set_property(
            "cmake_target_name", "dengjen-tashkeel-capi::dengjen-tashkeel-capi"
        )
        self.cpp_info.set_property("pkg_config_name", "dengjen-tashkeel-capi")
