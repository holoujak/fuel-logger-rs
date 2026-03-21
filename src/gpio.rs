use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

// ═══════════════════════════════════════════════════════════════════
// Real GPIO implementation (Raspberry Pi only)
// ═══════════════════════════════════════════════════════════════════

#[cfg(feature = "gpio")]
mod real {
    use super::*;
    use rppal::gpio::{Gpio, InputPin, OutputPin, Trigger};

    pub struct GpioController {
        gpio: Gpio,
    }

    impl GpioController {
        pub fn new() -> Result<Self> {
            let gpio = Gpio::new()?;
            info!("GPIO initialized (rppal)");
            Ok(Self { gpio })
        }

        pub fn setup_output_high(&self, pin: u8) -> Result<OutputPin> {
            let mut out = self.gpio.get(pin)?.into_output();
            out.set_high();
            debug!("GPIO {pin} configured as OUTPUT HIGH");
            Ok(out)
        }

        pub fn setup_input_pullup(&self, pin: u8) -> Result<InputPin> {
            let input = self.gpio.get(pin)?.into_input_pullup();
            debug!("GPIO {pin} configured as INPUT PULLUP");
            Ok(input)
        }

        pub fn setup_input_pullup_with_counter(
            &self,
            pin: u8,
            counter: Arc<Mutex<u64>>,
        ) -> Result<InputPin> {
            let mut input = self.gpio.get(pin)?.into_input_pullup();
            input.set_async_interrupt(Trigger::FallingEdge, move |_level| {
                let mut count = counter.lock().unwrap();
                *count += 1;
            })?;
            debug!("GPIO {pin} configured as INPUT PULLUP with falling-edge counter");
            Ok(input)
        }

        pub fn setup_input_pullup_with_callback<F>(&self, pin: u8, callback: F) -> Result<InputPin>
        where
            F: FnMut() + Send + 'static,
        {
            let mut input = self.gpio.get(pin)?.into_input_pullup();
            let mut cb = callback;
            input.set_async_interrupt(Trigger::FallingEdge, move |_level| {
                cb();
            })?;
            debug!("GPIO {pin} configured as INPUT PULLUP with falling-edge callback");
            Ok(input)
        }
    }

    // Re-export rppal types for station.rs
    pub use rppal::gpio::{InputPin, Level, OutputPin};
}

// ═══════════════════════════════════════════════════════════════════
// Mock GPIO implementation (development on non-RPi)
// ═══════════════════════════════════════════════════════════════════

#[cfg(not(feature = "gpio"))]
mod mock {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Level {
        Low,
        High,
    }

    /// Mock output pin – logs set_high/set_low but does nothing.
    pub struct OutputPin {
        pin: u8,
    }

    impl OutputPin {
        pub fn set_high(&mut self) {
            debug!("[mock] GPIO {} → HIGH", self.pin);
        }
        pub fn set_low(&mut self) {
            debug!("[mock] GPIO {} → LOW", self.pin);
        }
    }

    /// Mock input pin – always reads High (button not pressed).
    pub struct InputPin {
        pin: u8,
    }

    impl InputPin {
        pub fn read(&self) -> Level {
            Level::High
        }
    }

    pub struct GpioController;

    impl GpioController {
        pub fn new() -> Result<Self> {
            warn!("GPIO running in MOCK mode (no real hardware)");
            Ok(Self)
        }

        pub fn setup_output_high(&self, pin: u8) -> Result<OutputPin> {
            debug!("[mock] GPIO {pin} configured as OUTPUT HIGH");
            Ok(OutputPin { pin })
        }

        pub fn setup_input_pullup(&self, pin: u8) -> Result<InputPin> {
            debug!("[mock] GPIO {pin} configured as INPUT PULLUP");
            Ok(InputPin { pin })
        }

        pub fn setup_input_pullup_with_counter(
            &self,
            pin: u8,
            _counter: Arc<Mutex<u64>>,
        ) -> Result<InputPin> {
            debug!("[mock] GPIO {pin} configured as INPUT PULLUP (counter ignored)");
            Ok(InputPin { pin })
        }

        pub fn setup_input_pullup_with_callback<F>(&self, pin: u8, _callback: F) -> Result<InputPin>
        where
            F: FnMut() + Send + 'static,
        {
            debug!("[mock] GPIO {pin} configured as INPUT PULLUP (callback ignored)");
            Ok(InputPin { pin })
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Public re-exports – the rest of the app uses these types
// ═══════════════════════════════════════════════════════════════════

#[cfg(feature = "gpio")]
pub use real::{GpioController, InputPin, Level, OutputPin};

#[cfg(not(feature = "gpio"))]
pub use mock::{GpioController, InputPin};
