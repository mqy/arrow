// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! IPC Compression Utilities

use std::collections::HashMap;

use lzzzz::lz4f;

use crate::error::{ArrowError, Result};
use crate::ipc::gen::Message::*;
use crate::ipc::gen::Schema::*;

pub trait IpcCompressionCodec {
    fn compress(&self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize>;
    fn decompress(&self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize>;
    fn get_compression_type(&self) -> CompressionType;
}

pub type Codec = Box<dyn IpcCompressionCodec>;

#[inline]
pub(crate) fn get_codec_by_type(ctype: CompressionType) -> Result<Codec> {
    if ctype == CompressionType::LZ4_FRAME {
        Ok(Box::new(Lz4CompressionCodec {}))
    } else {
        Err(ArrowError::InvalidArgumentError(format!(
            "IPC CompressionType {:?} not yet supported",
            ctype
        )))
    }
}

#[inline]
pub(crate) fn get_codec_for_read(
    schema_metadata: &HashMap<String, String>,
    metadata_version: MetadataVersion,
    body_compression: &Option<BodyCompression>,
) -> Result<Option<Codec>> {
    if let Some(bc) = body_compression {
        match bc.method() {
            BodyCompressionMethod::BUFFER => {}
            _ => {
                return Err(ArrowError::IoError(format!(
                    "Encountered unsupported body compression method for V5: {:?}",
                    bc.method()
                )));
            }
        }
        if let CompressionType::LZ4_FRAME = bc.codec() {
            return Ok(Some(Box::new(Lz4CompressionCodec {})));
        } else {
            return Err(ArrowError::InvalidArgumentError(format!(
                "IPC codec {:?} not yet supported for V5",
                bc.codec()
            )));
        }
    }

    // Possibly obtain codec information from experimental serialization format
    // in 0.17.x. Ref `cpp/src/arrow/util/compression.cc`.
    if metadata_version == MetadataVersion::V4 {
        if let Some(codec) = schema_metadata.get("ARROW:experimental_compression") {
            if codec == "lz4" {
                return Ok(Some(Box::new(Lz4CompressionCodec {})));
            } else {
                return Err(ArrowError::IoError(format!(
                    "IPC codec {:?} not yet supported for V4",
                    codec
                )));
            }
        }
    }

    Ok(None)
}

pub struct Lz4CompressionCodec {}
const LZ4_BUFFER_SIZE: usize = 4096;

impl IpcCompressionCodec for Lz4CompressionCodec {
    fn compress(&self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize> {
        // compress with lz4 frame.
        let preferences = lz4f::Preferences::default();
        lz4f::compress_to_vec(input_buf, output_buf, &preferences).map_err(|err| {
            ArrowError::CDataInterface(format!(
                "failed to compress with lzzzCodec: {}",
                err
            ))
        })
    }

    fn decompress(&self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize> {
        lz4f::decompress_to_vec(input_buf, output_buf).map_err(|err| {
            ArrowError::CDataInterface(format!(
                "failed to decompress with lzzzCodec: {}",
                err
            ))
        })
    }

    fn get_compression_type(&self) -> CompressionType {
        CompressionType::LZ4_FRAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::Rng;

    use crate::util::test_util::seedable_rng;

    const INPUT_BUFFER_LEN: usize = 256;

    #[test]
    fn test_lz4_roundtrip() {
        if false {
            let mut rng = seedable_rng();
            let mut bytes: Vec<u8> = Vec::with_capacity(INPUT_BUFFER_LEN);

            (0..INPUT_BUFFER_LEN).for_each(|_| {
                bytes.push(rng.gen::<u8>());
            });
        }

        let bytes: Vec<u8> = Vec::with_capacity(0);
        let codec = Lz4CompressionCodec {};
        let mut compressed = Vec::new();
        codec.compress(&bytes, &mut compressed).unwrap();

        let mut decompressed = Vec::new();
        let _ = codec.decompress(&compressed, &mut decompressed).unwrap();
        assert_eq!(decompressed, bytes);
    }
}
