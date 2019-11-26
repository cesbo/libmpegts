// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU


/// MPEG-TS Elementary Stream Types
#[derive(Debug, PartialEq)]
pub enum StreamType {
    /// Video stream:
    ///
    /// - ISO/IEC 11172 Video
    /// - ISO/IEC 13818-2 Video
    /// - ISO/IEC 14496-2 Visual
    /// - ISO/IEC 14496-10 Video | H.264
    /// - ISO/IEC 23008-2 Video | H.265
    VIDEO,
    /// Audio stream:
    ///
    /// - ISO/IEC 11172 Audio
    /// - ISO/IEC 13818-3 Audio
    /// - ISO/IEC 13818-7 Audio (ADTS)
    /// - ISO/IEC 14496-3 Audio (LATM)
    /// - Dolby Digital (AC-3)
    AUDIO,
    /// Application Information Table
    AIT,
    /// DVB Subtitles
    SUB,
    /// Teletext
    TTX,
    /// Private data
    DATA,
}


impl Default for StreamType {
    #[inline]
    fn default() -> Self { StreamType::DATA }
}
