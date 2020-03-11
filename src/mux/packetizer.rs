use mpegts::es::{
    PES,
    PESError,
    EsStream
};


#[derive(Debug, Error)]
pub enum PacketizerError {
    #[error_from]
    PES(PESError),
}


pub type Result<T> = std::result::Result<T, PacketizerError>;


struct Packetizer {
    stream: EsStream,
}


impl Packetizer {
    fn new(stream: EsStream) -> Self {
        Self {
            stream
        }
    }

    fn make_pes(&mut self) -> Result<PES> {
        PES::new(&mut self.stream)?;
    }
}
