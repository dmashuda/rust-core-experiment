"""Hermetic Android SDK + NDK, downloaded by Bazel (no ANDROID_HOME/ANDROID_NDK_HOME).

Only what this repo uses: one platform (android.jar), one build-tools
(aapt2, d8, apksigner, zipalign) and one NDK, for macOS and x86_64 Linux
hosts. Every archive is pinned by sha256 (verified against the SHA-1s in
Google's https://dl.google.com/android/repository/repository2-3.xml).

The generated repos have exactly the shape rules_android and
rules_android_ndk expect: their own BUILD templates are reused, only the
fetching is ours. Other hosts (arm64 Linux, which Google ships no build-tools
or NDK for; Windows) get empty stubs, so Android targets are skipped there.

Not included: platform-tools (adb), the emulator and system images; those
are for running apps, not building them.
"""

_BASE_URL = "https://dl.google.com/android/repository/"

# Keep in sync with apps/android/AndroidManifest.xml (minSdkVersion).
MIN_SDK_VERSION = 26

_SDK = struct(
    api_level = "37.0",
    platform = struct(
        file = "platform-37.0_r02.zip",
        sha256 = "840b23e827f96e64aea4c89a1194aac3dc5f6bad37edb231c5c795d890330e8d",
        strip_prefix = "android-37.0",
    ),
    build_tools_version = "36.0.0",
    build_tools = {
        "darwin": struct(
            file = "build-tools_r36_macosx.zip",
            sha256 = "04e7f3a72044de4926fa038fa0e251a37bba1e1c3fb8beab6f8401bfd9eb4bf3",
        ),
        "linux": struct(
            file = "build-tools_r36_linux.zip",
            sha256 = "5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b",
        ),
    },
    # Google's 36.0.0 archives unpack to "android-16/".
    build_tools_strip_prefix = "android-16",
)

_NDK = struct(
    version = "r30",  # 30.0.16248370
    archives = {
        "darwin": struct(
            file = "android-ndk-r30-darwin.zip",
            sha256 = "d125634de97b26deb1e1bb1a562f9d839aa5803d1784a1e414485f5ccbe6739f",
        ),
        "linux": struct(
            file = "android-ndk-r30-linux.zip",
            sha256 = "753611f410d002cfcd3f3dc2ef49aad532089d3180b436c060a90bf0fcb64df2",
        ),
    },
    strip_prefix = "android-ndk-r30",
    # Darwin's prebuilt dir holds universal (arm64 + x86_64) binaries.
    clang_directory = {
        "darwin": "toolchains/llvm/prebuilt/darwin-x86_64",
        "linux": "toolchains/llvm/prebuilt/linux-x86_64",
    },
)

def _host(os_name, arch):
    """Returns "darwin", "linux", or None when Google ships no tools for the host."""
    if os_name == "mac os x":
        return "darwin"
    if os_name == "linux" and arch in ("amd64", "x86_64"):
        return "linux"
    return None

def _download(rctx, archive, output, strip_prefix):
    rctx.download_and_extract(
        url = _BASE_URL + archive.file,
        sha256 = archive.sha256,
        output = output,
        stripPrefix = strip_prefix,
    )

def _android_sdk_impl(rctx):
    host = rctx.attr.host
    if not host:
        rctx.template("BUILD.bazel", rctx.attr._empty_template)
        return

    platform_dir = "platforms/android-" + _SDK.api_level
    _download(rctx, _SDK.platform, platform_dir, _SDK.platform.strip_prefix)
    _download(
        rctx,
        _SDK.build_tools[host],
        "build-tools/" + _SDK.build_tools_version,
        _SDK.build_tools_strip_prefix,
    )

    # Same as rules_android 0.8.0's android_sdk_repository: the template
    # loads ":helper.bzl", so it must live in this repo.
    rctx.symlink(rctx.attr._helper, "helper.bzl")
    rctx.template(
        "BUILD.bazel",
        rctx.attr._template,
        substitutions = {
            "__repository_name__": rctx.name,
            "__build_tools_version__": _SDK.build_tools_version,
            "__build_tools_directory__": _SDK.build_tools_version,
            "__api_levels__": '"%s"' % _SDK.api_level,
            "__default_api_level__": _SDK.api_level,
            "__system_image_dirs__": "",
        },
    )
    return rctx.repo_metadata(reproducible = True)

