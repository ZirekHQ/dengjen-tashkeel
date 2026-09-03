import os

from conan import ConanFile
from conan.errors import ConanInvalidConfiguration
from conan.tools.files import copy, get

# dengjen-tashkeel-capi ships as a prebuilt cdylib + C header in each GitHub
# Release (see https://github.com/ZirekHQ/dengjen-tashkeel/issues/23) -- Conan
# has no Rust build environment, so this recipe downloads the release archive
# for the current os/arch instead of building from source.
#
# ConanCenter requires recipes to build from source, so this one isn't
# submitted there. Consumers use it locally instead:
#
#   conan create packaging/conan --version=1.5.3
#
# which builds and stores dengjen-tashkeel-capi/1.5.3 in their local cache.

# sha256s below are the actual hashes of the v1.5.3 release assets,
# cross-checked against the .sha256 files published alongside them.
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


class DengjenTashkeelCapiConan(ConanFile):
    name = "dengjen-tashkeel-capi"
    version = "1.5.3"
    description = "Arabic-text diacritic restoration using neural networks (C API)"
    homepage = "https://github.com/ZirekHQ/dengjen-tashkeel"
    license = "MIT OR Apache-2.0"
    settings = "os", "arch", "compiler"

    def validate(self):
        if (str(self.settings.os), str(self.settings.arch)) not in _RELEASE_ASSETS:
            raise ConanInvalidConfiguration(
                f"dengjen-tashkeel-capi has no prebuilt binary for "
                f"{self.settings.os}/{self.settings.arch}"
            )
        # The published Windows archive is MSVC-built (MSVC-linked import
        # lib, MSVC CRT) -- a MinGW profile can't link against it.
        if self.settings.os == "Windows" and self.settings.compiler != "msvc":
            raise ConanInvalidConfiguration(
                "dengjen-tashkeel-capi's Windows binary is built with MSVC; "
                f"compiler={self.settings.compiler} is not supported."
            )

    def package_id(self):
        # Prebuilt release-only binary: one package per os/arch, independent
        # of the consumer's build_type. compiler is dropped from the id too
        # except that validate() above already rejects non-msvc on Windows,
        # so no compiler variant reaches this point on that platform.
        # package_id() can only read/mutate self.info.settings (a pre-seeded
        # copy of self.settings) -- reading self.settings itself is forbidden
        # here, so drop the unwanted axis instead of rebuilding it from scratch.
        del self.info.settings.compiler

    def build(self):
        target_triple, ext, sha256 = _RELEASE_ASSETS[
            (str(self.settings.os), str(self.settings.arch))
        ]
        archive = f"dengjen-tashkeel-capi-{target_triple}.{ext}"
        get(
            self,
            f"https://github.com/ZirekHQ/dengjen-tashkeel/releases/download/v{self.version}/{archive}",
            sha256=sha256,
            destination=self.build_folder,
        )

    def package(self):
        # get() extracts the release archive into build_folder as-is, which
        # means every file lands under a top-level
        # dengjen-tashkeel-capi-<target-triple>/ directory (that's how the
        # archives are built). A bare filename pattern only matches Conan's
        # copy() at that exact relative path, so it silently copies nothing
        # for a nested file -- hence the leading "*" on every pattern here,
        # plus keep_path=False so the package layout is flat regardless of
        # how the archive is laid out.
        copy(self, "*dengjen_tashkeel.h", src=self.build_folder,
             dst=os.path.join(self.package_folder, "include"), keep_path=False)
        copy(self, "*.so", src=self.build_folder,
             dst=os.path.join(self.package_folder, "lib"), keep_path=False)
        copy(self, "*.dylib", src=self.build_folder,
             dst=os.path.join(self.package_folder, "lib"), keep_path=False)
        copy(self, "*.dll", src=self.build_folder,
             dst=os.path.join(self.package_folder, "bin"), keep_path=False)
        copy(self, "*.dll.lib", src=self.build_folder,
             dst=os.path.join(self.package_folder, "lib"), keep_path=False)
        copy(self, "*LICENSE-MIT", src=self.build_folder,
             dst=os.path.join(self.package_folder, "licenses"), keep_path=False)
        copy(self, "*LICENSE-APACHE", src=self.build_folder,
             dst=os.path.join(self.package_folder, "licenses"), keep_path=False)

    def package_info(self):
        self.cpp_info.libs = ["dengjen_tashkeel_capi"]
