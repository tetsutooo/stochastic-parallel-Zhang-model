use indicatif::ProgressBar;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::core::{Parameters, System};
use crate::io;
use crate::line_plot::{AxisScale, LinePlot};

#[derive(Default)]
struct FastU64Hasher {
    hash: u64,
}

impl Hasher for FastU64Hasher {
    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.hash = n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.hash = (self.hash ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3);
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

type FastU64Map = HashMap<u64, u64, BuildHasherDefault<FastU64Hasher>>;

#[inline]
fn bump_count(counts: &mut Vec<u64>, value: u64) {
    let k = value as usize;
    if k >= counts.len() {
        counts.resize(k + 1, 0);
    }
    counts[k] += 1;
}

fn build_distribution_from_map(counts: &FastU64Map, steps: u64) -> Vec<(f64, f64)> {
    let denom = steps as f64;
    let mut dist: Vec<(f64, f64)> = counts
        .iter()
        .map(|(&k, &v)| (k as f64, v as f64 / denom))
        .collect();
    dist.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    dist
}

fn build_distribution_from_vec(counts: &[u64], steps: u64) -> Vec<(f64, f64)> {
    let denom = steps as f64;
    counts
        .iter()
        .enumerate()
        .filter(|&(_, &v)| v > 0)
        .map(|(k, &v)| (k as f64, v as f64 / denom))
        .collect()
}

struct SizeDurationStats {
    acc: Vec<(f64, f64, f64)>,
}

impl SizeDurationStats {
    fn new() -> Self {
        Self { acc: Vec::new() }
    }

    #[inline]
    fn update(&mut self, duration: u64, size: u64) {
        let d = duration as usize;
        if d >= self.acc.len() {
            self.acc.resize(d + 1, (0.0, 0.0, 0.0));
        }
        let e = &mut self.acc[d];
        let x = size as f64;
        e.0 += 1.0;
        let delta = x - e.1;
        e.1 += delta / e.0;
        e.2 += delta * (x - e.1);
    }

