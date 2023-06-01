# syntax=docker/dockerfile:1

FROM rust:buster AS base

RUN mkdir /root/google && cd /root/google && git clone https://github.com/three014/googleapis.git
WORKDIR /code
RUN cargo init
COPY routedatacollect/Cargo.toml /code/routedatacollect/Cargo.toml
COPY job_scheduler/Cargo.toml /code/job_scheduler/Cargo.toml
COPY Cargo.toml /code/Cargo.toml
COPY . /code
ARG PB_REL="https://github.com/protocolbuffers/protobuf/releases"
RUN curl -LO $PB_REL/download/v3.15.8/protoc-3.15.8-linux-x86_64.zip && unzip protoc-3.15.8-linux-x86_64.zip -d /usr/local && chmod a+x /usr/local/bin/protoc && rm protoc-3.15.8-linux-x86_64.zip
RUN cargo fetch

FROM base AS development
CMD [ "cargo", "run", "--offline", "--bin", "routedatacollect" ]

FROM base AS builder
RUN cargo install --path ./routedatacollect

FROM debian:buster-slim AS release
COPY --from=builder /usr/local/cargo/bin/routedatacollect /usr/local/bin/routedatacollect
RUN mkdir /var/log/routedatacollect && chmod a+wr /var/log/routedatacollect/
CMD [ "routedatacollect" ]