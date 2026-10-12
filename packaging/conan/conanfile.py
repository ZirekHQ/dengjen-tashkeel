import glob
import os

from conan import ConanFile
from conan.errors import ConanInvalidConfiguration
from conan.tools.files import copy, download, get


_RELEASE_ASSETS = {
    ("Macos", "armv8"): (
        "aarch64-apple-darwin",
        "tar.xz",
        "c5fa816c7c213056c2512d0c5b88022585bd2b3cced4152f6a51dae6a8fb55b1",
    ),
    ("Linux", "x86_64"): (
        "x86_64-unknown-linux-gnu",
        "tar.xz",
        "6cd4a34e19b5341e932a933ee693b23dfb7b8a86b9e53ffd55d2243e8c18cdd4",
    ),
    ("Windows", "x86_64"): (
        "x86_64-pc-windows-msvc",
        "zip",
        "e791d8af5516f8c7f8531f07221e85a69f012d051c9eb622e3268d5ab75d1a38",
    ),
}


# cargo-dist's Windows archive carries the DLL only; the import library is a separate asset.
_WINDOWS_IMPORT_LIBRARY = (
    "dengjen_tashkeel_capi-x86_64-pc-windows-msvc.dll.lib",
    "0a185d6288fa1cfe2bbd8e35dfdb63aea9810216fceb78ed7945b6b0150a2e2a",
)


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
    version = "2.0.0"
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
        if self.settings.os == "Windows":
            asset, import_library_sha256 = _WINDOWS_IMPORT_LIBRARY
            download(
                self,
                f"{base_url}/v{self.version}/{asset}",
                os.path.join(self.build_folder, "dengjen_tashkeel_capi.dll.lib"),
                sha256=import_library_sha256,
            )

    def package(self) -> None:
        for pattern, subdir in _PACKAGE_LAYOUT:
            copy(self, pattern, src=self.build_folder,
                 dst=os.path.join(self.package_folder, subdir), keep_path=False)
        if self.settings.os == "Macos":
            lib_dir = os.path.join(self.package_folder, "lib")
            for dylib in glob.glob(os.path.join(lib_dir, "*.dylib")):
                self.run(f'install_name_tool -id "@rpath/{os.path.basename(dylib)}" "{dylib}"')

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
