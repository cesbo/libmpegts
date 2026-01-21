#[macro_export]
macro_rules! pack_bits {
    ($type:ty, $($name:ident : $size:expr => $val:expr),* $(,)?) => {{
        const SIZE: usize = core::mem::size_of::<$type>();
        let mut _pos: u32 = SIZE as u32 * 8;
        let mut _result: $type = 0;
        $(
            let _ = stringify!($name);
            _pos -= $size;
            _result |= (($val as $type) & ((1 << $size) - 1)) << _pos;
        )*
        _result.to_be_bytes()
    }};
}

#[macro_export]
macro_rules! unpack_bits {
    ($type:ty, $data:expr, $($name:ident : $size:expr),* $(,)?) => {{
        const SIZE: usize = core::mem::size_of::<$type>();
        let mut _pos: u32 = SIZE as u32 * 8;
        let _val = <$type>::from_be_bytes($data);
        $(
            _pos -= $size;
            let $name = (_val >> _pos) & ((1 << $size) - 1);
        )*
        ($($name),*)
    }};
}
