//! RIFF/WAVE, BWF and RF64 decoding.
//!
//! Field recorders write three shapes this module has to accept: plain WAVE, Broadcast Wave
//! with a `bext` chunk carrying the slate description and a sample-accurate time reference,
//! and RF64 for anything past 4 GB. The chunk walker is shared, so a file that mixes them
//! (an RF64 BWF, which is what a 96 kHz 24-bit multitrack recorder produces) decodes too.

use std::path::Path;

use crate::buffer::AudioBuffer;
use crate::error::AudioError;

/// Format tag for integer PCM.
const FORMAT_PCM: u16 = 0x0001;
/// Format tag for IEEE floating point.
const FORMAT_FLOAT: u16 = 0x0003;
/// Format tag for `WAVE_FORMAT_EXTENSIBLE`, whose real tag lives in the sub-format GUID.
const FORMAT_EXTENSIBLE: u16 = 0xFFFE;
/// The size the `bext` chunk must reach before the time reference is readable.
const BEXT_MIN_LEN: usize = 348;

/// The Broadcast Wave fields the review UI shows and the exporter copies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BwfMetadata {
    /// `bext` description, which field recorders fill with the slate.
    pub description: String,
    /// The recorder or company that wrote the file.
    pub originator: String,
    /// The originator's own reference, often the roll and take.
    pub originator_reference: String,
    /// Origination date as written, `YYYY-MM-DD`.
    pub origination_date: String,
    /// Origination time as written, `HH:MM:SS`.
    pub origination_time: String,
    /// Sample offset since midnight, the BWF time reference.
    pub time_reference: u64,
}

impl BwfMetadata {
    /// Whether every field is empty, which means the chunk carried nothing useful.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.description.is_empty()
            && self.originator.is_empty()
            && self.originator_reference.is_empty()
            && self.origination_date.is_empty()
            && self.origination_time.is_empty()
            && self.time_reference == 0
    }
}

/// A decoded file: the samples plus whatever BWF metadata it carried.
#[derive(Debug, Clone, PartialEq)]
pub struct WavFile {
    /// The decoded samples.
    pub audio: AudioBuffer,
    /// The `bext` fields, when the file has a readable chunk.
    pub bwf: Option<BwfMetadata>,
}

/// Decode a WAV, BWF or RF64 file from memory.
///
/// # Errors
///
/// Returns [`AudioError::NotRiff`] or [`AudioError::NotWave`] for a foreign container,
/// [`AudioError::MissingChunk`] when `fmt ` or `data` is absent,
/// [`AudioError::Truncated`] when a chunk overruns the file, and the format and shape
/// variants of [`AudioError`] for a file this decoder cannot represent.
pub fn parse(bytes: &[u8]) -> Result<WavFile, AudioError> {
    let mut reader = Reader::new(bytes);
    let magic = reader.take_array::<4>("RIFF")?;
    let is_rf64 = match &magic {
        b"RIFF" => false,
        b"RF64" => true,
        other => return Err(AudioError::NotRiff(*other)),
    };
    let _riff_size = reader.u32("RIFF")?;
    let form = reader.take_array::<4>("WAVE")?;
    if &form != b"WAVE" {
        return Err(AudioError::NotWave(form));
    }

    let mut format: Option<Format> = None;
    let mut data: Option<&[u8]> = None;
    let mut bwf: Option<BwfMetadata> = None;
    let mut ds64: Option<Ds64> = None;

    while reader.remaining() >= 8 {
        let id = reader.take_array::<4>("chunk")?;
        let declared = reader.u32("chunk")?;
        let size = usize::try_from(declared).unwrap_or(usize::MAX);

        match &id {
            b"ds64" => {
                let body = reader.chunk_body("ds64", size)?;
                ds64 = Some(Ds64::parse(body)?);
                continue;
            }
            b"fmt " => {
                let body = reader.chunk_body("fmt ", size)?;
                format = Some(Format::parse(body)?);
                continue;
            }
            b"bext" => {
                let body = reader.chunk_body("bext", size)?;
                bwf = Some(parse_bext(body));
                continue;
            }
            b"data" => {
                let effective = if is_rf64 && declared == u32::MAX {
                    ds64.map_or(0, |d| d.data_size)
                } else {
                    u64::from(declared)
                };
                let body =
                    reader.chunk_body("data", usize::try_from(effective).unwrap_or(usize::MAX))?;
                data = Some(body);
                continue;
            }
            _ => {
                reader.skip_chunk("chunk", size)?;
                continue;
            }
        }
    }

    if is_rf64 && ds64.is_none() {
        return Err(AudioError::Rf64WithoutDs64);
    }
    let format = format.ok_or(AudioError::MissingChunk("fmt "))?;
    let data = data.ok_or(AudioError::MissingChunk("data"))?;

    let audio = format.decode(data)?;
    Ok(WavFile { audio, bwf })
}

