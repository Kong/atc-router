# vim:set ft= ts=4 sw=4 et:

use Test::Nginx::Socket::Lua;
use Cwd qw(cwd);

repeat_each(2);

plan tests => repeat_each() * blocks() * 5;

my $pwd = cwd();

our $HttpConfig = qq{
    lua_package_path "$pwd/lib/?.lua;;";
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
            assert(r:add_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ^= \"/foo\" && tcp.port == 80"))

            local c = context.new(s)
            c:add_value("http.path", "/foo/bar")
            c:add_value("tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            ngx.say(c:get_matched_count())

            local uuid, prefix = c:get_match(0)
            ngx.say(uuid)
            ngx.say(prefix)
        }
    }
--- request
GET /t
--- response_body
true
1
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
/foo
--- no_error_log
[error]
[warn]
[crit]



=== TEST 2: multiple routes
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
            assert(r:add_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ^= \"/foo\" && tcp.port == 80"))
            assert(r:add_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150d",
                                 "http.path ^= \"/\""))

            local c = context.new(s)
            c:add_value("http.path", "/foo/bar")
            c:add_value("tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            ngx.say(c:get_matched_count())

            local matches = {}

            local uuid, prefix = c:get_match(0)
            matches[1] = { uuid = uuid, prefix = prefix, }

            uuid, prefix = c:get_match(1)
            matches[2] = { uuid = uuid, prefix = prefix, }

            table.sort(matches, function(a, b)
                return a.uuid < b.uuid
            end)

            for i, m in ipairs(matches) do
                ngx.say("i = " .. i .. " uuid = " .. m.uuid .. " prefix = " .. m.prefix)
            end
        }
    }
--- request
GET /t
--- response_body
true
2
i = 1 uuid = a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c prefix = /foo
i = 2 uuid = a921a9aa-ec0e-4cf3-a6cc-1aa5583d150d prefix = /
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
            assert(r:add_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ^= \"/foo\" && tcp.port == 80"))

            local c = context.new(s)
            c:add_value("http.path", "/foo/bar")
            c:add_value("tcp.port", 80)

            local matched = r:execute(c)
            ngx.say(matched)

            ngx.say(c:get_matched_count())

            local uuid, prefix = c:get_match(0)
            ngx.say(uuid)
            ngx.say(prefix)

            assert(r:remove_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c"))

            c = context.new(s)
            c:add_value("http.path", "/foo/bar")
            c:add_value("tcp.port", 80)

            matched = r:execute(c)
            ngx.say(matched)

            ngx.say(c:get_matched_count())
        }
    }
--- request
GET /t
--- response_body
true
1
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
/foo
false
0
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
            ngx.say(r:add_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
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
