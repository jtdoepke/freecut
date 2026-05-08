//! Dimension scaling boundary.
//!
//! All dimensional `u32` fields in `domain`, `optimizer`, and `render` are stored as
//! integer **milli-units**: `1` internal unit equals `1 / MILLI_PER_UNIT` of a user unit
//! (`CutSettings::unit`). User-facing edges (UI, CSV import, PDF export, project file
//! load) translate between this scaled integer and human decimal/fraction notation
//! through the helpers in this module.

pub const MILLI_PER_UNIT: u32 = 1000;

/// Maximum representable dimension in milli-units (= 100,000 user units).
pub const MAX_DIMENSION_MILLI: u32 = 100_000 * MILLI_PER_UNIT;

/// Convert milli-units to a user-facing decimal (`12500` → `12.5`).
#[must_use]
pub fn decimal_from_milli(milli: u32) -> f64 {
    f64::from(milli) / f64::from(MILLI_PER_UNIT)
}

/// Convert a user-facing decimal to milli-units (rounded, saturating).
///
/// Returns `None` for `NaN`, infinities, or negative values.
#[must_use]
pub fn milli_from_decimal(value: f64) -> Option<u32> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let scaled = value * f64::from(MILLI_PER_UNIT);
    let rounded = scaled.round();
    if rounded > f64::from(MAX_DIMENSION_MILLI) {
        Some(MAX_DIMENSION_MILLI)
    } else {
        // rounded is finite, non-negative, <= MAX_DIMENSION_MILLI which fits u32
        Some(rounded as u32)
    }
}

/// Format milli-units as a user-facing decimal string with up to 3 fractional digits,
/// trailing zeros trimmed (`12500` → `"12.5"`, `12000` → `"12"`, `125` → `"0.125"`).
#[must_use]
pub fn format_dimension(milli: u32) -> String {
    let whole = milli / MILLI_PER_UNIT;
    let frac = milli % MILLI_PER_UNIT;
    if frac == 0 {
        return whole.to_string();
    }
    let mut frac_str = format!("{frac:03}");
    while frac_str.ends_with('0') {
        frac_str.pop();
    }
    format!("{whole}.{frac_str}")
}

/// Parse a positive user-entered dimension into milli-units.
///
/// Accepts:
/// - decimals with `.` or `,`: `"100"`, `"100.5"`, `"0.125"`, `"100,5"`
/// - proper fractions: `"1/2"`, `"3/8"`
/// - mixed numerals with space or dash: `"3 1/4"`, `"3-1/4"`
///
/// Rejects: empty input, non-numeric tokens, negative values, zero, denominator zero,
/// values above `MAX_DIMENSION_MILLI`.
///
/// Result is rounded to the nearest milli-unit; sub-1/1000 fractions (e.g. 1/64 ≈ 0.0156)
/// round to 3 decimal places.
#[allow(clippy::missing_errors_doc)]
pub fn parse_dimension(text: &str) -> Result<u32, String> {
    let milli = parse_dimension_allowing_zero(text)?;
    if milli == 0 {
        return Err(positive_error_message());
    }
    Ok(milli)
}

/// Same as [`parse_dimension`] but allows zero. Useful for kerf or linear-kerf inputs
/// where zero is a meaningful value (no kerf / disabled linear kerf).
#[allow(clippy::missing_errors_doc)]
pub fn parse_dimension_allowing_zero(text: &str) -> Result<u32, String> {
    let value = parse_dimension_value(text)?;
    if value < 0.0 {
        return Err(format_error_message());
    }
    let milli = milli_from_decimal(value).ok_or_else(format_error_message)?;
    if milli >= MAX_DIMENSION_MILLI {
        return Err(format!(
            "Wert ist zu groß (Maximum {} pro Einheit)",
            MAX_DIMENSION_MILLI / MILLI_PER_UNIT
        ));
    }
    Ok(milli)
}

fn parse_dimension_value(text: &str) -> Result<f64, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format_error_message());
    }

    // Split into "<integer> <fraction>" if present (space or dash separator).
    if let Some((whole_part, frac_part)) = split_mixed(trimmed) {
        let whole = parse_integer(whole_part)?;
        let frac = parse_fraction(frac_part)?;
        if frac < 0.0 {
            return Err(format_error_message());
        }
        return Ok(whole + frac.copysign(whole.signum_or_one()));
    }

    if trimmed.contains('/') {
        return parse_fraction(trimmed);
    }

    parse_decimal(trimmed)
}

