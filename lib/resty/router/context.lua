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


function _M.new(router)
    local context = clib.context_new(router.router)
    local c = setmetatable({
        context = ffi_gc(context, context_free),
        schema = router.schema,
    }, _MT)

    return c
end


local function add_value_impl(ctx, field, value, index)
    if not value then
        return true
    end

    local typ, err = ctx.schema:get_field_type(field)
    if not typ then
        return nil, err
    end

    if typ == "String" then
        CACHED_VALUE[0].tag = C.CValue_Str
        CACHED_VALUE[0].str._0 = value
        CACHED_VALUE[0].str._1 = #value

    elseif typ == "IpAddr" then
        CACHED_VALUE[0].tag = C.CValue_IpAddr
        CACHED_VALUE[0].ip_addr = value

    elseif typ == "Int" then
        CACHED_VALUE[0].tag = C.CValue_Int
        CACHED_VALUE[0].int_ = value
    end

    local errbuf = get_string_buf(ERR_BUF_MAX_LEN)
    local errbuf_len = get_size_ptr()
    errbuf_len[0] = ERR_BUF_MAX_LEN 
    local res
    if index ~= nil then
       res = clib.context_add_value_by_index(ctx.context, index, CACHED_VALUE, errbuf, errbuf_len)
    else
       res = clib.context_add_value(ctx.context, field, CACHED_VALUE, errbuf, errbuf_len)
    end
    if res == false then
        return nil, ffi_string(errbuf, errbuf_len[0])
    end

    return true
end


function _M:add_value(field, value)
    return add_value_impl(self, field, value)
end


function _M:add_value_by_index(field, value, index)
    return add_value_impl(self, field, value, index)
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


function _M:reset()
    clib.context_reset(self.context)
end

return _M
