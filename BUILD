load("@rules_rust//rust:defs.bzl", "rust_shared_library")
load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")

filegroup(
    name = "all_srcs",
    srcs = glob(
        include = ["**"],
        exclude = ["*.bazel"],
    ),
)

rust_shared_library(
    name = "atc_router",
    srcs = [":all_srcs"],
    aliases = aliases(),
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
    ),
    visibility = ["//visibility:public"],
    deps = all_crate_deps(
        normal = True,
    ),
)
