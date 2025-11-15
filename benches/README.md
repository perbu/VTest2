# HTTP/2 Performance Benchmarks

This directory contains comprehensive performance benchmarks for comparing the C and Rust HTTP/2 implementations in VTest2.

## Overview

The benchmark suite measures performance across multiple dimensions:

### 1. Frame Encoding/Decoding
- **Frame Header Operations**: Encode/decode frame headers for DATA, HEADERS, and SETTINGS frames
- **DATA Frame Encoding**: Performance across various payload sizes (256B, 1KB, 4KB, 16KB, 64KB)
- **SETTINGS Frame Encoding**: Empty settings, full settings, and ACK frames
- **Throughput Measurement**: Bytes processed per second for various frame sizes

### 2. Stream Management
- **State Transitions**: Performance of stream state machine transitions
  - Idle → Open
  - Open → HalfClosedLocal
  - Open → Closed
- **Concurrent Streams**: Creating and managing multiple streams (10, 50, 100)
- **Priority Management**: Setting stream priorities and dependencies

### 3. Flow Control
- **Window Operations**:
  - Small data consumption (1KB)
  - Large data consumption (32KB)
  - Window size increases
  - Multiple sequential operations
- **Window Exhaustion**: Behavior when flow control windows are depleted

### 4. Connection Establishment
- **Connection Preface**: Validation and copying of HTTP/2 preface
- **Settings Exchange**:
  - Initial settings frame encoding
  - Settings ACK encoding
  - Full handshake simulation
- **Performance**: Time to establish connection ready state

### 5. Large Body Transfers
- **Body Fragmentation**: Splitting large bodies into frames
  - 1MB body transfer
  - 10MB body transfer
- **Frame Encoding**: Encoding all fragments for complete transfer
- **Throughput**: MB/s for large transfers

### 6. HPACK Compression
- **Header Encoding**:
  - Simple headers (`:method`, `:path`, `:scheme`, `:authority`)
  - Complex headers (with custom headers and authorization)
- **Header Decoding**: Decompressing encoded header blocks
- **Compression Ratio**: Bytes saved via HPACK

### 7. Integration Tests
- **Complete Request Encoding**:
  - Simple GET requests
  - POST requests with body
- **End-to-End Performance**: Full request/response cycle timing

## Running the Benchmarks

### Rust Benchmarks (Criterion)

Run all benchmarks:
```bash
cargo bench --bench h2_performance
```

Run specific benchmark groups:
```bash
cargo bench --bench h2_performance -- frame_encoding
cargo bench --bench h2_performance -- stream_management
cargo bench --bench h2_performance -- flow_control
cargo bench --bench h2_performance -- connection_setup
cargo bench --bench h2_performance -- large_transfers
cargo bench --bench h2_performance -- hpack
cargo bench --bench h2_performance -- integration
```

Run specific benchmarks:
```bash
cargo bench --bench h2_performance -- "encode_data_header"
cargo bench --bench h2_performance -- "concurrent_streams"
```

### C + Rust Comparison

Run the comprehensive comparison script:
```bash
./scripts/benchmark_h2.sh
```

This script will:
1. Build both C and Rust implementations
2. Run C implementation benchmarks using VTC tests
3. Run Rust implementation benchmarks using Criterion
4. Generate comparison reports

## Benchmark Results

### Criterion Output

Criterion generates detailed HTML reports at:
```
target/criterion/report/index.html
```

Open this file in a browser to view:
- Mean execution time with confidence intervals
- Standard deviation and outliers
- Regression analysis
- Historical comparison charts
- Detailed statistical analysis

### Text Output

The comparison script generates text reports at:
```
benchmark_results/benchmark_YYYYMMDD_HHMMSS.txt
```

## Understanding the Results

### Metrics Reported

1. **Mean Time**: Average execution time across all iterations
2. **Standard Deviation**: Variability in execution time
3. **Throughput**: Operations or bytes per second
4. **Confidence Interval**: 95% confidence interval for the mean

### Interpreting Performance

