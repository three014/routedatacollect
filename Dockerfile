FROM rust:buster AS base

RUN mkdir /root/google && cd /root/google && git clone https://github.com/three014/googleapis.git
WORKDIR /code
RUN cargo init
COPY route_data_collect/Cargo.toml /code/route_data_collect/Cargo.toml
COPY job_scheduler/Cargo.toml /code/job_scheduler/Cargo.toml
COPY Cargo.toml /code/Cargo.toml
COPY . /code
ARG PB_REL="https://github.com/protocolbuffers/protobuf/releases"
RUN curl -LO $PB_REL/download/v3.15.8/protoc-3.15.8-linux-x86_64.zip && unzip protoc-3.15.8-linux-x86_64.zip -d /usr/local && chmod a+x /usr/local/bin/protoc
RUN cargo fetch

FROM base AS development

EXPOSE 8000

CMD [ "cargo", "run", "--offline", "--bin", "routedatacollect-server" ]


FROM base AS dev-envs

EXPOSE 8000
RUN <<EOF
apt update
apt install -y --no-install-recommends git
EOF

RUN <<EOF
useradd -s /bin/bash -m vscode
groupadd docker
usermod -aG docker vscode
EOF

# install Docker tools (cli, buildx, compose)
COPY --from=gloursdocker/docker / /
CMD [ "cargo", "run", "--offline", "--bin", "routedatacollect-server" ]

FROM base AS builder

RUN cargo build --release --offline

FROM debian:buster-slim

EXPOSE 8000

COPY --from=builder /code/target/release/routedatacollect-server /routedatacollect-server

CMD [ "/routedatacollect-server" ]
