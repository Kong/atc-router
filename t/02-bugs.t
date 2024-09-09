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

=== TEST 1: invalid UTF-8 sequence returns the decoding error
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local context = require("resty.router.context")
            local router = require("resty.router.router")

            local s = schema.new()

            s:add_field("http.path", "String")
            local r = router.new()
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ^= \"/foo\" && tcp.port == 80"))

            local BAD_UTF8 = {
                "\x80",
                "\xbf",
                "\xfc\x80\x80\x80\x80\xaf",
            }

            local c = context.new(r)
            for _, v in ipairs(BAD_UTF8) do
                local ok, err = c:add_value("http.path", v)
                ngx.say(err)
            end
        }
    }
--- request
GET /t
--- response_body
invalid utf-8 sequence of 1 bytes from index 0
invalid utf-8 sequence of 1 bytes from index 0
invalid utf-8 sequence of 1 bytes from index 0
--- no_error_log
[error]
[warn]
[crit]



=== TEST 2: NULL bytes does not cause UTF-8 issues (it is valid UTF-8)
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local context = require("resty.router.context")
            local router = require("resty.router.router")

            local s = schema.new()

            s:add_field("http.path", "String")
            local r = router.new()

            local c = context.new(r)
            assert(c:add_value("http.path", "\x00"))
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



=== TEST 3: long strings don't cause a panic when parsing fails
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            s:add_field("http.path", "String")

            local r = router.new(s)
            local uuid = "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c"

            for _, len in ipairs({
                128,
                256,
                512,
                1024,
                2048,
                4096,
            }) do
                local input = string.rep("a", len)
                local ok, err = r:add_matcher(0, uuid, input)
                assert(not ok, "expected add_matcher() to fail")
                assert(type(err) == "string", "expected an error string")
            end

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



=== TEST 4: able to parse and handle string with NULL bytes inside
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.body", "String")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.body =^ \"world\""))

            local c = context.new(r)
            c:add_value("http.body", "hello\x00world")

            local matched = r:execute(c)
            ngx.say(matched)

            local uuid = c:get_result("http.body")
            ngx.say(uuid)

            c:reset()
            c:add_value("http.body", "world\x00hello")

            local matched = r:execute(c)
            ngx.say(matched)
        }
    }
--- request
GET /t
--- response_body
true
a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
false
--- no_error_log
[error]
[warn]
[crit]
