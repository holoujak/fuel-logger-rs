use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::gpio::{GpioController, InputPin};

/// Decoded Wiegand event sent to the station manager.
#[derive(Debug, Clone)]
pub struct WiegandEvent {
    pub station_id: u32,
    pub bits: u32,
    pub code: u32,
}

/// Shared state for accumulating Wiegand bits from the two data lines.
struct WiegandState {
    in_code: bool,
    bits: u32,
    num: u32,
    last_bit_time: Instant,
}

impl WiegandState {
    fn new() -> Self {
        Self {
            in_code: false,
            bits: 0,
            num: 0,
            last_bit_time: Instant::now(),
        }
    }

    /// Record a single bit (0 or 1) from a data line interrupt.
    fn receive_bit(&mut self, value: u32) {
        if !self.in_code {
            self.bits = 1;
            self.num = value;
            self.in_code = true;
        } else {
            self.bits += 1;
            self.num = (self.num << 1) | value;
        }
        self.last_bit_time = Instant::now();
    }

    /// If enough time has elapsed since the last bit, return the accumulated
    /// (bits, code) and reset. Returns `None` if still receiving.
    fn try_complete(&mut self, bit_timeout: Duration) -> Option<(u32, u32)> {
        if self.in_code && self.last_bit_time.elapsed() > bit_timeout {
            let result = (self.bits, self.num);
            self.in_code = false;
            self.bits = 0;
            self.num = 0;
            Some(result)
        } else {
            None
        }
    }
}

/// One Wiegand reader instance (two GPIO pins, D0 and D1).
///
/// Uses rppal async interrupts on both pins and a background task that
/// checks for bit-timeout to emit complete codes.
pub struct WiegandReader {
    _pin0: InputPin,
    _pin1: InputPin,
}

impl WiegandReader {
    /// Create a new Wiegand reader.
    ///
    /// * `station_id` – identifier forwarded with each decoded event.
    /// * `gpio` – shared GPIO controller (avoids creating a new `Gpio` per reader).
    /// * `gpio_d0`, `gpio_d1` – BCM pin numbers for data lines.
    /// * `tx` – channel sender for decoded events.
    /// * `bit_timeout` – max time between bits before the code is considered complete.
    pub fn new(
        station_id: u32,
        gpio: &GpioController,
        gpio_d0: u8,
        gpio_d1: u8,
        tx: mpsc::UnboundedSender<WiegandEvent>,
        bit_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let state = Arc::new(Mutex::new(WiegandState::new()));

        // ── D0 pin (falling edge = logical 0) ───────────────────────
        let state0 = state.clone();
        let pin0 = gpio.setup_input_pullup_with_callback(gpio_d0, move || {
            state0.lock().unwrap().receive_bit(0);
        })?;

        // ── D1 pin (falling edge = logical 1) ───────────────────────
        let state1 = state.clone();
        let pin1 = gpio.setup_input_pullup_with_callback(gpio_d1, move || {
            state1.lock().unwrap().receive_bit(1);
        })?;

        // ── Timeout checker task ────────────────────────────────────
        let state_check = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1)).await;
                let maybe_event = state_check.lock().unwrap().try_complete(bit_timeout);
                if let Some((bits, code)) = maybe_event {
                    let event = WiegandEvent {
                        station_id,
                        bits,
                        code,
                    };
                    debug!("Wiegand decoded: station={station_id} bits={bits} code={code}");
                    if tx.send(event).is_err() {
                        warn!("Wiegand event channel closed for station {station_id}");
                        return;
                    }
                }
            }
        });

        Ok(Self {
            _pin0: pin0,
            _pin1: pin1,
        })
    }
}
