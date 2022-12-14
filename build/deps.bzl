"""Setup dependencies after repostories are downloaded."""

load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")
load("@rules_rust//crate_universe:repositories.bzl", "crate_universe_dependencies")
load("@rules_rust//crate_universe:defs.bzl", "crates_repository")

def atc_router_dependencies(cargo_home_isolated = True):
    """
    atc_router_dependencies setup rust toolchain and cargo dependency repositories.

    Args:
        cargo_home_isolated (bool): cargo_home_isolated to False to reuse system CARGO_HOME
        for faster builds. cargo_home_isolated is default False and will use isolated
        Cargo home, which takes about 2 minutes to bootstrap.
    """
    rules_rust_dependencies()

    rust_register_toolchains(version = "1.65.0", edition = "2021")

    crate_universe_dependencies()

    crates_repository(
        name = "crate_index",
        cargo_lockfile = "@atc_router//:Cargo.lock",
        # lockfile = "//:Cargo.Bazel.lock",
        manifests = ["@atc_router//:Cargo.toml"],
        isolated = cargo_home_isolated,
    )
