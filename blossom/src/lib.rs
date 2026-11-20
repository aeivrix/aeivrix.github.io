#![allow(static_mut_refs)] // single-threaded wasm

use std::f32::consts::TAU;

// opcodes — keep in sync with replay() in index.html
const SAVE: f32 = 0.0;
const RESTORE: f32 = 1.0;
const TRANSLATE: f32 = 2.0;
const ROTATE: f32 = 3.0;
const SCALE: f32 = 4.0;
const BEGIN: f32 = 5.0;
const MOVE: f32 = 6.0;
const LINE: f32 = 7.0;
const QUAD: f32 = 8.0;
const BEZIER: f32 = 9.0;
const ARC: f32 = 10.0;
const ELLIPSE: f32 = 11.0;
const FILL: f32 = 12.0; // grey, alpha
const STROKE: f32 = 13.0; // grey, alpha, width
const LINEAR_FILL: f32 = 14.0; // x0 y0 x1 y1, n, (offset grey alpha) * n
const RADIAL_FILL: f32 = 15.0; // x0 y0 r0 x1 y1 r1, n, (offset grey alpha) * n
const ALPHA: f32 = 16.0;
const ROUND_CAPS: f32 = 18.0;
const TIME: f32 = 19.0; // t in 0..1: when the page should draw what follows (growth order)
const SPRITE: f32 = 21.0; // x y half n, then n floats: a piece the page animates opening around (x, y)

struct Pen {
    seed: u32,
    out: Vec<f32>,
    spots: Vec<f32>,
    limbs: Vec<f32>, // x0 y0 x1 y1 width t0 t1 per branch piece; the page draws these growing
}

static mut PEN: Pen = Pen { seed: 7, out: Vec::new(), spots: Vec::new(), limbs: Vec::new() };

