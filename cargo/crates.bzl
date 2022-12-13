"""
@generated
cargo-raze generated Bazel file.

DO NOT EDIT! Replaced on runs of cargo-raze
"""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "new_git_repository")  # buildifier: disable=load
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")  # buildifier: disable=load
load("@bazel_tools//tools/build_defs/repo:utils.bzl", "maybe")  # buildifier: disable=load

def raze_fetch_remote_crates():
    """This function defines a collection of repos and should be called in a WORKSPACE file"""
    maybe(
        http_archive,
        name = "raze__aho_corasick__0_7_20",
        url = "https://crates.io/api/v1/crates/aho-corasick/0.7.20/download",
        type = "tar.gz",
        sha256 = "cc936419f96fa211c1b9166887b38e5e40b19958e5b895be7c1f93adec7071ac",
        strip_prefix = "aho-corasick-0.7.20",
        build_file = Label("//cargo/remote:BUILD.aho-corasick-0.7.20.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__block_buffer__0_10_3",
        url = "https://crates.io/api/v1/crates/block-buffer/0.10.3/download",
        type = "tar.gz",
        sha256 = "69cce20737498f97b993470a6e536b8523f0af7892a4f928cceb1ac5e52ebe7e",
        strip_prefix = "block-buffer-0.10.3",
        build_file = Label("//cargo/remote:BUILD.block-buffer-0.10.3.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__cfg_if__1_0_0",
        url = "https://crates.io/api/v1/crates/cfg-if/1.0.0/download",
        type = "tar.gz",
        sha256 = "baf1de4339761588bc0619e3cbc0120ee582ebb74b53b4efbf79117bd2da40fd",
        strip_prefix = "cfg-if-1.0.0",
        build_file = Label("//cargo/remote:BUILD.cfg-if-1.0.0.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__cidr__0_2_1",
        url = "https://crates.io/api/v1/crates/cidr/0.2.1/download",
        type = "tar.gz",
        sha256 = "300bccc729b1ada84523246038aad61fead689ac362bb9d44beea6f6a188c34b",
        strip_prefix = "cidr-0.2.1",
        build_file = Label("//cargo/remote:BUILD.cidr-0.2.1.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__cpufeatures__0_2_5",
        url = "https://crates.io/api/v1/crates/cpufeatures/0.2.5/download",
        type = "tar.gz",
        sha256 = "28d997bd5e24a5928dd43e46dc529867e207907fe0b239c3477d924f7f2ca320",
        strip_prefix = "cpufeatures-0.2.5",
        build_file = Label("//cargo/remote:BUILD.cpufeatures-0.2.5.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__crypto_common__0_1_6",
        url = "https://crates.io/api/v1/crates/crypto-common/0.1.6/download",
        type = "tar.gz",
        sha256 = "1bfb12502f3fc46cca1bb51ac28df9d618d813cdc3d2f25b9fe775a34af26bb3",
        strip_prefix = "crypto-common-0.1.6",
        build_file = Label("//cargo/remote:BUILD.crypto-common-0.1.6.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__digest__0_10_6",
        url = "https://crates.io/api/v1/crates/digest/0.10.6/download",
        type = "tar.gz",
        sha256 = "8168378f4e5023e7218c89c891c0fd8ecdb5e5e4f18cb78f38cf245dd021e76f",
        strip_prefix = "digest-0.10.6",
        build_file = Label("//cargo/remote:BUILD.digest-0.10.6.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__generic_array__0_14_6",
        url = "https://crates.io/api/v1/crates/generic-array/0.14.6/download",
        type = "tar.gz",
        sha256 = "bff49e947297f3312447abdca79f45f4738097cc82b06e72054d2223f601f1b9",
        strip_prefix = "generic-array-0.14.6",
        build_file = Label("//cargo/remote:BUILD.generic-array-0.14.6.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__lazy_static__1_4_0",
        url = "https://crates.io/api/v1/crates/lazy_static/1.4.0/download",
        type = "tar.gz",
        sha256 = "e2abad23fbc42b3700f2f279844dc832adb2b2eb069b2df918f455c4e18cc646",
        strip_prefix = "lazy_static-1.4.0",
        build_file = Label("//cargo/remote:BUILD.lazy_static-1.4.0.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__libc__0_2_138",
        url = "https://crates.io/api/v1/crates/libc/0.2.138/download",
        type = "tar.gz",
        sha256 = "db6d7e329c562c5dfab7a46a2afabc8b987ab9a4834c9d1ca04dc54c1546cef8",
        strip_prefix = "libc-0.2.138",
        build_file = Label("//cargo/remote:BUILD.libc-0.2.138.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__memchr__2_5_0",
        url = "https://crates.io/api/v1/crates/memchr/2.5.0/download",
        type = "tar.gz",
        sha256 = "2dffe52ecf27772e601905b7522cb4ef790d2cc203488bbd0e2fe85fcb74566d",
        strip_prefix = "memchr-2.5.0",
        build_file = Label("//cargo/remote:BUILD.memchr-2.5.0.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__once_cell__1_16_0",
        url = "https://crates.io/api/v1/crates/once_cell/1.16.0/download",
        type = "tar.gz",
        sha256 = "86f0b0d4bf799edbc74508c1e8bf170ff5f41238e5f8225603ca7caaae2b7860",
        strip_prefix = "once_cell-1.16.0",
        build_file = Label("//cargo/remote:BUILD.once_cell-1.16.0.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pest__2_5_1",
        url = "https://crates.io/api/v1/crates/pest/2.5.1/download",
        type = "tar.gz",
        sha256 = "cc8bed3549e0f9b0a2a78bf7c0018237a2cdf085eecbbc048e52612438e4e9d0",
        strip_prefix = "pest-2.5.1",
        build_file = Label("//cargo/remote:BUILD.pest-2.5.1.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pest_consume__1_1_3",
        url = "https://crates.io/api/v1/crates/pest_consume/1.1.3/download",
        type = "tar.gz",
        sha256 = "79447402d15d18e7142e14c72f2e63fa3d155be1bc5b70b3ccbb610ac55f536b",
        strip_prefix = "pest_consume-1.1.3",
        build_file = Label("//cargo/remote:BUILD.pest_consume-1.1.3.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pest_consume_macros__1_1_0",
        url = "https://crates.io/api/v1/crates/pest_consume_macros/1.1.0/download",
        type = "tar.gz",
        sha256 = "9d8630a7a899cb344ec1c16ba0a6b24240029af34bdc0a21f84e411d7f793f29",
        strip_prefix = "pest_consume_macros-1.1.0",
        build_file = Label("//cargo/remote:BUILD.pest_consume_macros-1.1.0.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pest_derive__2_5_1",
        url = "https://crates.io/api/v1/crates/pest_derive/2.5.1/download",
        type = "tar.gz",
        sha256 = "cdc078600d06ff90d4ed238f0119d84ab5d43dbaad278b0e33a8820293b32344",
        strip_prefix = "pest_derive-2.5.1",
        build_file = Label("//cargo/remote:BUILD.pest_derive-2.5.1.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pest_generator__2_5_1",
        url = "https://crates.io/api/v1/crates/pest_generator/2.5.1/download",
        type = "tar.gz",
        sha256 = "28a1af60b1c4148bb269006a750cff8e2ea36aff34d2d96cf7be0b14d1bed23c",
        strip_prefix = "pest_generator-2.5.1",
        build_file = Label("//cargo/remote:BUILD.pest_generator-2.5.1.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pest_meta__2_5_1",
        url = "https://crates.io/api/v1/crates/pest_meta/2.5.1/download",
        type = "tar.gz",
        sha256 = "fec8605d59fc2ae0c6c1aefc0c7c7a9769732017c0ce07f7a9cfffa7b4404f20",
        strip_prefix = "pest_meta-2.5.1",
        build_file = Label("//cargo/remote:BUILD.pest_meta-2.5.1.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__proc_macro2__1_0_47",
        url = "https://crates.io/api/v1/crates/proc-macro2/1.0.47/download",
        type = "tar.gz",
        sha256 = "5ea3d908b0e36316caf9e9e2c4625cdde190a7e6f440d794667ed17a1855e725",
        strip_prefix = "proc-macro2-1.0.47",
        build_file = Label("//cargo/remote:BUILD.proc-macro2-1.0.47.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__quote__1_0_21",
        url = "https://crates.io/api/v1/crates/quote/1.0.21/download",
        type = "tar.gz",
        sha256 = "bbe448f377a7d6961e30f5955f9b8d106c3f5e449d493ee1b125c1d43c2b5179",
        strip_prefix = "quote-1.0.21",
        build_file = Label("//cargo/remote:BUILD.quote-1.0.21.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__regex__1_7_0",
        url = "https://crates.io/api/v1/crates/regex/1.7.0/download",
        type = "tar.gz",
        sha256 = "e076559ef8e241f2ae3479e36f97bd5741c0330689e217ad51ce2c76808b868a",
        strip_prefix = "regex-1.7.0",
        build_file = Label("//cargo/remote:BUILD.regex-1.7.0.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__regex_syntax__0_6_28",
        url = "https://crates.io/api/v1/crates/regex-syntax/0.6.28/download",
        type = "tar.gz",
        sha256 = "456c603be3e8d448b072f410900c09faf164fbce2d480456f50eea6e25f9c848",
        strip_prefix = "regex-syntax-0.6.28",
        build_file = Label("//cargo/remote:BUILD.regex-syntax-0.6.28.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__sha1__0_10_5",
        url = "https://crates.io/api/v1/crates/sha1/0.10.5/download",
        type = "tar.gz",
        sha256 = "f04293dc80c3993519f2d7f6f511707ee7094fe0c6d3406feb330cdb3540eba3",
        strip_prefix = "sha1-0.10.5",
        build_file = Label("//cargo/remote:BUILD.sha1-0.10.5.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__syn__1_0_105",
        url = "https://crates.io/api/v1/crates/syn/1.0.105/download",
        type = "tar.gz",
        sha256 = "60b9b43d45702de4c839cb9b51d9f529c5dd26a4aff255b42b1ebc03e88ee908",
        strip_prefix = "syn-1.0.105",
        build_file = Label("//cargo/remote:BUILD.syn-1.0.105.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__thiserror__1_0_37",
        url = "https://crates.io/api/v1/crates/thiserror/1.0.37/download",
        type = "tar.gz",
        sha256 = "10deb33631e3c9018b9baf9dcbbc4f737320d2b576bac10f6aefa048fa407e3e",
        strip_prefix = "thiserror-1.0.37",
        build_file = Label("//cargo/remote:BUILD.thiserror-1.0.37.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__thiserror_impl__1_0_37",
        url = "https://crates.io/api/v1/crates/thiserror-impl/1.0.37/download",
        type = "tar.gz",
        sha256 = "982d17546b47146b28f7c22e3d08465f6b8903d0ea13c1660d9d84a6e7adcdbb",
        strip_prefix = "thiserror-impl-1.0.37",
        build_file = Label("//cargo/remote:BUILD.thiserror-impl-1.0.37.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__typenum__1_16_0",
        url = "https://crates.io/api/v1/crates/typenum/1.16.0/download",
        type = "tar.gz",
        sha256 = "497961ef93d974e23eb6f433eb5fe1b7930b659f06d12dec6fc44a8f554c0bba",
        strip_prefix = "typenum-1.16.0",
        build_file = Label("//cargo/remote:BUILD.typenum-1.16.0.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__ucd_trie__0_1_5",
        url = "https://crates.io/api/v1/crates/ucd-trie/0.1.5/download",
        type = "tar.gz",
        sha256 = "9e79c4d996edb816c91e4308506774452e55e95c3c9de07b6729e17e15a5ef81",
        strip_prefix = "ucd-trie-0.1.5",
        build_file = Label("//cargo/remote:BUILD.ucd-trie-0.1.5.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__unicode_ident__1_0_5",
        url = "https://crates.io/api/v1/crates/unicode-ident/1.0.5/download",
        type = "tar.gz",
        sha256 = "6ceab39d59e4c9499d4e5a8ee0e2735b891bb7308ac83dfb4e80cad195c9f6f3",
        strip_prefix = "unicode-ident-1.0.5",
        build_file = Label("//cargo/remote:BUILD.unicode-ident-1.0.5.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__uuid__1_2_2",
        url = "https://crates.io/api/v1/crates/uuid/1.2.2/download",
        type = "tar.gz",
        sha256 = "422ee0de9031b5b948b97a8fc04e3aa35230001a722ddd27943e0be31564ce4c",
        strip_prefix = "uuid-1.2.2",
        build_file = Label("//cargo/remote:BUILD.uuid-1.2.2.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__version_check__0_9_4",
        url = "https://crates.io/api/v1/crates/version_check/0.9.4/download",
        type = "tar.gz",
        sha256 = "49874b5167b65d7193b8aba1567f5c7d93d001cafc34600cee003eda787e483f",
        strip_prefix = "version_check-0.9.4",
        build_file = Label("//cargo/remote:BUILD.version_check-0.9.4.bazel"),
    )
