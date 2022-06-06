local _M = {}


local _M = { __index = _M, }


function _M.new()
    local schema = clib.schema_new()
    local s = setmetatable({
        schema = ffi.gc(schema, clib.schema_free),
        fields = {},
    }, _MT)

    return s
end


do
    function _M:add_field(field, typ)
        if self.fields[field] then
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
    end
end


return _M
