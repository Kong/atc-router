# vim:set ft= ts=4 sw=4 et:

use Test::Nginx::Socket::Lua;
use Cwd qw(cwd);

repeat_each(2);

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

=== TEST 1: contains operator
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.path", "String")
            s:add_field("tcp.port", "Int")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path contains \"keyword\" && tcp.port == 80"))

            local c = context.new(s, #r:get_fields())
            c:add_value(1, "http.path", "/foo/keyword/bar")
            c:add_value(2, "tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, matched_value = c:get_result("http.path")
            ngx.say(uuid)
            ngx.say(matched_value)
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
nil
--- no_error_log
[error]
[warn]
[crit]




=== TEST 2: contains operator should mismatch
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.path", "String")
            s:add_field("tcp.port", "Int")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path contains \"keyword\" && tcp.port == 80"))

            local c = context.new(s, #r:get_fields())
            c:add_value(1, "http.path", "/foo/bar")
            c:add_value(2, "tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, matched_value = c:get_result("http.path")
            ngx.say(uuid)
            ngx.say(matched_value)
        }
    }
--- request
GET /t
--- response_body
false
nil
nil
--- no_error_log
[error]
[warn]
[crit]

