//! The fuzz target: a parser for a tiny binary star-chart format.
//!
//! | Offset    | Field              | Rule                                          |
//! |-----------|--------------------|-----------------------------------------------|
//! | 0..3      | magic              | `ORI`                                         |
//! | 3         | version            | 1 or 2                                        |
//! | 4         | star count `n`     | 1..=7                                         |
//! | 5..5+4n   | `n` star records   | x, y, magnitude (i8, tenths), flags           |
//! | then      | sections (v2 only) | `C` lines · `N` name — repeated until the end |
//!
//! Star flags: 1 variable · 2 multiple · 4 giant · 8 supergiant; the high four bits are reserved.
//! A bright variable supergiant is "a Betelgeuse"; a multiple giant of the second magnitude is
//! "a Mintaka".
//!
//! [`Build::Buggy`] contains one deliberate bug, in the name section. [`Build::Fixed`] is the
//! same code with that line fixed. Everything else is shared, so their coverage is comparable.

use crate::points::{Coverage, P};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    Buggy,
    Fixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    BadMagic = 1,
    BadVersion = 2,
    NoStars = 3,
    TooManyStars = 4,
    Truncated = 5,
    ReservedFlags = 6,
    UnknownSection = 7,
    BadLine = 8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Chart<'a> {
    pub version: u8,
    pub stars: u8,
    pub lines: usize,
    pub name: Option<&'a [u8]>,
}

pub fn parse<'a>(data: &'a [u8], build: Build, cov: &mut Coverage) -> Result<Chart<'a>, ParseError> {
    // The magic, one byte at a time. Each match is its own branch, so a coverage-guided fuzzer
    // can climb it step by step; production fuzzers get the same effect from comparison tracing.
    if data.first() != Some(&b'O') {
        return Err(ParseError::BadMagic);
    }
    cov.hit(P::MagicO);
    if data.get(1) != Some(&b'R') {
        return Err(ParseError::BadMagic);
    }
    cov.hit(P::MagicOR);
    if data.get(2) != Some(&b'I') {
        return Err(ParseError::BadMagic);
    }
    cov.hit(P::MagicORI);

    let version = *data.get(3).ok_or(ParseError::Truncated)?;
    match version {
        1 => cov.hit(P::Version1),
        2 => cov.hit(P::Version2),
        _ => {
            cov.hit(P::VersionBad);
            return Err(ParseError::BadVersion);
        }
    }

    let n = *data.get(4).ok_or(ParseError::Truncated)?;
    if n == 0 {
        cov.hit(P::NoStars);
        return Err(ParseError::NoStars);
    }
    if n > 7 {
        cov.hit(P::TooManyStars);
        return Err(ParseError::TooManyStars);
    }
    cov.hit(P::CountOk);

    let records_end = 5 + 4 * n as usize;
    let Some(records) = data.get(5..records_end) else {
        cov.hit(P::RecordsTruncated);
        return Err(ParseError::Truncated);
    };
    for (i, record) in records.as_chunks::<4>().0.iter().enumerate() {
        star(record, cov)?;
        match i {
            0 => cov.hit(P::Star1),
            1 => cov.hit(P::Star2),
            2 => cov.hit(P::Star3),
            6 => cov.hit(P::Star7),
            _ => {}
        }
    }

    let mut chart = Chart { version, stars: n, lines: 0, name: None };
    if version == 1 {
        cov.hit(P::End);
        return Ok(chart);
    }

    let mut at = records_end;
    while let Some(&tag) = data.get(at) {
        match tag {
            b'C' => {
                cov.hit(P::SectionLines);
                let m = *data.get(at + 1).ok_or(ParseError::Truncated)? as usize;
                let pairs = data.get(at + 2..at + 2 + 2 * m).ok_or(ParseError::Truncated)?;
                for pair in pairs.as_chunks::<2>().0 {
                    if pair[0] >= n || pair[1] >= n {
                        cov.hit(P::LineBad);
                        return Err(ParseError::BadLine);
                    }
                }
                cov.hit(P::LinesOk);
                chart.lines += m;
                at += 2 + 2 * m;
            }
            b'N' => {
                cov.hit(P::SectionName);
                let len = *data.get(at + 1).ok_or(ParseError::Truncated)? as usize;
                cov.hit(P::NameLen);
                let start = at + 2;
                let name = match build {
                    // The bug: the length byte comes from the input, and this line trusts it.
                    Build::Buggy => &data[start..start + len],
                    // The fix: ask for the range, and treat "not there" as a malformed chart.
                    Build::Fixed => data.get(start..start + len).ok_or(ParseError::Truncated)?,
                };
                cov.hit(P::NameOk);
                if !name.is_empty() && name.iter().all(u8::is_ascii_uppercase) {
                    cov.hit(P::NameUpper);
                }
                if name == b"MINTAKA" {
                    cov.hit(P::NameMintaka);
                }
                chart.name = Some(name);
                at = start + len;
            }
            _ => {
                cov.hit(P::SectionUnknown);
                return Err(ParseError::UnknownSection);
            }
        }
    }
    cov.hit(P::End);
    Ok(chart)
}

fn star(record: &[u8; 4], cov: &mut Coverage) -> Result<(), ParseError> {
    let [x, y, mag, flags] = *record;
    let mag = mag as i8;
    if flags & 0xF0 != 0 {
        cov.hit(P::FlagReserved);
        return Err(ParseError::ReservedFlags);
    }
    cov.hit(match mag {
        i8::MIN..=-1 => P::MagBright,
        0..=14 => P::MagFirst,
        15..=24 => P::MagSecond,
        _ => P::MagFaint,
    });
    cov.hit(match (x >= 128, y < 128) {
        (true, true) => P::QuadNE,
        (false, true) => P::QuadNW,
        (true, false) => P::QuadSE,
        (false, false) => P::QuadSW,
    });
    if flags & 1 != 0 {
        cov.hit(P::FlagVariable);
    }
    if flags & 2 != 0 {
        cov.hit(P::FlagMultiple);
    }
    if flags & 4 != 0 {
        cov.hit(P::FlagGiant);
    }
    if flags & 8 != 0 {
        cov.hit(P::FlagSupergiant);
    }
    if flags & 0b1001 == 0b1001 && mag < 10 {
        cov.hit(P::RedSupergiant);
    }
    if flags & 0b0110 == 0b0110 && (15..25).contains(&mag) {
        cov.hit(P::BlueMultiple);
    }
    Ok(())
}
