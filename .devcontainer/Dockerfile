FROM mcr.microsoft.com/devcontainers/rust:0-1-buster
RUN apt-get update && apt install -y git curl
RUN mkdir /root/google && cd /root/google && git clone https://github.com/three014/googleapis.git
ARG PB_REL="https://github.com/protocolbuffers/protobuf/releases"
RUN curl -LO $PB_REL/download/v3.15.8/protoc-3.15.8-linux-x86_64.zip && unzip protoc-3.15.8-linux-x86_64.zip -d /usr/local && chmod a+x /usr/local/bin/protoc
