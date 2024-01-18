"""Setup repostories."""

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def atc_router_repositories():
    http_archive(
        name = "bazel_skylib",
        sha256 = "cd55a062e763b9349921f0f5db8c3933288dc8ba4f76dd9416aac68acee3cb94"
        urls = [
            "https://mirror.bazel.build/github.com/bazelbuild/bazel-skylib/releases/download/1.5.0/bazel-skylib-1.5.0.tar.gz",
            "https://github.com/bazelbuild/bazel-skylib/releases/download/1.5.0/bazel-skylib-1.5.0.tar.gz",
        ],
    )

    http_archive(
        name = "rules_rust",
        sha256 = "a761d54e49db06f863468e6bba4a13252b1bd499e8f706da65e279b3bcbc5c52",
        urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.36.2/rules_rust-v0.36.2.tar.gz"],
    )
