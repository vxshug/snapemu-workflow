FROM rust:1.79.0-alpine3.20
ENV RUST_BACKTRACE 1
RUN apk add musl-dev pkgconfig make perl protoc openssl-dev curl patch bash python3 g++ gcompat
#RUN update-ca-certificates

WORKDIR /usr/src/app

