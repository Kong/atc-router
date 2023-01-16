# Bazel project for atc-router


To use in other Bazel projects, add the following to your WORKSPACE file:

```python

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")
load("@bazel_tools//tools/build_defs/repo:utils.bzl", "maybe")

git_repository(
    name = "atc_router",
    branch = "some-tag",
    remote = "https://github.com/Kong/atc-router",
)

load("@atc_router//build:repos.bzl", "atc_router_repositories")

atc_router_repositories()

load("@atc_router//build:deps.bzl", "atc_router_dependencies")

atc_router_dependencies(cargo_home_isolated = False) # use system `$CARGO_HOME` to speed up builds

load("@atc_router//build:crates.bzl", "atc_router_crates")

atc_router_crates()


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