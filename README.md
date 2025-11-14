# VTest2

HTTP test program derived from Varnish's varnishtest. Tests HTTP clients, servers, and proxies using `.vtc` test scripts.

Plug-in replacement for [vtest1](https://github.com/vtest/VTest).

## Building

```bash
make vtest                              # Build standalone
make varnishtest VARNISH_SRC=/path      # Build with Varnish support
make test                               # Build and run tests
```

### Dependencies

**Linux:** libpcre2-dev, zlib, libssl-dev
**macOS:** Same via Homebrew (OpenSSL via `brew install openssl@3`)

## Usage

```bash
./vtest tests/a00001.vtc                # Run single test
./vtest -j4 tests/*.vtc                 # Run tests in parallel
```

Test files start with `vtest` or `varnishtest` followed by a description. See `tests/` directory for examples.

## Rust HTTP/2 Implementation

VTest2 includes a complete HTTP/2 protocol implementation written in Rust, providing:

- **Low-level frame control** - Direct frame construction for testing edge cases
- **Complete frame support** - All HTTP/2 frame types (DATA, HEADERS, SETTINGS, PING, etc.)
- **Flow control** - Connection and stream-level window management
- **Stream multiplexing** - Multiple concurrent streams per connection
- **HPACK compression** - Header compression/decompression
- **TLS with ALPN** - HTTP/2 over TLS with protocol negotiation

### Testing Features

- 192+ passing tests (153 unit, 24 HTTP/2 integration, 6 HTTP, 9 network)
- Frame encoding/decoding validation
- Flow control violation detection
- Invalid frame sequence testing
- Large body transfer handling
- Concurrent stream management

See `HTTP2.md` for detailed documentation and usage examples.

## Syncing with Varnish-Cache

For maintainers: `make update` syncs shared code from Varnish-Cache. Set `VARNISHSRC` to use a local repo instead of cloning.

## Documentation

- `CLAUDE.md` - Architecture details and development guide
- `TLS-IMPL.md` - TLS support documentation
- `HTTP2.md` - HTTP/2 implementation and testing guide
