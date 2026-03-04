# Name

ATC Router library for Kong.

# Table of Contents

* [Name](#name)
* [Semantics](#semantics)
* [Synopsis](#synopsis)
* [APIs](#apis)
    * [resty.router.schema](#restyrouterschema)
        * [new](#new)
        * [add\_field](#add_field)
        * [get\_field\_type](#get_field_type)
    * [resty.router.router](#restyrouterrouter)
        * [new](#new)
        * [add\_matcher](#add_matcher)
        * [remove\_matcher](#remove_matcher)
        * [enable\_prefilter](#enable_prefilter)
        * [disable\_prefilter](#disable_prefilter)
        * [execute](#execute)
        * [get\_fields](#get_fields)
        * [validate](#validate)
    * [resty.router.context](#restyroutercontext)
        * [new](#new)
        * [add\_value](#add_value)
        * [get\_result](#get_result)
        * [reset](#reset)
* [Copyright and license](#copyright-and-license)

# Semantics

At the core of the library, ATC Router is a [DSL] that supports simple predicate
and logical combinations between the predicates.

[DSL]:https://en.wikipedia.org/wiki/Domain-specific_language

Each data referred in the DSL has a type, the type can be one of the following:

* `"String"` - a UTF-8 string value
* `IpCidr` - an IP address range in CIDR format
* `IpAddr` - a single IP address that can be checked against an `IpCidr`
* `Int` - an 64-bit signed integer

Please refer to the [documentation](https://docs.konghq.com/gateway/latest/reference/expressions-language/)
on Kong website for how the language is used in practice.

# Synopsis

```
lua_package_path '/path/to/atc-router/lib/?.lua;;';

# run `make build` to generate dynamic library

lua_package_cpath '/path/to/atc-router/target/debug/?.so;;';

# A simple example creates schema, router and context, and use them to check if
# "http.path" starts with "/foo" and if "tcp.port" equals 80.

location = /simple_example {
    content_by_lua_block {
        local schema = require("resty.router.schema")
        local router = require("resty.router.router")
        local context = require("resty.router.context")

        local s = schema.new()

        s:add_field("http.path", "String")
        s:add_field("tcp.port", "Int")

        local r = router.new(s)
        assert(r:add_matcher(0, "a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c",
                             "http.path ^= \"/foo\" && tcp.port == 80"))

        local c = context.new(s)
        c:add_value("http.path", "/foo/bar")
        c:add_value("tcp.port", 80)

        local matched = r:execute(c)
        ngx.say(matched)

        local uuid, prefix = c:get_result("http.path")
        ngx.say(uuid)
        ngx.say(prefix)
    }
}
```

# APIs

## resty.router.schema

### new

**syntax:** *s = schema.new()*

**context:** *any*

Create a new schema instance that can later be used by `router` and `context`.

[Back to TOC](#table-of-contents)

### add\_field

**syntax:** *res, err = s:add_field(field, field_type)*

**context:** *any*

Adds the field named `field` into the schema. Type can be one of the ones mentioned
in the [Semantics](#semantics) section above.

If an error occurred, `nil` and a string describing the error will be returned.

[Back to TOC](#table-of-contents)

### get\_field\_type

**syntax:** *typ, err = s:get_field_type(field)*

**context:** *any*

Gets the field type from the schema.

If an error occurred, `nil` and a string describing the error will be returned.

[Back to TOC](#table-of-contents)

## resty.router.router

### new

**syntax:** *r = router.new(schema)*

**context:** *any*

Create a new router instance that can later be used for performing matches. `schema`
must refer to an existing schema instance.

[Back to TOC](#table-of-contents)

### add\_matcher

**syntax:** *res, err = r:add_matcher(priority, uuid, atc)*

**context:** *any*

Add a matcher to the router. `priority` is a 64-bit unsigned integer that instructs
the priority for which the matchers should be evaluated. `uuid` is the string
representation of the UUID of the matcher which will be used later for match results.
`atc` is the matcher written in ATC DSL syntax.

If an error occurred or the matcher has syntax/semantics errors,
`nil` and a string describing the error will be returned.

[Back to TOC](#table-of-contents)

### remove\_matcher

**syntax:** *res, err = r:remove_matcher(uuid)*

**context:** *any*

Remove matcher with `uuid` from the router.

Returns `true` if the matcher has successfully been removed. `false` if the
matcher does not exist.

[Back to TOC](#table-of-contents)

### enable\_prefilter

**syntax:** *r:enable_prefilter(field)*

**context:** *any*

Enables prefiltering on the specified `field`. The field must be of type `String`
in the router's schema. When enabled, the router uses the prefilter to narrow down
candidate matchers before performing full evaluation, which can improve match
performance.

[Back to TOC](#table-of-contents)

### disable\_prefilter

**syntax:** *r:disable_prefilter()*

**context:** *any*

Disables prefiltering on the router, reverting to the default matching behavior.

[Back to TOC](#table-of-contents)

### execute

**syntax:** *res, err = r:execute(context)*

**context:** *any*

Executes the router against value provided inside the `context` instance.

`context` must use the same schema as the router, otherwise Lua error will be thrown.

Returns `true` if at least one matcher produced a valid match. `false` if the
none of the matcher matched.

[Back to TOC](#table-of-contents)

### get\_fields

**syntax:** *res = r:get_fields()*

**context:** *any*

Returns the currently used field names by all matchers inside the router as
an Lua array. It can help reduce unnecessarily producing values that are not
actually used by the user supplied matchers.

[Back to TOC](#table-of-contents)

### validate

**syntax:** *fields, err = router.validate(schema, expr)*

**context:** *any*

Validates an expression against a given schema.

Returns the fields used in the provided expression when the expression is valid. If the expression is invalid,
`nil` and a string describing the reason will be returned.

[Back to TOC](#table-of-contents)

## resty.router.context

### new

**syntax:** *c = context.new(schema)*

**context:** *any*

Create a new context instance that can later be used for storing contextual information.
for router matches. `schema` must refer to an existing schema instance.

[Back to TOC](#table-of-contents)

### add\_value

**syntax:** *res, err = c:add_value(field, value)*

**context:** *any*

Provides `value` for `field` inside the context.

Returns `true` if field exists and value has successfully been provided.

If an error occurred, `nil` and a string describing the error will be returned.

[Back to TOC](#table-of-contents)

### get\_result

**syntax:** *uuid, matched_value, captures = c:get_result(matched_field)*

**context:** *any*

After a successful router match, gets the match result from the context.

If `matched_field` is provided, then `matched_value` will be returned with the value
matched by the specified field. If `matched_field` is `nil` or field did
not match, then `nil` is returned for `matched_value`.

If the context did not contain a valid match result, `nil` is returned.

Otherwise, the string UUID, value matching field `matched_field` and
regex captures from the matched route are returned.

[Back to TOC](#table-of-contents)

### reset

**syntax:** *c:reset()*

**context:** *any*

This resets context `c` without deallocating the underlying memory
so the context can be used again as if it was just created.

[Back to TOC](#table-of-contents)

# Copyright and license

Copyright © 2022-2023 Kong, Inc.

Licensed under the [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).

Files in the project may not be copied, modified, or distributed except according to those terms.

[Back to TOC](#table-of-contents)