impl Pen {
    // mulberry32
    fn rand(&mut self) -> f32 {
        self.seed = self.seed.wrapping_add(0x6D2B_79F5);
        let mut t = self.seed;
        t = (t ^ (t >> 15)).wrapping_mul(1 | t);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64 / 4_294_967_296.0) as f32
    }
    fn rr(&mut self, a: f32, b: f32) -> f32 {
        a + self.rand() * (b - a)
    }
    fn op(&mut self, v: &[f32]) {
        self.out.extend_from_slice(v);
    }
    fn gradient(&mut self, head: &[f32], stops: &[(f32, f32, f32)]) {
        self.op(head);
        self.op(&[stops.len() as f32]);
        for &(o, g, a) in stops {
            self.op(&[o, g, a]);
        }
    }

    /// one petal pointing along +x: wavy rim, notched tip, dark base, cupped edges, veins
    fn petal(&mut self, r: f32) {
        let w = r * self.rr(0.55, 0.68);
        let mut j = [0.0; 7];
        for v in j.iter_mut() {
            *v = self.rr(-0.04, 0.04) * r;
        }
        self.op(&[BEGIN, MOVE, 0.0, 0.0]);
        self.op(&[BEZIER, r * 0.2, -w * 0.7 + j[0], r * 0.55, -w * 1.05 + j[1], r * 0.85, -w * 0.7 + j[2]]);
        self.op(&[QUAD, r, -w * 0.4, r * 0.97, -w * 0.12]);
        self.op(&[QUAD, r * 0.9, j[6] * w / r, r * 0.97, w * 0.12]);
        self.op(&[QUAD, r, w * 0.4, r * 0.85, w * 0.7 + j[3]]);
        self.op(&[BEZIER, r * 0.55, w * 1.05 + j[4], r * 0.2, w * 0.7 + j[5], 0.0, 0.0]);
        self.gradient(&[LINEAR_FILL, 0.0, 0.0, r, 0.0], &[(0.0, 90.0, 1.0), (0.3, 216.0, 1.0), (0.75, 246.0, 1.0), (1.0, 226.0, 1.0)]);
        // edges curl away from the light
        self.gradient(&[LINEAR_FILL, 0.0, -w, 0.0, w], &[(0.0, 0.0, 0.28), (0.35, 0.0, 0.0), (0.65, 0.0, 0.0), (1.0, 0.0, 0.18)]);
        for k in [-0.35, 0.0, 0.35] {
            self.op(&[BEGIN, MOVE, r * 0.12, 0.0, QUAD, r * 0.5, w * k * 0.6, r * 0.82, w * k]);
            self.op(&[STROKE, 0.0, 0.16, (r * 0.012).max(0.5)]);
        }
    }

    fn flower(&mut self, x: f32, y: f32, r: f32, open: bool) {
        let (spin, tilt, off) = (self.rr(0.0, TAU), self.rr(0.5, 1.0), self.rr(0.0, TAU));
        self.op(&[SAVE, TRANSLATE, x, y, ROTATE, spin, SCALE, 1.0, tilt]); // tilted towards/away from us
        let n = if open { 5 } else { 3 };
        for k in 0..n {
            let spread = if open { TAU } else { 2.2 };
            let a = off + k as f32 / n as f32 * spread + self.rr(-0.18, 0.18);
            let len = r * if open { self.rr(0.85, 1.05) } else { self.rr(0.55, 0.7) };
            self.op(&[SAVE, ROTATE, a]);
            self.petal(len);
            self.op(&[RESTORE]);
        }
        if open {
            self.op(&[BEGIN, ARC, 0.0, 0.0, r * 0.4]);
            self.gradient(&[RADIAL_FILL, 0.0, 0.0, 0.0, 0.0, 0.0, r * 0.4], &[(0.0, 17.0, 1.0), (0.6, 25.0, 0.6), (1.0, 30.0, 0.0)]);
            for _ in 0..22 {
                // stamens: pale filaments, grey anthers
                let (a, l) = (self.rr(0.0, TAU), r * self.rr(0.2, 0.45));
                let (sx, sy) = (a.cos() * l, a.sin() * l);
                let (cx, cy) = (sx * 0.5 + self.rr(-2.0, 2.0), sy * 0.5 + self.rr(-2.0, 2.0));
                self.op(&[BEGIN, MOVE, 0.0, 0.0, QUAD, cx, cy, sx, sy, STROKE, 220.0, 0.8, (r * 0.018).max(0.4)]);
                let grey = if self.rand() < 0.3 { 238.0 } else { 138.0 };
                let dot = r * self.rr(0.025, 0.045);
                self.op(&[BEGIN, ARC, sx, sy, dot, FILL, grey, 1.0]);
            }
        }
        self.op(&[RESTORE]);
    }

    fn bud(&mut self, x: f32, y: f32, r: f32, a: f32) {
        self.op(&[SAVE, TRANSLATE, x, y, ROTATE, a, BEGIN, ELLIPSE, 0.0, 0.0, r, r * 0.62]);
        self.gradient(&[RADIAL_FILL, r * 0.3, -r * 0.2, 0.0, 0.0, 0.0, r], &[(0.0, 242.0, 1.0), (1.0, 85.0, 1.0)]);
        self.op(&[BEGIN, ELLIPSE, -r * 0.85, 0.0, r * 0.35, r * 0.4, FILL, 34.0, 1.0, RESTORE]); // sepals
    }

    fn leaf(&mut self, x: f32, y: f32, r: f32, a: f32) {
        self.op(&[SAVE, TRANSLATE, x, y, ROTATE, a, BEGIN, MOVE, 0.0, 0.0]);
        self.op(&[BEZIER, r * 0.3, -r * 0.42, r * 0.75, -r * 0.3, r, 0.0]);
        self.op(&[BEZIER, r * 0.75, r * 0.3, r * 0.3, r * 0.42, 0.0, 0.0]);
        self.gradient(&[LINEAR_FILL, 0.0, -r * 0.4, 0.0, r * 0.4], &[(0.0, 60.0, 1.0), (0.5, 38.0, 1.0), (1.0, 18.0, 1.0)]);
        self.op(&[BEGIN, MOVE, 0.0, 0.0, LINE, r * 0.9, 0.0, STROKE, 255.0, 0.12, 0.8, RESTORE]);
    }

    fn stalk(&mut self, x: f32, y: f32, rad: f32, bx: f32, by: f32) {
        let (cx, cy) = ((bx + x) / 2.0 + self.rr(-4.0, 4.0), (by + y) / 2.0 + self.rr(-4.0, 4.0));
        self.op(&[BEGIN, MOVE, bx, by, QUAD, cx, cy, x, y, STROKE, 42.0, 1.0, (rad * 0.06).max(1.0)]);
    }

    /// a blossom; some sit in shadow (dimmer) so the cluster has depth
    fn bloom(&mut self, x: f32, y: f32, rad: f32) {
        let depth = self.rand();
        let alpha = if depth < 0.25 { 0.45 } else if depth < 0.5 { 0.75 } else { 1.0 };
        // no canvas blur: chromium blurs every shape separately, which is very slow
        self.op(&[SAVE, ALPHA, alpha]);
        let open = self.rand() > 0.15;
        self.flower(x, y, rad, open);
        self.op(&[RESTORE]);
    }

    /// moves everything emitted since `start` into a unit drawn at path length `t`
    fn unit(&mut self, start: usize, t: f32, units: &mut Vec<(f32, Vec<f32>)>) {
        let ops = self.out.split_off(start);
        units.push((t, ops));
    }

    /// like unit(), but wrapped so the page can animate it opening around (x, y)
    fn sprite(&mut self, start: usize, t: f32, x: f32, y: f32, half: f32, units: &mut Vec<(f32, Vec<f32>)>) {
        let body = self.out.split_off(start);
        let mut ops = vec![SPRITE, x, y, half, body.len() as f32];
        ops.extend(body);
        units.push((t, ops));
    }

    /// a tapered, knotted branch segment, then its children; records every tip.
    /// `t` is the path length from the root, so pieces can be drawn in the order they'd grow
    #[allow(clippy::too_many_arguments)]
    fn grow(&mut self, x: f32, y: f32, a: f32, len: f32, w: f32, depth: u32, t: f32, tips: &mut Vec<[f32; 8]>, units: &mut Vec<(f32, Vec<f32>)>) {
        let bend = self.rr(-0.35, 0.35);
        let (mx, my) = (x + (a + bend * 0.5).cos() * len * 0.5, y + (a + bend * 0.5).sin() * len * 0.5);
        let (ex, ey) = (x + (a + bend).cos() * len, y + (a + bend).sin() * len);
        let at = |t: f32| {
            let u = 1.0 - t;
            (u * u * x + 2.0 * u * t * mx + t * t * ex, u * u * y + 2.0 * u * t * my + t * t * ey)
        };
        for q in 0..6 {
            let (t0, t1) = (q as f32 / 6.0, (q + 1) as f32 / 6.0);
            let ((x0, y0), (x1, y1)) = (at(t0), at(t1));
            let ww = w * (1.0 - t0 * 0.3);
            self.limbs.extend_from_slice(&[x0, y0, x1, y1, ww, t + len * t0, t + len * t1]);
        }
        if self.rand() < 0.4 {
            let start = self.out.len();
            self.op(&[BEGIN, ARC, ex, ey, w * 0.55, FILL, 48.0, 1.0]); // knot
            self.unit(start, t + len, units);
        }
        tips.push([ex, ey, a + bend, depth as f32, mx, my, t + len * 0.5, t + len]);
        if depth == 0 {
            return;
        }
        let kids = if depth > 2 { 2 } else { 1 + (self.rand() < 0.5) as u32 };
        for _ in 0..kids {
            let (turn, shrink) = (self.rr(-0.75, 0.75), self.rr(0.6, 0.8));
            self.grow(ex, ey, a + bend + turn, len * shrink, w * 0.7, depth - 1, t + len, tips, units);
        }
    }

    fn cluster(&mut self, ax: f32, ay: f32, a: f32, len: f32, depth: u32, r: f32) {
        self.out.clear();
        self.limbs.clear();
        self.op(&[ROUND_CAPS]);
        let (mut tips, mut units) = (Vec::new(), Vec::new());
        self.grow(ax, ay, a, len * 0.55, (r * 0.3).max(3.0), depth, 0.0, &mut tips, &mut units);
        for [x, y, ang, d, mx, my, tm, te] in tips {
            if self.rand() < 0.5 {
                let (lr, la) = (r * self.rr(0.9, 1.4), ang + self.rr(-1.3, 1.3));
                let (start, lag) = (self.out.len(), self.rr(0.2, 0.6) * len);
                self.leaf(mx, my, lr, la);
                self.sprite(start, tm + lag, mx, my, lr * 1.1, &mut units);
            }
            if d <= 2.0 {
                let n = 1 + (self.rand() < 0.45) as u32;
                for _ in 0..n {
                    let (fx, fy, fr) = (x + self.rr(-1.6, 1.6) * r, y + self.rr(-1.6, 1.6) * r, r * self.rr(0.75, 1.1));
                    // blossoms open a while after their twig arrives, not all at once
                    let (start, lag) = (self.out.len(), self.rr(0.15, 0.9) * len);
                    self.stalk(fx, fy, fr, x, y);
                    self.unit(start, te + lag, &mut units);
                    let start = self.out.len();
                    self.bloom(fx, fy, fr);
                    self.sprite(start, te + lag + 0.05 * len, fx, fy, fr * 1.25, &mut units);
                    self.spots.extend_from_slice(&[fx, fy]);
                }
                if self.rand() < 0.6 {
                    let (start, lag) = (self.out.len(), self.rr(0.0, 0.4) * len);
                    let (bx, by) = (x + ang.cos() * r * 1.3, y + ang.sin() * r * 1.3);
                    self.bud(bx, by, r * 0.3, ang);
                    self.sprite(start, te + lag, bx, by, r * 0.45, &mut units);
                }
            }
        }
        // emit in growth order, each piece tagged with its normalised time
        units.sort_by(|a, b| a.0.total_cmp(&b.0));
        let last_limb = self.limbs.chunks(7).map(|l| l[6]).fold(0.0, f32::max);
        let end = units.last().map_or(0.0, |u| u.0).max(last_limb).max(1.0);
        for l in self.limbs.chunks_mut(7) {
            l[5] /= end;
            l[6] /= end;
        }
        for (t, ops) in units {
            self.op(&[TIME, t / end]);
            self.out.extend(ops);
        }
    }
}

