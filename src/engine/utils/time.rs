use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_u128_timestamp() -> u128 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("something went wrong while getting the timestamp");
    now.as_secs() as u128 * 1_000_000_000 + now.subsec_nanos() as u128
}
