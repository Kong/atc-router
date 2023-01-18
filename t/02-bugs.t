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

            local s = schema.new()

            s:add_field("http.path", "String")

            local BAD_UTF8 = {
                "\x80",
                "\xbf",
                "\xfc\x80\x80\x80\x80\xaf",
            }

            local c = context.new(s)
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

            local s = schema.new()

            s:add_field("http.path", "String")

            local c = context.new(s)
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

=== TEST 3: long path don't cause crashes
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local bigstring = "/foo/"
            for i = 1,4097 do
              bigstring = bigstring .. "X"
            end

            local s = schema.new()

            s:add_field("http.path", "String")
            s:add_field("tcp.port", "Int")

            local r = router.new(s)
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 "http.path ^= \"/foo\" && tcp.port == 80"))

            local c = context.new(s)
            c:add_value("http.path", bigstring)
            c:add_value("tcp.port", 80)

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
