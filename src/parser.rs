use std::io::{BufReader, Read};
use crate::{
    endian::Endian,
    error::{Result, StdfError},
    reader::RecordReader,
    records::{
        AtrRecord, BpsRecord, DtrRecord, EpsRecord, FarRecord, FtrRecord, GdrRecord,
        HbrRecord, MirRecord, MprRecord, MrrRecord, PcrRecord, PirRecord, PmrRecord,
        PrrRecord, PtrRecord, SbrRecord, SdrRecord, TsrRecord, WirRecord, WrrRecord,
    },
};

#[derive(Debug)]
pub enum StdfRecord {
    Far(FarRecord),
    Atr(AtrRecord),
    Mir(MirRecord),
    Mrr(MrrRecord),
    Pcr(PcrRecord),
    Hbr(HbrRecord),
    Sbr(SbrRecord),
    Pmr(PmrRecord),
    Sdr(SdrRecord),
    Wir(WirRecord),
    Wrr(WrrRecord),
    Pir(PirRecord),
    Prr(PrrRecord),
    Tsr(TsrRecord),
    Ptr(PtrRecord),
    Mpr(MprRecord),
    Ftr(FtrRecord),
    Bps(BpsRecord),
    Eps(EpsRecord),
    Gdr(GdrRecord),
    Dtr(DtrRecord),
    Unknown { typ: u8, sub: u8, data: Vec<u8> },
}

impl StdfRecord {
    pub fn record_type(&self) -> &'static str {
        match self {
            Self::Far(_)        => "FAR",
            Self::Atr(_)        => "ATR",
            Self::Mir(_)        => "MIR",
            Self::Mrr(_)        => "MRR",
            Self::Pcr(_)        => "PCR",
            Self::Hbr(_)        => "HBR",
            Self::Sbr(_)        => "SBR",
            Self::Pmr(_)        => "PMR",
            Self::Sdr(_)        => "SDR",
            Self::Wir(_)        => "WIR",
            Self::Wrr(_)        => "WRR",
            Self::Pir(_)        => "PIR",
            Self::Prr(_)        => "PRR",
            Self::Tsr(_)        => "TSR",
            Self::Ptr(_)        => "PTR",
            Self::Mpr(_)        => "MPR",
            Self::Ftr(_)        => "FTR",
            Self::Bps(_)        => "BPS",
            Self::Eps(_)        => "EPS",
            Self::Gdr(_)        => "GDR",
            Self::Dtr(_)        => "DTR",
            Self::Unknown { .. } => "UNK",
        }
    }

    pub(crate) fn dispatch(
        typ: u8, sub: u8,
        r:   &mut RecordReader<'_>,
        _offset: u64,
    ) -> Result<StdfRecord> {
        match (typ, sub) {
            (0,  20) => Ok(Self::Atr(AtrRecord::parse(r)?)),
            (1,  10) => Ok(Self::Mir(MirRecord::parse(r)?)),
            (1,  20) => Ok(Self::Mrr(MrrRecord::parse(r)?)),
            (1,  30) => Ok(Self::Pcr(PcrRecord::parse(r)?)),
            (1,  40) => Ok(Self::Hbr(HbrRecord::parse(r)?)),
            (1,  50) => Ok(Self::Sbr(SbrRecord::parse(r)?)),
            (1,  60) => Ok(Self::Pmr(PmrRecord::parse(r)?)),
            (1,  80) => Ok(Self::Sdr(SdrRecord::parse(r)?)),
            (2,  10) => Ok(Self::Wir(WirRecord::parse(r)?)),
            (2,  20) => Ok(Self::Wrr(WrrRecord::parse(r)?)),
            (5,  10) => Ok(Self::Pir(PirRecord::parse(r)?)),
            (5,  20) => Ok(Self::Prr(PrrRecord::parse(r)?)),
            (10, 30) => Ok(Self::Tsr(TsrRecord::parse(r)?)),
            (15, 10) => Ok(Self::Ptr(PtrRecord::parse(r)?)),
            (15, 15) => Ok(Self::Mpr(MprRecord::parse(r)?)),
            (15, 20) => Ok(Self::Ftr(FtrRecord::parse(r)?)),
            (20, 10) => Ok(Self::Bps(BpsRecord::parse(r)?)),
            (20, 20) => Ok(Self::Eps(EpsRecord::parse(r)?)),
            (50, 10) => Ok(Self::Gdr(GdrRecord::parse(r)?)),
            (50, 30) => Ok(Self::Dtr(DtrRecord::parse(r)?)),
            _ => {
                let data = r.read_remaining();
                Ok(Self::Unknown { typ, sub, data })
            }
        }
    }
}

