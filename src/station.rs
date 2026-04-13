use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use chrono_tz::Europe::Prague;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::config::{Config, StationConfig};
use crate::gpio::{GpioController, InputPin, Level, OutputPin};
use crate::models::StationInfo;
use crate::snapshot;
use crate::tuf2000::Tuf2000Client;
use crate::wiegand::{WiegandEvent, WiegandReader};

// ─── Station status (mirrors the Python enum) ───────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationStatus {
    Idle,
    Waiting,
    On,
    Pause,
}

impl std::fmt::Display for StationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Waiting => write!(f, "waiting"),
            Self::On => write!(f, "on"),
            Self::Pause => write!(f, "pause"),
        }
    }
}

// ─── Per-station runtime state ──────────────────────────────────────────────

struct StationRuntime {
    cfg: StationConfig,
    status: StationStatus,
    relay_pin: OutputPin,
    led_pin: OutputPin,
    buzzer_pin: Option<OutputPin>,
    start_pin: InputPin,
    stop_pin: InputPin,
    pause_pin: Option<InputPin>,
    flow_meter_start: f32,
    /// Accumulated keypad digits (cleared on timeout / enter).
    keypad_code: String,
    keypad_last_key: Instant,
    /// When the station was activated (relay turned on).
    relay_start: Option<chrono::DateTime<chrono_tz::Tz>>,
    last_resume: Option<chrono::DateTime<chrono_tz::Tz>>,
    /// Accumulated length in seconds from previous ON segments (before pause).
    accumulated_length: i64,
    /// DB user id of current session.
    active_user_id: Option<i32>,
    active_user_name: Option<String>,
    /// Timer for the WAITING→IDLE timeout.
    waiting_since: Option<Instant>,
    /// Timer for safety auto-off.
    relay_on_since: Option<Instant>,
    /// Per-station buzzer blink tick counter (avoids shared static).
    buzzer_tick: AtomicU32,
    /// Background snapshot capture handle.
    snapshot_handle: Option<tokio::task::JoinHandle<Option<String>>>,
}

// ─── Constants ──────────────────────────────────────────────────────────────

const WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const SAFE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const KEYPAD_TIMEOUT: Duration = Duration::from_secs(5);

// ─── Station Manager ────────────────────────────────────────────────────────

pub struct StationManager {
    stations: Mutex<HashMap<u32, StationRuntime>>,
    pool: SqlitePool,
    tuf2000: Option<Tuf2000Client>,
    config: Config,
    gpio: GpioController,
    wiegand_tx: mpsc::UnboundedSender<WiegandEvent>,
    wiegand_rx: Mutex<Option<mpsc::UnboundedReceiver<WiegandEvent>>>,
    shutdown: Mutex<bool>,
}

impl std::fmt::Debug for StationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StationManager").finish_non_exhaustive()
    }
}

impl StationManager {
    pub fn new(
        config: Config,
        pool: SqlitePool,
        gpio: GpioController,
        tuf2000: Option<Tuf2000Client>,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<WiegandEvent>();
        let mut stations_map = HashMap::new();

        for sc in &config.stations {
            let relay_pin = gpio.setup_output_high(sc.relay_gpio)?;
            let led_pin = gpio.setup_output_high(sc.keyboard_led_gpio)?;
            let buzzer_pin = sc
                .buzzer_gpio
                .map(|p| gpio.setup_output_high(p))
                .transpose()?;
            let start_pin = gpio.setup_input_pullup(sc.start_gpio)?;
            let stop_pin = gpio.setup_input_pullup(sc.stop_gpio)?;
            let pause_pin = sc
                .pause_gpio
                .map(|p| gpio.setup_input_pullup(p))
                .transpose()?;

            info!("Station {} ({}) initialized", sc.id, sc.name);

            stations_map.insert(
                sc.id,
                StationRuntime {
                    cfg: sc.clone(),
                    status: StationStatus::Idle,
                    relay_pin,
                    led_pin,
                    buzzer_pin,
                    start_pin,
                    stop_pin,
                    pause_pin,
                    flow_meter_start: 0.0,
                    keypad_code: String::new(),
                    keypad_last_key: Instant::now(),
                    relay_start: None,
                    last_resume: None,
                    accumulated_length: 0,
                    active_user_id: None,
                    active_user_name: None,
                    waiting_since: None,
                    relay_on_since: None,
                    buzzer_tick: AtomicU32::new(0),
                    snapshot_handle: None,
                },
            );
        }

        Ok(Self {
            stations: Mutex::new(stations_map),
            pool,
            config,
            gpio,
            wiegand_tx: tx,
            wiegand_rx: Mutex::new(Some(rx)),
            tuf2000,
            shutdown: Mutex::new(false),
        })
    }