/// start a new tree: reseed and forget earlier flower spots
#[no_mangle]
pub extern "C" fn reset(seed: u32) {
    unsafe {
        PEN.seed = seed;
        PEN.spots.clear();
    }
}

/// grow one branch cluster from (ax, ay) heading `angle`; returns the command count
#[no_mangle]
pub extern "C" fn cluster(ax: f32, ay: f32, angle: f32, len: f32, depth: u32, r: f32) -> usize {
    unsafe {
        PEN.cluster(ax, ay, angle, len, depth, r);
        PEN.out.len()
    }
}

/// a single loose petal (for the falling ones); returns the command count
#[no_mangle]
pub extern "C" fn petal(r: f32) -> usize {
    unsafe {
        PEN.out.clear();
        PEN.petal(r);
        PEN.out.len()
    }
}

#[no_mangle]
pub extern "C" fn commands() -> *const f32 {
    unsafe { PEN.out.as_ptr() }
}

#[no_mangle]
pub extern "C" fn spots() -> *const f32 {
    unsafe { PEN.spots.as_ptr() }
}

#[no_mangle]
pub extern "C" fn spot_count() -> usize {
    unsafe { PEN.spots.len() / 2 }
}

/// branch pieces of the last cluster, 7 floats each (see Pen::limbs)
#[no_mangle]
pub extern "C" fn limbs() -> *const f32 {
    unsafe { PEN.limbs.as_ptr() }
}

