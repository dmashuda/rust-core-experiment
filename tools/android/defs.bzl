"""Shared settings for Android targets."""

load("@android_ndk_status//:defs.bzl", "HAS_ANDROID_NDK")

# Put on every Android target: without a local SDK ($ANDROID_HOME) and NDK
# ($ANDROID_NDK_HOME) the targets are skipped by `bazel build //...` instead
# of failing it. (@androidsdk//:has_androidsdk comes from rules_android's
# no-SDK stub vs. real SDK repo.)
ANDROID_TARGET_COMPATIBLE_WITH = select({
    "@androidsdk//:has_androidsdk": [] if HAS_ANDROID_NDK else ["@platforms//:incompatible"],
    "//conditions:default": ["@platforms//:incompatible"],
})
