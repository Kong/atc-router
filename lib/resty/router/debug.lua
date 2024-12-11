local _M = {}
local _MT = { __index = _M, }


local ffi = require("ffi")
local cdefs = require("resty.router.cdefs")
local ffi_new = ffi.new
local ffi_gc = ffi.gc
local assert = assert
local tonumber = tonumber
local setmetatable = setmetatable


local ERR_BUF_MAX_LEN = cdefs.ERR_BUF_MAX_LEN
local clib = cdefs.clib


function _M.new(schema, routes_n)
    local d = setmetatable({}, _MT)
    return d
end

function _M.router_get_duration(router)
    local add_matcher = ffi_new("size_t[1]")
    local remove_matcher = ffi_new("size_t[1]")
    local execute = ffi_new("size_t[1]")

    clib.debug_router_get_duration(router.router, add_matcher, remove_matcher, execute)

    return tonumber(add_matcher[0]), tonumber(remove_matcher[0]), tonumber(execute[0])
end

function _M.router_get_counter(router)
    local add_matcher = ffi_new("size_t[1]")
    local remove_matcher = ffi_new("size_t[1]")
    local execute = ffi_new("size_t[1]")

    clib.debug_router_get_counter(router.router, add_matcher, remove_matcher, execute)

    return tonumber(add_matcher[0]), tonumber(remove_matcher[0]), tonumber(execute[0])
end

return _M

