# HTTP/2 Performance Benchmark Suite - Summary

This document provides a comprehensive overview of the HTTP/2 performance benchmarking infrastructure created for VTest2.

## Created Files

### 1. Rust Benchmarks
**File:** `/home/user/VTest2/benches/h2_performance.rs` (22KB)

A comprehensive criterion-based benchmark suite measuring Rust HTTP/2 implementation performance across multiple dimensions.

### 2. VTC Test Files
Located in `/home/user/VTest2/tests/`:
- `bench_h2_simple.vtc` - Single stream request/response
- `bench_h2_concurrent_10.vtc` - 10 concurrent HTTP/2 streams
- `bench_h2_large_1mb.vtc` - 1MB body transfer test
- `bench_h2_large_10mb.vtc` - 10MB body transfer test
- `bench_h2_settings.vtc` - Settings exchange test

### 3. Benchmark Execution Script
**File:** `/home/user/VTest2/scripts/benchmark_h2.sh` (12KB, executable)

Shell script to run both C and Rust benchmarks and generate comparison reports.

### 4. Documentation
**File:** `/home/user/VTest2/benches/README.md` (8.2KB)

Comprehensive documentation on benchmark usage, configuration, and interpretation.

### 5. Cargo Configuration
Updated `/home/user/VTest2/Cargo.toml` to include:
- `criterion` dependency with HTML report features
- Benchmark target configuration

## Benchmark Coverage

### Frame Operations (Rust)

#### 1. Frame Header Encoding/Decoding
- **DATA frame headers** - Most common frame type
- **HEADERS frame headers** - Request/response headers
- **SETTINGS frame headers** - Connection configuration
- **Measurement:** Nanoseconds per operation

#### 2. Frame Encoding by Size
- **Sizes tested:** 256B, 1KB, 4KB, 16KB, 64KB
- **Features:** Plain frames and padded frames
- **Throughput:** Bytes per second
- **Purpose:** Identify optimal frame sizes for different scenarios

#### 3. SETTINGS Frame Operations
- Empty settings (minimal overhead)
- Full settings (all 6 parameters)
- Settings ACK (zero-length payload)

### Stream Management (Rust)

#### 4. Stream State Transitions
Tests the stream state machine performance:
- **Idle → Open:** Initial headers sent
- **Open → HalfClosedLocal:** Send END_STREAM
- **Open → Closed:** Both sides close
- **Measurement:** Nanoseconds per transition

#### 5. Concurrent Stream Operations
- **Stream counts:** 10, 50, 100 concurrent streams
- **Operations:** Stream creation and initialization
- **Purpose:** Measure scalability of stream multiplexing

#### 6. Stream Priority Management
- Setting priority specifications
- Multiple streams with different priorities
- Priority spec creation overhead

### Flow Control (Rust)

#### 7. Flow Control Window Operations
- **Small consumption:** 1KB chunks (typical)
- **Large consumption:** 32KB chunks (high throughput)
- **Window updates:** Replenishing consumed window
- **Multiple operations:** Sequential operations pattern
- **Measurement:** Nanoseconds per operation

### Connection Establishment (Rust)

#### 8. Connection Preface
- Preface validation (24 bytes)
- Preface copying to buffer
- **Purpose:** Measure connection startup overhead

#### 9. Settings Exchange
- Initial SETTINGS frame encoding
- SETTINGS ACK encoding
- Full handshake simulation (preface + settings)
- **Purpose:** Measure connection establishment time

### Large Body Transfers (Rust)

#### 10. Body Fragmentation
- **1MB transfer:** ~64 frames at default 16KB frame size
- **10MB transfer:** ~640 frames
- **Operations measured:**
  - Fragmentation logic
  - Encoding all fragments
- **Throughput:** MB/s for complete transfer
- **Purpose:** Identify performance bottlenecks in large transfers

### HPACK Compression (Rust)

#### 11. Header Encoding
- **Simple headers:** Pseudo-headers only (`:method`, `:path`, `:scheme`, `:authority`)
- **Complex headers:** With custom headers (content-type, authorization, user-agent)
- **Measurement:** Time to encode header sets
- **Purpose:** Evaluate compression overhead

#### 12. Header Decoding
- Decoding pre-encoded simple header sets
- **Purpose:** Measure decompression performance

### Integration Tests (Rust)