/// Decode a WAV, BWF or RF64 file from disk.
///
/// # Errors
///
/// Returns [`AudioError::Io`] when the file cannot be read, and everything [`parse`] returns.
pub fn decode_file(path: &Path) -> Result<WavFile, AudioError> {
    let bytes = std::fs::read(path).map_err(|source| AudioError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse(&bytes)
}

/// The 64-bit sizes an RF64 file keeps in its `ds64` chunk.
#[derive(Debug, Clone, Copy)]
struct Ds64 {
    data_size: u64,
}

impl Ds64 {
    fn parse(body: &[u8]) -> Result<Self, AudioError> {
        if body.len() < 24 {
            return Err(AudioError::Truncated {
                chunk: "ds64",
                claimed: 24,
                available: body.len() as u64,
            });
        }
        Ok(Self {
            data_size: u64::from_le_bytes(body[8..16].try_into().unwrap_or([0; 8])),
        })
    }
}

/// The `fmt ` chunk, reduced to what decoding needs.
#[derive(Debug, Clone, Copy)]
struct Format {
    tag: u16,
    channels: u16,
    sample_rate: u32,
    bits: u16,
}

impl Format {
    fn parse(body: &[u8]) -> Result<Self, AudioError> {
        if body.len() < 16 {
            return Err(AudioError::Truncated {
                chunk: "fmt ",
                claimed: 16,
                available: body.len() as u64,
            });
        }
        let declared = u16::from_le_bytes([body[0], body[1]]);
        let channels = u16::from_le_bytes([body[2], body[3]]);
        let sample_rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
        let bits = u16::from_le_bytes([body[14], body[15]]);

        // An extensible header repeats the real format tag in the first two bytes of the
        // sub-format GUID at offset 24.
        let tag = if declared == FORMAT_EXTENSIBLE && body.len() >= 26 {
            u16::from_le_bytes([body[24], body[25]])
        } else {
            declared
        };

        if channels == 0 {
            return Err(AudioError::NoChannels);
        }
        if sample_rate == 0 {
            return Err(AudioError::NoSampleRate);
        }
        Ok(Self {
            tag,
            channels,
            sample_rate,
            bits,
        })
    }

    fn bytes_per_sample(&self) -> Result<usize, AudioError> {
        match (self.tag, self.bits) {
            (FORMAT_PCM, 8 | 16 | 24 | 32) => Ok(usize::from(self.bits / 8)),
            (FORMAT_FLOAT, 32 | 64) => Ok(usize::from(self.bits / 8)),
            (FORMAT_PCM | FORMAT_FLOAT, bits) => Err(AudioError::UnsupportedBitDepth {
                format: self.tag,
                bits,
            }),
            (tag, _) => Err(AudioError::UnsupportedFormat(tag)),
        }
    }

    fn decode(&self, data: &[u8]) -> Result<AudioBuffer, AudioError> {
        let width = self.bytes_per_sample()?;
        let frame_bytes = width * usize::from(self.channels);
        // A trailing partial frame is padding, not audio; dropping it keeps the buffer
        // rectangular without inventing samples.
        let frames = data.len() / frame_bytes;
        let mut samples = Vec::with_capacity(frames * usize::from(self.channels));

        for frame in data.chunks_exact(frame_bytes).take(frames) {
            for sample in frame.chunks_exact(width) {
                samples.push(decode_sample(self.tag, self.bits, sample));
            }
        }

        AudioBuffer::new(self.sample_rate, self.channels, samples)
    }
}

/// Scale one little-endian sample to `f32` in `[-1.0, 1.0)`.
fn decode_sample(tag: u16, bits: u16, bytes: &[u8]) -> f32 {
    match (tag, bits) {
        (FORMAT_PCM, 8) => (f32::from(bytes[0]) - 128.0) / 128.0,
        (FORMAT_PCM, 16) => f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) / 32_768.0,
        (FORMAT_PCM, 24) => {
            let raw = i32::from_le_bytes([0, bytes[0], bytes[1], bytes[2]]) >> 8;
            raw as f32 / 8_388_608.0
        }
        (FORMAT_PCM, 32) => {
            let raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            raw as f32 / 2_147_483_648.0
        }
        (FORMAT_FLOAT, 32) => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        (FORMAT_FLOAT, 64) => f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]) as f32,
        _ => 0.0,
    }
}

