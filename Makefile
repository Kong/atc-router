OS=$(shell uname -s)

ifeq ($(OS), Darwin)
SHLIB_EXT=dylib
else
SHLIB_EXT=so
endif

NPM_PKG_VERSION=1.0.2-6

OPENRESTY_PREFIX=/usr/local/openresty

#LUA_VERSION := 5.1
PREFIX ?=          /usr/local
LUA_INCLUDE_DIR ?= $(PREFIX)/include
LUA_LIB_DIR ?=     $(PREFIX)/lib/lua/$(LUA_VERSION)
INSTALL ?= install

.PHONY: all test install build wasm wasm-publish clean

all: ;

build: target/release/libatc_router.so target/release/libatc_router.a

target/release/libatc_router.%: src/*.rs
ifeq (, $(shell cargo))
$(error "cargo not found in PATH, consider doing \"curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\"")
endif
	cargo build --release

install: build
	$(INSTALL) -d $(DESTDIR)$(LUA_LIB_DIR)/resty/router/
	$(INSTALL) -m 664 lib/resty/router/*.lua $(DESTDIR)$(LUA_LIB_DIR)/resty/router/
	$(INSTALL) -m 775 target/release/libatc_router.$(SHLIB_EXT) $(DESTDIR)$(LUA_LIB_DIR)/libatc_router.so

wasm-build:
ifeq (, $(shell which wasm-pack))
$(error "wasm-pack not found in PATH, consider doing \"curl --proto '=https' --tlsv1.2 -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh\"")
endif
	wasm-pack build --scope kong
	sed -i '' 's/"version": ".*"/"version": "$(NPM_PKG_VERSION)"/g' pkg/package.json

wasm-publish:
ifeq (, $(shell which wasm-pack))
$(error "wasm-pack not found in PATH, consider doing \"curl --proto '=https' --tlsv1.2 -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh\"")
endif
ifeq (, $(shell which npm))
$(error "npm not found in PATH, consider visiting https://docs.npmjs.com/downloading-and-installing-node-js-and-npm")
endif
	cd pkg && \
	pwd && \
	npm publish --access public

clean:
	rm -rf target