#### 13. Complete Request Encoding
- **GET request:** Headers-only, END_STREAM flag
- **POST request:** Headers + 1KB body
- **Measurement:** Complete request encoding time
- **Purpose:** End-to-end operation performance

### C Implementation Tests (VTC)

#### 14. Simple Request/Response
- End-to-end HTTP/2 request through VTest
- Includes connection setup, headers, body, teardown
- **Iterations:** 100
- **Measurement:** Total execution time per iteration

#### 15. Concurrent Streams
- 10 concurrent HTTP/2 streams
- **Iterations:** 10 (more expensive)
- **Measurement:** Time to complete all 10 requests

#### 16. Large Body Transfers
- **1MB transfer:** Performance with moderately large bodies
- **10MB transfer:** Performance with very large bodies
- **Iterations:** 5-10 (very expensive)
- **Measurement:** Time and throughput (MB/s)

#### 17. Settings Exchange
- Connection establishment performance
- **Iterations:** 100
- **Measurement:** Handshake completion time

## Metrics Measured

### Performance Metrics

1. **Time Metrics:**
   - Mean execution time
   - Standard deviation
   - Confidence intervals (95%)
   - Outlier detection and handling

2. **Throughput Metrics:**
   - Operations per second
   - Bytes per second
   - MB/s for large transfers
   - Requests per second

3. **Scalability Metrics:**
   - Performance vs concurrent streams
   - Performance vs frame size
   - Performance vs body size

### Statistical Analysis (Criterion)

Criterion provides:
- **Warmup phase:** Eliminates cold-start effects
- **Statistical sampling:** Multiple iterations for accuracy
- **Outlier detection:** Identifies and handles anomalous measurements
- **Regression detection:** Compares against saved baselines
- **Confidence intervals:** 95% confidence for mean time
- **Historical tracking:** Trend analysis over time

## Usage Instructions

### Running Rust Benchmarks Only

```bash
# Run all benchmarks
cargo bench --bench h2_performance

# Run specific groups
cargo bench --bench h2_performance -- frame_encoding
cargo bench --bench h2_performance -- stream_management
cargo bench --bench h2_performance -- flow_control
cargo bench --bench h2_performance -- connection_setup
cargo bench --bench h2_performance -- large_transfers
cargo bench --bench h2_performance -- hpack
cargo bench --bench h2_performance -- integration

# Run specific benchmark
cargo bench --bench h2_performance -- "encode_data_header"

# Save baseline for comparison
cargo bench --bench h2_performance -- --save-baseline main

# Compare against baseline
cargo bench --bench h2_performance -- --baseline main
```

### Running Complete Comparison

```bash
# Build and run both C and Rust benchmarks
./scripts/benchmark_h2.sh
```

This will:
1. Check prerequisites (vtest binary, cargo)
2. Build C implementation (make vtest)
3. Build Rust implementation (cargo build --release)
4. Run C benchmarks with VTC tests
5. Run Rust benchmarks with criterion
6. Generate comparison report
7. Create results in `benchmark_results/`

### Viewing Results

**Rust benchmarks (Criterion):**
```bash
# Open HTML report in browser
open target/criterion/report/index.html
# or
xdg-open target/criterion/report/index.html  # Linux
```

The HTML report includes:
- Interactive charts
- Statistical analysis
- Confidence intervals
- Historical comparisons
- PDF export option

**Comparison report:**
```bash
cat benchmark_results/benchmark_YYYYMMDD_HHMMSS.txt
```

## Interpretation Guide

### Understanding Rust Results

**Example output:**
```
frame_header_encode/encode_data_header
                        time:   [45.234 ns 45.567 ns 45.901 ns]
```

This means:
- **Mean time:** 45.567 nanoseconds
- **Confidence interval:** 45.234 - 45.901 ns (95% confidence)
- **Lower = Better:** Faster encoding is better

**Throughput example:**
```
data_frame_sizes/1024   time:   [1.2345 µs 1.2567 µs 1.2789 µs]
                        thrpt:  [800.23 MiB/s 814.82 MiB/s 829.41 MiB/s]
```

This means:
- Encoding 1KB takes ~1.26 microseconds
- Throughput is ~815 MB/s
- **Higher throughput = Better**

### Understanding C Results

**Example output:**
```
Simple request/response:    12.45 ms    80 ops/sec
```

This means:
- Average time per VTC test iteration: 12.45ms
- Can perform ~80 complete operations per second
- **Lower time = Better**

### Comparison Metrics

