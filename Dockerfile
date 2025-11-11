FROM rust:1.90 as base

RUN mkdir /app
WORKDIR /app

RUN cargo install cargo-chef --locked

FROM base as planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM ubuntu:24.04 AS runner

RUN apt-get update && apt-get install -y ca-certificates && apt-get clean

RUN mkdir /app
WORKDIR /app

COPY --from=builder /app/target/release/todoist-bot /app/todoist-bot
COPY --from=builder /app/target/release/emoji-sync /app/emoji-sync
ADD docker_entrypoint.sh /app/docker_entrypoint.sh
ADD emoji/ /app/emoji/
RUN chmod +x /app/docker_entrypoint.sh

ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]

CMD ["/app/docker_entrypoint.sh"]
EXPOSE 3000