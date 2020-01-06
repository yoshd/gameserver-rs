FROM rust:1.40.0 as builder
RUN useradd -m build

RUN rustup component add rustfmt

# Rust SDK depends on https://github.com/pingcap/grpc-rs and it requires CMake and Go

RUN apt-get update && apt-get install -y cmake

ENV GO_VERSION=1.13.5 \
    GO_CHECKSUM=512103d7ad296467814a6e3f635631bd35574cab3369a97a323c9a585ccaa569
RUN mkdir -p /usr/local/go \
    && curl -fSO https://dl.google.com/go/go${GO_VERSION}.linux-amd64.tar.gz \
    && shasum -a 256 go${GO_VERSION}.linux-amd64.tar.gz | grep ${GO_CHECKSUM} \
    && tar xf go${GO_VERSION}.linux-amd64.tar.gz -C /usr/local/go --strip-components=1 \
    && rm -f go${GO_VERSION}.linux-amd64.tar.gz
ENV PATH $PATH:/usr/local/go/bin

COPY . /home/builder/gameserver-rs
WORKDIR /home/builder/gameserver-rs
RUN cargo build --release


FROM debian:stretch
RUN useradd -m server

COPY --from=builder /home/builder/gameserver-rs/target/release/gameserver /home/server/gameserver
RUN chown -R server /home/server && \
    chmod o+x /home/server/gameserver

USER server
ENTRYPOINT /home/server/gameserver
