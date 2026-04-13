use std::sync::Arc;

use tracing::info;

use crate::modbus::{
    build_read_holding_registers_frame, decode_f32_be, decode_u32_be,
    parse_read_holding_registers_response, Rs485Modbus,
};

pub struct Tuf2000Client {
    modbus: Arc<tokio::sync::Mutex<Rs485Modbus>>,
}

impl Tuf2000Client {
    pub fn new(modbus: Arc<tokio::sync::Mutex<Rs485Modbus>>) -> Self {
        Self { modbus }
    }

    pub async fn read_total_accumulator(&self, slave_id: u8) -> anyhow::Result<f32> {
        info!(
            "MODBUS read_total_accumulator request: slave_id={}",
            slave_id
        );

        // TUF2000 mapping:
        // 0025-0026: Net accumulator (LONG)
        // 0027-0028: Net decimal fraction (REAL4)
        let register_number = 25;

        let frame = build_read_holding_registers_frame(slave_id, register_number, 4)?;
        let mut mb = self.modbus.lock().await;
        let response = mb.query(&frame).await?;
        let flow = parse_read_holding_registers_response(&response)?;
        if flow.len() < 4 {
            Err(anyhow::anyhow!(
                "Expected at least 4 registers in MODBUS response, got {}",
                flow.len()
            ))?;
        }

        let integer_part = decode_u32_be(&flow[0..2])? as f32;
        let decimal_part = decode_f32_be(&flow[2..4])?;
        let total = integer_part + decimal_part;

        info!(
            "MODBUS total accumulator: slave_id={} regs=[{}, {}, {}, {}] integer_part={} decimal_part={} total={}",
            slave_id,
            flow[0],
            flow[1],
            flow[2],
            flow[3],
            integer_part,
            decimal_part,
            total
        );

        Ok(total)
    }
}
