local _M = {}


function _M.new(schema)
    local router = clib.router_new(schema)
    local r = setmetatable({
        router = ffi.gc(router, clib.router_free),
        schema = schema,
    }, _MT)

    return r
end


function _M:add_matcher(uuid, atc)
    clib.router_add_matcher(self.router, uuid, atc)
end


return _M
