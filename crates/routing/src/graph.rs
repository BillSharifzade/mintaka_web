//! The road graph and its binary file (`assets/data/dushanbe.bin`, written by `prep`).
//!
//! Nodes are intersections; each arc follows one road segment, whose full polyline is kept for
//! drawing. Positions are metres in a local projection (x east, y south). Weights are lengths in
//! decimetres, rounded up — so straight-line distance never overestimates, and A* stays exact.
//! The Contraction Hierarchy is stored as two arc sets per node: `up` (to higher-ranked nodes)
//! and `dn` (from higher-ranked nodes), each with the contracted `mid` node of a shortcut.

pub const MAGIC: &[u8; 4] = b"MKRG";
pub const VERSION: u32 = 1;
pub const NONE: u32 = u32::MAX;
/// Low bit of an arc's segment reference: the arc runs against the polyline's direction.
pub const REVERSED: u32 = 1;

/// Metres per degree of latitude and longitude at `lat0` (WGS 84 series expansion).
pub fn metres_per_degree(lat0: f64) -> (f64, f64) {
    let phi = lat0.to_radians();
    let m_lat = 111_132.954 - 559.822 * (2.0 * phi).cos() + 1.175 * (4.0 * phi).cos();
    let m_lon = 111_412.84 * phi.cos() - 93.5 * (3.0 * phi).cos();
    (m_lat, m_lon)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Graph {
    pub lat0: f64,
    pub lon0: f64,
    /// Node positions, metres.
    pub xy: Vec<[f32; 2]>,
    /// Forward arcs, CSR: arcs of `u` are `off[u]..off[u + 1]`.
    pub off: Vec<u32>,
    pub to: Vec<u32>,
    pub w: Vec<u32>,
    /// `segment << 1 | REVERSED?`
    pub seg: Vec<u32>,
    /// Segment polylines: points `seg_off[s]..seg_off[s + 1]` of `pts`, metres from `origin`.
    pub seg_off: Vec<u32>,
    pub pts: Vec<[u16; 2]>,
    pub origin: [f32; 2],
    /// Contraction order (0 = contracted first).
    pub rank: Vec<u32>,
    pub up_off: Vec<u32>,
    pub up_to: Vec<u32>,
    pub up_w: Vec<u32>,
    pub up_mid: Vec<u32>,
    pub dn_off: Vec<u32>,
    pub dn_from: Vec<u32>,
    pub dn_w: Vec<u32>,
    pub dn_mid: Vec<u32>,
}

impl Graph {
    pub fn nodes(&self) -> usize {
        self.xy.len()
    }

    /// Latitude/longitude → this graph's metres (x east, y south).
    pub fn project(&self, lat: f64, lon: f64) -> [f32; 2] {
        let (m_lat, m_lon) = metres_per_degree(self.lat0);
        [((lon - self.lon0) * m_lon) as f32, (-(lat - self.lat0) * m_lat) as f32]
    }

    pub fn point(&self, i: usize) -> [f32; 2] {
        let p = self.pts[i];
        [self.origin[0] + p[0] as f32, self.origin[1] + p[1] as f32]
    }

    /// The polyline of base arc `a`, in travel direction.
    pub fn arc_points(&self, a: usize) -> impl Iterator<Item = [f32; 2]> + '_ {
        let s = (self.seg[a] >> 1) as usize;
        let rev = self.seg[a] & REVERSED != 0;
        let (lo, hi) = (self.seg_off[s] as usize, self.seg_off[s + 1] as usize);
        let n = hi - lo;
        (0..n).map(move |k| self.point(if rev { hi - 1 - k } else { lo + k }))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        let u32s = |b: &mut Vec<u8>, v: &[u32]| v.iter().for_each(|x| b.extend_from_slice(&x.to_le_bytes()));
        b.extend_from_slice(MAGIC);
        for n in [
            VERSION,
            self.xy.len() as u32,
            self.to.len() as u32,
            (self.seg_off.len() - 1) as u32,
            self.pts.len() as u32,
            self.up_to.len() as u32,
            self.dn_from.len() as u32,
        ] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.extend_from_slice(&self.lat0.to_le_bytes());
        b.extend_from_slice(&self.lon0.to_le_bytes());
        b.extend_from_slice(&self.origin[0].to_le_bytes());
        b.extend_from_slice(&self.origin[1].to_le_bytes());
        for p in &self.xy {
            b.extend_from_slice(&p[0].to_le_bytes());
            b.extend_from_slice(&p[1].to_le_bytes());
        }
        for v in [&self.off, &self.to, &self.w, &self.seg, &self.seg_off] {
            u32s(&mut b, v);
        }
        for p in &self.pts {
            b.extend_from_slice(&p[0].to_le_bytes());
            b.extend_from_slice(&p[1].to_le_bytes());
        }
        for v in [&self.rank, &self.up_off, &self.up_to, &self.up_w, &self.up_mid, &self.dn_off, &self.dn_from, &self.dn_w, &self.dn_mid] {
            u32s(&mut b, v);
        }
        b
    }

    pub fn from_bytes(b: &[u8]) -> Result<Graph, String> {
        let mut r = Reader { b, at: 0 };
        if r.take(4)? != MAGIC {
            return Err("not a Mintaka road graph".into());
        }
        let version = r.u32()?;
        if version != VERSION {
            return Err(format!("graph version {version}, expected {VERSION}"));
        }
        let (n, m, nseg, npts, nup, ndn) = (r.u32()? as usize, r.u32()? as usize, r.u32()? as usize, r.u32()? as usize, r.u32()? as usize, r.u32()? as usize);
        let lat0 = f64::from_le_bytes(r.take(8)?.try_into().map_err(|_| "header")?);
        let lon0 = f64::from_le_bytes(r.take(8)?.try_into().map_err(|_| "header")?);
        let origin = [r.f32()?, r.f32()?];
        let mut xy = Vec::with_capacity(n);
        for _ in 0..n {
            xy.push([r.f32()?, r.f32()?]);
        }
        let off = r.u32s(n + 1)?;
        let to = r.u32s(m)?;
        let w = r.u32s(m)?;
        let seg = r.u32s(m)?;
        let seg_off = r.u32s(nseg + 1)?;
        let mut pts = Vec::with_capacity(npts);
        for _ in 0..npts {
            pts.push([r.u16()?, r.u16()?]);
        }
        let g = Graph {
            lat0,
            lon0,
            xy,
            off,
            to,
            w,
            seg,
            seg_off,
            pts,
            origin,
            rank: r.u32s(n)?,
            up_off: r.u32s(n + 1)?,
            up_to: r.u32s(nup)?,
            up_w: r.u32s(nup)?,
            up_mid: r.u32s(nup)?,
            dn_off: r.u32s(n + 1)?,
            dn_from: r.u32s(ndn)?,
            dn_w: r.u32s(ndn)?,
            dn_mid: r.u32s(ndn)?,
        };
        if r.at != b.len() {
            return Err("trailing bytes in graph".into());
        }
        if g.off.last().copied() != Some(m as u32) || g.up_off.last().copied() != Some(nup as u32) || g.dn_off.last().copied() != Some(ndn as u32) {
            return Err("corrupt arc offsets".into());
        }
        Ok(g)
    }
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.at..self.at + n).ok_or("truncated graph")?;
        self.at += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "u32")?))
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().map_err(|_| "u16")?))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "f32")?))
    }
    fn u32s(&mut self, n: usize) -> Result<Vec<u32>, String> {
        let s = self.take(n * 4)?;
        Ok(s.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
    }
}
