# Name

ATC Router library for Kong.

Table of Contents
=================

* [Name](#name)
* [Semantics](#semantics)
* [APIs](#apis)
    * [resty.router.schema](#restyrouterschema)
        * [new](#new)
        * [add\_field](#add_field)
        * [get\_field\_type](#get_field_type)
    * [resty.router.router](#restyrouterrouter)
        * [new](#new)
        * [add\_matcher](#add_matcher)
        * [remove\_matcher](#remove_matcher)
        * [execute](#execute)
        * [get\_fields](#get_fields)
        * [validate](#validate)
    * [resty.router.context](#restyroutercontext)
        * [new](#new)
        * [add\_value](#add_value)
        * [get\_result](#get_result)
* [Copyright and license](#copyright-and-license)

# Semantics

At the core of the library, ATC Router is a DSL that supports simple predicate
and logical combinations between the predicates.

Each data referred in the DSL has a type, the type can be one of the following:

* `"String"` - a UTF-8 string value
* `IpCidr` - an IP address range in CIDR format
* `IpAddr` - a single IP address that can be checked against an `IpCidr`
* `Int` - an 64-bit signed integer

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

### execute

**syntax:** *res, err = r:execute(context)*

**context:** *any*

Executes the router against value provided inside the `context` instance.

`context` must use the same schema as the router, otherwise Lua error will be thrown.

Returns `true` if at least one matcher produced a valid match. `false` if the
none of the matcher matched.

[Back to TOC](#table-of-contents)

### get\_fields

**syntax:** *res, err = r:get_fields()*

**context:** *any*

Returns the currently used field names by all matchers inside the router as
an Lua array. It can help reduce unnecessarily producing values that are not
actually used by the user supplied matchers.

[Back to TOC](#table-of-contents)

### validate

**syntax:** *ok, err = r.validate(schema, expr)*

**context:** *any*

Validates an expression against a given schema.

**params:**

- *expr*: The expression to validate.
- *schema*: The schema to validate against.

**returns:**

- *is_valid*: A boolean value, true if the expression is valid according to the schema, false otherwise.
- *err*: A string describing the error, if the expression is not valid. Otherwise, this value will be nil.

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

# Copyright and license

Copyright (c) 2022-2023 Kong, Inc.

Licensed under the Apache License, Version 2.0 <LICENSE or
[https://www.apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0)>.
Files in the project may not be copied, modified, or distributed except according to those terms.

[Back to TOC](#table-of-contents)