/// Read the fixed `bext` fields, trimming the NUL and space padding recorders leave behind.
fn parse_bext(body: &[u8]) -> BwfMetadata {
    let field = |range: std::ops::Range<usize>| -> String {
        let end = range.end.min(body.len());
        let start = range.start.min(end);
        body.get(start..end)
            .map(|bytes| {
                String::from_utf8_lossy(bytes)
                    .trim_end_matches(['\0', ' '])
                    .to_string()
            })
            .unwrap_or_default()
    };

    BwfMetadata {
        description: field(0..256),
        originator: field(256..288),
        originator_reference: field(288..320),
        origination_date: field(320..330),
        origination_time: field(330..338),
        time_reference: if body.len() >= BEXT_MIN_LEN {
            u64::from_le_bytes(body[338..346].try_into().unwrap_or([0; 8]))
        } else {
            0
        },
    }
}

/// A bounds-checked cursor over the file.
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    fn take_array<const N: usize>(&mut self, chunk: &'static str) -> Result<[u8; N], AudioError> {
        let slice = self.take(chunk, N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    fn u32(&mut self, chunk: &'static str) -> Result<u32, AudioError> {
        Ok(u32::from_le_bytes(self.take_array::<4>(chunk)?))
    }

    fn take(&mut self, chunk: &'static str, len: usize) -> Result<&'a [u8], AudioError> {
        if self.remaining() < len {
            return Err(AudioError::Truncated {
                chunk,
                claimed: len as u64,
                available: self.remaining() as u64,
            });
        }
        let slice = &self.bytes[self.position..self.position + len];
        self.position += len;
        Ok(slice)
    }

    /// Take a chunk body and step over the pad byte an odd length implies.
    fn chunk_body(&mut self, chunk: &'static str, len: usize) -> Result<&'a [u8], AudioError> {
        let body = self.take(chunk, len)?;
        if len % 2 == 1 && self.remaining() > 0 {
            self.position += 1;
        }
        Ok(body)
    }

    fn skip_chunk(&mut self, chunk: &'static str, len: usize) -> Result<(), AudioError> {
        self.chunk_body(chunk, len).map(|_| ())
    }
}

/// Encode a buffer as 16-bit PCM WAVE, the interchange format Whisper reads.
///
/// Samples are clamped to `[-1.0, 1.0]` first, so a float file that peaks above full scale
/// is limited rather than wrapped to the opposite polarity.
#[must_use]
pub fn encode_pcm16(buffer: &AudioBuffer) -> Vec<u8> {
    let channels = buffer.channels();
    let block_align = channels * 2;
    let data_len = buffer.samples().len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);

    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&FORMAT_PCM.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&buffer.sample_rate().to_le_bytes());
    out.extend_from_slice(&(buffer.sample_rate() * u32::from(block_align)).to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());

    for sample in buffer.samples() {
        let scaled = (sample.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
        out.extend_from_slice(&scaled.to_le_bytes());
    }

    out
}

/// Write a buffer as 16-bit PCM WAVE.
///
/// # Errors
///
/// Returns [`AudioError::Io`] when the file cannot be written.
pub fn write_pcm16(path: &Path, buffer: &AudioBuffer) -> Result<(), AudioError> {
    std::fs::write(path, encode_pcm16(buffer)).map_err(|source| AudioError::Io {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a WAVE file with one `fmt ` and one `data` chunk.
    fn wav(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&tag.to_le_bytes());
        fmt.extend_from_slice(&channels.to_le_bytes());
        fmt.extend_from_slice(&rate.to_le_bytes());
        let block_align = channels * bits / 8;
        fmt.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
        fmt.extend_from_slice(&block_align.to_le_bytes());
        fmt.extend_from_slice(&bits.to_le_bytes());
        assemble(&[(*b"fmt ", fmt), (*b"data", data.to_vec())])
    }

    /// Wrap chunks in a RIFF container with correct sizes.
    fn assemble(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(b"WAVE");
        for (id, payload) in chunks {
            body.extend_from_slice(id);
            body.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            body.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                body.push(0);
            }
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn bext(description: &str, time_reference: u64) -> Vec<u8> {
        let mut body = vec![0_u8; 602];
        body[..description.len()].copy_from_slice(description.as_bytes());
        body[256..256 + 4].copy_from_slice(b"Zoom");
        body[288..288 + 6].copy_from_slice(b"ROLL01");
        body[320..330].copy_from_slice(b"2026-11-14");
        body[330..338].copy_from_slice(b"09:41:00");
        body[338..346].copy_from_slice(&time_reference.to_le_bytes());
        body
    }

    #[test]
    fn sixteen_bit_stereo_decodes_to_scaled_samples() {
        let data = [0x00, 0x00, 0xFF, 0x7F, 0x00, 0x80, 0x00, 0x40];

        let file = parse(&wav(FORMAT_PCM, 2, 48_000, 16, &data)).expect("decoded");

        assert_eq!(file.audio.sample_rate(), 48_000);
        assert_eq!(file.audio.channels(), 2);
        assert_eq!(file.audio.frames(), 2);
        let samples = file.audio.samples();
        assert!((samples[0] - 0.0).abs() < 1e-6);
        assert!((samples[1] - 0.999_969_5).abs() < 1e-6);
        assert!((samples[2] + 1.0).abs() < 1e-6);
        assert!((samples[3] - 0.5).abs() < 1e-6);
        assert_eq!(file.bwf, None);
    }

    #[test]
    fn eight_bit_pcm_is_unsigned_and_centred_on_128() {
        let data = [0_u8, 128, 255];

        let file = parse(&wav(FORMAT_PCM, 1, 8_000, 8, &data)).expect("decoded");

        assert_eq!(file.audio.samples(), &[-1.0, 0.0, 0.992_187_5]);
    }

    #[test]
    fn twenty_four_bit_pcm_sign_extends() {
        // -1 LSB, 0, +1 LSB, and half scale.
        let data = [
            0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x40,
        ];

        let file = parse(&wav(FORMAT_PCM, 1, 96_000, 24, &data)).expect("decoded");

        let samples = file.audio.samples();
        assert!((samples[0] + 1.0 / 8_388_608.0).abs() < 1e-9);
        assert_eq!(samples[1], 0.0);
        assert!((samples[2] - 1.0 / 8_388_608.0).abs() < 1e-9);
        assert!((samples[3] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn thirty_two_bit_pcm_scales_by_two_to_the_thirty_one() {
        let data = i32::MIN.to_le_bytes();

        let file = parse(&wav(FORMAT_PCM, 1, 48_000, 32, &data)).expect("decoded");

        assert_eq!(file.audio.samples(), &[-1.0]);
    }

    #[test]
    fn thirty_two_bit_float_passes_through() {
        let data = 0.25_f32.to_le_bytes();

        let file = parse(&wav(FORMAT_FLOAT, 1, 48_000, 32, &data)).expect("decoded");

        assert_eq!(file.audio.samples(), &[0.25]);
    }

    #[test]
    fn sixty_four_bit_float_narrows_to_thirty_two() {
        let data = (-0.5_f64).to_le_bytes();

        let file = parse(&wav(FORMAT_FLOAT, 1, 48_000, 64, &data)).expect("decoded");

        assert_eq!(file.audio.samples(), &[-0.5]);
    }

    #[test]
    fn an_extensible_header_reads_the_subformat_tag() {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&FORMAT_EXTENSIBLE.to_le_bytes());
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&48_000_u32.to_le_bytes());
        fmt.extend_from_slice(&96_000_u32.to_le_bytes());
        fmt.extend_from_slice(&2_u16.to_le_bytes());
        fmt.extend_from_slice(&16_u16.to_le_bytes());
        fmt.extend_from_slice(&22_u16.to_le_bytes());
        fmt.extend_from_slice(&16_u16.to_le_bytes());
        fmt.extend_from_slice(&4_u32.to_le_bytes());
        fmt.extend_from_slice(&FORMAT_PCM.to_le_bytes());
        fmt.extend_from_slice(&[0_u8; 14]);
        let file = parse(&assemble(&[
            (*b"fmt ", fmt),
            (*b"data", 0x4000_i16.to_le_bytes().to_vec()),
        ]))
        .expect("decoded");

        assert_eq!(file.audio.channels(), 1);
        assert!((file.audio.samples()[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn an_odd_sized_chunk_is_padded_and_skipped() {
        let mut bytes = assemble(&[
            (*b"junk", vec![1, 2, 3]),
            (*b"fmt ", {
                let mut fmt = Vec::new();
                fmt.extend_from_slice(&FORMAT_PCM.to_le_bytes());
                fmt.extend_from_slice(&1_u16.to_le_bytes());
                fmt.extend_from_slice(&16_000_u32.to_le_bytes());
                fmt.extend_from_slice(&32_000_u32.to_le_bytes());
                fmt.extend_from_slice(&2_u16.to_le_bytes());
                fmt.extend_from_slice(&16_u16.to_le_bytes());
                fmt
            }),
            (*b"data", 0x2000_i16.to_le_bytes().to_vec()),
        ]);
        // The assembler already padded `junk`; assert the file still parses.
        bytes.truncate(bytes.len());
        let file = parse(&bytes).expect("decoded");

        assert_eq!(file.audio.frames(), 1);
    }

    #[test]
    fn a_bext_chunk_yields_the_slate_and_time_reference() {
        let bytes = assemble(&[
            (*b"fmt ", {
                let mut fmt = Vec::new();
                fmt.extend_from_slice(&FORMAT_PCM.to_le_bytes());
                fmt.extend_from_slice(&1_u16.to_le_bytes());
                fmt.extend_from_slice(&48_000_u32.to_le_bytes());
                fmt.extend_from_slice(&96_000_u32.to_le_bytes());
                fmt.extend_from_slice(&2_u16.to_le_bytes());
                fmt.extend_from_slice(&16_u16.to_le_bytes());
                fmt
            }),
            (*b"bext", bext("INT kitchen, door slam", 48_000)),
            (*b"data", 0_i16.to_le_bytes().to_vec()),
        ]);

        let file = parse(&bytes).expect("decoded");
        let bwf = file.bwf.expect("bext present");

        assert_eq!(bwf.description, "INT kitchen, door slam");
        assert_eq!(bwf.originator, "Zoom");
        assert_eq!(bwf.originator_reference, "ROLL01");
        assert_eq!(bwf.origination_date, "2026-11-14");
        assert_eq!(bwf.origination_time, "09:41:00");
        assert_eq!(bwf.time_reference, 48_000);
        assert!(!bwf.is_empty());
    }

    #[test]
    fn an_empty_bext_is_reported_as_empty() {
        let metadata = parse_bext(&[0_u8; 602]);

        assert!(metadata.is_empty());
        assert_eq!(metadata.time_reference, 0);
    }

    #[test]
    fn a_short_bext_still_parses_its_text_fields() {
        let metadata = parse_bext(b"take 3");

        assert_eq!(metadata.description, "take 3");
        assert_eq!(metadata.time_reference, 0);
    }

    #[test]
    fn an_rf64_file_reads_its_data_size_from_ds64() {
        let mut ds64 = Vec::new();
        ds64.extend_from_slice(&0_u64.to_le_bytes());
        ds64.extend_from_slice(&4_u64.to_le_bytes());
        ds64.extend_from_slice(&2_u64.to_le_bytes());
        ds64.extend_from_slice(&0_u32.to_le_bytes());

        let mut fmt = Vec::new();
        fmt.extend_from_slice(&FORMAT_PCM.to_le_bytes());
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&48_000_u32.to_le_bytes());
        fmt.extend_from_slice(&96_000_u32.to_le_bytes());
        fmt.extend_from_slice(&2_u16.to_le_bytes());
        fmt.extend_from_slice(&16_u16.to_le_bytes());

        let mut body = Vec::new();
        body.extend_from_slice(b"WAVE");
        for (id, payload) in [
            (*b"ds64", ds64),
            (*b"fmt ", fmt),
            (*b"data", vec![0x00, 0x20, 0x00, 0xE0]),
        ] {
            body.extend_from_slice(&id);
            let size = if &id == b"data" {
                u32::MAX
            } else {
                payload.len() as u32
            };
            body.extend_from_slice(&size.to_le_bytes());
            body.extend_from_slice(&payload);
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RF64");
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&body);

        let file = parse(&bytes).expect("decoded");

        assert_eq!(file.audio.frames(), 2);
        assert!((file.audio.samples()[0] - 0.25).abs() < 1e-6);
        assert!((file.audio.samples()[1] + 0.25).abs() < 1e-6);
    }

    #[test]
    fn an_rf64_file_without_ds64_is_rejected() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RF64");
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        let error = parse(&bytes).expect_err("no ds64");

        assert!(matches!(error, AudioError::Rf64WithoutDs64));
    }

    #[test]
    fn a_foreign_container_is_rejected() {
        let error = parse(b"OggS\0\0\0\0").expect_err("not riff");

        assert!(matches!(error, AudioError::NotRiff(magic) if &magic == b"OggS"));
    }

    #[test]
    fn a_riff_container_that_is_not_wave_is_rejected() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(b"AVI ");

        let error = parse(&bytes).expect_err("not wave");

        assert!(matches!(error, AudioError::NotWave(form) if &form == b"AVI "));
    }

    #[test]
    fn a_file_without_fmt_is_rejected() {
        let bytes = assemble(&[(*b"data", vec![0, 0])]);

        let error = parse(&bytes).expect_err("no fmt");

        assert!(matches!(error, AudioError::MissingChunk("fmt ")));
    }

    #[test]
    fn a_file_without_data_is_rejected() {
        let bytes = assemble(&[(*b"fmt ", {
            let mut fmt = Vec::new();
            fmt.extend_from_slice(&FORMAT_PCM.to_le_bytes());
            fmt.extend_from_slice(&1_u16.to_le_bytes());
            fmt.extend_from_slice(&8_000_u32.to_le_bytes());
            fmt.extend_from_slice(&16_000_u32.to_le_bytes());
            fmt.extend_from_slice(&2_u16.to_le_bytes());
            fmt.extend_from_slice(&16_u16.to_le_bytes());
            fmt
        })]);

        let error = parse(&bytes).expect_err("no data");

        assert!(matches!(error, AudioError::MissingChunk("data")));
    }

    #[test]
    fn a_data_chunk_that_overruns_the_file_is_rejected() {
        let mut bytes = wav(FORMAT_PCM, 1, 8_000, 16, &[0, 0, 0, 0]);
        // Rewrite the data chunk size to claim more than the file holds.
        let data_size_at = bytes.len() - 4 - 4;
        bytes[data_size_at..data_size_at + 4].copy_from_slice(&9_999_u32.to_le_bytes());

        let error = parse(&bytes).expect_err("overrun");

        assert!(matches!(error, AudioError::Truncated { chunk: "data", .. }));
    }

    #[test]
    fn an_unsupported_format_tag_is_rejected() {
        let error = parse(&wav(0x0006, 1, 8_000, 8, &[0])).expect_err("alaw");

        assert!(matches!(error, AudioError::UnsupportedFormat(0x0006)));
    }

    #[test]
    fn an_unsupported_bit_depth_is_rejected() {
        let error = parse(&wav(FORMAT_PCM, 1, 8_000, 12, &[0, 0])).expect_err("12-bit");

        assert!(matches!(
            error,
            AudioError::UnsupportedBitDepth {
                format: FORMAT_PCM,
                bits: 12
            }
        ));
    }

    #[test]
    fn a_zero_channel_count_is_rejected() {
        let error = parse(&wav(FORMAT_PCM, 0, 8_000, 16, &[])).expect_err("no channels");

        assert!(matches!(error, AudioError::NoChannels));
    }

    #[test]
    fn a_zero_sample_rate_is_rejected() {
        let error = parse(&wav(FORMAT_PCM, 1, 0, 16, &[])).expect_err("no rate");

        assert!(matches!(error, AudioError::NoSampleRate));
    }

    #[test]
    fn a_trailing_partial_frame_is_dropped() {
        // Three bytes for a two-byte frame: one frame plus padding.
        let file = parse(&wav(FORMAT_PCM, 1, 8_000, 16, &[0x00, 0x20, 0xFF])).expect("decoded");

        assert_eq!(file.audio.frames(), 1);
    }

    #[test]
    fn an_empty_data_chunk_decodes_to_no_frames() {
        let file = parse(&wav(FORMAT_PCM, 1, 8_000, 16, &[])).expect("decoded");

        assert!(file.audio.is_empty());
    }

    #[test]
    fn decode_file_reads_from_disk() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("take.wav");
        std::fs::write(&path, wav(FORMAT_PCM, 1, 16_000, 16, &[0x00, 0x20])).expect("written");

        let file = decode_file(&path).expect("decoded");

        assert_eq!(file.audio.sample_rate(), 16_000);
        assert_eq!(file.audio.frames(), 1);
    }

    #[test]
    fn decode_file_names_a_missing_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("absent.wav");

        let error = decode_file(&path).expect_err("missing");

        match error {
            AudioError::Io { path: reported, .. } => {
                assert!(reported.ends_with("absent.wav"), "got {reported}");
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn a_truncated_header_is_rejected() {
        let error = parse(b"RIF").expect_err("short");

        assert!(matches!(error, AudioError::Truncated { chunk: "RIFF", .. }));
    }

    #[test]
    fn an_encoded_buffer_round_trips_through_the_parser() {
        let buffer = AudioBuffer::new(16_000, 1, vec![0.0, 0.5, -0.5, 0.25]).expect("buffer");

        let file = parse(&encode_pcm16(&buffer)).expect("decoded");

        assert_eq!(file.audio.sample_rate(), 16_000);
        assert_eq!(file.audio.channels(), 1);
        assert_eq!(file.audio.frames(), 4);
        for (decoded, original) in file.audio.samples().iter().zip(buffer.samples()) {
            assert!((decoded - original).abs() < 1e-4, "{decoded} vs {original}");
        }
    }

    #[test]
    fn an_encoded_header_declares_its_sizes() {
        let buffer = AudioBuffer::new(48_000, 2, vec![0.0; 8]).expect("buffer");

        let bytes = encode_pcm16(&buffer);

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(
            u32::from_le_bytes(bytes[16..20].try_into().expect("size")),
            16
        );
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().expect("size")),
            16
        );
        assert_eq!(bytes.len(), 44 + 16);
    }

    #[test]
    fn encoding_clamps_instead_of_wrapping() {
        let buffer = AudioBuffer::new(16_000, 1, vec![2.0, -2.0]).expect("buffer");

        let file = parse(&encode_pcm16(&buffer)).expect("decoded");

        assert!((file.audio.samples()[0] - 1.0).abs() < 1e-4);
        assert!((file.audio.samples()[1] + 1.0).abs() < 1e-4);
    }

    #[test]
    fn writing_a_buffer_produces_a_readable_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("prepared.wav");
        let buffer = AudioBuffer::new(16_000, 1, vec![0.1; 160]).expect("buffer");

        write_pcm16(&path, &buffer).expect("written");
        let file = decode_file(&path).expect("decoded");

        assert_eq!(file.audio.frames(), 160);
    }

    #[test]
    fn writing_to_a_missing_directory_names_the_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("absent").join("prepared.wav");
        let buffer = AudioBuffer::new(16_000, 1, vec![0.0]).expect("buffer");

        let error = write_pcm16(&path, &buffer).expect_err("missing directory");

        match error {
            AudioError::Io { path: reported, .. } => {
                assert!(reported.ends_with("prepared.wav"), "got {reported}");
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }
}
