local ffi = require("ffi")


-- generated from "cbindgen -l c", do not edit manually
ffi.cdef([[
typedef enum Type {
  String,
  IpCidr,
  IpAddr,
  Int,
  Regex,
} Type;

typedef struct Context Context;

typedef struct Router Router;

typedef struct Schema Schema;

typedef enum CValue_Tag {
  CString,
  CIpCidr,
  CIpAddr,
  CInt,
} CValue_Tag;

typedef struct CValue {
  CValue_Tag tag;
  union {
    struct {
      const int8_t *c_string;
    };
    struct {
      const int8_t *c_ip_cidr;
    };
    struct {
      const int8_t *c_ip_addr;
    };
    struct {
      int64_t c_int;
    };
  };
} CValue;

struct Schema *schema_new(void);

void schema_free(struct Schema *schema);

void schema_add_field(struct Schema *schema, const int8_t *field, enum Type typ);

struct Router *router_new(const struct Schema *schema);

void router_free(struct Router *router);

bool router_add_matcher(struct Router *router,
                        uintptr_t priority,
                        const int8_t *uuid,
                        const int8_t *atc,
                        uint8_t *errbuf,
                        uintptr_t *errbuf_len);

bool router_remove_matcher(struct Router *router, uintptr_t priority, const int8_t *uuid);

bool router_execute(const struct Router *router, struct Context *context);

uintptr_t router_get_fields(const struct Router *router,
                            const uint8_t **fields,
                            uintptr_t *fields_len);

struct Context *context_new(const struct Schema *schema);

void context_free(struct Context *context);

bool context_add_value(struct Context *context,
                       const int8_t *field,
                       const struct CValue *value,
                       uint8_t *errbuf,
                       uintptr_t *errbuf_len);

intptr_t context_get_result(const struct Context *context,
                            uint8_t *uuid_hex,
                            const int8_t *matched_field,
                            const uint8_t **matched_value,
                            uintptr_t *matched_value_len,
                            const uint8_t **capture_names,
                            uintptr_t *capture_names_len,
                            const uint8_t **capture_values,
                            uintptr_t *capture_values_len);
]])


local ERR_BUF_MAX_LEN = 2048


-- From: https://github.com/openresty/lua-resty-signal/blob/master/lib/resty/signal.lua
local load_shared_lib
do
    local tostring = tostring
    local string_gmatch = string.gmatch
    local string_match = string.match
    local io_open = io.open
    local io_close = io.close
    local table_new = require("table.new")

    local cpath = package.cpath

    function load_shared_lib(so_name)
        local tried_paths = table_new(32, 0)
        local i = 1

        for k, _ in string_gmatch(cpath, "[^;]+") do
            local fpath = tostring(string_match(k, "(.*/)"))
            fpath = fpath .. so_name
            -- Don't get me wrong, the only way to know if a file exist is
            -- trying to open it.
            local f = io_open(fpath)
            if f ~= nil then
                io_close(f)
                return ffi.load(fpath)
            end

            tried_paths[i] = fpath
            i = i + 1
        end

        return nil, tried_paths
    end  -- function
end  -- do


local clib, tried_paths = load_shared_lib("libatc_router.so")
if not clib then
    error("could not load libatc_router.so from the following paths:\n" ..
          table.concat(tried_paths, "\n"), 2)
end


return {
    clib = clib,
    ERR_BUF_MAX_LEN = ERR_BUF_MAX_LEN,

    context_free = function(c)
        clib.context_free(c)
    end,

    schema_free = function(s)
        clib.schema_free(s)
    end,

    router_free = function(r)
        clib.router_free(r)
    end,
}
