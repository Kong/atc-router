"""Setup repostories."""

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def atc_router_repositories():
    http_archive(
        name = "bazel_skylib",
        sha256 = "bc283cdfcd526a52c3201279cda4bc298652efa898b10b4db0837dc51652756f",
        urls = [
            "https://mirror.bazel.build/github.com/bazelbuild/bazel-skylib/releases/download/1.7.1/bazel-skylib-1.7.1.tar.gz",
            "https://github.com/bazelbuild/bazel-skylib/releases/download/1.7.1/bazel-skylib-1.7.1.tar.gz",
        ],
    )


    http_archive(
        name = "rules_rust",
        integrity = "sha256-Zx3bP+Xrz53TTQUeynNS+68z+lO/Ye7Qt1pMNIKeVIA=",
        urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.52.2/rules_rust-v0.52.2.tar.gz"],
    )
