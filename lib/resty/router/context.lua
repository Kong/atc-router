local _M = {}
local _MT = { __index = _M, }


local ffi = require("ffi")
local base = require("resty.core.base")
local clib = require("resty.router.cdefs")


local C = ffi.C
local CACHED_VALUE = ffi.new("CValue[1]")
local UUID_BUF = ffi.new("uint8_t[36]")
local get_string_buf = base.get_string_buf
local get_size_ptr = base.get_size_ptr
local ffi_string = ffi.string
local tonumber = tonumber


function _M.new(schema)
    local context = clib.context_new(schema.schema)
    local c = setmetatable({
        context = ffi.gc(context, clib.context_free),
        schema = schema,
    }, _MT)

    return c
end


function _M:add_value(field, value)
    local typ, err = self.schema:get_field_type(field)
    if not typ then
        return nil, err
    end

    if typ == "String" then
        CACHED_VALUE[0].tag = C.CString
        CACHED_VALUE[0].c_string = value

    elseif typ == "IpCidr" then
        assert(false) -- unimplemented
        CACHED_VALUE[0].tag = C.IpCidr

    elseif typ == "Int" then
        CACHED_VALUE[0].tag = C.Int
        CACHED_VALUE[0].c_int = value
    end

    clib.context_add_value(self.context, field, CACHED_VALUE)
end


function _M:get_matched_count()
    return tonumber(clib.context_get_matched_count(self.context))
end


function _M:get_match(index)
    local prefix_buf = get_string_buf(2048)
    local prefix_len = get_size_ptr()

    clib.context_get_match(self.context, index, UUID_BUF, prefix_buf, prefix_len)

    local uuid = ffi_string(UUID_BUF, 36)
    local prefix
    if prefix_len[0] > 0 then
        prefix = ffi_string(prefix_buf, prefix_len[0])
    end

    return uuid, prefix
end


return _M
