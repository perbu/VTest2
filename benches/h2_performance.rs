//! HTTP/2 Performance Benchmarks
//!
//! Comprehensive performance benchmarks comparing C and Rust HTTP/2 implementations.
//!
//! This benchmark suite measures:
//! - Frame encoding/decoding performance
//! - Connection establishment (preface + settings exchange)
//! - Single stream request/response
//! - Multiple concurrent streams (10, 50, 100)
//! - Large body transfers (1MB, 10MB)
//! - HPACK header compression/decompression
//! - Flow control window management
//!
//! Run with: cargo bench --bench h2_performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use vtest2::http::h2::{
    codec::FrameCodec,
    frames::{FrameType, FrameFlags, DataFrame, HeadersFrame, SettingsFrame, PrioritySpec},
    settings::{Settings, SettingsBuilder},
    flow_control::FlowControlWindow,
    stream::{StreamState, H2Stream},
    CONNECTION_PREFACE, DEFAULT_INITIAL_WINDOW_SIZE, DEFAULT_MAX_FRAME_SIZE,
};
use bytes::Bytes;
use std::time::Duration;

// ========== Frame Encoding/Decoding Benchmarks ==========

fn bench_frame_header_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_header_encode");

    group.bench_function("encode_data_header", |b| {
        b.iter(|| {
            let header = FrameCodec::encode_header(
                black_box(FrameType::Data),
                black_box(FrameFlags::from_u8(0x01)),
                black_box(1),
                black_box(1024),
            );
            black_box(header);
        });
    });

    group.bench_function("encode_headers_header", |b| {
        b.iter(|| {
            let header = FrameCodec::encode_header(
                black_box(FrameType::Headers),
                black_box(FrameFlags::from_u8(0x05)),
                black_box(1),
                black_box(4096),
            );
            black_box(header);
        });
    });

    group.bench_function("encode_settings_header", |b| {
        b.iter(|| {
            let header = FrameCodec::encode_header(
                black_box(FrameType::Settings),
                black_box(FrameFlags::empty()),
                black_box(0),
                black_box(36),
            );
            black_box(header);
        });
    });

    group.finish();
}

fn bench_frame_header_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_header_decode");

    // Pre-encoded headers for decoding benchmarks
    let data_header = FrameCodec::encode_header(FrameType::Data, FrameFlags::from_u8(0x01), 1, 1024);
    let headers_header = FrameCodec::encode_header(FrameType::Headers, FrameFlags::from_u8(0x05), 1, 4096);
    let settings_header = FrameCodec::encode_header(FrameType::Settings, FrameFlags::empty(), 0, 36);

    group.bench_function("decode_data_header", |b| {
        b.iter(|| {
            let result = FrameCodec::decode_header(black_box(&data_header));
            black_box(result);
        });
    });

    group.bench_function("decode_headers_header", |b| {
        b.iter(|| {
            let result = FrameCodec::decode_header(black_box(&headers_header));
            black_box(result);
        });
    });

    group.bench_function("decode_settings_header", |b| {
        b.iter(|| {
            let result = FrameCodec::decode_header(black_box(&settings_header));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_data_frame_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_frame_encode");
    group.throughput(Throughput::Bytes(1024));

    let data = Bytes::from(vec![0u8; 1024]);

    group.bench_function("1kb_no_padding", |b| {
        b.iter(|| {
            let frame = DataFrame::new(black_box(1), black_box(data.clone()), black_box(false));
            let encoded = FrameCodec::encode_data_frame(black_box(&frame));
            black_box(encoded);
        });
    });

    group.bench_function("1kb_with_padding", |b| {
        b.iter(|| {
            let mut frame = DataFrame::new(black_box(1), black_box(data.clone()), black_box(false));
            frame.padding = Some(16);
            let encoded = FrameCodec::encode_data_frame(black_box(&frame));
            black_box(encoded);
        });
    });

    group.finish();
}

fn bench_data_frame_various_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_frame_sizes");

    for size in [256, 1024, 4096, 16384, 65536].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        let data = Bytes::from(vec![0u8; *size]);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let frame = DataFrame::new(black_box(1), black_box(data.clone()), black_box(false));
                let encoded = FrameCodec::encode_data_frame(black_box(&frame));
                black_box(encoded);
            });
        });
    }

    group.finish();
}

