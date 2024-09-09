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

            local s = schema.new()
            s:add_field("http.headers.foo", "String")

            local expr = "http.headers.foo == \"bar\""
            local r, err = router.validate(s, expr)

            ngx.say(type(r))
            ngx.say(err)
            ngx.say(r[http.headers.foo])
        }
    }
--- request
GET /t
--- response_body
table
nil
0
--- no_error_log
[error]
[warn]
[crit]


=== TEST 2: test type inconsistency (schema is String, expr is Int)
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

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
--- response_body_like
nil
Type mismatch between the LHS and RHS values of predicate
--- no_error_log
[error]
[warn]
[crit]


=== TEST 3: test invalid schema + invalid expr
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            s:add_field("http.headers.foo", "String")

            local expr = "== 123"
            local r, err = router.validate(s, expr)

            ngx.say(r)
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
nil
 --> 1:1
  |
1 | == 123
  | ^---
  |
  = expected term

--- no_error_log
[error]
[warn]
[crit]


=== TEST 4: test valid schema + invalid expr
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            s:add_field("http.headers.foo", "String")

            local expr = "== \"bar\""
            local r, err = router.validate(s, expr)

            ngx.say(r)
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
nil
 --> 1:1
  |
1 | == "bar"
  | ^---
  |
  = expected term

--- no_error_log
[error]
[warn]
[crit]


=== TEST 5: valid regex
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            s:add_field("http.headers.foo", "String")

            local expr = [[http.headers.foo ~ "/\\\\/*user$"]]
            local r, err = router.validate(s, expr)
            ngx.say(type(r))
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
table
nil
--- no_error_log
[error]
[warn]
[crit]


=== TEST 6: invalid regex
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            s:add_field("http.headers.foo", "String")

            local expr = [[http.headers.foo ~ "([."]]
            local r, err = router.validate(s, expr)
            ngx.say(r)
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
nil
 --> 1:20
  |
1 | http.headers.foo ~ "([."
  |                    ^---^
  |
  = regex parse error:
    ([.
     ^
error: unclosed character class

--- no_error_log
[error]
[warn]
[crit]


=== TEST 7: Rust regex 1.8.x will not think the regex is invalid
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            s:add_field("http.headers.foo", "String")

            local expr = [[http.headers.foo ~ "/\\/*user$"]]
            local r, err = router.validate(s, expr)
            ngx.say(type(r))
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
table
nil
--- no_error_log
[error]
[warn]
[crit]



=== TEST 8: pratt parser propagates parser error
--- http_config eval: $::HttpConfig
--- config
    location = /t {
        content_by_lua_block {
            local schema = require("resty.router.schema")
            local router = require("resty.router.router")

            local s = schema.new()
            s:add_field("http.headers.foo", "String")

            local expr = [[http.headers.foo == "a" && http.headers.foo ~ "([."]]
            local r, err = router.validate(s, expr)
            ngx.say(r)
            ngx.say(err)
        }
    }
--- request
GET /t
--- response_body
nil
 --> 1:47
  |
1 | http.headers.foo == "a" && http.headers.foo ~ "([."
  |                                               ^---^
  |
  = regex parse error:
    ([.
     ^
error: unclosed character class

--- no_error_log
[error]
[warn]
[crit]
