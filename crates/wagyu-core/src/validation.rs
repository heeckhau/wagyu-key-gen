//! Input parsing and validation, ported from `ethstaker_deposit/utils/validation.py` and from
//! the checks the Electron app did with `web3-utils`.

use std::str::FromStr;

use alloy_primitives::Address;
use bls::Hash256;

use crate::chain::{ChainSetting, GWEI_PER_ETH, MAX_DEPOSIT_AMOUNT_GWEI};
use crate::error::{Error, Result};

pub const BLS_WITHDRAWAL_PREFIX: u8 = 0x00;
pub const EXECUTION_ADDRESS_WITHDRAWAL_PREFIX: u8 = 0x01;
pub const COMPOUNDING_WITHDRAWAL_PREFIX: u8 = 0x02;
pub const MIN_PASSWORD_LENGTH: usize = 12;

/// Parses an Ethereum address with the same rules as `web3-utils`' `isAddress`, which the UI
/// used: an optional `0x` prefix, 40 hex digits, and if the digits are mixed case they must form a
/// valid EIP-55 checksum.
pub fn parse_address(input: &str) -> Result<Address> {
    let trimmed = input.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if digits.len() != 40 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::InvalidAddress);
    }
    let has_lower = digits.bytes().any(|b| b.is_ascii_lowercase());
    let has_upper = digits.bytes().any(|b| b.is_ascii_uppercase());
    if has_lower && has_upper {
        Address::parse_checksummed(format!("0x{digits}"), None).map_err(|_| Error::InvalidAddress)
    } else {
        Address::from_str(digits).map_err(|_| Error::InvalidAddress)
    }
}

pub fn is_address(input: &str) -> bool {
    parse_address(input).is_ok()
}

/// Parses an optional withdrawal address: `None` and the empty string mean "no address".
pub fn parse_optional_address(input: Option<&str>) -> Result<Option<Address>> {
    match input.map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => parse_address(s).map(Some),
    }
}

/// Converts a deposit amount typed by the user (a decimal number in the chain's own unit, ETH or
/// GNO) into the gwei value that goes into the deposit message.
///
/// Rules (from `validate_deposit_amount`): exact decimal arithmetic, at most 9 decimals (1 gwei),
/// `min_deposit_amount <= amount <= 2048 / multiplier`. The result is multiplied by the chain
/// multiplier, so on Gnosis `1` GNO becomes `32_000_000_000`.
pub fn deposit_amount_to_gwei(amount: &str, chain: &ChainSetting) -> Result<u64> {
    let amount_gwei = parse_decimal_gwei(amount)?;
    if amount_gwei < chain.min_deposit_amount_gwei {
        return Err(Error::InvalidAmount(format!(
            "Amount must be at least {}.",
            format_gwei(chain.min_deposit_amount_gwei)
        )));
    }
    let max_gwei = MAX_DEPOSIT_AMOUNT_GWEI / chain.multiplier;
    if amount_gwei > max_gwei {
        return Err(Error::InvalidAmount(format!(
            "Amount must be at most {}.",
            format_gwei(max_gwei)
        )));
    }
    Ok(amount_gwei * chain.multiplier)
}

/// Parses `"32"`, `"1.5"`, `"0.000000001"` into gwei without floating point. Rejects more than
/// nine decimals, signs, exponents and anything else that is not `digits[.digits]`.
fn parse_decimal_gwei(amount: &str) -> Result<u64> {
    let invalid = || Error::InvalidAmount("Amount must be a positive decimal number.".to_string());
    let (int_part, frac_part) = match amount.split_once('.') {
        Some((i, f)) => (i, f),
        None => (amount, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(invalid());
    }
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }
    if frac_part.len() > 9 {
        return Err(Error::InvalidAmount(
            "Amount cannot have greater precision than 1 gwei (9 decimals).".to_string(),
        ));
    }
    let whole: u64 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().map_err(|_| invalid())?
    };
    let mut frac = frac_part.to_string();
    while frac.len() < 9 {
        frac.push('0');
    }
    let frac: u64 = if frac.is_empty() {
        0
    } else {
        frac.parse().map_err(|_| invalid())?
    };
    whole
        .checked_mul(GWEI_PER_ETH)
        .and_then(|w| w.checked_add(frac))
        .ok_or_else(invalid)
}

