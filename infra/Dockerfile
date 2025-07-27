# Stage 1: build and run benchmarks
FROM rust:1.88.0-bookworm AS builder

RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
        python3 python3-pip git curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
COPY Cargo.toml Cargo.lock ./
COPY benches ./benches
COPY src ./src

RUN cargo build --release --locked

RUN mkdir -p target/criterion

RUN cargo bench

# Stage 2 analysis of data
FROM python:3.12-slim AS final

WORKDIR /analysis

COPY data-analysis/requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY --from=builder /workspace/target/criterion ./target/criterion

COPY data-analysis/*.py ./

RUN mkdir -p results artifacts target/criterion

ENTRYPOINT ["bash", "-c", "\
    echo 'Step 1: Converting Criterion data to CSV...' && \
    python export_raw_data.py && \
    echo 'Step 2: Generating visualizations...' && \
    python visualize_benchmarks.py --data-dir target/criterion --output-dir results && \
    echo 'Step 3: Running efficiency analysis...' && \
    python evaluate_efficiency.py --data-dir target/criterion && \
    cp -r target/criterion/* artifacts/ 2>/dev/null || true && \
    cp -r results/* artifacts/ 2>/dev/null || true && \
    echo 'Analysis complete!' \
"]