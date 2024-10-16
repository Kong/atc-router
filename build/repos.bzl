"""Setup repostories."""

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def atc_router_repositories():
    http_archive(
        name = "rules_cc",
        sha256 = "2037875b9a4456dce4a79d112a8ae885bbc4aad968e6587dca6e64f3a0900cdf",
        strip_prefix = "rules_cc-0.0.9",
        urls = [
            "https://github.com/bazelbuild/rules_cc/releases/download/0.0.9/rules_cc-0.0.9.tar.gz"
        ],
    )

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
        integrity = "sha256-CRrEsKZ7wlLCyE2Eu7lBq3mrwecfvTIk2kr9+6c3VPA=",
        urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.52.1/rules_rust-v0.52.1.tar.gz"],
    )
