"""Makes the Android NDK optional.

rules_android_ndk's own extension fail()s when $ANDROID_NDK_HOME is unset, and
because its toolchains are registered globally, Bazel fetches that repo during
*any* C/C++ toolchain resolution. Without this wrapper, a machine with no NDK
(Linux CI, a Mac without Android Studio) cannot even build //core.
"""

load("@rules_android_ndk//:rules.bzl", "android_ndk_repository")

def _ndk_stub_impl(rctx):
    rctx.file("BUILD.bazel", "# $ANDROID_NDK_HOME is not set: no Android NDK toolchains.\n")

_ndk_stub = repository_rule(implementation = _ndk_stub_impl)

def _ndk_status_impl(rctx):
    rctx.file("BUILD.bazel", "")
    rctx.file("defs.bzl", "HAS_ANDROID_NDK = {}\n".format(rctx.attr.present))

_ndk_status = repository_rule(
    implementation = _ndk_status_impl,
    attrs = {"present": attr.bool()},
)

def _optional_android_ndk_impl(module_ctx):
    # module_ctx.getenv records the variable, so changing it re-runs this.
    present = bool(module_ctx.getenv("ANDROID_NDK_HOME"))
    if present:
        android_ndk_repository(name = "androidndk")
    else:
        _ndk_stub(name = "androidndk")
    _ndk_status(name = "android_ndk_status", present = present)

optional_android_ndk = module_extension(implementation = _optional_android_ndk_impl)
