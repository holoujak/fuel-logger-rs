use anyhow::{bail, Result};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tracing::{debug, info};

use crate::gpio::{GpioController, OutputPin};

pub struct Rs485Modbus {
    port: SerialStream,
    baud: u32,
    re_pin: Option<OutputPin>,
    de_pin: Option<OutputPin>,
}

impl Rs485Modbus {
    pub fn new(
        gpio: &GpioController,
        port_path: &str,
        baud: u32,
        re_gpio: Option<u8>,
        de_gpio: Option<u8>,
    ) -> Result<Self> {
        let port = tokio_serial::new(port_path, baud)
            .data_bits(tokio_serial::DataBits::Eight)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .timeout(Duration::from_millis(500))
            .open_native_async()?;

        let re_pin = if let Some(pin) = re_gpio {
            let mut rp = gpio.setup_output_high(pin)?; // HIGH = receiver disabled
            rp.set_high();
            rp.set_low(); // → idle: receive mode
            Some(rp)
        } else {
            None
        };

        let de_pin = if let Some(pin) = de_gpio {
            let mut dp = gpio.setup_output_high(pin)?; // LOW = driver disabled
            dp.set_low();
            Some(dp)
        } else {
            None
        };

        Ok(Self {
            port,
            baud,
            re_pin,
            de_pin,
        })
    }

