# MPEG-TS Library

High-performance library for MPEG-TS processing in highload systems.

## Features

- Zero-copy parsing of TS packets and PSI tables
    - PAT
    - CAT
    - PMT
    - SDT
    - NIT
    - EIT
    - TDT
    - TOT
- PSI section building and packetization
    - PAT
    - CAT
    - PMT
    - SDT
    - NIT
    - EIT
    - TDT
    - TOT
- In-place PSI section editing (PMT, EIT)
- DVB descriptors parsing and encoding
- DVB character set text decoding
- PES header parsing and packetization (PTS/DTS)
- SPTS multiplexer with automatic PAT/PMT/PCR
- PCR arithmetic and PCR synthesis for streams with missing or broken PCR
- Stream slicing with sync recovery
