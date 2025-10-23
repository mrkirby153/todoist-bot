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

FROM debian:bookworm-slim AS runner

RUN apt-get update && apt-get install -y ca-certificates && apt-get clean

RUN mkdir /app
WORKDIR /app

COPY --from=builder /app/target/release/todoist-bot /app/todoist-bot

CMD ["/app/todoist-bot"]
EXPOSE 3000