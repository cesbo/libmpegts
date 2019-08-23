// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU


use std::fmt;

use super::*;


/// Struct to debug adaptation field
pub struct TsAdaptation<'a>(&'a [u8]);


impl<'a> TsAdaptation<'a> {
    #[inline]
    pub fn new(packet: &'a [u8]) -> Self {
        debug_assert!(packet.len() >= PACKET_SIZE);
        TsAdaptation(packet)
    }
}


impl<'a> fmt::Debug for TsAdaptation<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if ! is_adaptation(self.0) {
            return fmt::Debug::fmt(&false, f)
        }

        let mut s = f.debug_struct("TsAdaptation");
        let len = get_adaptation_size(self.0);
        s.field("length", &len);
        if len == 0 {
            return s.finish()
        }

        let p = &(self.0)[5 ..];
        let pcr_flag = (p[0] & 0x10) != 0;

        s.field("discontinuity", &((p[0] & 0x80) != 0));
        s.field("random_access", &((p[0] & 0x40) != 0));
        s.field("es_priority", &((p[0] & 0x20) != 0));
        s.field("PCR_flag", &pcr_flag);
        s.field("OPCR_flag", &((p[0] & 0x08) != 0));
        s.field("splicing_point", &((p[0] & 0x04) != 0));
        s.field("private_data", &((p[0] & 0x02) != 0));
        s.field("af_extension", &((p[0] & 0x01) != 0));

        if pcr_flag {
            s.field("pcr", &get_pcr(self.0));
        }

        s.finish()
    }
}


/// Struct to debug TS packet header
pub struct TsPacket<'a>(&'a [u8]);


impl<'a> TsPacket<'a> {
    #[inline]
    pub fn new(packet: &'a [u8]) -> Self {
        debug_assert!(packet.len() >= PACKET_SIZE);
        TsPacket(packet)
    }
}


impl<'a> fmt::Debug for TsPacket<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TsPacket")
            .field("sync", &is_sync(self.0))
            .field("error", &(((self.0)[1] & 0x80) >> 7))
            .field("pusi", &is_pusi(self.0))
            .field("pid", &get_pid(self.0))
            .field("scrambling", &(((self.0)[3] & 0xC0) >> 6))
            .field("adaptation", &TsAdaptation::new(self.0))
            .field("payload", &is_payload(self.0))
            .field("cc", &get_cc(self.0))
            .finish()
    }
}
