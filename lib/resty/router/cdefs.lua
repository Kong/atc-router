local ffi = require("ffi")


ffi.cdef([[
typedef enum Type {
  String,
  IpCidr,
  Int,
} Type;

typedef struct Context Context;

typedef struct Router Router;

typedef struct Schema Schema;

typedef enum CValue_Tag {
  CString,
  CInt,
} CValue_Tag;

typedef struct CValue {
  CValue_Tag tag;
  union {
    struct {
      const int8_t *c_string;
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
                        const int8_t *uuid,
                        const int8_t *atc,
                        uint8_t *errbuf,
                        uintptr_t *errbuf_len);

bool router_remove_matcher(struct Router *router, const int8_t *uuid);

bool router_execute(const struct Router *router, struct Context *context);

struct Context *context_new(const struct Schema *schema);

void context_free(struct Context *context);

void context_add_value(struct Context *context, const int8_t *field, const struct CValue *value);

uintptr_t context_get_matched_count(const struct Context *context);

void context_get_match(const struct Context *context,
                       uintptr_t index,
                       uint8_t *uuid,
                       uint8_t *prefix,
                       uintptr_t *prefix_len);
]])


local clib = ffi.load("/home/datong.sun/code/kong/atc-router/target/debug/libatc_router.so")


return clib
