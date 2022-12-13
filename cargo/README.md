# Bazel project for atc-router

To add or change a crates.io dependency, run `make raze`,
then add the new dependency to `deps` in the `BUILD` file located in
the top-level directory:

```python
    rust_shared_library(
        name = "atc_router",
        srcs = [":all_srcs"],
        proc_macro_deps = [
            "//cargo:pest_derive",
        ],
        visibility = ["//visibility:public"],
        deps = [
            # ... existing dependencies
            "//cargo:new_deps",
        ],
    )
```


To use in other Bazel projects, add the following to your WORKSPACE file:

```python

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")
load("@bazel_tools//tools/build_defs/repo:utils.bzl", "maybe")

git_repository(
    name = "atc_router",
    branch = "some-tag",
    remote = "https://github.com/Kong/atc-router",
)

load("@atc_router//cargo:deps.bzl", "atc_router_repositories")

atc_router_repositories()

load("@atc_router//cargo:rust.bzl", "atc_router_dependencies")

atc_router_dependencies()
```

In your rule, add `atc_router` as dependency:

```python
configure_make(
    name = "openresty",
    # ...
    deps = [
        "@atc_router",
    ],
)
```