The script shows side-by-side comparisons:
```
Simple request/response   C: 12.45 ms   Rust: N/A
```

**Note:** Direct comparison is difficult because:
- C benchmarks measure end-to-end VTC test execution
- Rust benchmarks measure individual component performance
- C includes test harness overhead, I/O, setup/teardown
- Rust measures pure computation time

## Performance Expectations

### Typical Results

Based on modern hardware (4-core CPU, 3GHz+):

**Frame operations:** 40-100 nanoseconds per operation
**Stream state transitions:** 50-150 nanoseconds
**Flow control operations:** 20-50 nanoseconds
**HPACK encoding:** 1-5 microseconds per header set
**Large frame encoding (16KB):** 2-10 microseconds
**Connection establishment:** 5-20 microseconds

**C VTC tests:**
- Simple request: 5-20ms
- Concurrent streams: 50-200ms
- 1MB transfer: 10-50ms
- 10MB transfer: 100-500ms

### Performance Considerations

1. **Frame Size Trade-offs:**
   - Smaller frames: Lower latency, higher overhead
   - Larger frames: Higher throughput, more memory

2. **Concurrent Streams:**
   - More streams: Better multiplexing
   - Overhead increases linearly with stream count

3. **HPACK Compression:**
   - First request: Higher overhead (building dynamic table)
   - Subsequent requests: Better compression

4. **Flow Control:**
   - Larger windows: Better throughput
   - Smaller windows: Better fairness

## Continuous Benchmarking

### Baseline Management

```bash
# Before making changes
cargo bench --bench h2_performance -- --save-baseline before

# After changes
cargo bench --bench h2_performance -- --baseline before

# Criterion will highlight regressions
```

### CI Integration

Add to `.github/workflows/benchmark.yml`:
```yaml
name: Benchmarks
on: [push, pull_request]
jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: cargo bench --bench h2_performance -- --output-format bencher | tee output.txt
      - name: Store results
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-results
          path: output.txt
```

## Troubleshooting

### Common Issues

**1. Benchmarks fail to compile:**
```bash
cargo clean
cargo build --release
cargo bench --bench h2_performance
```

**2. Inconsistent results:**
- Close resource-intensive applications
- Disable CPU frequency scaling:
  ```bash
  # Linux
  sudo cpupower frequency-set --governor performance
  ```
- Run multiple times and look for consistency

**3. VTC tests fail:**
```bash
make clean
make vtest
./vtest tests/bench_h2_simple.vtc  # Test individually
```

**4. Benchmarks take too long:**
- Reduce sample size in `h2_performance.rs`
- Run specific benchmark groups only
- Use `--quick` flag for faster (less accurate) results

### Getting Help

- Check `/home/user/VTest2/benches/README.md` for detailed documentation
- Review Criterion documentation: https://bheisler.github.io/criterion.rs/book/
- Check HTTP/2 spec: RFC 7540
- Review VTest2 documentation in `CLAUDE.md` and `HTTP2.md`

## Future Enhancements

### Potential Additions

1. **Memory benchmarks:** Track allocations and memory usage
2. **Latency percentiles:** P50, P95, P99 measurements
3. **Priority scheduling:** Benchmark priority tree operations
4. **Server push:** Benchmark push promise handling
5. **Error handling:** Benchmark error path performance
6. **Concurrency scaling:** Test with 500, 1000+ streams
7. **Real-world patterns:** Simulate actual HTTP/2 traffic patterns

### Profiling Integration

For deeper analysis:
```bash
# CPU profiling
cargo build --release
perf record --call-graph dwarf ./target/release/deps/h2_performance-*
perf report

# Flamegraph
cargo flamegraph --bench h2_performance

# Memory profiling
valgrind --tool=massif ./target/release/deps/h2_performance-*
```

## Summary

This benchmark suite provides:

✅ **Comprehensive coverage** of HTTP/2 operations
✅ **Statistical rigor** via Criterion framework
✅ **Both implementations** (C and Rust) tested
✅ **Multiple perspectives** (micro and end-to-end)
✅ **Detailed reporting** (HTML and text formats)
✅ **Continuous tracking** for regression detection
✅ **Well documented** for easy usage and extension

The benchmarks are production-ready and can be used for:
- Performance regression detection
- Optimization validation
- Implementation comparison
- Capacity planning
- Performance tuning

---

**Created:** 2025-11-14
**Version:** 1.0
**Maintained by:** VTest2 Project
