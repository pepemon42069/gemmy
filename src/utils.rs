use std::time::SystemTime;

pub(crate) fn price_to_bytes(value: u64) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

pub(crate) fn bytes_to_price(value: Vec<u8>) -> u64 {
    if value.len() != 8 {
        panic!("invalid vec_u8 length");
    }
    u64::from_be_bytes(value.try_into().unwrap())
}

pub(crate) fn get_timestamp_now_micros() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(timestamp) => timestamp.as_micros(),
        Err(_) => panic!("failed to generate timestamp")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::utils::{price_to_bytes, bytes_to_price, get_timestamp_now_micros};

    #[test]
    fn it_converts_price_to_bytes() {
        let pass: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 1, 0];
        let value: u64 = 256;
        let result = price_to_bytes(value);
        assert_eq!(pass, result);
    }

    #[test]
    fn it_converts_bytes_to_price() {
        let pass: u64 = 256;
        let value: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 1, 0];
        let result = bytes_to_price(value);
        assert_eq!(pass, result);
    }

    #[test]
    #[should_panic]
    fn it_fails_to_convert_bytes_to_price() {
        let value: Vec<u8> = vec![0, 0, 0, 0, 0, 1, 0];
        bytes_to_price(value);
    }

    #[test]
    fn it_generates_timestamp() {
        get_timestamp_now_micros();
    }
}