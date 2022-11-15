FROM rust as build

COPY . /src
WORKDIR /src
ENV LUA_LIB_DIR /usr/local/openresty/lualib
RUN make install || \
    RUSTFLAGS="-C target-feature=-crt-static" make clean install

FROM scratch

COPY --from=build /usr/local/openresty /usr/local/openresty
