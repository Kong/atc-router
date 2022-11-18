ARG OSTYPE=linux-gnu
ARG ARCHITECTURE=x86_64

FROM --platform=linux/amd64 kong/kong-build-tools:apk-1.8.0 as x86_64-linux-musl
FROM --platform=linux/amd64 kong/kong-build-tools:deb-1.8.0 as x86_64-linux-gnu
FROM --platform=linux/arm64 kong/kong-build-tools:apk-1.8.0 as aarch64-linux-musl
FROM --platform=linux/arm64 kong/kong-build-tools:deb-1.8.0 as aarch64-linux-gnu

FROM $ARCHITECTURE-$OSTYPE as build

WORKDIR /src
COPY . /src
ENV CARGO_NET_GIT_FETCH_WITH_CLI true
ENV LUA_LIB_DIR /usr/local/openresty/lualib
ENV DESTDIR /tmp/build
RUN RUSTFLAGS="-C target-feature=-crt-static" make install || make clean install

FROM scratch

COPY --from=build /tmp/build /
