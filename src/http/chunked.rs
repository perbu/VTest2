//! Chunked transfer encoding support
//!
//! This module provides encoding and decoding for HTTP chunked transfer encoding.

use super::{Error, Result, CRLF};
use std::io::{self, Read, Write};

/// Chunked encoder
///
/// Encodes data in HTTP chunked transfer encoding format
pub struct ChunkedEncoder<W: Write> {
    writer: W,
}

impl<W: Write> ChunkedEncoder<W> {
    /// Create a new chunked encoder
    pub fn new(writer: W) -> Self {
        ChunkedEncoder { writer }
    }

    /// Write a chunk of data
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        // Write chunk size in hex
        write!(self.writer, "{:x}{}", data.len(), CRLF)?;

        // Write chunk data
        self.writer.write_all(data)?;

        // Write trailing CRLF
        self.writer.write_all(CRLF.as_bytes())?;

        Ok(())
    }

    /// Write the final chunk (0-sized chunk)
    pub fn finish(&mut self) -> Result<()> {
        write!(self.writer, "0{}{}", CRLF, CRLF)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Get a reference to the underlying writer
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Get a mutable reference to the underlying writer
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Consume the encoder and return the underlying writer
    pub fn into_inner(self) -> W {
        self.writer
    }
}

/// Chunked decoder
///
/// Decodes HTTP chunked transfer encoding format
pub struct ChunkedDecoder {
    state: DecoderState,
    chunk_size: usize,
    chunk_read: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DecoderState {
    ChunkSize,
    ChunkData,
    ChunkEnd,
    Trailer,
    Complete,
}

impl ChunkedDecoder {
    /// Create a new chunked decoder
    pub fn new() -> Self {
        ChunkedDecoder {
            state: DecoderState::ChunkSize,
            chunk_size: 0,
            chunk_read: 0,
        }
    }

