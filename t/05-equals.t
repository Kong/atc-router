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

=== TEST 1: multi value field
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.headers.foo", "String")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.headers.foo == \"bar\""))

            local c = context.new(r)
            c:add_value("http.headers.foo", "bar")
            c:add_value("http.headers.foo", "bar")
            c:add_value("http.headers.foo", "bar")

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, prefix = c:get_result("http.headers.foo")
            ngx.say(uuid)
            ngx.say(prefix)
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
bar
--- no_error_log
[error]
[warn]
[crit]




=== TEST 2: multi value field expect mismatch
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.headers.foo", "String")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.headers.foo == \"bar\""))

            local c = context.new(r)
            c:add_value("http.headers.foo", "bar")
            c:add_value("http.headers.foo", "bar")
            c:add_value("http.headers.foo", "barX")

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, prefix = c:get_result("http.headers.foo")
            ngx.say(uuid)
            ngx.say(prefix)
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


=== TEST 3: empty value
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.headers.foo", "String")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.headers.foo == \"bar\""))

            local c = context.new(r)

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid, prefix = c:get_result("http.headers.foo")
            ngx.say(uuid)
            ngx.say(prefix)
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
