# vim:set ft= ts=4 sw=4 et:

use Test::Nginx::Socket::Lua;
use Cwd qw(cwd);

repeat_each(1);

plan tests => repeat_each() * blocks() * 5;

my $pwd = cwd();

our $HttpConfig = qq{
    lua_package_path "$pwd/lib/?.lua;;";
    lua_package_cpath "$pwd/target/debug/?.so;;";
};

no_long_string();
no_diff();

run_tests();

__DATA__

=== TEST 1: gc schema, router
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            local r = router.new(s)

            schema = nil
            router = nil

            rawset(package.loaded, "resty.router.schema", nil)
            rawset(package.loaded, "resty.router.router", nil)
            rawset(package.loaded, "resty.router.cdefs", nil)

            collectgarbage()

            s = nil
            r = nil

            collectgarbage()

            ngx.say("ok")
        }
    }
--- request
GET /t
--- response_body
ok
--- no_error_log
[error]
[warn]
[crit]



