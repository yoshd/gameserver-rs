FROM gameserver-base as builder

# for cache
COPY Cargo.toml Cargo.lock /home/builder/gameserver-rs/frontend/
RUN mkdir /home/builder/gameserver-rs/frontend/src && echo "fn main() {}" >> /home/builder/gameserver-rs/frontend/src/main.rs
WORKDIR /home/builder/gameserver-rs/frontend
RUN cargo build --release

COPY . /home/builder/gameserver-rs/frontend
RUN cargo build --release

FROM debian:stretch
RUN useradd -m server

COPY --from=builder /home/builder/gameserver-rs/frontend/target/release/frontend /home/server/frontend
RUN chown -R server /home/server && \
    chmod o+x /home/server/frontend

USER server
ENTRYPOINT /home/server/frontend
