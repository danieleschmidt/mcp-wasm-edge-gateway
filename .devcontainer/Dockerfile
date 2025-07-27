FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

# Install additional system dependencies
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        # Build tools
        build-essential \
        pkg-config \
        libssl-dev \
        libudev-dev \
        # WASM tools
        binaryen \
        wabt \
        # Cross-compilation tools
        gcc-aarch64-linux-gnu \
        gcc-arm-linux-gnueabihf \
        # Hardware security tools
        tpm2-tools \
        libtspi-dev \
        # Monitoring tools
        htop \
        iotop \
        # Network tools
        curl \
        wget \
        netcat \
        # Development tools
        git \
        vim \
        jq \
        # Performance tools
        valgrind \
        perf-tools-unstable \
    && apt-get autoremove -y && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Install Rust targets for cross-compilation
USER vscode
RUN rustup target add \
    wasm32-wasi \
    wasm32-unknown-unknown \
    aarch64-unknown-linux-gnu \
    armv7-unknown-linux-gnueabihf \
    x86_64-pc-windows-gnu \
    aarch64-apple-darwin

# Install WASM tools
RUN cargo install \
    wasm-pack \
    wasm-bindgen-cli \
    wasmtime-cli \
    wasmer-cli \
    cargo-watch \
    cargo-expand \
    cargo-audit \
    cargo-deny \
    cargo-outdated \
    cargo-machete \
    cargo-bloat \
    twiggy

# Install additional development tools
RUN cargo install \
    just \
    hyperfine \
    tokei \
    git-cliff \
    cargo-release

# Configure Git for container
RUN git config --global --add safe.directory /workspace

# Set up shell aliases and functions
RUN echo 'alias ll="ls -la"' >> ~/.bashrc \
    && echo 'alias la="ls -la"' >> ~/.bashrc \
    && echo 'alias ..="cd .."' >> ~/.bashrc \
    && echo 'alias ...="cd ../.."' >> ~/.bashrc \
    && echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc

# Create workspace directories
USER root
RUN mkdir -p /workspace/.cache /workspace/target \
    && chown -R vscode:vscode /workspace

USER vscode