fn bench_settings_frame_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("settings_frame_encode");

    group.bench_function("empty_settings", |b| {
        b.iter(|| {
            let settings = Settings::default();
            let frame = SettingsFrame::new(black_box(settings));
            let encoded = FrameCodec::encode_settings_frame(black_box(&frame));
            black_box(encoded);
        });
    });

    group.bench_function("full_settings", |b| {
        b.iter(|| {
            let settings = SettingsBuilder::new()
                .header_table_size(4096)
                .enable_push(true)
                .max_concurrent_streams(100)
                .initial_window_size(65535)
                .max_frame_size(16384)
                .max_header_list_size(8192)
                .build()
                .unwrap();
            let frame = SettingsFrame::new(black_box(settings));
            let encoded = FrameCodec::encode_settings_frame(black_box(&frame));
            black_box(encoded);
        });
    });

    group.bench_function("settings_ack", |b| {
        b.iter(|| {
            let frame = SettingsFrame::ack();
            let encoded = FrameCodec::encode_settings_frame(black_box(&frame));
            black_box(encoded);
        });
    });

    group.finish();
}

// ========== Stream State Management Benchmarks ==========

fn bench_stream_state_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_state_transitions");

    group.bench_function("idle_to_open", |b| {
        b.iter(|| {
            let mut stream = H2Stream::new(black_box(1));
            assert_eq!(stream.state(), StreamState::Idle);
            let _ = stream.send_headers(black_box(false));
            black_box(stream);
        });
    });

    group.bench_function("open_to_half_closed_local", |b| {
        b.iter(|| {
            let mut stream = H2Stream::new(black_box(1));
            let _ = stream.send_headers(black_box(false));
            let _ = stream.send_data(black_box(1024), black_box(true));
            black_box(stream);
        });
    });

    group.bench_function("open_to_closed", |b| {
        b.iter(|| {
            let mut stream = H2Stream::new(black_box(1));
            let _ = stream.send_headers(black_box(false));
            let headers_frame = HeadersFrame::new(black_box(1), Bytes::new(), black_box(false), black_box(true));
            let _ = stream.receive_headers(black_box(&headers_frame));
            let _ = stream.send_data(black_box(1024), black_box(true));
            let data_frame = DataFrame::new(black_box(1), Bytes::from(vec![0u8; 1024]), black_box(true));
            let _ = stream.receive_data(black_box(&data_frame));
            black_box(stream);
        });
    });

    group.finish();
}

// ========== Flow Control Benchmarks ==========