    /// Decode a chunk from the input buffer
    ///
    /// Returns (bytes_consumed, bytes_decoded, is_complete)
    pub fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize, bool)> {
        let mut input_pos = 0;
        let mut output_pos = 0;

        while input_pos < input.len() && output_pos < output.len() {
            match self.state {
                DecoderState::ChunkSize => {
                    // Find CRLF
                    if let Some(crlf_pos) = find_crlf(&input[input_pos..]) {
                        let line = String::from_utf8_lossy(&input[input_pos..input_pos + crlf_pos]);

                        // Parse chunk size (hex)
                        let size_str = line.split(';').next().unwrap().trim();
                        self.chunk_size = usize::from_str_radix(size_str, 16)
                            .map_err(|_| Error::InvalidChunkSize(size_str.to_string()))?;

                        input_pos += crlf_pos + 2;
                        self.chunk_read = 0;

                        if self.chunk_size == 0 {
                            self.state = DecoderState::Trailer;
                        } else {
                            self.state = DecoderState::ChunkData;
                        }
                    } else {
                        // Need more data
                        break;
                    }
                }

                DecoderState::ChunkData => {
                    let remaining_in_chunk = self.chunk_size - self.chunk_read;
                    let available_input = input.len() - input_pos;
                    let available_output = output.len() - output_pos;

                    let to_copy = remaining_in_chunk.min(available_input).min(available_output);

                    output[output_pos..output_pos + to_copy]
                        .copy_from_slice(&input[input_pos..input_pos + to_copy]);

                    input_pos += to_copy;
                    output_pos += to_copy;
                    self.chunk_read += to_copy;

                    if self.chunk_read == self.chunk_size {
                        self.state = DecoderState::ChunkEnd;
                    } else {
                        // Need more data or output space
                        break;
                    }
                }

                DecoderState::ChunkEnd => {
                    // Expect CRLF
                    if input.len() - input_pos >= 2 {
                        if &input[input_pos..input_pos + 2] != b"\r\n" {
                            return Err(Error::Protocol("Expected CRLF after chunk".to_string()));
                        }
                        input_pos += 2;
                        self.state = DecoderState::ChunkSize;
                    } else {
                        // Need more data
                        break;
                    }
                }

                DecoderState::Trailer => {
                    // Find final CRLF
                    if input.len() - input_pos >= 2 {
                        if &input[input_pos..input_pos + 2] == b"\r\n" {
                            input_pos += 2;
                            self.state = DecoderState::Complete;
                            return Ok((input_pos, output_pos, true));
                        } else {
                            // Trailer headers - skip until we find empty line
                            if let Some(crlf_pos) = find_crlf(&input[input_pos..]) {
                                input_pos += crlf_pos + 2;
                            } else {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }

                DecoderState::Complete => {
                    return Ok((input_pos, output_pos, true));
                }
            }
        }

        Ok((input_pos, output_pos, self.state == DecoderState::Complete))
    }

    /// Check if decoding is complete
    pub fn is_complete(&self) -> bool {
        self.state == DecoderState::Complete
    }

    /// Reset the decoder for reuse
    pub fn reset(&mut self) {
        self.state = DecoderState::ChunkSize;
        self.chunk_size = 0;
        self.chunk_read = 0;
    }
}

impl Default for ChunkedDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Find CRLF in buffer
fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

/// Decode complete chunked body from bytes
pub fn decode_chunked_body(input: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ChunkedDecoder::new();
    let mut output = Vec::new();
    let mut input_pos = 0;

    while input_pos < input.len() {
        let mut temp = vec![0u8; 8192];
        let (consumed, decoded, complete) =
            decoder.decode(&input[input_pos..], &mut temp)?;

        output.extend_from_slice(&temp[..decoded]);
        input_pos += consumed;

        if complete {
            break;
        }
    }

    if !decoder.is_complete() {
        return Err(Error::Incomplete);
    }

    Ok(output)
}

/// Encode data as chunked body
pub fn encode_chunked_body(data: &[u8], chunk_size: usize) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut encoder = ChunkedEncoder::new(&mut output);

    for chunk in data.chunks(chunk_size) {
        encoder.write_chunk(chunk)?;
    }

    encoder.finish()?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_single_chunk() {
        let mut output = Vec::new();
        let mut encoder = ChunkedEncoder::new(&mut output);

        encoder.write_chunk(b"Hello").unwrap();
        encoder.finish().unwrap();

        let expected = b"5\r\nHello\r\n0\r\n\r\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_encode_multiple_chunks() {
        let mut output = Vec::new();
        let mut encoder = ChunkedEncoder::new(&mut output);

        encoder.write_chunk(b"Hello").unwrap();
        encoder.write_chunk(b"World").unwrap();
        encoder.finish().unwrap();

        let expected = b"5\r\nHello\r\n5\r\nWorld\r\n0\r\n\r\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_decode_single_chunk() {
        let input = b"5\r\nHello\r\n0\r\n\r\n";
        let output = decode_chunked_body(input).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_decode_multiple_chunks() {
        let input = b"5\r\nHello\r\n5\r\nWorld\r\n0\r\n\r\n";
        let output = decode_chunked_body(input).unwrap();
        assert_eq!(output, b"HelloWorld");
    }

    #[test]
    fn test_decode_with_extension() {
        // Chunk extensions (after semicolon) should be ignored
        let input = b"5;extension=value\r\nHello\r\n0\r\n\r\n";
        let output = decode_chunked_body(input).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_encode_chunked_body_helper() {
        let data = b"Hello, World!";
        let output = encode_chunked_body(data, 5).unwrap();

        // Should be split into chunks of 5 bytes
        let decoded = decode_chunked_body(&output).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_decoder_incremental() {
        let input = b"5\r\nHello\r\n0\r\n\r\n";
        let mut decoder = ChunkedDecoder::new();
        let mut output = vec![0u8; 100];
        let mut total_decoded = 0;
        let mut total_consumed = 0;

        // Feed data in small chunks (more realistic than 1 byte at a time)
        let chunk_sizes = [3, 4, 3, 2, 5]; // Total = 17, input is 17 bytes
        for &chunk_size in &chunk_sizes {
            let end = (total_consumed + chunk_size).min(input.len());
            if total_consumed >= input.len() {
                break;
            }

            let (consumed, decoded, complete) =
                decoder.decode(&input[total_consumed..end], &mut output[total_decoded..]).unwrap();

            total_consumed += consumed;
            total_decoded += decoded;

            if complete {
                break;
            }
        }

        assert_eq!(&output[..total_decoded], b"Hello");
        assert!(decoder.is_complete());
    }

    #[test]
    fn test_empty_chunks_ignored() {
        let mut output = Vec::new();
        let mut encoder = ChunkedEncoder::new(&mut output);

        encoder.write_chunk(b"").unwrap(); // Should be ignored
        encoder.write_chunk(b"Hello").unwrap();
        encoder.write_chunk(b"").unwrap(); // Should be ignored
        encoder.finish().unwrap();

        let expected = b"5\r\nHello\r\n0\r\n\r\n";
        assert_eq!(output, expected);
    }
}
