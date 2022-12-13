load("@rules_rust//rust:defs.bzl", "rust_shared_library")

exports_files(
    ["WORKSPACE"],
)

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
    deps = [
        "//cargo:cidr",
        "//cargo:lazy_static",
        "//cargo:pest",
        "//cargo:pest_consume",
        "//cargo:regex",
        "//cargo:uuid",
    ],
    proc_macro_deps = [
        "//cargo:pest_derive",
    ],
    visibility = ["//visibility:public"],
)
