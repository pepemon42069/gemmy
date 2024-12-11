use std::time::SystemTime;

pub(crate) fn get_timestamp_now_micros() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(timestamp) => timestamp.as_micros(),
        Err(_) => panic!("failed to generate timestamp")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::utils::get_timestamp_now_micros;

    #[test]
    fn it_generates_timestamp() {
        get_timestamp_now_micros();
    }
}