fn format_gwei(gwei: u64) -> String {
    let whole = gwei / GWEI_PER_ETH;
    let frac = gwei % GWEI_PER_ETH;
    if frac == 0 {
        whole.to_string()
    } else {
        let s = format!("{whole}.{frac:09}");
        s.trim_end_matches('0').to_string()
    }
}

/// Keystore passwords must be at least 12 characters (`validate_password_strength`).
pub fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err(Error::PasswordTooShort);
    }
    Ok(())
}

/// Splits a user-typed list such as `1,2,3`, `[1, 2, 3]` or `1 2 3` into its items
/// (`normalize_input_list`).
pub fn normalize_input_list(input: &str) -> Vec<String> {
    input
        .trim_matches(|c| "[({})]".contains(c))
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parses a non-negative integer in `low <= n < high` (`validate_int_range`).
pub fn validate_int_range(input: &str, low: u64, high: u64) -> Result<u64> {
    let err = || {
        Error::InvalidInput(format!(
            "'{input}' is not a valid non-negative integer in range."
        ))
    };
    let n: u64 = input.trim().parse().map_err(|_| err())?;
    if n < low || n >= high {
        return Err(err());
    }
    Ok(n)
}

/// Parses beacon-chain validator indices, each in `0 <= i < 2^32`.
pub fn parse_validator_indices(input: &str) -> Result<Vec<u64>> {
    let items = normalize_input_list(input);
    if items.is_empty() {
        return Err(Error::InvalidInput(
            "Please input at least one validator index.".to_string(),
        ));
    }
    items
        .iter()
        .map(|s| validate_int_range(s, 0, 1 << 32))
        .collect()
}

/// Parses one `0x00...` BLS withdrawal credential (`validate_bls_withdrawal_credentials`).
pub fn parse_bls_withdrawal_credentials(input: &str) -> Result<Hash256> {
    let trimmed = input.trim();
    let digits = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let bytes = hex::decode(digits).map_err(|_| {
        Error::InvalidInput("BLS withdrawal credentials must be hex encoded.".to_string())
    })?;
    if bytes.len() != 32 {
        return Err(Error::InvalidInput(
            "BLS withdrawal credentials must be 32 bytes.".to_string(),
        ));
    }
    if bytes[0] == EXECUTION_ADDRESS_WITHDRAWAL_PREFIX && bytes[1..12].iter().all(|b| *b == 0) {
        return Err(Error::InvalidInput(
            "These withdrawal credentials already point to an execution address (0x01 form)."
                .to_string(),
        ));
    }
    if bytes[0] != BLS_WITHDRAWAL_PREFIX {
        return Err(Error::InvalidInput(
            "BLS withdrawal credentials must start with 0x00.".to_string(),
        ));
    }
    Ok(Hash256::from_slice(&bytes))
}

/// Parses a comma separated list of BLS withdrawal credentials.
pub fn parse_bls_withdrawal_credentials_list(input: &str) -> Result<Vec<Hash256>> {
    let items = normalize_input_list(input);
    if items.is_empty() {
        return Err(Error::InvalidInput(
            "Please input at least one BLS withdrawal credential.".to_string(),
        ));
    }
    items
        .iter()
        .map(|s| parse_bls_withdrawal_credentials(s))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{GNOSIS, MAINNET};

    #[test]
    fn addresses_like_web3_is_address() {
        let checksummed = "0x00000000219ab540356cBB839Cbe05303d7705Fa";
        assert!(is_address(checksummed));
        assert!(is_address(&checksummed.to_lowercase()));
        assert!(is_address(&checksummed[2..].to_lowercase()));
        assert!(is_address(&format!(
            "0x{}",
            checksummed[2..].to_uppercase()
        )));
        // Bad checksum (one letter case flipped).
        assert!(!is_address("0x00000000219ab540356cBB839Cbe05303d7705FA"));
        assert!(!is_address("0x00000000219ab540356cBB839Cbe05303d7705F"));
        assert!(!is_address("0xZZ000000219ab540356cbb839cbe05303d7705fa"));
        assert!(!is_address(""));
        assert_eq!(parse_optional_address(Some("")).unwrap(), None);
        assert_eq!(parse_optional_address(None).unwrap(), None);
        assert!(parse_optional_address(Some("nope")).is_err());
    }

    #[test]
    fn amount_conversion_is_exact() {
        assert_eq!(
            deposit_amount_to_gwei("32", &MAINNET).unwrap(),
            32_000_000_000
        );
        assert_eq!(
            deposit_amount_to_gwei("1.000000001", &MAINNET).unwrap(),
            1_000_000_001
        );
        assert_eq!(
            deposit_amount_to_gwei("1", &GNOSIS).unwrap(),
            32_000_000_000
        );
        assert_eq!(
            deposit_amount_to_gwei("2.5", &GNOSIS).unwrap(),
            80_000_000_000
        );
        assert_eq!(
            deposit_amount_to_gwei("0.03125", &GNOSIS).unwrap(),
            1_000_000_000
        );
        assert_eq!(
            deposit_amount_to_gwei("2048", &MAINNET).unwrap(),
            MAX_DEPOSIT_AMOUNT_GWEI
        );
        assert_eq!(
            deposit_amount_to_gwei("64", &GNOSIS).unwrap(),
            MAX_DEPOSIT_AMOUNT_GWEI
        );
        for bad in [
            "1.0000000001",
            "0.99999",
            "0",
            "-1",
            "a",
            " ",
            "",
            "2048.000000001",
            "1e3",
        ] {
            assert!(deposit_amount_to_gwei(bad, &MAINNET).is_err(), "{bad:?}");
        }
        assert!(deposit_amount_to_gwei("2048", &GNOSIS).is_err());
        assert!(deposit_amount_to_gwei("0.03124", &GNOSIS).is_err());
    }

    #[test]
    fn list_parsing() {
        for (input, expected) in [
            ("1", vec!["1"]),
            ("1,2,3", vec!["1", "2", "3"]),
            ("[1,2,3]", vec!["1", "2", "3"]),
            ("(1,2,3)", vec!["1", "2", "3"]),
            ("{1,2,3}", vec!["1", "2", "3"]),
            ("1 2 3", vec!["1", "2", "3"]),
            ("1  2  3", vec!["1", "2", "3"]),
            ("1; 2, 3", vec!["1", "2", "3"]),
        ] {
            assert_eq!(normalize_input_list(input), expected);
        }
        assert_eq!(parse_validator_indices("1, 2").unwrap(), vec![1, 2]);
        assert!(parse_validator_indices("").is_err());
        assert!(parse_validator_indices("1,a").is_err());
        assert!(parse_validator_indices("4294967296").is_err());
    }

    #[test]
    fn int_range() {
        assert_eq!(validate_int_range("2", 0, 4).unwrap(), 2);
        assert_eq!(validate_int_range("0", 0, 4).unwrap(), 0);
        for bad in ["-1", "4", "0.2", "a"] {
            assert!(validate_int_range(bad, 0, 4).is_err(), "{bad}");
        }
    }

    #[test]
    fn bls_credentials() {
        let cred = "0x00bd0b5a34de5fb17df08410b5e615dda87caf4fb72d0aac91ce5e52fc6aa8de";
        assert!(parse_bls_withdrawal_credentials(cred).is_ok());
        assert!(parse_bls_withdrawal_credentials(&cred[2..]).is_ok());
        assert!(parse_bls_withdrawal_credentials(
            "0x01000000000000000000000000000000219ab540356cbb839cbe05303d7705fa"
        )
        .is_err());
        assert!(parse_bls_withdrawal_credentials(
            "0x02bd0b5a34de5fb17df08410b5e615dda87caf4fb72d0aac91ce5e52fc6aa8de"
        )
        .is_err());
        assert!(parse_bls_withdrawal_credentials("0x00bd").is_err());
        assert!(parse_bls_withdrawal_credentials("zz").is_err());
        assert_eq!(
            parse_bls_withdrawal_credentials_list(&format!("{cred}, {cred}"))
                .unwrap()
                .len(),
            2
        );
        assert!(parse_bls_withdrawal_credentials_list("").is_err());
    }

    #[test]
    fn password_length() {
        assert!(validate_password("MyPasswordIs").is_ok());
        assert!(validate_password("MyPassword").is_err());
    }
}