fn record_name(typ: u8, sub: u8) -> &'static str {
    match (typ, sub) {
        (0,  10) => "FAR", (0,  20) => "ATR",
        (1,  10) => "MIR", (1,  20) => "MRR", (1, 30) => "PCR",
        (1,  40) => "HBR", (1,  50) => "SBR", (1, 60) => "PMR", (1, 80) => "SDR",
        (2,  10) => "WIR", (2,  20) => "WRR",
        (5,  10) => "PIR", (5,  20) => "PRR",
        (10, 30) => "TSR",
        (15, 10) => "PTR", (15, 15) => "MPR", (15, 20) => "FTR",
        (20, 10) => "BPS", (20, 20) => "EPS",
        (50, 10) => "GDR", (50, 30) => "DTR",
        _        => "UNK",
    }
}

pub struct StdfParser<R: Read> {
    inner:              BufReader<R>,
    file_offset:        u64,
    /// File offset of the start (4-byte header) of the most recently
    /// yielded (or failed) record. STDF records are variable-length, so
    /// this must be tracked explicitly rather than assuming a fixed stride.
    last_record_offset: u64,
    endian:             Endian,
    initialized:        bool,
    done:               bool,
}

impl<R: Read> StdfParser<R> {
    pub fn new(r: R) -> Self {
        Self {
            inner:              BufReader::new(r),
            file_offset:        0,
            last_record_offset: 0,
            endian:             Endian::Little,
            initialized:        false,
            done:               false,
        }
    }

    pub fn file_offset(&self) -> u64 { self.file_offset }

    /// File offset (start of the 4-byte header) of the record most
    /// recently returned by `next()` — or, if `next()` returned `Err`,
    /// the record whose header/body read failed.
    pub fn last_record_offset(&self) -> u64 { self.last_record_offset }

    fn read_next(&mut self) -> Result<Option<StdfRecord>> {
        let mut hdr = [0u8; 4];
        match self.inner.read_exact(&mut hdr) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(StdfError::Io(e.to_string())),
        }
        let rec_start = self.file_offset;
        self.last_record_offset = rec_start;
        self.file_offset += 4;

        let rec_len = if !self.initialized {
            u16::from_le_bytes([hdr[0], hdr[1]])
        } else {
            self.endian.u16([hdr[0], hdr[1]])
        };
        let typ = hdr[2];
        let sub = hdr[3];

        let mut body = vec![0u8; rec_len as usize];
        self.inner.read_exact(&mut body).map_err(|e| StdfError::Io(e.to_string()))?;
        self.file_offset += rec_len as u64;

        if !self.initialized {
            if typ != FarRecord::TYP || sub != FarRecord::SUB {
                return Err(StdfError::MissingFar { typ, sub });
            }
            let mut r = RecordReader::new(&body, "FAR", rec_start, Endian::Little);
            let far   = FarRecord::parse(&mut r)?;
            self.endian      = far.endian()?;
            self.initialized = true;
            return Ok(Some(StdfRecord::Far(far)));
        }

        let rname = record_name(typ, sub);
        let mut r = RecordReader::new(&body, rname, rec_start, self.endian);
        Ok(Some(StdfRecord::dispatch(typ, sub, &mut r, rec_start)?))
    }
}

impl<R: Read> Iterator for StdfParser<R> {
    type Item = Result<StdfRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done { return None; }
        match self.read_next() {
            Ok(Some(rec)) => Some(Ok(rec)),
            Ok(None)      => { self.done = true; None }
            Err(e)        => { self.done = true; Some(Err(e)) }
        }
    }
}