- **Lower time = Better performance** for timing benchmarks
- **Higher throughput = Better performance** for data transfer benchmarks
- **Smaller confidence intervals = More consistent performance**
- **Outliers**: Identified and handled by Criterion's statistical analysis

### Comparing C vs Rust

The benchmark script provides:
- Direct timing comparisons where applicable
- Percentage differences between implementations
- Identification of faster implementation
- Notes about measurement methodology differences

Note: C benchmarks measure end-to-end VTC test execution (including setup/teardown), while Rust benchmarks measure individual component performance. This makes direct comparison challenging but provides insights into both implementations.

## Benchmark Configuration

### Criterion Settings

From `h2_performance.rs`:
- **Sample size**: 100-1000 iterations depending on benchmark
- **Measurement time**: 10-15 seconds per benchmark
- **Warmup**: Automatic warmup before measurement
- **Outlier detection**: Statistical outlier identification and handling

### VTC Test Settings

From `benchmark_h2.sh`:
- **Iterations**: 100 (for simple tests)
- **Warmup**: 10 iterations
- **Reduced iterations**: 10-50 for expensive tests (large transfers)

## Customizing Benchmarks

### Adding New Rust Benchmarks

1. Add benchmark function to `h2_performance.rs`:
```rust
fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");
    group.bench_function("test_case", |b| {
        b.iter(|| {
            // Code to benchmark
        });
    });
    group.finish();
}
```

2. Add to criterion_group macro:
```rust
criterion_group! {
    name = my_group;
    config = Criterion::default();
    targets = bench_my_feature
}
```

3. Add to criterion_main:
```rust
criterion_main!(my_group, /* other groups */);
```

### Adding New VTC Tests

1. Create test file in `tests/`:
```vtc
vtest "My benchmark test"

server s1 {
    # Server behavior
} -start

client c1 -connect ${s1_sock} {
    # Client behavior
} -run
```

2. Add to benchmark script's C benchmark function

## Performance Optimization Tips

### For HTTP/2 Implementation

1. **Frame Encoding**: Minimize memory allocations during encoding
2. **HPACK**: Consider static table usage for common headers
3. **Flow Control**: Batch window updates when possible
4. **Stream Management**: Use efficient data structures for stream lookup
5. **Buffer Management**: Reuse buffers to reduce allocations

### For Benchmarking

1. **System Load**: Run benchmarks on idle system
2. **CPU Frequency**: Disable CPU frequency scaling
3. **Background Processes**: Minimize background activity
4. **Multiple Runs**: Run benchmarks multiple times
5. **Profiling**: Use perf/flamegraph for detailed analysis

## Continuous Benchmarking

### Tracking Performance Regressions

1. Run benchmarks before changes:
```bash
cargo bench --bench h2_performance -- --save-baseline main
```

2. Make your changes

3. Compare against baseline:
```bash
cargo bench --bench h2_performance -- --baseline main
```

Criterion will highlight any performance regressions.

### CI Integration

Add to CI pipeline:
```yaml
- name: Run benchmarks
  run: cargo bench --bench h2_performance -- --output-format bencher | tee output.txt
```

Store results for historical tracking.

## Troubleshooting

### Benchmark Fails to Compile

```bash
cargo clean
cargo build --release
cargo bench --bench h2_performance
```

### VTC Tests Fail

Ensure vtest is built:
```bash
make clean
make vtest
```

### Inconsistent Results

- Close unnecessary applications
- Disable CPU frequency scaling
- Run multiple times and average results
- Check system load with `top` or `htop`

### Missing Dependencies

Install criterion:
```bash
cargo update
cargo build --dev --release
```

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [RFC 7540 - HTTP/2](https://tools.ietf.org/html/rfc7540)
- [RFC 7541 - HPACK](https://tools.ietf.org/html/rfc7541)
- [VTest2 HTTP/2 Documentation](../HTTP2.md)

## Contributing

When adding new benchmarks:

1. Follow existing naming conventions
2. Add appropriate throughput measurements
3. Include documentation comments
4. Update this README
5. Verify benchmarks run successfully
6. Check for performance regressions

## License

Same license as VTest2 (BSD-2-Clause)
