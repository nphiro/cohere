use base32::Alphabet::Rfc4648;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn validate_totp(secret: &str, otp: &str, time_step: u64) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let counters = [now / time_step, (now / time_step) - 1];

    for counter in counters.iter() {
        match generate_totp(secret, *counter) {
            Ok(generated_otp) => {
                if generated_otp == otp {
                    return Ok(());
                }
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(String::from("Invalid OTP"))
}

fn generate_totp(secret: &str, counter: u64) -> Result<String, String> {
    let key = base32::decode(Rfc4648 { padding: false }, &secret.to_uppercase())
        .ok_or("Invalid Base32 Error")?;

    let mut msg = [0u8; 8];
    msg.copy_from_slice(&counter.to_be_bytes());

    let mut mac = Hmac::<Sha1>::new_from_slice(&key).unwrap();
    mac.update(&msg);
    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0x0f) as usize;
    let binary_code = ((u32::from(result[offset]) & 0x7f) << 24)
        | ((u32::from(result[offset + 1]) & 0xff) << 16)
        | ((u32::from(result[offset + 2]) & 0xff) << 8)
        | (u32::from(result[offset + 3]) & 0xff);

    Ok(format!("{:06}", binary_code % 1_000_000))
}
