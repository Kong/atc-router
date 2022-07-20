local _M = {}
local cdefs = require("resty.router.cdefs")
local ffi = require("ffi")


local _MT = { __index = _M, }


local setmetatable = setmetatable
local ffi_gc = ffi.gc
local clib = cdefs.clib
local schema_free = cdefs.schema_free


function _M.new()
    local schema = clib.schema_new()
    local s = setmetatable({
        schema = ffi_gc(schema, schema_free),
        field_types = {},
        field_ctypes = {},
        clib = clib,
    }, _MT)

    return s
end


function _M:add_field(field, typ)
    if self.field_types[field] then
        return nil, "field " .. field .. " already exists"
    end

    local ctype

    if typ == "String" then
        ctype = clib.String

    elseif typ == "IpCidr" then
        ctype = clib.IpCidr

    elseif typ == "IpAddr" then
        ctype = clib.IpAddr

    elseif typ == "Int" then
        ctype = clib.Int

    else
        error("Unknown type: " .. typ, 2)
    end

    clib.schema_add_field(self.schema, field, ctype)

    self.field_types[field] = typ
    self.field_ctypes[field] = ctype

    return true
end


function _M:get_field_type(field)
    local typ = self.field_types[field]

    if not typ then
        local name = field:match("(.+)%..+")
        if name then
            typ = self.field_types[name .. ".*"]
            if not typ then
                return nil, "field " .. field .. " unknown"
            end
        end
    end

    return typ
end


return _M
