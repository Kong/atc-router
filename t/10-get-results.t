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
=== TEST 2: create schema, router, context
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")
            local context = require("resty.router.context")

            local s = schema.new()

            s:add_field("service.id", "String")
            s:add_field("route.id", "String")

            local r = router.new(s)
            assert(r:add_matcher(0, "b921a9aa-ec0e-4cf3-a6cc-1aa5583d150c", "service.id == \"123\""))
            assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c", "route.id == \"456\""))

            local c = context.new(s)
            c:add_value("service.id", "123")
            c:add_value("route.id", "456")

            local matched = r:execute(c, true)
            ngx.say(tostring(matched))

            local results = c:get_all_results()
            ngx.say("Results count: " .. tostring(#results))

            for i, result in ipairs(results) do
                ngx.say("Result " .. i .. ":")
                ngx.say("  UUID: " .. tostring(result.uuid))
                ngx.say("  Matched field: " .. tostring(result.matched_field))
                ngx.say("  Matched value: " .. tostring(result.matched_value))
                ngx.say("  Capture count: " .. tostring(result.capture_count))
            end
        }
    }
--- request
GET /t
--- response_body_like
true
Results count: 2
Result 1:
  UUID: b921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
  Matched field: service.id
  Matched value: 123
  Capture count: 0
Result 2:
  UUID: a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c
  Matched field: route.id
  Matched value: 456
  Capture count: 0
--- no_error_log
[error]
[warn]
[crit]
