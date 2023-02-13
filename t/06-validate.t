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

=== TEST 1: test valid schema + expr
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.headers.foo", "String")
            local expr = "http.headers.foo == \"bar\""
            local r, err = router.validate(s, expr)

            ngx.say(r)
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
true
nil
--- no_error_log
[error]
[warn]
[crit]

=== TEST 1: test invalid schema + expr
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("http.headers.foo", "String")
            local expr = "http.headers.foo == 123"
            local r, err = router.validate(s, expr)

            ngx.say(r)
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
false
nil
--- no_error_log
[error]
[warn]
[crit]