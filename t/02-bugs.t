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
