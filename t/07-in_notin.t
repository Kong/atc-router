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

=== TEST 1: in operator has correct type check
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
                                  "tcp.port in 80"))

            ngx.say(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                  "http.path in 80"))

        }
    }
--- request
GET /t
--- response_body
nilIn/NotIn operators only supports string in string or IP in CIDR
nilIn/NotIn operators only supports string in string or IP in CIDR
--- no_error_log
[error]
[warn]
[crit]



=== TEST 2: in operator works with String operands
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
                                 "http.path in \"/foo/bar\""))

            local c = context.new(s)
            c:add_value("http.path", "foo/")

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, prefix = c:get_result("http.path")
            ngx.say(uuid)
            ngx.say(prefix)

            c:add_value("http.path", "nxfoo/")
            local matched = r:execute(c)
            ngx.say(matched)
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
nil
false
--- no_error_log
[error]
[warn]
[crit]



=== TEST 3: in operator works with IPAddr and IpCidr operands
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("l3.ip", "IpAddr")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "l3.ip in 192.168.12.0/24"))

            local c = context.new(s)
            c:add_value("l3.ip", "192.168.12.1")

            local matched = r:execute(c)
            ngx.say(matched)

            c:add_value("l3.ip", "192.168.1.1")

            local matched = r:execute(c)
            ngx.say(matched)
        }
    }
--- request
GET /t
--- response_body
true
false
--- no_error_log
[error]
[warn]
[crit]
