FROM rust as build

COPY . /src
WORKDIR /src
ENV LUA_LIB_DIR /usr/local/openresty/lualib
RUN RUSTFLAGS="-C target-feature=-crt-static" make install

FROM scratch

COPY --from=build /usr/local/openresty /usr/local/openresty
