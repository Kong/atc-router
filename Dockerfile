ARG PACKAGE_TYPE=rpm

FROM kong/kong-build-tools:apk-1.6.4 as APK
FROM kong/kong-build-tools:deb-1.6.4 as DEB

FROM $PACKAGE_TYPE as build

COPY . /src
WORKDIR /src
ENV CARGO_NET_GIT_FETCH_WITH_CLI true
RUN make install LUA_LIB_DIR=/usr/local/openresty/lualib || \
    RUSTFLAGS="-C target-feature=-crt-static" make clean install LUA_LIB_DIR=/usr/local/openresty/lualib

FROM scratch

COPY --from=build /usr/local/openresty /usr/local/openresty