fn bench_flow_control(c: &mut Criterion) {
    let mut group = c.benchmark_group("flow_control");

    group.bench_function("consume_small", |b| {
        b.iter(|| {
            let mut window = FlowControlWindow::new();
            let result = window.consume(black_box(1024)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("consume_large", |b| {
        b.iter(|| {
            let mut window = FlowControlWindow::new();
            let result = window.consume(black_box(32768)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("increase_window", |b| {
        b.iter(|| {
            let mut window = FlowControlWindow::new();
            let _ = window.consume(black_box(32768));
            let _ = window.increase(black_box(32768));
            black_box(window);
        });
    });

    group.bench_function("multiple_operations", |b| {
        b.iter(|| {
            let mut window = FlowControlWindow::new();
            for _ in 0..10 {
                let _ = window.consume(black_box(1024));
                let _ = window.increase(black_box(512));
            }
            black_box(window);
        });
    });

    group.finish();
}

// ========== Connection Establishment Benchmarks ==========

fn bench_connection_preface(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_preface");
    group.throughput(Throughput::Bytes(CONNECTION_PREFACE.len() as u64));

    group.bench_function("verify_preface", |b| {
        b.iter(|| {
            let preface = CONNECTION_PREFACE;
            let valid = preface == b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
            black_box(valid);
        });
    });

    group.bench_function("copy_preface", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(CONNECTION_PREFACE.len());
            buf.extend_from_slice(black_box(CONNECTION_PREFACE));
            black_box(buf);
        });
    });

    group.finish();
}

fn bench_settings_exchange(c: &mut Criterion) {
    let mut group = c.benchmark_group("settings_exchange");

    group.bench_function("encode_initial_settings", |b| {
        b.iter(|| {
            let settings = SettingsBuilder::new()
                .initial_window_size(DEFAULT_INITIAL_WINDOW_SIZE)
                .max_frame_size(DEFAULT_MAX_FRAME_SIZE)
                .build()
                .unwrap();
            let frame = SettingsFrame::new(black_box(settings));
            let encoded = FrameCodec::encode_settings_frame(black_box(&frame));
            black_box(encoded);
        });
    });

    group.bench_function("encode_settings_ack", |b| {
        b.iter(|| {
            let frame = SettingsFrame::ack();
            let encoded = FrameCodec::encode_settings_frame(black_box(&frame));
            black_box(encoded);
        });
    });

    group.bench_function("full_handshake_encode", |b| {
        b.iter(|| {
            // Client preface
            let mut buf = Vec::with_capacity(CONNECTION_PREFACE.len() + 100);
            buf.extend_from_slice(CONNECTION_PREFACE);

            // Initial SETTINGS
            let settings = SettingsBuilder::new()
                .initial_window_size(DEFAULT_INITIAL_WINDOW_SIZE)
                .max_frame_size(DEFAULT_MAX_FRAME_SIZE)
                .build()
                .unwrap();
            let frame = SettingsFrame::new(settings);
            buf.extend_from_slice(&FrameCodec::encode_settings_frame(&frame));

            black_box(buf);
        });
    });

    group.finish();
}

// ========== Large Body Transfer Benchmarks ==========

fn bench_large_body_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_body_fragmentation");

    for body_size in [1024 * 1024, 10 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*body_size as u64));
        let body = vec![0u8; *body_size];
        let max_frame_size = DEFAULT_MAX_FRAME_SIZE as usize;

        group.bench_with_input(
            BenchmarkId::new("fragment", format!("{}MB", body_size / (1024 * 1024))),
            body_size,
            |b, _| {
                b.iter(|| {
                    let chunks: Vec<_> = body.chunks(max_frame_size).collect();
                    let frame_count = chunks.len();
                    black_box(frame_count);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("encode_all_frames", format!("{}MB", body_size / (1024 * 1024))),
            body_size,
            |b, _| {
                b.iter(|| {
                    let mut frames = Vec::new();
                    let chunks: Vec<_> = body.chunks(max_frame_size).collect();

                    for (i, chunk) in chunks.iter().enumerate() {
                        let is_last = i == chunks.len() - 1;
                        let frame = DataFrame::new(
                            black_box(1),
                            Bytes::from(chunk.to_vec()),
                            black_box(is_last),
                        );
                        frames.push(FrameCodec::encode_data_frame(&frame));
                    }

                    black_box(frames);
                });
            },
        );
    }

    group.finish();
}

// ========== Concurrent Stream Benchmarks ==========

fn bench_concurrent_streams(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_streams");

    for stream_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(stream_count),
            stream_count,
            |b, &count| {
                b.iter(|| {
                    let mut streams = Vec::with_capacity(count);
                    for i in 0..count {
                        let stream_id = (i * 2 + 1) as u32; // Odd IDs for client
                        let mut stream = H2Stream::new(black_box(stream_id));
                        let _ = stream.send_headers(black_box(false));
                        streams.push(stream);
                    }
                    black_box(streams);
                });
            },
        );
    }

    group.finish();
}

fn bench_stream_priority(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_priority");

    group.bench_function("set_priority", |b| {
        b.iter(|| {
            let mut stream = H2Stream::new(black_box(1));
            let priority = PrioritySpec::new(black_box(0), black_box(false), black_box(16));
            stream.set_priority(black_box(priority));
            black_box(stream);
        });
    });

    group.bench_function("multiple_streams_with_priority", |b| {
        b.iter(|| {
            let mut streams = Vec::with_capacity(10);
            for i in 0..10 {
                let stream_id = (i * 2 + 1) as u32;
                let mut stream = H2Stream::new(black_box(stream_id));
                let priority = PrioritySpec::new(black_box(0), black_box(false), black_box(16 + i as u8));
                stream.set_priority(black_box(priority));
                streams.push(stream);
            }
            black_box(streams);
        });
    });

    group.finish();
}

// ========== HPACK Benchmarks ==========

fn bench_hpack_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("hpack_encoding");

    group.bench_function("encode_simple_headers", |b| {
        b.iter(|| {
            let mut encoder = hpack::Encoder::new();
            let headers = vec![
                (b":method".to_vec(), b"GET".to_vec()),
                (b":path".to_vec(), b"/".to_vec()),
                (b":scheme".to_vec(), b"https".to_vec()),
                (b":authority".to_vec(), b"example.com".to_vec()),
            ];
            let mut buf = Vec::new();
            for (name, value) in headers {
                let header_pair = vec![(black_box(&name[..]), black_box(&value[..]))];
                encoder.encode_into(header_pair, black_box(&mut buf)).unwrap();
            }
            black_box(buf);
        });
    });

    group.bench_function("encode_headers_with_custom", |b| {
        b.iter(|| {
            let mut encoder = hpack::Encoder::new();
            let headers = vec![
                (b":method".to_vec(), b"POST".to_vec()),
                (b":path".to_vec(), b"/api/v1/data".to_vec()),
                (b":scheme".to_vec(), b"https".to_vec()),
                (b":authority".to_vec(), b"api.example.com".to_vec()),
                (b"content-type".to_vec(), b"application/json".to_vec()),
                (b"authorization".to_vec(), b"Bearer token123456".to_vec()),
                (b"user-agent".to_vec(), b"VTest2/1.0".to_vec()),
            ];
            let mut buf = Vec::new();
            for (name, value) in headers {
                let header_pair = vec![(black_box(&name[..]), black_box(&value[..]))];
                encoder.encode_into(header_pair, black_box(&mut buf)).unwrap();
            }
            black_box(buf);
        });
    });

    group.finish();
}

fn bench_hpack_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("hpack_decoding");

    // Pre-encode headers for decoding benchmarks
    let mut encoder = hpack::Encoder::new();
    let simple_headers = vec![
        (b":method".to_vec(), b"GET".to_vec()),
        (b":path".to_vec(), b"/".to_vec()),
        (b":scheme".to_vec(), b"https".to_vec()),
        (b":authority".to_vec(), b"example.com".to_vec()),
    ];
    let mut simple_encoded = Vec::new();
    for (name, value) in &simple_headers {
        let header_pair = vec![(&name[..], &value[..])];
        encoder.encode_into(header_pair, &mut simple_encoded).unwrap();
    }

    group.bench_function("decode_simple_headers", |b| {
        b.iter(|| {
            let mut decoder = hpack::Decoder::new();
            let result = decoder.decode(black_box(&simple_encoded)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// ========== Integration Benchmarks ==========

fn bench_full_request_response_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_request_response");

    group.bench_function("simple_get_request", |b| {
        b.iter(|| {
            // Encode HEADERS frame
            let mut encoder = hpack::Encoder::new();
            let headers = vec![
                (b":method".to_vec(), b"GET".to_vec()),
                (b":path".to_vec(), b"/".to_vec()),
                (b":scheme".to_vec(), b"https".to_vec()),
                (b":authority".to_vec(), b"example.com".to_vec()),
            ];
            let mut header_block = Vec::new();
            for (name, value) in headers {
                let header_pair = vec![(&name[..], &value[..])];
                encoder.encode_into(header_pair, &mut header_block).unwrap();
            }

            let headers_frame = HeadersFrame::new(
                black_box(1),
                Bytes::from(header_block),
                black_box(true),
                black_box(true),
            );
            let encoded = FrameCodec::encode_headers_frame(black_box(&headers_frame));
            black_box(encoded);
        });
    });

    group.bench_function("post_request_with_body", |b| {
        let body_data = vec![0u8; 1024];
        b.iter(|| {
            // Encode HEADERS frame
            let mut encoder = hpack::Encoder::new();
            let headers = vec![
                (b":method".to_vec(), b"POST".to_vec()),
                (b":path".to_vec(), b"/api/data".to_vec()),
                (b":scheme".to_vec(), b"https".to_vec()),
                (b":authority".to_vec(), b"api.example.com".to_vec()),
                (b"content-type".to_vec(), b"application/json".to_vec()),
            ];
            let mut header_block = Vec::new();
            for (name, value) in headers {
                let header_pair = vec![(&name[..], &value[..])];
                encoder.encode_into(header_pair, &mut header_block).unwrap();
            }

            let headers_frame = HeadersFrame::new(
                black_box(1),
                Bytes::from(header_block),
                black_box(false),
                black_box(true),
            );
            let encoded_headers = FrameCodec::encode_headers_frame(&headers_frame);

            // Encode DATA frame
            let data_frame = DataFrame::new(
                black_box(1),
                Bytes::from(body_data.clone()),
                black_box(true),
            );
            let encoded_data = FrameCodec::encode_data_frame(&data_frame);

            black_box((encoded_headers, encoded_data));
        });
    });

    group.finish();
}

// ========== Benchmark Groups ==========

criterion_group! {
    name = frame_encoding;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(1000);
    targets =
        bench_frame_header_encode,
        bench_frame_header_decode,
        bench_data_frame_encode,
        bench_data_frame_various_sizes,
        bench_settings_frame_encode
}

criterion_group! {
    name = stream_management;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(1000);
    targets =
        bench_stream_state_transitions,
        bench_concurrent_streams,
        bench_stream_priority
}

criterion_group! {
    name = flow_control_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(1000);
    targets = bench_flow_control
}

criterion_group! {
    name = connection_setup;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(1000);
    targets =
        bench_connection_preface,
        bench_settings_exchange
}

criterion_group! {
    name = large_transfers;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .sample_size(100);
    targets = bench_large_body_fragmentation
}

criterion_group! {
    name = hpack;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(1000);
    targets =
        bench_hpack_encoding,
        bench_hpack_decoding
}

criterion_group! {
    name = integration;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(500);
    targets = bench_full_request_response_encode
}

criterion_main!(
    frame_encoding,
    stream_management,
    flow_control_benches,
    connection_setup,
    large_transfers,
    hpack,
    integration
);