    fn finalize(&self) -> Vec<(f64, f64, f64)> {
        self.acc
            .iter()
            .enumerate()
            .filter(|&(_, &(n, _, _))| n > 0.0)
            .map(|(duration, &(n, mean, m2))| {
                let var = if n > 1.0 { m2 / (n - 1.0) } else { 0.0 };
                (duration as f64, mean, var.max(0.0).sqrt())
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PowerLawFit {
    pub a: f64,
    pub b: f64,
}

const A_MIN: f64 = 1e-12;
const A_MAX: f64 = 1e12;
const B_ABS_MAX: f64 = 10.0;
const MAX_BACKTRACK: usize = 60;

pub fn fit_power_law(
    data: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    a_init: f64,
    b_init: f64,
) -> Option<PowerLawFit> {
    let pts: Vec<(f64, f64)> = data
        .iter()
        .copied()
        .filter(|&(x, y)| x >= x_min && x <= x_max && x > 0.0 && y > 0.0)
        .collect();
    if pts.len() < 3 {
        return None;
    }

    let mut a: f64 = a_init;
    let mut b: f64 = b_init;

    let max_iter = 200000_usize;
    let tol = 1e-10_f64;

    let accumulate = |a: f64, b: f64, pts: &[(f64, f64)]| {
        let mut jtj = [[0.0_f64; 2]; 2];
        let mut jtr = [0.0_f64; 2];
        let mut ssr = 0.0_f64;
        for &(x, y) in pts {
            let xb = x.powf(b);
            let f = a * xb;
            let r = y - f;
            let dfa = xb;
            let dfb = f * x.ln();

            jtj[0][0] += dfa * dfa;
            jtj[0][1] += dfa * dfb;
            jtj[1][1] += dfb * dfb;
            jtr[0] += dfa * r;
            jtr[1] += dfb * r;
            ssr += r * r;
        }
        jtj[1][0] = jtj[0][1];
        (jtj, jtr, ssr)
    };

    for _ in 0..max_iter {
        let (jtj, jtr, ssr_cur) = accumulate(a, b, &pts);
        let det = jtj[0][0] * jtj[1][1] - jtj[0][1] * jtj[1][0];
        if det.abs() < 1e-30 {
            return None;
        }
        let da = (jtj[1][1] * jtr[0] - jtj[0][1] * jtr[1]) / det;
        let db = (-jtj[1][0] * jtr[0] + jtj[0][0] * jtr[1]) / det;

        let mut t = 1.0_f64;
        let mut stepped = false;
        for _ in 0..MAX_BACKTRACK {
            let a_try = (a + t * da).clamp(A_MIN, A_MAX);
            let b_try = (b + t * db).clamp(-B_ABS_MAX, B_ABS_MAX);
            let (_, _, ssr_try) = accumulate(a_try, b_try, &pts);
            if ssr_try.is_finite() && ssr_try < ssr_cur {
                a = a_try;
                b = b_try;
                stepped = true;
                break;
            }
            t *= 0.5;
        }
        if !stepped {
            break;
        }

        if (t * da).abs() < tol * (a.abs() + tol) && (t * db).abs() < tol * (b.abs() + tol) {
            break;
        }
    }

    Some(PowerLawFit { a, b })
}

fn calculate(system: &mut System, pb: &ProgressBar, init_steps: u64, steps: u64) {
    let mut size_counts: FastU64Map = FastU64Map::default();
    let mut duration_counts: Vec<u64> = Vec::new();
    let mut size_duration_stats_acc = SizeDurationStats::new();

    let n_chunks = 1000;
    let chunk_size = steps / n_chunks;

    for _i in 0..n_chunks {
        for _j in 0..chunk_size {
            let (size, duration): (u64, u64) = system.update();
            *size_counts.entry(size).or_insert(0) += 1;
            bump_count(&mut duration_counts, duration);
            size_duration_stats_acc.update(duration, size);
        }
        pb.inc(chunk_size);
    }

    let summary = system.simplified_format();

    let size_dist = build_distribution_from_map(&size_counts, steps);
    let size_dat_path = system.get_dat_file_path("size", init_steps, steps);
    io::write_v_f64_pair(&size_dat_path, &size_dist).unwrap();

    let size_fit: Option<PowerLawFit> = fit_power_law(
        &size_dist,
        20.0,
        size_dist.last().unwrap().0 / 20.0,
        0.12,
        -1.5,
    );

    let size_png_path = system.get_png_file_path("size", init_steps, steps);
    let mut size_plot = LinePlot::new()
        .data(size_dist)
        .title(format!("Size distribution ({})", summary))
        .x_label("size")
        .y_label("normalized frequency")
        .axis_scale(AxisScale::LogLog)
        .color(0)
        .legend("data");
    if let Some(fit) = size_fit {
        size_plot = size_plot.power_law(fit.a, fit.b);
    }
    size_plot.save(&size_png_path).unwrap();

    let duration_dist = build_distribution_from_vec(&duration_counts, steps);
    let duration_dat_path = system.get_dat_file_path("duration", init_steps, steps);
    io::write_v_f64_pair(&duration_dat_path, &duration_dist).unwrap();

    let duration_fit: Option<PowerLawFit> = fit_power_law(
        &duration_dist,
        20.0,
        duration_dist.last().unwrap().0 / 20.0,
        0.12,
        -1.5,
    );

    let duration_png_path = system.get_png_file_path("duration", init_steps, steps);
    let mut duration_plot = LinePlot::new()
        .data(duration_dist.clone())
        .title(format!("Duration distribution ({})", summary))
        .x_label("duration")
        .y_label("normalized frequency")
        .axis_scale(AxisScale::LogLog)
        .color(1)
        .legend("data");
    if let Some(fit) = duration_fit {
        duration_plot = duration_plot.power_law(fit.a, fit.b);
    }
    duration_plot.save(&duration_png_path).unwrap();

    let size_duration_stats = size_duration_stats_acc.finalize();
    let size_duration_stats_dat_path = system.get_dat_file_path("size_duration", init_steps, steps);
    io::write_v_f64_triple(&size_duration_stats_dat_path, &size_duration_stats).unwrap();

    let size_duration_scaling: Vec<(f64, f64)> = size_duration_stats
        .iter()
        .map(|&(duration, mean_size, _std_size)| (duration as f64, mean_size as f64))
        .collect();

    let size_duration_scaling_fit: Option<PowerLawFit> = fit_power_law(
        &size_duration_scaling,
        20.0,
        size_duration_scaling.last().unwrap().0 / 20.0,
        1.0,
        1.8,
    );

    let size_duration_scaling_png_path =
        system.get_png_file_path("size_duration_scaling", init_steps, steps);
    let mut size_duration_scaling_plot = LinePlot::new()
        .data(size_duration_scaling.clone())
        .title(format!("Size duration scaling ({})", summary))
        .x_label("duration")
        .y_label("size")
        .axis_scale(AxisScale::LogLog)
        .color(2)
        .legend("data");
    if let Some(fit) = size_duration_scaling_fit {
        size_duration_scaling_plot = size_duration_scaling_plot.power_law(fit.a, fit.b);
    }
    size_duration_scaling_plot
        .save(&size_duration_scaling_png_path)
        .unwrap();
}

pub fn calculate1(param: &Parameters, pb: &ProgressBar, init_steps: u64, steps: u64) {
    let mut system = System::new(param.clone());
    if init_steps == 0 {
        system.init();
    } else {
        system.load(init_steps).unwrap();
    }

    calculate(&mut system, pb, init_steps, steps);

    system.save(init_steps + steps).unwrap();
}
