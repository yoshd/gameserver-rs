FROM gameserver-base as builder

COPY . /home/builder/gameserver-rs/director
WORKDIR /home/builder/gameserver-rs/director
RUN cargo build --release

FROM debian:stretch
RUN useradd -m worker

COPY --from=builder /home/builder/gameserver-rs/director/target/release/director /home/worker/director
RUN chown -R worker /home/worker && \
    chmod o+x /home/worker/director

USER worker
ENTRYPOINT /home/worker/director
