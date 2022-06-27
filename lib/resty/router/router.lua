local _M = {}
local _MT = { __index = _M, }


local ffi = require("ffi")
local base = require("resty.core.base")
local cdefs = require("resty.router.cdefs")
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


function _M.new(schema)
    local router = clib.router_new(schema.schema)
    local r = setmetatable({
        router = ffi_gc(router, clib.router_free),
        schema = schema,
        priorities = {},
    }, _MT)

    return r
end


function _M:add_matcher(priority, uuid, atc)
    local errbuf = get_string_buf(ERR_BUF_MAX_LEN)
    local errbuf_len = get_size_ptr()

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

    local total = tonumber(clib.router_get_fields(self.router, nil, nil))
    if total == 0 then
        return out
    end

    local fields = ffi_new("const uint8_t *[?]", total)
    local fields_len = ffi_new("size_t [?]", total)
    fields_len[0] = total

    clib.router_get_fields(self.router, fields, fields_len)

    for i = 0, total - 1 do
        out_n = out_n + 1
        out[out_n] = ffi_string(fields[i], fields_len[i])
    end

    return out
end


return _M
