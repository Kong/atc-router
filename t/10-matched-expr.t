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

=== TEST 1: multiple regexes OR'd together, request hits the first branch
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()
            s:add_field("http.path", "String")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ~ r#\"^/[a-z0-9_-]+/v2/authenticate\"# || http.path ~ r#\"^/[a-z0-9_-]+/v2/fetch-principal\"#"))

            local c = context.new(s)
            c:add_value("http.path", "/foo/v2/authenticate")

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, matched_value, _, matched_expr = c:get_result("http.path")
            ngx.say(matched_value)
            ngx.say(matched_expr)
        }
    }
--- request
GET /t
--- response_body
true
/foo/v2/authenticate
^/[a-z0-9_-]+/v2/authenticate
--- no_error_log
[error]
[warn]
[crit]



=== TEST 2: multiple regexes OR'd together, request hits the second branch
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()
            s:add_field("http.path", "String")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ~ r#\"^/[a-z0-9_-]+/v2/authenticate\"# || http.path ~ r#\"^/[a-z0-9_-]+/v2/fetch-principal\"#"))

            local c = context.new(s)
            c:add_value("http.path", "/foo/v2/fetch-principal")

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, matched_value, _, matched_expr = c:get_result("http.path")
            ngx.say(matched_value)
            ngx.say(matched_expr)
        }
    }
--- request
GET /t
--- response_body
true
/foo/v2/fetch-principal
^/[a-z0-9_-]+/v2/fetch-principal
--- no_error_log
[error]
[warn]
[crit]
