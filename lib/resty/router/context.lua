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
local ffi_new = ffi.new
local tonumber = tonumber
local new_tab = require("table.new")


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


function _M:get_result()
    local captures_len = tonumber(clib.context_get_result(self.context, nil, nil, nil, nil, nil, nil, nil))
    if captures_len == -1 then
        return nil
    end

    local matched_path = ffi_new("const uint8_t *[1]")
    local matched_path_len = ffi_new("size_t [1]")

    local capture_names, capture_names_len, capture_values, capture_values_len
    if captures_len > 0 then
        capture_names = ffi_new("const uint8_t *[?]", captures_len)
        capture_names_len = ffi_new("size_t [?]", captures_len)
        capture_values = ffi_new("const uint8_t *[?]", captures_len)
        capture_values_len = ffi_new("size_t [?]", captures_len)

        capture_names_len[0] = captures_len
        capture_values_len[0] = captures_len
    end

    clib.context_get_result(self.context, UUID_BUF, matched_path, matched_path_len,
                           capture_names, capture_names_len, capture_values,
                           capture_values_len)

    local uuid = ffi_string(UUID_BUF, 36)
    local matched_path = matched_path_len[0] > 0 and ffi_string(matched_path[0], matched_path_len[0]) or nil

    local captures

    if captures_len > 0 then
        captures = new_tab(0, captures_len)

        for i = 0, captures_len - 1 do
            local name = ffi_string(capture_names[i], capture_names_len[i])
            local value = ffi_string(capture_values[i], capture_values_len[i])

            captures[name] = value
        end
    end

    return uuid, matched_path, captures
end


return _M
