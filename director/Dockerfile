FROM gameserver-base as builder

# for cache
COPY Cargo.toml Cargo.lock /home/builder/gameserver-rs/director/
RUN mkdir /home/builder/gameserver-rs/director/src && echo "fn main() {}" >> /home/builder/gameserver-rs/director/src/main.rs
WORKDIR /home/builder/gameserver-rs/director
RUN cargo build --release

COPY . /home/builder/gameserver-rs/director
RUN cargo build --release

FROM debian:stretch
RUN useradd -m worker

COPY --from=builder /home/builder/gameserver-rs/director/target/release/director /home/worker/director
RUN chown -R worker /home/worker && \
    chmod o+x /home/worker/director

USER worker
ENTRYPOINT /home/worker/director