    /// Sends a MODBUS ASCII request and returns the response as a String.
    pub async fn query(&mut self, frame: &[u8]) -> Result<String> {
        debug!(
            "MODBUS TX: {} bytes: {}",
            frame.len(),
            String::from_utf8_lossy(frame)
        );

        // TX mode (if GPIO pins are configured)
        if let Some(ref mut re) = self.re_pin {
            re.set_high();
        }
        if let Some(ref mut de) = self.de_pin {
            de.set_high();
        }

        self.port.write_all(frame).await?;
        self.port.flush().await?;

        // Keep driver enabled long enough for the last byte to leave UART.
        // At 8N1/8N2 this is roughly 10-11 bits per byte on the wire.
        let tx_bits = (frame.len() as u64) * 11;
        let tx_ms = ((tx_bits * 1000) / (self.baud as u64)).max(1);
        tokio::time::sleep(Duration::from_millis(tx_ms + 2)).await;

        // RX mode (if GPIO pins are configured)
        if let Some(ref mut de) = self.de_pin {
            de.set_low();
        }
        if let Some(ref mut re) = self.re_pin {
            re.set_low();
        }

        debug!("MODBUS waiting for response (timeout=2000ms)...");

        let response = tokio::time::timeout(Duration::from_millis(2000), async {
            let mut acc = Vec::with_capacity(256);
            let mut chunk = [0u8; 64];
            let mut last_byte_time = std::time::Instant::now();

            loop {
                match self.port.read(&mut chunk).await {
                    Ok(0) => {
                        bail!("Serial port returned EOF while waiting for MODBUS response");
                    }
                    Ok(n) => {
                        acc.extend_from_slice(&chunk[..n]);
                        last_byte_time = std::time::Instant::now();
                        debug!(
                            "MODBUS RX: received {} bytes, buffer now {} bytes total",
                            n,
                            acc.len()
                        );

                        // A MODBUS ASCII frame ends with CR, LF, or both.
                        // Stop reading if we see either or if we haven't received anything for 100ms.
                        if acc.ends_with(b"\r\n") || acc.ends_with(b"\n") || acc.ends_with(b"\r") {
                            debug!("MODBUS RX: detected frame end (CRLF/LF/CR)");
                            break;
                        }
                        if acc.len() >= 512 {
                            debug!("MODBUS RX: buffer reached 512 bytes, stopping");
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // Timeout on read: check if we should give up or keep waiting.
                        if last_byte_time.elapsed() > Duration::from_millis(200) && !acc.is_empty()
                        {
                            debug!(
                                "MODBUS RX: no data for 200ms, stopping with {} bytes",
                                acc.len()
                            );
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(e) => {
                        bail!("Serial port read error: {e}");
                    }
                }
            }
            Ok::<String, anyhow::Error>(String::from_utf8_lossy(&acc).into_owned())
        })
        .await
        .map_err(|_| anyhow::anyhow!("Timed out waiting for MODBUS response (>2000ms)"))??;

        info!("MODBUS RX response: {}", response.trim_end());
        parse_modbus_ascii_response(&response)?;
        Ok(response)
    }
}

/// Builds a MODBUS ASCII frame: :AAFFDDDDD...LL\r\n
pub fn build_frame(addr: u8, func: u8, data: &[u8]) -> Vec<u8> {
    let mut payload = vec![addr, func];
    payload.extend_from_slice(data);
    let lrc = lrc_checksum(&payload);
    let hex: String = payload.iter().map(|b| format!("{:02X}", b)).collect();
    format!(":{}{:02X}\r\n", hex, lrc).into_bytes()
}

/// Builds a MODBUS ASCII frame for function 0x03 (Read Holding Registers).
///
/// `register_number` is the 1-based register number from device documentation
/// (e.g. 0001 means first register). It is converted to MODBUS zero-based address.
pub fn build_read_holding_registers_frame(
    addr: u8,
    register_number: u16,
    count: u16,
) -> Result<Vec<u8>> {
    if register_number == 0 {
        bail!("register_number must be >= 1");
    }
    if count == 0 {
        bail!("count must be >= 1");
    }

    let start = register_number - 1;
    let data = [
        (start >> 8) as u8,
        (start & 0xFF) as u8,
        (count >> 8) as u8,
        (count & 0xFF) as u8,
    ];
    Ok(build_frame(addr, 0x03, &data))
}

/// Parses MODBUS ASCII response for function 0x03 and returns register words.
pub fn parse_read_holding_registers_response(s: &str) -> Result<Vec<u16>> {
    debug!(
        "Parsing MODBUS read_holding_registers response: {:?}",
        s.trim()
    );

    let bytes = parse_ascii_frame(s)?;
    if bytes.len() < 5 {
        bail!("Response frame too short: {} bytes", bytes.len());
    }

    let func = bytes[1];
    if (func & 0x80) != 0 {
        let exc = bytes.get(2).copied().unwrap_or(0);
        bail!("MODBUS exception response: function=0x{func:02X}, code=0x{exc:02X}");
    }
    if func != 0x03 {
        bail!("Unexpected MODBUS function in response: 0x{func:02X}");
    }

    let byte_count = bytes[2] as usize;
    debug!(
        "MODBUS response: byte_count={} (declared), frame_len={}",
        byte_count,
        bytes.len()
    );

    // Handle case where device reports incorrect byte_count but sends less data.
    // Calculate actual data available.
    let actual_data_start = 3;
    let actual_data_bytes = bytes.len() - actual_data_start;

    if actual_data_bytes == 0 {
        bail!("No register data in MODBUS response");
    }

    if actual_data_bytes != byte_count && actual_data_bytes % 2 == 0 {
        debug!(
            "MODBUS byte_count mismatch: declared={}, actual={}. Using actual data.",
            byte_count, actual_data_bytes
        );
    } else if byte_count != actual_data_bytes {
        bail!(
            "Byte count mismatch in MODBUS response: expected {}, got {}",
            byte_count,
            actual_data_bytes
        );
    }

    if (actual_data_bytes % 2) != 0 {
        bail!(
            "Register payload length must be even, got {} bytes",
            actual_data_bytes
        );
    }

    let mut out = Vec::with_capacity(actual_data_bytes / 2);
    for chunk in bytes[actual_data_start..].chunks_exact(2) {
        out.push(u16::from_be_bytes([chunk[0], chunk[1]]));
    }
    debug!(
        "Successfully parsed {} registers from MODBUS response",
        out.len()
    );
    Ok(out)
}

/// Decodes two 16-bit registers to a big-endian u32 value.
pub fn decode_u32_be(words: &[u16]) -> Result<u32> {
    if words.len() < 2 {
        bail!("Expected at least 2 registers, got {}", words.len());
    }
    let bytes = [
        (words[0] >> 8) as u8,
        (words[0] & 0xFF) as u8,
        (words[1] >> 8) as u8,
        (words[1] & 0xFF) as u8,
    ];
    Ok(u32::from_be_bytes(bytes))
}

/// Decodes two 16-bit registers to a big-endian IEEE754 f32 value.
pub fn decode_f32_be(words: &[u16]) -> Result<f32> {
    Ok(f32::from_bits(decode_u32_be(words)?))
}

fn lrc_checksum(data: &[u8]) -> u8 {
    let sum: u8 = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    (!sum).wrapping_add(1)
}

fn parse_ascii_frame(s: &str) -> Result<Vec<u8>> {
    let raw = s.trim();
    debug!("Parsing ASCII frame: {:?}", raw);

    let start = raw
        .find(':')
        .ok_or_else(|| anyhow::anyhow!("Missing ':' at the start of MODBUS ASCII response"))?;

    // Extract only the first MODBUS ASCII frame from the buffer.
    // Devices may append unsolicited text records on the same serial stream.
    let rest = &raw[start + 1..];
    let end = rest
        .find(|c: char| !c.is_ascii_hexdigit())
        .unwrap_or(rest.len());
    let hex = &rest[..end];

    if !hex.len().is_multiple_of(2) {
        bail!(
            "MODBUS ASCII payload must contain an even number of hex chars: '{}' (len={})",
            hex,
            hex.len()
        );
    }

    let mut frame = Vec::with_capacity(hex.len() / 2);
    let mut i = 0;
    while i < hex.len() {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| {
            anyhow::anyhow!(
                "Invalid hex in MODBUS ASCII payload at offset {}: {} (chars: '{}')",
                i,
                e,
                &hex[i..i + 2]
            )
        })?;
        frame.push(byte);
        i += 2;
    }

    if frame.len() < 3 {
        bail!("MODBUS ASCII frame too short: {} bytes", frame.len());
    }

    let data_len = frame.len() - 1;
    let expected_lrc = lrc_checksum(&frame[..data_len]);
    let got_lrc = frame[data_len];
    if expected_lrc != got_lrc {
        bail!("LRC mismatch: expected 0x{expected_lrc:02X}, got 0x{got_lrc:02X}");
    }

    debug!("ASCII frame parsed successfully: {} bytes", frame.len());
    Ok(frame[..data_len].to_vec())
}

fn parse_modbus_ascii_response(s: &str) -> Result<()> {
    let s = s.trim();
    debug!("Validating MODBUS ASCII frame format: {:?}", s);

    if !s.starts_with(':') {
        bail!("Missing ':' at the start of MODBUS ASCII response");
    }
    if s.is_empty() || s.len() < 5 {
        bail!("MODBUS response too short: {} chars", s.len());
    }

    // Check for proper termination
    if s.ends_with("\r\n") || s.ends_with("\n") || s.ends_with("\r") {
        debug!("MODBUS frame properly terminated");
    } else {
        debug!("MODBUS frame missing terminator, but accepting it (will be trimmed)");
    }

    Ok(())
}
