# =============================================================================
# Stage 1 — dependencies cache
#   Pre-build an empty project so Cargo's registry + crate downloads are
#   cached in a separate layer.  Re-runs only when Cargo.toml/lock changes.
# =============================================================================
FROM rust:1.82-bookworm AS deps

WORKDIR /build

# Install system libraries required to compile Iced (X11 + Wayland + OpenGL)
RUN apt-get update -y && apt-get install -y --no-install-recommends \
    pkg-config \
    libxkbcommon-dev \
    libwayland-dev \
    libegl1-mesa-dev \
    libgl1-mesa-dev \
    mesa-common-dev \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libx11-dev \
    libxi-dev \
    libdbus-1-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy only the manifests first to exploit layer caching
COPY Cargo.toml Cargo.lock ./

# Create a stub main so Cargo can resolve and download crates without the
# real source.  We build in debug so the cache fill is fast.
RUN mkdir -p src/frontend src/backend && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build 2>/dev/null || true && \
    rm -rf src


# =============================================================================
# Stage 2 — test runner
#   Builds the real source and executes the full test suite.
#   Entry point: cargo test
# =============================================================================
FROM deps AS test

WORKDIR /build

# Install ExifTool (needed by integration paths; tests use in-memory DB so
# ExifTool absence is handled gracefully in unit tests)
RUN apt-get update -y && apt-get install -y --no-install-recommends \
    libimage-exiftool-perl \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Touch source files so Cargo sees them as newer than the stub build
RUN touch src/main.rs src/frontend/main.rs

CMD ["cargo", "test", "--all", "--", "--test-thread=1"]


# =============================================================================
# Stage 3 — release builder
#   Compiles an optimised release binary.
# =============================================================================
FROM deps AS builder

WORKDIR /build

COPY . .
RUN touch src/main.rs src/frontend/main.rs && \
    cargo build --release


# =============================================================================
# Stage 4 — runtime image
#   Slim image that only contains the binary and its runtime dependencies.
#   Requires the host to expose an X11 socket for the GUI.
# =============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update -y && apt-get install -y --no-install-recommends \
    # Iced runtime (X11 / Wayland / OpenGL)
    libxkbcommon0 \
    libwayland-client0 \
    libgl1 \
    libegl1 \
    mesa-utils \
    libxcb1 \
    libxcb-render0 \
    libxcb-shape0 \
    libxcb-xfixes0 \
    libx11-6 \
    libxi6 \
    libdbus-1-3 \
    # ExifTool
    libimage-exiftool-perl \
    # CA certs for any future network calls
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for better container security
RUN useradd -ms /bin/bash appuser
USER appuser
WORKDIR /home/appuser

COPY --from=builder /build/target/release/metadata-cleaner /usr/local/bin/metadata-cleaner

# Data directory for the SQLite database
RUN mkdir -p /home/appuser/.local/share

ENTRYPOINT ["metadata-cleaner"]
