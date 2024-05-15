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

=== TEST 1: Equals/NotEquals works Int
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("net.port", "Int")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "net.port == 8000"))
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-8aa5583d150c",
                                 "net.port != 8000"))

            local c = context.new(s)
            c:add_value("net.port", 8000)

            local matched = r:execute(c)
            ngx.say(matched)
            ngx.say(c:get_result())

            c = context.new(s)
            c:add_value("net.port", 8001)

            matched = r:execute(c)
            ngx.say(matched)
            ngx.say(c:get_result())
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150cnilnil0
true
a921a9aa-ec0e-4cf3-a6cc-8aa5583d150cnilnil0
--- no_error_log
[error]
[warn]
[crit]



=== TEST 2: Equals/NotEquals works String
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
                                 "http.path == \"/foo\""))
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-8aa5583d150c",
                                 "http.path != \"/foo\""))

            local c = context.new(s)
            c:add_value("http.path", "/foo")

            local matched = r:execute(c)
            ngx.say(matched)
            ngx.say(c:get_result())

            c = context.new(s)
            c:add_value("http.path", "/foo1")

            matched = r:execute(c)
            ngx.say(matched)
            ngx.say(c:get_result())
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150cnilnil0
true
a921a9aa-ec0e-4cf3-a6cc-8aa5583d150cnilnil0
--- no_error_log
[error]
[warn]
[crit]



=== TEST 3: Equals/NotEquals works IpAddr
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("net.ip", "IpAddr")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "net.ip == 192.168.1.1"))
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-8aa5583d150c",
                                 "net.ip != 192.168.1.1"))

            local c = context.new(s)
            c:add_value("net.ip", "192.168.1.1")

            local matched = r:execute(c)
            ngx.say(matched)
            ngx.say(c:get_result())

            c = context.new(s)
            c:add_value("net.ip", "192.168.1.2")

            matched = r:execute(c)
            ngx.say(matched)
            ngx.say(c:get_result())
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150cnilnil0
true
a921a9aa-ec0e-4cf3-a6cc-8aa5583d150cnilnil0
--- no_error_log
[error]
[warn]
[crit]