fn split_mixed(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if (b == b' ' || b == b'-') && idx > 0 {
            let (left, rest) = text.split_at(idx);
            // skip the separator character
            let right = &rest[1..];
            if !right.is_empty() && right.contains('/') {
                return Some((left, right));
            }
        }
    }
    None
}

fn parse_integer(text: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .map_err(|_| format_error_message())
}

fn parse_decimal(text: &str) -> Result<f64, String> {
    let normalized = text.trim().replace(',', ".");
    normalized
        .parse::<f64>()
        .map_err(|_| format_error_message())
}

fn parse_fraction(text: &str) -> Result<f64, String> {
    let (num_str, den_str) = text.split_once('/').ok_or_else(format_error_message)?;
    let num: f64 = parse_decimal(num_str)?;
    let den: f64 = parse_decimal(den_str)?;
    if den == 0.0 {
        return Err("Bruch mit Nenner 0 ist ungültig".to_string());
    }
    Ok(num / den)
}

fn format_error_message() -> String {
    "muss eine positive Zahl sein (Dezimal- oder Bruchwert, z. B. 12,5 oder 1/2)".to_string()
}

fn positive_error_message() -> String {
    "muss größer als 0 sein".to_string()
}

trait F64Ext {
    fn signum_or_one(self) -> f64;
}

impl F64Ext for f64 {
    fn signum_or_one(self) -> f64 {
        if self < 0.0 {
            -1.0
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_integers() {
        assert_eq!(parse_dimension("100"), Ok(100_000));
        assert_eq!(parse_dimension("  42 "), Ok(42_000));
    }

    #[test]
    fn parses_decimals_with_dot_and_comma() {
        assert_eq!(parse_dimension("12.5"), Ok(12_500));
        assert_eq!(parse_dimension("0.125"), Ok(125));
        assert_eq!(parse_dimension("100,5"), Ok(100_500));
    }

    #[test]
    fn parses_proper_fractions() {
        assert_eq!(parse_dimension("1/2"), Ok(500));
        assert_eq!(parse_dimension("3/8"), Ok(375));
        assert_eq!(parse_dimension("1/16"), Ok(63)); // 0.0625 rounds away from zero to 0.063
    }

    #[test]
    fn parses_mixed_numerals() {
        assert_eq!(parse_dimension("3 1/4"), Ok(3_250));
        assert_eq!(parse_dimension("3-1/4"), Ok(3_250));
        assert_eq!(parse_dimension("12 1/2"), Ok(12_500));
    }

    #[test]
    fn rejects_invalid_input() {
        assert!(parse_dimension("").is_err());
        assert!(parse_dimension("abc").is_err());
        assert!(parse_dimension("-1").is_err());
        assert!(parse_dimension("0").is_err());
        assert!(parse_dimension("1/0").is_err());
        assert!(parse_dimension("nan").is_err());
    }

    #[test]
    fn allowing_zero_accepts_zero_but_still_rejects_negative() {
        assert_eq!(parse_dimension_allowing_zero("0"), Ok(0));
        assert_eq!(parse_dimension_allowing_zero("0.0"), Ok(0));
        assert_eq!(parse_dimension_allowing_zero("12.5"), Ok(12_500));
        assert!(parse_dimension_allowing_zero("-1").is_err());
        assert!(parse_dimension_allowing_zero("abc").is_err());
    }

    #[test]
    fn rejects_values_at_or_above_max() {
        assert!(parse_dimension("100000").is_err());
        assert!(parse_dimension("99999").is_ok());
    }

    #[test]
    fn formats_milli_to_decimal_with_trimmed_zeros() {
        assert_eq!(format_dimension(12_500), "12.5");
        assert_eq!(format_dimension(12_000), "12");
        assert_eq!(format_dimension(125), "0.125");
        assert_eq!(format_dimension(0), "0");
        assert_eq!(format_dimension(1), "0.001");
    }

    #[test]
    fn round_trips_through_milli_decimal() {
        for v in [1_u32, 1_000, 12_500, 99_999_000] {
            let dec = decimal_from_milli(v);
            assert_eq!(milli_from_decimal(dec), Some(v));
        }
    }

    #[test]
    fn milli_from_decimal_rejects_nan_and_negative() {
        assert_eq!(milli_from_decimal(f64::NAN), None);
        assert_eq!(milli_from_decimal(f64::INFINITY), None);
        assert_eq!(milli_from_decimal(-1.0), None);
    }

    #[test]
    fn milli_from_decimal_saturates_above_max() {
        assert_eq!(milli_from_decimal(1.0e9), Some(MAX_DIMENSION_MILLI));
    }
}
