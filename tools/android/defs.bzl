"""Shared settings for Android targets."""

load("@android_toolchain_status//:defs.bzl", "HAS_ANDROID_TOOLCHAIN")

# Put on every Android target: on hosts where Google ships no SDK/NDK tools
# (arm64 Linux, Windows) the targets are skipped by `bazel build //...`
# instead of failing it. See tools/android/toolchains.bzl.
ANDROID_TARGET_COMPATIBLE_WITH = [] if HAS_ANDROID_TOOLCHAIN else ["@platforms//:incompatible"]
