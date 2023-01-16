workspace(name = "atc_router")

load("//build:repos.bzl", "atc_router_repositories")

atc_router_repositories()

load("//build:deps.bzl", "atc_router_dependencies")

atc_router_dependencies(cargo_home_isolated = False)  # use system `$CARGO_HOME` to speed up builds

load("//build:crates.bzl", "atc_router_crates")

atc_router_crates()
