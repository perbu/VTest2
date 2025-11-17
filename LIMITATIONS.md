# VTest2 Limitations and Known Issues

This document tracks known limitations and issues in VTest2 that should be addressed in future versions.

## Buffer Size Limitations

### HTTP Response Buffer (2MB limit)
- **Location**: `src/vtc_http.c:2008`
- **Current Size**: 2MB (2048 * 1024 bytes)
- **Issue**: Tests with large response bodies (>2MB) will fail with assertion error
- **Workaround**: Keep test response bodies under ~1.9MB to account for headers
- **Example**: `bench_h2_large_10mb.vtc` was originally designed for 10MB but had to be reduced to 1.5MB
- **Fix Required**: Consider making buffer size configurable or dynamically allocating based on Content-Length

## Test Organization

### Terminal-Based Tests
Terminal emulation tests have been moved to `tests/terminal/` subdirectory to keep them separate from standard HTTP/HTTP2 tests. These tests use the `process` command with terminal features like:
- Screen dumps (`-screen-dump`)
- Text matching (`-match-text`, `-expect-text`)
- Cursor position checking (`-expect-cursor`)
- Terminal escape sequences

Current terminal tests:
- `tests/terminal/a00000.vtc` - Comprehensive vtest self-test with extensive terminal emulation
- `tests/terminal/a00001.vtc` - Teken terminal emulator test (requires `vttest` binary)
- `tests/terminal/a00009.vtc` - Process text matching test

## Test Execution Issues Fixed

### vtest Binary Path
- **Issue**: Tests that invoke `vtest` as a subprocess (meta-tests) failed because `vtest` was not in PATH
- **Tests Affected**: `a00000.vtc`, `a00023.vtc`
- **Fix**: Modified tests to use `${pwd}/vtest` macro instead of bare `vtest` command
- **Status**: Fixed in this commit

## Skipped Tests

Several tests are skipped due to missing features or dependencies:
- `tests/a00014.vtc` - Unknown reason for skip
- `tests/a00025.vtc` - Unknown reason for skip
- `tests/a00027.vtc` - Unknown reason for skip
- `tests/a00028.vtc` - Unknown reason for skip
- `tests/a00029.vtc` - Unknown reason for skip
- `tests/a02022.vtc` - Unknown reason for skip
- `tests/a02027.vtc` - Unknown reason for skip
- `tests/terminal/a00001.vtc` - Requires `vttest` binary (not installed by default)

**TODO**: Investigate why tests are skipped and document specific requirements

## Potential Future Improvements

### 1. Configurable Buffer Sizes
Allow users to configure HTTP buffer sizes via command-line options or test-level directives:
```vtc
# Proposed syntax
client c1 -rxbuf 10m -connect ${s1_sock} {
    txreq
    rxresp
}
```

### 2. Dynamic Buffer Allocation
Automatically allocate receive buffers based on Content-Length header when known, falling back to current default for streaming responses.

### 3. Better Error Messages
When buffer overflow occurs, provide clearer error message indicating:
- Current buffer size
- Required size based on Content-Length
- Suggestion to reduce test size or increase buffer

### 4. Test Discovery
Currently tests must be explicitly listed. Consider:
- Recursive test discovery in subdirectories
- Pattern-based test filtering (e.g., `./vtest tests/**/*.vtc`)
- Test categories/tags for selective execution

## Notes

This file should be updated whenever:
- New limitations are discovered
- Workarounds are found for existing issues
- Issues are fixed (move from active to "Fixed" section)
- Architectural decisions are made that constrain functionality
