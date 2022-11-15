FROM rust as build

COPY . /src
WORKDIR /src
RUN make install LUA_LIB_DIR=/usr/local/openresty/lualib

FROM scratch

COPY --from=build /usr/local/openresty /usr/local/openresty
