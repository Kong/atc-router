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

=== TEST 1: create schema, router, context
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
                                 "http.path ^= \"/foo\" && tcp.port == 80"))

            local c = context.new(s, #r:get_fields())
            c:add_value(1, "http.path", "/foo/bar")
            c:add_value(2, "tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, prefix = c:get_result("http.path")
            ngx.say(uuid)
            ngx.say(prefix)
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
/foo
--- no_error_log
[error]
[warn]
[crit]



=== TEST 2: multiple routes, different priority
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
            assert(r:add_matcher(1, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ^= \"/foo\" && tcp.port == 80"))
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150d",
                                 "http.path ^= \"/\""))

            local c = context.new(s, #r:get_fields())
            c:add_value(1, "http.path", "/foo/bar")
            c:add_value(2, "tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)


            local uuid, prefix = c:get_result("http.path")
            ngx.say("uuid = " .. uuid .. " prefix = " .. prefix)
        }
    }
--- request
GET /t
--- response_body
true
uuid = a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c prefix = /foo
--- no_error_log
[error]
[warn]
[crit]



=== TEST 3: remove matcher
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
                                 "http.path ^= \"/foo\" && tcp.port == 80"))

            local c = context.new(s, #r:get_fields())
            c:add_value(1, "http.path", "/foo/bar")
            c:add_value(2, "tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, prefix = c:get_result("http.path")
            ngx.say(uuid)
            ngx.say(prefix)

            assert(r:remove_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c"))
            assert(#r:get_fields() == 0)
            
            c = context.new(s, #r:get_fields())
            c:add_value(1, "http.path", "/foo/bar")
            c:add_value(2, "tcp.port", 80)

            matched = r:execute(c)
            ngx.say(matched)
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
/foo
false
--- no_error_log
[error]
[warn]
[crit]



=== TEST 4: invalid ATC DSL
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
            ngx.say(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                  "http.path = \"/foo\" && tcp.port == 80"))
        }
    }
--- request
GET /t
--- response_body
nil --> 1:11
  |
1 | http.path = "/foo" && tcp.port == 80
  |           ^---
  |
  = expected binary_operator
--- no_error_log
[error]
[warn]
[crit]



=== TEST 5: context:reset()
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
                                 "http.path ^= \"/foo\" && tcp.port == 80"))

            local c = context.new(s, #r:get_fields())
            c:add_value(1, "http.path", "/foo/bar")
            c:add_value(2, "tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, prefix = c:get_result("http.path")
            ngx.say(uuid)
            ngx.say(prefix)

            c:reset()

            local uuid, prefix = c:get_result("http.path")
            ngx.say(uuid)
            ngx.say(prefix)

            local matched = r:execute(c)
            ngx.say(matched)
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
/foo
nil
nil
false
--- no_error_log
[error]
[warn]
[crit]