    // ── Public info for the web API ─────────────────────────────────

    pub fn get_stations_info(&self) -> Vec<StationInfo> {
        let stations = self.stations.lock().unwrap();
        let mut infos: Vec<StationInfo> = stations
            .values()
            .map(|s| {
                let current_length = match (s.status, &s.last_resume) {
                    (StationStatus::On, Some(resume)) => {
                        let now = Utc::now().with_timezone(&Prague);
                        Some(s.accumulated_length + (now - *resume).num_seconds())
                    }
                    (StationStatus::Pause, _) => Some(s.accumulated_length),
                    _ => None,
                };
                StationInfo {
                    id: s.cfg.id,
                    name: s.cfg.name.clone(),
                    status: s.status.to_string(),
                    current_length_secs: current_length,
                    flow_meter_start: 0,
                    active_user: s.active_user_name.clone(),
                }
            })
            .collect();
        infos.sort_by_key(|i| i.id);
        infos
    }

    // ── Shutdown ────────────────────────────────────────────────────

    pub async fn shutdown(&self) {
        *self.shutdown.lock().unwrap() = true;
        let mut stations = self.stations.lock().unwrap();
        for (_, s) in stations.iter_mut() {
            s.relay_pin.set_high();
            s.led_pin.set_high();
            if let Some(ref mut buz) = s.buzzer_pin {
                buz.set_high();
            }
        }
        info!("All GPIOs released (set HIGH)");
    }

    // ── Main hardware loop ──────────────────────────────────────────

    pub async fn run_hardware_loop(self: &Arc<Self>) -> Result<()> {
        // Start Wiegand readers
        let mut _readers = Vec::new();
        for sc in &self.config.stations {
            let reader = WiegandReader::new(
                sc.id,
                &self.gpio,
                sc.keyboard_d0_gpio,
                sc.keyboard_d1_gpio,
                self.wiegand_tx.clone(),
                Duration::from_millis(5),
            )?;
            _readers.push(reader);
            info!("Wiegand reader started for station {}", sc.id);
        }

        let mut rx = self
            .wiegand_rx
            .lock()
            .unwrap()
            .take()
            .expect("run_hardware_loop called twice");

        let poll_interval = Duration::from_millis(1);

        loop {
            if *self.shutdown.lock().unwrap() {
                break;
            }

            // Process Wiegand events (non-blocking drain)
            while let Ok(event) = rx.try_recv() {
                self.handle_wiegand_event(event).await;
            }

            // Poll buttons and timers
            self.poll_buttons().await;

            tokio::time::sleep(poll_interval).await;
        }

        Ok(())
    }

    // ── Wiegand event handling ──────────────────────────────────────

    async fn handle_wiegand_event(&self, event: WiegandEvent) {
        // Determine what action to take under the lock, then release it
        // before doing any async work (DB calls).
        let action = {
            let mut stations = self.stations.lock().unwrap();
            let Some(station) = stations.get_mut(&event.station_id) else {
                return;
            };

            match event.bits {
                // Single key press (4-bit Wiegand)
                4 => {
                    let code = event.code;
                    debug!("Key pressed on station {}: {code}", event.station_id);
                    match code {
                        10 => {
                            station.keypad_code.clear();
                            None
                        }
                        11 => {
                            let password = std::mem::take(&mut station.keypad_code);
                            Some(format!("{:0>10}", password))
                        }
                        digit @ 0..=9 => {
                            station.keypad_code.push_str(&digit.to_string());
                            station.keypad_last_key = Instant::now();
                            None
                        }
                        _ => None,
                    }
                }
                // Chip read (26-bit Wiegand)
                26 => {
                    let card_number = (event.code >> 1) & 0xFFFF;
                    Some(format!("{:0>10}", card_number))
                }
                _ => {
                    debug!(
                        "Unknown Wiegand bit count {} on station {}",
                        event.bits, event.station_id
                    );
                    None
                }
            }
            // MutexGuard dropped here
        };

        // Async DB call happens outside the lock
        if let Some(password) = action {
            self.validate_password(event.station_id, &password, &self.pool)
                .await;
        }
    }

    // ── Password validation (DB lookup) ─────────────────────────────

