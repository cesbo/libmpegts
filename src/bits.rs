#[macro_export]
macro_rules! set_bits {
    ($shift:expr, $val:expr) => {
        $val << $shift
    };

    ($shift:expr, $val:expr, $size:expr) => {
        set_bits!($shift - $size, $val)
    };

    ($shift:expr, $val:expr, $size:expr, $($args:tt)*) => {
        set_bits!($shift - $size, $val) | set_bits!($shift - $size, $($args)*)
    };
}
