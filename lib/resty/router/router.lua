local _M = {}
local _MT = { __index = _M, }


local ffi = require("ffi")
local base = require("resty.core.base")
local cdefs = require("resty.router.cdefs")
local tb_new = require("table.new")
local get_string_buf = base.get_string_buf
local get_size_ptr = base.get_size_ptr
local ffi_string = ffi.string
local ffi_new = ffi.new
local ffi_gc = ffi.gc
local assert = assert
local tonumber = tonumber
local setmetatable = setmetatable


local ERR_BUF_MAX_LEN = cdefs.ERR_BUF_MAX_LEN
local clib = cdefs.clib
local router_free = cdefs.router_free


function _M.new(schema, routes_n)
    local router = clib.router_new(schema.schema)
    -- Note on this weird looking finalizer:
    --
    -- You may be tempted to change it to ffi_gc(router, clib.router_free)
    -- This isn't 100% safe, particularly with `busted` clearing the global
    -- environment between each runs. `clib` could be GC'ed before this entity,
    -- causing instruction fetch faults because the `router` finalizer will
    -- attempt to execute from unmapped memory region
    local r = setmetatable({
        router = ffi_gc(router, router_free),
        schema = schema,
        priorities = tb_new(0, routes_n or 10),
    }, _MT)

    return r
end


function _M:add_matcher(priority, uuid, atc)
    local errbuf = get_string_buf(ERR_BUF_MAX_LEN)
    local errbuf_len = get_size_ptr()
    errbuf_len[0] = ERR_BUF_MAX_LEN

    if clib.router_add_matcher(self.router, priority, uuid, atc, errbuf, errbuf_len) == false then
        return nil, ffi_string(errbuf, errbuf_len[0])
    end

    self.priorities[uuid] = priority

    return true
end


function _M:remove_matcher(uuid)
    local priority = self.priorities[uuid]
    if not priority then
        return false
    end

    self.priorities[uuid] = nil

    return clib.router_remove_matcher(self.router, priority, uuid) == true
end


function _M:execute(context)
    assert(context.schema == self.schema)
    return clib.router_execute(self.router, context.context) == true
end


function _M:get_fields()
    local out = {}
    local out_n = 0
    local router = self.router

    local total = tonumber(clib.router_get_fields(router, nil, nil, nil))
    if total == 0 then
        return out
    end

    local fields = ffi_new("const uint8_t *[?]", total)
    local fields_len = ffi_new("size_t [?]", total)
    local indexes = ffi_new("size_t [?]", total)
    fields_len[0] = total

    clib.router_get_fields(router, fields, fields_len, indexes)
    for i = 0, total - 1 do
        out[ffi_string(fields[i], fields_len[i])] = indexes[i]
    end

    return out
end


do
    local ROUTERS = setmetatable({}, { __mode = "k" })
    local DEFAULT_UUID = "00000000-0000-0000-0000-000000000000"
    local DEFAULT_PRIORITY = 0

    -- validate an expression against a schema
    -- @param expr the expression to validate
    -- @param schema the schema to validate against
    -- @return true if the expression is valid, (nil, error) otherwise
    function _M.validate(schema, expr)
        local r = ROUTERS[schema]

        if not r then
            r = _M.new(schema, 1)
            ROUTERS[schema] = r
        end

        local ok, err = r:add_matcher(DEFAULT_PRIORITY, DEFAULT_UUID, expr)
        if not ok then
            return nil, err
        end

        local fields = r:get_fields()

        assert(r:remove_matcher(DEFAULT_UUID))

        return fields
    end
end


return _M
