load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")
load("@rules_rust//crate_universe:repositories.bzl", "crate_universe_dependencies")
load("//cargo:crates.bzl", "raze_fetch_remote_crates")

def atc_router_dependencies():
    rules_rust_dependencies()

    rust_register_toolchains(version = "1.65.0", edition = "2021")

    crate_universe_dependencies()

    raze_fetch_remote_crates()
