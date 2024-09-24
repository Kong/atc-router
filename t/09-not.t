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

=== TEST 1: not operator negates result from inside expression
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
                                 [[!(http.path ^= "/abc")]]))

            local c = context.new(r)
            c:add_value("http.path", "/abc/d")

            local matched = r:execute(c)
            ngx.say(matched)

            c:reset()

            c:add_value("http.path", "/abb/d")

            local matched = r:execute(c)
            ngx.say(matched)

            assert(r:remove_matcher("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c"))
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                                 [[!(http.path =^ "/")]]))

            c:reset()

            c:add_value("http.path", "/abb/d/")
            local matched = r:execute(c)
            ngx.say(matched)

            c:reset()

            c:add_value("http.path", "/abb/d")
            local matched = r:execute(c)
            ngx.say(matched)
        }
    }
--- request
GET /t
--- response_body
false
true
false
true
--- no_error_log
[error]
[warn]
[crit]
