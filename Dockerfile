FROM rust:1.67 AS build-stage
WORKDIR /code
COPY . .
RUN cargo build --release

FROM scratch AS export-stage
COPY --from=build-stage /code/target/release/proxy-client /
