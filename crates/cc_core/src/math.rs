use fixed::FixedI32;
use fixed::types::extra::U16;

/// 16.16 fixed-point type for deterministic simulation math.
/// Range: roughly -32768 to +32767 with 1/65536 precision.
pub type Fixed = FixedI32<U16>;

pub fn fixed_from_i32(v: i32) -> Fixed {
    Fixed::from_num(v)
}

pub fn fixed_from_f32(v: f32) -> Fixed {
    Fixed::from_num(v)
}

pub fn fixed_to_f32(v: Fixed) -> f32 {
    v.to_num::<f32>()
}

pub const FIXED_ZERO: Fixed = Fixed::ZERO;
pub const FIXED_ONE: Fixed = Fixed::ONE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_i32() {
        for v in [-100, -1, 0, 1, 42, 1000] {
            let f = fixed_from_i32(v);
            assert_eq!(f.to_num::<i32>(), v);
        }
    }

    #[test]
    fn basic_arithmetic() {
        let a = fixed_from_i32(10);
        let b = fixed_from_i32(3);
        assert_eq!((a + b).to_num::<i32>(), 13);
        assert_eq!((a - b).to_num::<i32>(), 7);
        assert_eq!((a * b).to_num::<i32>(), 30);
    }
}
