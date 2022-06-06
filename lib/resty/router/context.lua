local _M = {}


function _M.new(schema)
    local context = clib.context_new(schema)
    local c = setmetatable({
        context = ffi.gc(context, clib.context_free),
        schema = schema,
    }, _MT)

    return s
end


return _M
