load("@rules_rust//rust:defs.bzl", "rust_shared_library")

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
    proc_macro_deps = [
        "//cargo:pest_derive",
    ],
    visibility = ["//visibility:public"],
    deps = [
        "//cargo:cidr",
        "//cargo:lazy_static",
        "//cargo:pest",
        "//cargo:pest_consume",
        "//cargo:regex",
        "//cargo:uuid",
    ],
)
