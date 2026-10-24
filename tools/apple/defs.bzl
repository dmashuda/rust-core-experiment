"""Shared settings for Apple (Swift/Xcode) targets."""

load("@platforms//host:constraints.bzl", "HOST_CONSTRAINTS")

# Put on every Swift/Apple target. These need Xcode on the machine running
# Bazel, regardless of the target platform (iOS builds still need a Mac), so
# this keys off the *host*, not the target. On Linux the targets are skipped
# by `bazel build //...` instead of failing it. (Without this, rules_swift's
# globally registered Linux Swift toolchain points at an autodetected
# `swiftc`, which is a dangling label when Swift isn't installed.)
APPLE_TARGET_COMPATIBLE_WITH = [] if "@platforms//os:osx" in HOST_CONSTRAINTS else ["@platforms//:incompatible"]
