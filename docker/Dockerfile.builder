FROM rust:slim
ENV RUST_BACKTRACE 1
RUN apt-get update && apt-get install -y perl cmake make g++ librdkafka-dev curl pkg-config libssl-dev protobuf-compiler git patch
#RUN update-ca-certificates

WORKDIR /usr/src/app

