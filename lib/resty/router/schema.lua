local _M = {}
local clib = require("resty.router.cdefs")
local ffi = require("ffi")


local _MT = { __index = _M, }


function _M.new()
    local schema = clib.schema_new()
    local s = setmetatable({
        schema = ffi.gc(schema, clib.schema_free),
        field_types = {},
        field_ctypes = {},
    }, _MT)

    return s
end


do
    function _M:add_field(field, typ)
        if self.field_types[field] then
            return nil, "field " .. field .. " already exists"
        end

        local ctype

        if typ == "String" then
            ctype = clib.String

        elseif typ == "IpCidr" then
            ctype = clib.IpCidr

        elseif typ == "Int" then
            ctype = clib.Int

        else
            error("Unknown type: " .. typ, 2)
        end

        clib.schema_add_field(self.schema, field, ctype)

        self.field_types[field] = typ
        self.field_ctypes[field] = ctype
    end
end


function _M:get_field_type(field)
    local typ = self.field_types[field]

    if not typ then
        return nil, "field " .. field .. " unknown"
    end

    return typ
end


function _M:get_field_ctype(field)
    local typ = self.field_ctypes[field]

    if not typ then
        return nil, "field " .. field .. " unknown"
    end

    return typ
end


return _M
