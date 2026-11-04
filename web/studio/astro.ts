/**
 * The sky's geometry for the hero's star chart: the gnomonic projection the site generator uses
 * (crates/site/src/orion.rs `project`), its inverse, and the readout format. Angles in degrees;
 * the plane's x grows to the west and y to the south (a sky map held up with north on top).
 */

const RAD = Math.PI / 180;

/** Right ascension and declination → the projection plane around a centre (both in degrees). */
export function project(ra: number, dec: number, c0: [number, number]): [number, number] {
  const [ra0, de0] = [c0[0] * RAD, c0[1] * RAD];
  const [a, d] = [ra * RAD, dec * RAD];
  const cosC = Math.sin(de0) * Math.sin(d) + Math.cos(de0) * Math.cos(d) * Math.cos(a - ra0);
  const x = (Math.cos(d) * Math.sin(a - ra0)) / cosC;
  const y = (Math.cos(de0) * Math.sin(d) - Math.sin(de0) * Math.cos(d) * Math.cos(a - ra0)) / cosC;
  return [-x / RAD, -y / RAD];
}

/** The projection plane → right ascension (0–360°) and declination. */
export function unproject(X: number, Y: number, c0: [number, number]): [number, number] {
  const [ra0, de0] = [c0[0] * RAD, c0[1] * RAD];
  const xi = -X * RAD;
  const eta = -Y * RAD;
  const rho = Math.hypot(xi, eta);
  if (rho < 1e-12) return [c0[0], c0[1]];
  const c = Math.atan(rho);
  const dec = Math.asin(Math.cos(c) * Math.sin(de0) + (eta * Math.sin(c) * Math.cos(de0)) / rho);
  const ra = ra0 + Math.atan2(xi * Math.sin(c), rho * Math.cos(de0) * Math.cos(c) - eta * Math.sin(de0) * Math.sin(c));
  return [((ra / RAD) % 360 + 360) % 360, dec / RAD];
}

/** "RA 5h 55m · Dec +7° 24′" — to the minute, with a real minus sign. */
export function formatRaDec(ra: number, dec: number): string {
  const h = ra / 15;
  const hh = Math.floor(h);
  const mm = Math.floor((h - hh) * 60);
  const abs = Math.abs(dec);
  const dd = Math.floor(abs);
  const am = Math.floor((abs - dd) * 60);
  const two = (n: number) => String(n).padStart(2, '0');
  return `RA ${hh}h ${two(mm)}m · Dec ${dec < 0 ? '−' : '+'}${dd}° ${two(am)}′`;
}
