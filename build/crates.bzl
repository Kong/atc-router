"""Setup Crates repostories """

load("@crate_index//:defs.bzl", "crate_repositories")

def atc_router_crates():
    crate_repositories()
