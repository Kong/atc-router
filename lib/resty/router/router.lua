local _M = {}
local _MT = { __index = _M, }


local ffi = require("ffi")
local base = require("resty.core.base")
local clib = require("resty.router.cdefs")
local get_string_buf = base.get_string_buf
local get_size_ptr = base.get_size_ptr
local ffi_string = ffi.string


function _M.new(schema)
    local router = clib.router_new(schema.schema)
    local r = setmetatable({
        router = ffi.gc(router, clib.router_free),
        schema = schema,
    }, _MT)

    return r
end


function _M:add_matcher(uuid, atc)
    local errbuf = get_string_buf(2048)
    local errbuf_len = get_size_ptr()

    if clib.router_add_matcher(self.router, uuid, atc, errbuf, errbuf_len) == false then
        return nil, ffi_string(errbuf, errbuf_len[0])
    end

    return true
end


function _M:remove_matcher(uuid)
    return clib.router_remove_matcher(self.router, uuid) == true
end


function _M:execute(context)
    return clib.router_execute(self.router, context.context) == true
end


return _M
