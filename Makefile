OS=$(shell uname -s)

ifeq ($(OS), Darwin)
SHLIB_EXT=dylib
else
SHLIB_EXT=so
endif

OPENRESTY_PREFIX=/usr/local/openresty

#LUA_VERSION := 5.1
PREFIX ?=          /usr/local
LUA_INCLUDE_DIR ?= $(PREFIX)/include
LUA_LIB_DIR ?=     $(PREFIX)/lib/lua/$(LUA_VERSION)
INSTALL ?= install
RELEASE_FOLDER = target/$(CARGO_BUILD_TARGET)/release
DEBUG_RELEASE_FOLDER = target/$(CARGO_BUILD_TARGET)/debug

.PHONY: all test install build clean update-ffi-cdefs

all: ;

build: $(RELEASE_FOLDER)/libatc_router.$(SHLIB_EXT) $(RELEASE_FOLDER)/libatc_router.a

$(RELEASE_FOLDER)/libatc_router.%: src/*.rs
	cargo build --release

$(DEBUG_RELEASE_FOLDER)/libatc_router.%: src/*.rs
	cargo build

install-lualib:
	$(INSTALL) -d $(DESTDIR)$(LUA_LIB_DIR)/resty/router/
	$(INSTALL) -m 664 lib/resty/router/*.lua $(DESTDIR)$(LUA_LIB_DIR)/resty/router/

install: build install-lualib
	$(INSTALL) -m 775 $(RELEASE_FOLDER)/libatc_router.$(SHLIB_EXT) $(DESTDIR)$(LUA_LIB_DIR)/libatc_router.$(SHLIB_EXT)

install-debug: $(DEBUG_RELEASE_FOLDER)/libatc_router.% install-lualib
	$(INSTALL) -m 775 $(DEBUG_RELEASE_FOLDER)/libatc_router.$(SHLIB_EXT) $(DESTDIR)$(LUA_LIB_DIR)/libatc_router.$(SHLIB_EXT)

test: $(DEBUG_RELEASE_FOLDER)/libatc_router.%
	PATH="$(OPENRESTY_PREFIX)/nginx/sbin:$$PATH" \
	LUA_PATH="$(realpath lib)/?.lua;$(realpath lib)/?/init.lua;$$LUA_PATH" \
	LUA_CPATH="$(realpath $(DEBUG_RELEASE_FOLDER))/?.so;$$LUA_CPATH" \
	prove -r t/

valgrind: $(DEBUG_RELEASE_FOLDER)/libatc_router.%
	(PATH="$(OPENRESTY_PREFIX)/nginx/sbin:$$PATH" \
	LUA_PATH="$(realpath lib)/?.lua;$(realpath lib)/?/init.lua;$$LUA_PATH" \
	LUA_CPATH="$(realpath $(DEBUG_RELEASE_FOLDER))/?.so;$$LUA_CPATH" \
	prove -r t/) 2>&1 | tee /dev/stderr | grep -q "match-leak-kinds: definite" && exit 1 || exit 0

clean:
	rm -rf target

update-ffi-cdefs:
	./scripts/update-ffi-cdefs.sh
