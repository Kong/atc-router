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
    if not value then
        return true
    end

    local typ, err = self.schema:get_field_type(field)
    if not typ then
        return nil, err
    end

    if typ == "String" then
        CACHED_VALUE[0].tag = C.CString
        CACHED_VALUE[0].c_string = value

    elseif typ == "IpAddr" then
        CACHED_VALUE[0].tag = C.IpAddr
        CACHED_VALUE[0].c_ip_addr = value

    elseif typ == "Int" then
        CACHED_VALUE[0].tag = C.Int
        CACHED_VALUE[0].c_int = value
    end

    local errbuf = get_string_buf(2048)
    local errbuf_len = get_size_ptr()

    if clib.context_add_value(self.context, field, CACHED_VALUE, errbuf, errbuf_len) == false then
        return ffi_string(errbuf, errbuf_len[0])
    end

    return true
end


function _M:get_matched_count()
    return tonumber(clib.context_get_matched_count(self.context))
end


function _M:get_match(index, matched_field)
    local buf, len
    if matched_field then
      buf = get_string_buf(2048)
      len = get_size_ptr()
    end

    clib.context_get_match(self.context, index, UUID_BUF, matched_field, buf, len)

    local uuid = ffi_string(UUID_BUF, 36)

    local matched_value
    if matched_field and len[0] > 0 then
        matched_value = ffi_string(buf, len[0])
    end

    return uuid, matched_value
end


return _M
