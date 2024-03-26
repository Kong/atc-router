"""Setup Crates repostories """

load("@atc_router_crate_index//:defs.bzl", "crate_repositories")

def atc_router_crates():
    crate_repositories()
