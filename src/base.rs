/// Gets 12 bits unsigned integer from byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_u12(&[0x12, 0x34]), 0x0234);
/// ```
#[inline]
pub fn get_u12(ptr: &[u8]) -> u16 {
    get_u16(ptr) & 0x0FFF
}

/// Gets 16 bits unsigned integer from byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_u16(&[0x12, 0x34]), 0x1234);
/// ```
#[inline]
pub fn get_u16(ptr: &[u8]) -> u16 {
    (u16::from(ptr[0]) << 8) | u16::from(ptr[1])
}

/// Gets 22 bits unsigned integer from byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_u22(&[0x82, 0x34, 0xAB]), 0x234AB);
/// ```
#[inline]
pub fn get_u22(ptr: &[u8]) -> u32 {
    get_u24(ptr) & 0x003f_ffff
}

/// Gets 24 bits unsigned integer from byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_u24(&[0x12, 0x34, 0xAB]), 0x1234AB);
/// ```
#[inline]
pub fn get_u24(ptr: &[u8]) -> u32 {
    (u32::from(ptr[0]) << 16) | (u32::from(ptr[1]) << 8) | u32::from(ptr[2])
}

/// Gets 32 bits unsigned integer from byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_u32(&[0x12, 0x34, 0xAB, 0xCD]), 0x1234ABCD);
/// ```
#[inline]
pub fn get_u32(ptr: &[u8]) -> u32 {
    (u32::from(ptr[0]) << 24) | (u32::from(ptr[1]) << 16) | (u32::from(ptr[2]) << 8) | u32::from(ptr[3])
}

/// Sets 12 bits unsigned integer to byte array. Preserves first 4 bits in the first byte
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0xA0, 0x00];
/// set_u12(&mut x, 0x1234);
/// assert_eq!(x, [0xA2, 0x34]);
/// ```
#[inline]
pub fn set_u12(ptr: &mut [u8], value: u16) {
    let value = value & 0x0FFF;
    ptr[0] = (ptr[0] & 0xF0) | ((value >> 8) as u8);
    ptr[1] = (value) as u8;
}

/// Sets 16 bits unsigned integer to byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0x00, 0x00];
/// set_u16(&mut x, 0x1234);
/// assert_eq!(x, [0x12, 0x34]);
/// ```
#[inline]
pub fn set_u16(ptr: &mut [u8], value: u16) {
    ptr[0] = (value >> 8) as u8;
    ptr[1] = (value) as u8;
}

/// Sets 22 bits unsigned integer to byte array. Preserves first 2 bits in the first byte
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0x80, 0x00, 0x00];
/// set_u22(&mut x, 0x234AB);
/// assert_eq!(x, [0x82, 0x34, 0xAB]);
/// ```
#[inline]
pub fn set_u22(ptr: &mut [u8], value: u32) {
    let value = value & 0x003f_ffff;
    ptr[0] = (ptr[0] & 0xC0) | ((value >> 16) as u8);
    ptr[1] = (value >> 8) as u8;
    ptr[2] = (value) as u8;
}

/// Sets 24 bits unsigned integer to byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0x00, 0x00, 0x00];
/// set_u24(&mut x, 0x1234AB);
/// assert_eq!(x, [0x12, 0x34, 0xAB]);
/// ```
#[inline]
pub fn set_u24(ptr: &mut [u8], value: u32) {
    ptr[0] = (value >> 16) as u8;
    ptr[1] = (value >> 8) as u8;
    ptr[2] = (value) as u8;
}

/// Sets 32 bits unsigned integer to byte array
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0x00, 0x00, 0x00, 0x00];
/// set_u32(&mut x, 0x1234ABCD);
/// assert_eq!(x, [0x12, 0x34, 0xAB, 0xCD]);
/// ```
#[inline]
pub fn set_u32(ptr: &mut [u8], value: u32) {
    ptr[0] = (value >> 24) as u8;
    ptr[1] = (value >> 16) as u8;
    ptr[2] = (value >> 8) as u8;
    ptr[3] = (value) as u8;
}

/// Gets PID (13 bits unsigned integer) from byte array.
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_pid(&[0x32, 0x34]), 0x1234);
/// ```
#[inline]
pub fn get_pid(ptr: &[u8]) -> u16 {
    get_u16(ptr) & 0x1FFF
}

/// Sets PID (13 bits unsigned integer) to byte array.
/// Sets first 3 reserved bits (0xE000) in the first byte.
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0x00, 0x00];
/// set_pid(&mut x, 0x1234);
/// assert_eq!(x, [0xF2, 0x34]);
/// ```
#[inline]
pub fn set_pid(ptr: &mut [u8], value: u16) {
    set_u16(ptr, 0xE000 | value);
}

/// Gets unix timestamp from byte array (Modified Julian Date)
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_mjd_date(&[0xc0, 0x79]), 750470400);
/// ```
#[inline]
pub fn get_mjd_date(ptr: &[u8]) -> i64 {
    (i64::from(get_u16(ptr)) - 40587) * 86400
}

/// Sets unix timestamp to bute array (Modified Julian Date)
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0x00, 0x00];
/// set_mjd_date(&mut x, 750470400);
/// assert_eq!(x, [0xc0, 0x79]);
/// ```
#[inline]
pub fn set_mjd_date(ptr: &mut [u8], value: i64) {
    set_u16(ptr, (value / 86400 + 40587) as u16);
}

#[inline]
fn bcd_to_u32(ptr: u8) -> i32 {
    ((i32::from(ptr) & 0xF0) >> 4) * 10 + (i32::from(ptr) & 0x0F)
}

/// Gets unix timestamp from byte array (Binary Coded Decimal)
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// assert_eq!(get_bcd_time(&[0x01, 0x45, 0x30]), 1 * 3600 + 45 * 60 + 30);
/// ```
#[inline]
pub fn get_bcd_time(ptr: &[u8]) -> i32 {
    bcd_to_u32(ptr[0]) * 3600 + bcd_to_u32(ptr[1]) * 60 + bcd_to_u32(ptr[2])
}

#[inline]
fn u32_to_bcd(value: i32) -> u8 {
    (((value / 10) << 4) | (value % 10)) as u8
}

/// Sets unix timestamp to byte array (Binary Coded Decimal)
///
/// # Examples
///
/// ```
/// use mpegts::base::*;
/// let mut x: Vec<u8> = vec![0x00, 0x00, 0x00];
/// set_bcd_time(&mut x, 1 * 3600 + 45 * 60 + 30);
/// assert_eq!(x, [0x01, 0x45, 0x30]);
/// ```
#[inline]
pub fn set_bcd_time(ptr: &mut [u8], value: i32) {
    ptr[0] = u32_to_bcd(value / 3600 % 24);
    ptr[1] = u32_to_bcd(value / 60 % 60);
    ptr[2] = u32_to_bcd(value % 60);
}
