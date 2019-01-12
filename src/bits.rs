#[macro_export]
macro_rules! set_bits {
    ($shift:expr, $val:expr) => {
        $val << $shift
    };

    ($shift:expr, $val:expr, $bits:expr) => {
        set_bits!($shift - $bits, $val)
    };

    ($shift:expr, $val:expr, $bits:expr, $($args:tt)*) => {
        set_bits!($shift - $bits, $val) | set_bits!($shift - $bits, $($args)*)
    };
}
