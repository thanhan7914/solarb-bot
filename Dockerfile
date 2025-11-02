FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y curl git build-essential pkg-config libssl-dev libudev-dev ca-certificates llvm libclang-dev \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

# RUN curl --proto '=https' --tlsv1.2 -sSfL https://solana-install.solana.workers.dev | bash

# ENV PATH="/root/.cargo/bin:/root/.local/share/solana/install/active_release/bin:$PATH"

# RUN solana --version && rustc --version

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain 1.87.0

ENV PATH="/root/.cargo/bin:$PATH"

RUN rustc --version

WORKDIR /app

CMD ["/bin/bash"]
