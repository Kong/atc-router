OPENRESTY_PREFIX=/usr/local/openresty

#LUA_VERSION := 5.1
PREFIX ?=          /usr/local
LUA_INCLUDE_DIR ?= $(PREFIX)/include
LUA_LIB_DIR ?=     $(PREFIX)/lib/lua/$(LUA_VERSION)
INSTALL ?= install

.PHONY: all test install build clean

all: ;

build: target/release/libatc_router.so

target/release/libatc_router.so:
ifeq (, $(shell which cargo))
$(error "cargo not found in PATH, consider doing \"curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\"")
endif
	cargo build --release

install: build
	$(INSTALL) -d $(DESTDIR)$(LUA_LIB_DIR)/resty/router/
	$(INSTALL) -m 664 lib/resty/router/*.lua $(DESTDIR)$(LUA_LIB_DIR)/resty/router/
	$(INSTALL) -m 775 target/release/libatc_router.so $(DESTDIR)$(LUA_LIB_DIR)/

clean:
	rm -rf target
