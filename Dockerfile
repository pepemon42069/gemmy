FROM rust:alpine AS builder 
RUN apk add --no-cache protobuf bash g++ python3 make openssl-dev pkgconf 
ENV LDFLAGS="-L/usr/lib/ -lssl -lcrypto"
ENV OPENSSL_DIR="/usr"
WORKDIR /app
COPY . .
RUN cargo build --release

FROM rust:alpine
COPY --from=builder /app/target/release/gemmy-engine /app/gemmy-engine
ENTRYPOINT ["/app/gemmy-engine"]