    async fn validate_password(&self, station_id: u32, password: &str, pool: &SqlitePool) {
        info!("Validating password on station {station_id}");

        // Check station is idle
        {
            let stations = self.stations.lock().unwrap();
            if let Some(s) = stations.get(&station_id) {
                if s.status != StationStatus::Idle {
                    debug!("Station {station_id} not idle ({})", s.status);
                    return;
                }
            }
        }

        // DB lookup
        let user = sqlx::query_as::<_, (i32, String, bool, bool)>(
            "SELECT id, name, station1, station2 FROM users WHERE tag = ?",
        )
        .bind(password)
        .fetch_optional(pool)
        .await;

        let Ok(Some((user_id, user_name, station1, station2))) = user else {
            debug!("User with tag '{password}' not found");
            return;
        };

        let authorized = match station_id {
            1 => station1,
            2 => station2,
            _ => false,
        };

        if !authorized {
            info!("User '{user_name}' not authorized on station {station_id}");
            return;
        }

        info!("User '{user_name}' authorized on station {station_id}");

        let mut stations = self.stations.lock().unwrap();
        if let Some(s) = stations.get_mut(&station_id) {
            s.status = StationStatus::Waiting;
            s.led_pin.set_low(); // LED on (active-low)
            s.waiting_since = Some(Instant::now());
            s.active_user_id = Some(user_id);
            s.active_user_name = Some(user_name);
            s.accumulated_length = 0;
        }
    }

    // ── Button polling (runs every 100ms) ───────────────────────────