#[no_mangle]
pub extern "C" fn limb_count() -> usize {
    unsafe { PEN.limbs.len() / 7 }
}

#[cfg(test)]
mod tests {
    /// walk the stream with the same arities replay() uses; any drift breaks the page.
    /// returns the op count and every growth time tag, in stream order
    fn walk(b: &[f32], times: &mut Vec<f32>) -> usize {
        let (mut i, mut ops) = (0, 0);
        while i < b.len() {
            let op = b[i] as u32;
            i += 1;
            if op == 19 {
                times.push(b[i]);
            }
            if op == 21 {
                let n = b[i + 3] as usize;
                ops += walk(&b[i + 4..i + 4 + n], times);
            }
            i += match op {
                0 | 1 | 5 | 18 => 0,
                3 | 16 | 19 => 1,
                2 | 4 | 6 | 7 | 12 => 2,
                10 | 13 => 3,
                8 | 11 => 4,
                9 => 6,
                14 | 15 => {
                    let head = if op == 14 { 4 } else { 6 };
                    head + 1 + 3 * b[i + head] as usize
                }
                21 => 4 + b[i + 3] as usize,
                _ => panic!("bad opcode {op} at {}", i - 1),
            };
            ops += 1;
        }
        assert_eq!(i, b.len(), "stream ends mid-command");
        ops
    }

    #[test]
    fn cluster_stream_is_well_formed() {
        super::reset(42);
        let n = super::cluster(1280.0, 0.0, 2.0, 190.0, 4, 25.0);
        let b = unsafe { std::slice::from_raw_parts(super::commands(), n) };
        let mut times = Vec::new();
        assert!(walk(b, &mut times) > 100);
        assert!(b.iter().all(|v| v.is_finite()));
        // growth times are tagged in order and never past the end
        assert!(times.windows(2).all(|w| w[0] <= w[1]) && times.iter().all(|t| (0.0..=1.0).contains(t)));
        let limbs = unsafe { std::slice::from_raw_parts(super::limbs(), super::limb_count() * 7) };
        assert!(!limbs.is_empty() && limbs.chunks(7).all(|l| l[5] <= l[6] && l[5] >= 0.0 && l[6] <= 1.0));
        assert!(limbs[5] == 0.0, "the trunk starts growing at t = 0");
        assert!(super::spot_count() > 0);
        let n = super::petal(10.0);
        walk(unsafe { std::slice::from_raw_parts(super::commands(), n) }, &mut Vec::new());
    }
}
