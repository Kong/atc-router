workspace(name = "kong")

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

http_archive(
    name = "bazel_skylib",
    sha256 = "74d544d96f4a5bb630d465ca8bbcfe231e3594e5aae57e1edbf17a6eb3ca2506",
    urls = [
        "https://mirror.bazel.build/github.com/bazelbuild/bazel-skylib/releases/download/1.3.0/bazel-skylib-1.3.0.tar.gz",
        "https://github.com/bazelbuild/bazel-skylib/releases/download/1.3.0/bazel-skylib-1.3.0.tar.gz",
    ],
)

http_archive(
    name = "rules_rust",
    sha256 = "696b01deea96a5e549f1b5ae18589e1bbd5a1d71a36a243b5cf76a9433487cf2",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.11.0/rules_rust-v0.11.0.tar.gz"],
)
load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")

rules_rust_dependencies()

rust_register_toolchains(version = "1.65.0", edition = "2021")

load("@rules_rust//crate_universe:repositories.bzl", "crate_universe_dependencies")

crate_universe_dependencies()

load("//cargo:crates.bzl", "raze_fetch_remote_crates")
raze_fetch_remote_crates()