_android_sdk = repository_rule(
    implementation = _android_sdk_impl,
    attrs = {
        "host": attr.string(),
        "_helper": attr.label(default = "@rules_android//rules/android_sdk_repository:helper.bzl"),
        "_template": attr.label(default = "@rules_android//rules/android_sdk_repository:template.bzl"),
        "_empty_template": attr.label(default = "@rules_android//rules/android_sdk_repository:empty.template.bzl"),
    },
)

def _android_ndk_impl(rctx):
    host = rctx.attr.host
    if not host:
        rctx.file("BUILD.bazel", "# No Android NDK for this host OS/CPU: no NDK toolchains.\n")
        return

    # Extract privately, then lay out a partial symlink tree exactly like
    # rules_android_ndk's android_ndk_repository does, so its BUILD
    # templates apply unchanged.
    _download(rctx, _NDK.archives[host], "_ndk", _NDK.strip_prefix)
    ndk = str(rctx.path("_ndk")) + "/"
    clang_directory = _NDK.clang_directory[host]
    sysroot_directory = clang_directory + "/sysroot"

    for p in rctx.path(ndk + clang_directory).readdir():
        rel = str(p).replace(ndk, "")
        if rel != sysroot_directory:
            rctx.symlink(p, rel)
    for p in rctx.path(ndk + sysroot_directory).readdir():
        rctx.symlink(p, str(p).replace(ndk, ""))
    rctx.symlink(ndk + "sources", "sources")
    rctx.symlink(ndk + "sources", "ndk/sources")  # Same hack as upstream (#32).

    result = rctx.execute([clang_directory + "/bin/clang", "--print-resource-dir"])
    if result.return_code != 0:
        fail("Failed to execute NDK clang: %s" % result.stderr)
    clang_resource_directory = result.stdout.strip().split(clang_directory)[1].strip("/")

    rctx.template("BUILD.bazel", rctx.attr._template_ndk_root, {
        "{clang_directory}": clang_directory,
    })
    rctx.template("target_systems.bzl", rctx.attr._template_target_systems, {})
    rctx.template(clang_directory + "/BUILD.bazel", rctx.attr._template_ndk_clang, {
        # The templates load helpers from rules_android_ndk's own repo.
        "{repository_name}": rctx.attr._template_ndk_root.workspace_name,
        "{api_level}": str(MIN_SDK_VERSION),
        "{clang_resource_directory}": clang_resource_directory,
        "{sysroot_directory}": sysroot_directory,
        "{executable_extension}": "",
    })
    rctx.template(sysroot_directory + "/BUILD.bazel", rctx.attr._template_ndk_sysroot, {
        "{api_level}": str(MIN_SDK_VERSION),
    })
    return rctx.repo_metadata(reproducible = True)

_android_ndk = repository_rule(
    implementation = _android_ndk_impl,
    attrs = {
        "host": attr.string(),
        "_template_ndk_root": attr.label(default = "@rules_android_ndk//:BUILD.ndk_root.tpl"),
        "_template_target_systems": attr.label(default = "@rules_android_ndk//:target_systems.bzl.tpl"),
        "_template_ndk_clang": attr.label(default = "@rules_android_ndk//:BUILD.ndk_clang.tpl"),
        "_template_ndk_sysroot": attr.label(default = "@rules_android_ndk//:BUILD.ndk_sysroot.tpl"),
    },
)

def _status_impl(rctx):
    rctx.file("BUILD.bazel", "")
    rctx.file("defs.bzl", "HAS_ANDROID_TOOLCHAIN = {}\n".format(rctx.attr.present))

_status = repository_rule(
    implementation = _status_impl,
    attrs = {"present": attr.bool()},
)

def _android_toolchains_impl(module_ctx):
    host = _host(module_ctx.os.name, module_ctx.os.arch) or ""
    _android_sdk(name = "androidsdk", host = host)
    _android_ndk(name = "androidndk", host = host)
    _status(name = "android_toolchain_status", present = bool(host))
    return module_ctx.extension_metadata(
        root_module_direct_deps = ["androidsdk", "androidndk", "android_toolchain_status"],
        root_module_direct_dev_deps = [],
        # The result depends only on the host OS/CPU.
        reproducible = True,
    )

android_toolchains = module_extension(
    implementation = _android_toolchains_impl,
    os_dependent = True,
    arch_dependent = True,
)