    async fn poll_buttons(&self) {
        // Collect any log that needs saving while holding the lock,
        // then save it after releasing the lock (no MutexGuard across await).
        let (pending_log, pending_flow_starts) = {
            let mut stations = self.stations.lock().unwrap();

            let station_ids: Vec<u32> = stations.keys().copied().collect();
            let mut log_to_save: Option<(LogSaveInfo, f32)> = None;
            let mut flow_starts: Vec<(u32, u8)> = Vec::new();

            for sid in station_ids {
                let s = stations.get_mut(&sid).unwrap();

                // Keypad timeout
                if !s.keypad_code.is_empty() && s.keypad_last_key.elapsed() > KEYPAD_TIMEOUT {
                    s.keypad_code.clear();
                }

                let start_pressed = s.start_pin.read() == Level::Low;
                let stop_pressed = s.stop_pin.read() == Level::Low;
                let pause_pressed = s
                    .pause_pin
                    .as_ref()
                    .map(|p| p.read() == Level::Low)
                    .unwrap_or(false);

                match s.status {
                    StationStatus::Idle => {
                        // TODO: change back to waiting
                        // Timeout check
                        if let Some(since) = s.waiting_since {
                            if since.elapsed() > WAIT_TIMEOUT {
                                info!("Waiting timeout on station {sid}");
                                Self::reset_station(s);
                                continue;
                            }
                        }

                        if start_pressed {
                            info!("START pressed on station {sid} → relay ON");
                            s.active_user_id = Some(42);
                            s.active_user_name = Some("holoujak".to_string());
                            s.waiting_since = None;
                            s.relay_pin.set_low();
                            if let Some(ref mut buz) = s.buzzer_pin {
                                buz.set_low();
                            }

                            // Capture RTSP snapshot in background
                            if let Some(ref url) = s.cfg.camera_url {
                                let now = Utc::now().with_timezone(&Prague);
                                s.snapshot_handle = Some(snapshot::capture_snapshot_background(
                                    url.clone(),
                                    self.config.snapshot_dir.clone(),
                                    sid,
                                    now,
                                ));
                            }

                            let now = Utc::now().with_timezone(&Prague);
                            s.relay_start = Some(now);
                            s.last_resume = Some(now);
                            s.accumulated_length = 0;
                            s.relay_on_since = Some(Instant::now());
                            s.status = StationStatus::On;

                            if let Some(slave_id) = s.cfg.flow_meter_slave_id {
                                flow_starts.push((sid, slave_id));
                            }
                        }
                    }

                    StationStatus::On => {
                        // Safety timeout
                        if let Some(since) = s.relay_on_since {
                            if since.elapsed() > SAFE_TIMEOUT {
                                warn!("Safety timeout on station {sid}");
                                Self::accumulate_length(s);
                                log_to_save =
                                    Some((Self::extract_log_info(self, s), s.flow_meter_start));
                                Self::reset_station(s);
                                break; // release lock, save log
                            }
                        }

                        if stop_pressed {
                            info!("STOP pressed on station {sid} → relay OFF");
                            Self::accumulate_length(s);
                            log_to_save =
                                Some((Self::extract_log_info(self, s), s.flow_meter_start));
                            Self::reset_station(s);
                            break;
                        }

                        if pause_pressed {
                            info!("PAUSE pressed on station {sid}");
                            s.relay_pin.set_high();
                            Self::accumulate_length(s);
                            s.relay_on_since = None;
                            s.status = StationStatus::Pause;
                        }
                    }

                    StationStatus::Pause => {
                        // Blink buzzer using per-station tick counter
                        if let Some(ref mut buz) = s.buzzer_pin {
                            let t = s.buzzer_tick.fetch_add(1, Ordering::Relaxed);
                            if t % 10 < 3 {
                                buz.set_low();
                            } else {
                                buz.set_high();
                            }
                        }

                        if stop_pressed {
                            info!("STOP pressed on station {sid} during PAUSE → relay OFF");
                            log_to_save =
                                Some((Self::extract_log_info(self, s), s.flow_meter_start));
                            Self::reset_station(s);
                            break;
                        }

                        if start_pressed {
                            info!("RESUME pressed on station {sid}");
                            s.relay_pin.set_low();
                            if let Some(ref mut buz) = s.buzzer_pin {
                                buz.set_low();
                            }
                            let now = Utc::now().with_timezone(&Prague);
                            s.last_resume = Some(now);
                            s.relay_on_since = Some(Instant::now());
                            s.status = StationStatus::On;
                        }
                    }

                    _ => {}
                }
            }

            (log_to_save, flow_starts)
            // MutexGuard dropped here
        };

        // Async flow meter reads happen outside the lock.
        if let Some(tuf2000) = self.tuf2000.as_ref() {
            for (sid, slave_id) in pending_flow_starts {
                let total = match tuf2000.read_total_accumulator(slave_id).await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(
                            "Failed to read flow meter start value on station {} (slave {}): {}",
                            sid, slave_id, e
                        );
                        0.0
                    }
                };
                let mut stations = self.stations.lock().unwrap();
                if let Some(s) = stations.get_mut(&sid) {
                    s.flow_meter_start = total;
                }
            }
        }

        // Async DB save happens outside the lock.
        if let Some((mut log_info, flow_meter_start)) = pending_log {
            log_info.consumption = if let (Some(slave_id), Some(tuf2000)) = (
                self.config
                    .stations
                    .iter()
                    .find(|sc| sc.id as i32 == log_info.station)
                    .and_then(|sc| sc.flow_meter_slave_id),
                self.tuf2000.as_ref(),
            ) {
                let total = match tuf2000.read_total_accumulator(slave_id).await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(
                            "Failed to read flow meter stop value on station {} (slave {}): {}",
                            log_info.station, slave_id, e
                        );
                        flow_meter_start
                    }
                };
                (total - flow_meter_start).max(0.0)
            } else {
                0.0
            };

            self.save_log(log_info, &self.pool).await;
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────

    fn accumulate_length(s: &mut StationRuntime) {
        if let Some(resume) = s.last_resume {
            let now = Utc::now().with_timezone(&Prague);
            let delta = (now - resume).num_seconds();
            s.accumulated_length += delta;
        }
    }

    fn extract_log_info(&self, s: &mut StationRuntime) -> LogSaveInfo {
        LogSaveInfo {
            user_id: s.active_user_id.unwrap_or(0),
            station: s.cfg.id as i32,
            created_at: s.relay_start.map(|dt| dt.naive_local()),
            length: s.accumulated_length as i32,
            consumption: 0.0,
            snapshot_handle: s.snapshot_handle.take(),
        }
    }

    fn reset_station(s: &mut StationRuntime) {
        s.relay_pin.set_high();
        s.led_pin.set_high();
        if let Some(ref mut buz) = s.buzzer_pin {
            buz.set_high();
        }
        s.status = StationStatus::Idle;
        s.relay_start = None;
        s.last_resume = None;
        s.waiting_since = None;
        s.relay_on_since = None;
        s.active_user_id = None;
        s.active_user_name = None;
        s.accumulated_length = 0;
        s.flow_meter_start = 0.0;
        s.buzzer_tick.store(0, Ordering::Relaxed);
    }

    async fn save_log(&self, info: LogSaveInfo, pool: &SqlitePool) {
        info!(
            "Saving log: station={} length={}s consumption={:.2}l",
            info.station, info.length, info.consumption
        );

        // Await the background snapshot capture if one was started
        let snapshot_path: Option<String> = match info.snapshot_handle {
            Some(handle) => match handle.await {
                Ok(result) => result,
                Err(e) => {
                    warn!("Snapshot task panicked: {e}");
                    None
                }
            },
            None => None,
        };

        let created = info
            .created_at
            .unwrap_or_else(|| chrono::Local::now().naive_local());

        let result = sqlx::query(
            "INSERT INTO logs (user_id, created_at, station, length, consumption, snapshot_path) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(info.user_id)
        .bind(created)
        .bind(info.station)
        .bind(info.length)
        .bind(info.consumption)
        .bind(&snapshot_path)
        .execute(pool)
        .await;

        match result {
            Ok(_) => info!("Log saved successfully (snapshot: {:?})", snapshot_path),
            Err(e) => warn!("Failed to save log: {e}"),
        }
    }
}

struct LogSaveInfo {
    user_id: i32,
    station: i32,
    created_at: Option<chrono::NaiveDateTime>,
    length: i32,
    consumption: f32,
    snapshot_handle: Option<tokio::task::JoinHandle<Option<String>>>,
}
