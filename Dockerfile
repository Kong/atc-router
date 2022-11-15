FROM rust as build

COPY . /src
WORKDIR /src
ENV CARGO_NET_GIT_FETCH_WITH_CLI true
RUN apt-get update && \
    apt-get install -y git && \
    make install LUA_LIB_DIR=/usr/local/openresty/lualib || \
    RUSTFLAGS="-C target-feature=-crt-static" make clean install LUA_LIB_DIR=/usr/local/openresty/lualib

FROM scratch

COPY --from=build /usr/local/openresty /usr/local/openresty
