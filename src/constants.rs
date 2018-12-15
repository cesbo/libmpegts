pub const SIDE_WEST: u8 = 0;
pub const SIDE_EAST: u8 = 1;

pub const POLARIZATION_HORIZONTAL: u8 = 0;
pub const POLARIZATION_VERTICAL: u8   = 1;
pub const POLARIZATION_LEFT: u8       = 2;
pub const POLARIZATION_RIGHT: u8      = 3;

pub const ROF_A035: u8 = 0;
pub const ROF_A025: u8 = 1;
pub const ROF_A020: u8 = 2;

pub const MODULATION_AUTO: u8  = 0;
pub const MODULATION_QPSK: u8  = 1;
pub const MODULATION_8PSK: u8  = 2;
pub const MODULATION_16QAM: u8 = 3;

pub const NOT_DEFINED: u8 = 0b0000;
pub const FEC_1_2: u8     = 0b0001;
pub const FEC_2_3: u8     = 0b0010;
pub const FEC_3_4: u8     = 0b0011;
pub const FEC_5_6: u8     = 0b0100;
pub const FEC_7_8: u8     = 0b0101;
pub const FEC_8_9: u8     = 0b0110;
pub const FEC_3_5: u8     = 0b0111;
pub const FEC_4_5: u8     = 0b1000;
pub const FEC_9_10: u8    = 0b1001;
pub const FEC_NONE: u8    = 0b1111;
