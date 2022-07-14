local _M = {}
local _MT = { __index = _M, }


local ffi = require("ffi")
local base = require("resty.core.base")
local cdefs = require("resty.router.cdefs")


local ffi_new = ffi.new
local ffi_gc = ffi.gc
local get_string_buf = base.get_string_buf
local get_size_ptr = base.get_size_ptr
local ffi_string = ffi.string
local tonumber = tonumber
local setmetatable = setmetatable
local new_tab = require("table.new")
local C = ffi.C


local UUID_LEN = 36 -- hexadecimal representation of UUID
local CACHED_VALUE = ffi_new("CValue[1]")
local UUID_BUF = ffi_new("uint8_t[?]", UUID_LEN)
local ERR_BUF_MAX_LEN = cdefs.ERR_BUF_MAX_LEN
local clib = cdefs.clib
local context_free = cdefs.context_free


function _M.new(schema)
    local context = clib.context_new(schema.schema)
    local c = setmetatable({
        context = ffi_gc(context, context_free),
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
        CACHED_VALUE[0].tag = C.CIpAddr
        CACHED_VALUE[0].c_ip_addr = value

    elseif typ == "Int" then
        CACHED_VALUE[0].tag = C.CInt
        CACHED_VALUE[0].c_int = value
    end

    local errbuf = get_string_buf(ERR_BUF_MAX_LEN)
    local errbuf_len = get_size_ptr()

    if clib.context_add_value(self.context, field, CACHED_VALUE, errbuf, errbuf_len) == false then
        return nil, ffi_string(errbuf, errbuf_len[0])
    end

    return true
end


function _M:get_result(matched_field)
    local captures_len = tonumber(clib.context_get_result(
        self.context, nil, nil, nil, nil, nil, nil, nil, nil))
    if captures_len == -1 then
        return nil
    end

    local matched_value_buf, matched_value_len
    if matched_field then
        matched_value_buf = ffi_new("const uint8_t *[1]")
        matched_value_len = ffi_new("size_t [1]")
    end

    local capture_names, capture_names_len, capture_values, capture_values_len
    if captures_len > 0 then
        capture_names = ffi_new("const uint8_t *[?]", captures_len)
        capture_names_len = ffi_new("size_t [?]", captures_len)
        capture_values = ffi_new("const uint8_t *[?]", captures_len)
        capture_values_len = ffi_new("size_t [?]", captures_len)

        capture_names_len[0] = captures_len
        capture_values_len[0] = captures_len
    end

    clib.context_get_result(self.context, UUID_BUF, matched_field,
                            matched_value_buf, matched_value_len,
                            capture_names, capture_names_len, capture_values,
                            capture_values_len)

    local uuid = ffi_string(UUID_BUF, UUID_LEN)
    local matched_value
    if matched_field then
        matched_value = matched_value_len[0] > 0 and
                        ffi_string(matched_value_buf[0], matched_value_len[0]) or
                        nil
    end

    local captures

    if captures_len > 0 then
        captures = new_tab(0, captures_len)

        for i = 0, captures_len - 1 do
            local name = ffi_string(capture_names[i], capture_names_len[i])
            local value = ffi_string(capture_values[i], capture_values_len[i])

            local num = tonumber(name, 10)
            if num then
                name = num
            end

            captures[name] = value
        end
    end

    return uuid, matched_value, captures
end


return _M
