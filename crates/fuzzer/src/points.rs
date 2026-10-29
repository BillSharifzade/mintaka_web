//! Coverage points: the branches of the parser the fuzzer can discover.
//!
//! Real fuzzers get coverage from compiler instrumentation. Here the parser marks its own
//! branches with [`Coverage::hit`], which keeps the WebAssembly module tiny and lets the page
//! name every path. The discriminants are the bit positions in the coverage mask and the indexes
//! the island uses for labels and sky positions, so **only ever append** to this list.

/// One discoverable branch of the parser.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum P {
    // Header
    MagicO = 0,
    MagicOR = 1,
    MagicORI = 2,
    Version1 = 3,
    Version2 = 4,
    VersionBad = 5,
    NoStars = 6,
    TooManyStars = 7,
    CountOk = 8,
    RecordsTruncated = 9,
    // Star records
    Star1 = 10,
    Star2 = 11,
    Star3 = 12,
    Star7 = 13,
    MagBright = 14,
    MagFirst = 15,
    MagSecond = 16,
    MagFaint = 17,
    QuadNE = 18,
    QuadNW = 19,
    QuadSE = 20,
    QuadSW = 21,
    FlagVariable = 22,
    FlagMultiple = 23,
    FlagGiant = 24,
    FlagSupergiant = 25,
    FlagReserved = 26,
    RedSupergiant = 27,
    BlueMultiple = 28,
    // Sections (version 2)
    End = 29,
    SectionUnknown = 30,
    SectionLines = 31,
    LinesOk = 32,
    LineBad = 33,
    SectionName = 34,
    NameLen = 35,
    NameOk = 36,
    NameUpper = 37,
    NameMintaka = 38,
}

/// Number of coverage points. Must stay ≤ 64 (they live in a `u64` mask).
pub const COUNT: usize = 39;
const _: () = assert!(COUNT <= 64 && P::NameMintaka as usize == COUNT - 1);

/// The set of points one execution reached.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage(u64);

impl Coverage {
    #[inline(always)]
    pub fn hit(&mut self, p: P) {
        self.0 |= 1u64 << (p as u8);
    }

    #[inline(always)]
    pub fn bits(self) -> u64 {
        self.0
    }

    pub fn contains(self, p: P) -> bool {
        self.0 & (1u64 << (p as u8)) != 0
    }
}
