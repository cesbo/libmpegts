// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

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
