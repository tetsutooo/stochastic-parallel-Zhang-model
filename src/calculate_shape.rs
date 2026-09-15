use indicatif::ProgressBar;

use crate::core::{Parameters, System};
use crate::io;
use crate::multi_line_plot::{AxisScale, MultiLinePlot};

const TARGET_DURATIONS_SMALL: [u64; 7] = [40, 45, 50, 55, 60, 65, 70];
const TARGET_DURATIONS_LARGE: [u64; 7] = [130, 135, 140, 145, 150, 155, 160];
const TARGET_DURATIONS_ALL: [u64; 8] = [50, 60, 70, 80, 90, 100, 110, 120];

const DURATION_GROUPS: [(&str, &[u64]); 3] = [
    ("small", &TARGET_DURATIONS_SMALL),
    ("large", &TARGET_DURATIONS_LARGE),
    ("all", &TARGET_DURATIONS_ALL),
];

const WINDOW_HALF_WIDTH: u64 = 0;

fn linear_interp(xs: &[f64], ys: &[f64], u: f64) -> Option<f64> {
    if xs.len() < 2 || xs.len() != ys.len() {
        return None;
    }
    if u < xs[0] || u > *xs.last().unwrap() {
        return None;
    }

    let pos = xs.partition_point(|&x| x <= u);
    if pos == 0 {
        return Some(ys[0]);
    }
    if pos >= xs.len() {
        return Some(*ys.last().unwrap());
    }
    let x0 = xs[pos - 1];
    let x1 = xs[pos];
    let y0 = ys[pos - 1];
    let y1 = ys[pos];
    let alpha = (u - x0) / (x1 - x0);
    Some(y0 + alpha * (y1 - y0))
}

fn collapsed_curve(avg_shape: &[f64], target: u64, gamma: f64) -> (Vec<f64>, Vec<f64>) {
    let t_f = target as f64;
    let scale = t_f.powf(gamma - 1.0);
    let xs: Vec<f64> = (0..avg_shape.len())
        .map(|k| (k as f64 + 1.0) / t_f)
        .collect();
    let ys: Vec<f64> = avg_shape.iter().map(|&s| s / scale).collect();
    (xs, ys)
}

fn collapse_score(
    avg_shapes: &[Vec<f64>],
    targets: &[u64],
    counts: &[u64],
    gamma: f64,
    probes: &[f64],
) -> f64 {
    let curves: Vec<(Vec<f64>, Vec<f64>)> = avg_shapes
        .iter()
        .zip(targets.iter())
        .zip(counts.iter())
        .filter_map(|((shape, &target), &c)| {
            if c == 0 {
                None
            } else {
                Some(collapsed_curve(shape, target, gamma))
            }
        })
        .collect();
    if curves.len() < 2 {
        return f64::INFINITY;
    }

    let mut score = 0.0;
    let mut n_valid = 0usize;
    for &u in probes {
        let logs: Vec<f64> = curves
            .iter()
            .filter_map(|(xs, ys)| {
                let v = linear_interp(xs, ys, u)?;
                if v > 0.0 { Some(v.ln()) } else { None }
            })
            .collect();
        if logs.len() < 2 {
            continue;
        }
        let mean = logs.iter().sum::<f64>() / logs.len() as f64;
        let var = logs.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / logs.len() as f64;
        score += var;
        n_valid += 1;
    }
    if n_valid == 0 {
        return f64::INFINITY;
    }
    score / n_valid as f64
}

fn find_optimal_gamma(avg_shapes: &[Vec<f64>], targets: &[u64], counts: &[u64]) -> f64 {
    let probes: Vec<f64> = (10..=90).map(|k| k as f64 / 100.0).collect();

    let coarse: Vec<f64> = (100..=300).map(|i| i as f64 / 100.0).collect();
    let (mut best_gamma, mut best_score) = (coarse[0], f64::INFINITY);
    for &g in &coarse {
        let s = collapse_score(avg_shapes, targets, counts, g, &probes);
        if s < best_score {
            best_score = s;
            best_gamma = g;
        }
    }

    let lo = (best_gamma - 0.02).max(0.5);
    let hi = (best_gamma + 0.02).min(4.0);
    let n_fine = 200usize;
    for i in 0..=n_fine {
        let g = lo + (hi - lo) * (i as f64) / (n_fine as f64);
        let s = collapse_score(avg_shapes, targets, counts, g, &probes);
        if s < best_score {
            best_score = s;
            best_gamma = g;
        }
    }

    best_gamma
}

fn fit_amplitude_and_asymmetry(
    avg_shapes: &[Vec<f64>],
    targets: &[u64],
    counts: &[u64],
    gamma: f64,
) -> (f64, f64) {
    let mut s00 = 0.0;
    let mut s01 = 0.0;
    let mut s11 = 0.0;
    let mut t0 = 0.0;
    let mut t1 = 0.0;

    for ((shape, &target), &c) in avg_shapes.iter().zip(targets.iter()).zip(counts.iter()) {
        if c == 0 {
            continue;
        }
        let t_f = target as f64;
        let scale = t_f.powf(gamma - 1.0);
        for (k, &s) in shape.iter().enumerate() {
            let u = (k as f64 + 1.0) / t_f;
            if u <= 0.0 || u >= 1.0 {
                continue;
            }
            let base = (u * (1.0 - u)).powf(gamma - 1.0);
            let h_val = (u - 0.5) * base;
            let y = s / scale;
            s00 += base * base;
            s01 += base * h_val;
            s11 += h_val * h_val;
            t0 += base * y;
            t1 += h_val * y;
        }
    }

    let det = s00 * s11 - s01 * s01;
    if det.abs() < 1e-20 {
        return (0.0, 0.0);
    }
    let c_amp = (s11 * t0 - s01 * t1) / det;
    let b_coef = (-s01 * t0 + s00 * t1) / det;
    let asym = if c_amp.abs() > 1e-12 {
        -b_coef / c_amp
    } else {
        0.0
    };
    (c_amp, asym)
}

fn calculate(system: &mut System, pb: &ProgressBar, init_steps: u64, steps: u64) {
    let mut group_sums: Vec<Vec<Vec<f64>>> = DURATION_GROUPS
        .iter()
        .map(|&(_, targets)| targets.iter().map(|&t| vec![0.0; t as usize]).collect())
        .collect();
    let mut group_counts: Vec<Vec<u64>> = DURATION_GROUPS
        .iter()
        .map(|&(_, targets)| vec![0u64; targets.len()])
        .collect();

    let mut group_raw_samples: Vec<Vec<Vec<u64>>> = DURATION_GROUPS
        .iter()
        .map(|_| Vec::with_capacity(7))
        .collect();

    let mut shape: Vec<u64> = Vec::new();

    let max_window_hi = DURATION_GROUPS
        .iter()
        .flat_map(|&(_, targets)| targets.iter().map(|&t| t + WINDOW_HALF_WIDTH))
        .max()
        .unwrap();
    let mut window_table: Vec<Vec<(usize, usize)>> = vec![Vec::new(); max_window_hi as usize + 1];
    for (gi, &(_, targets)) in DURATION_GROUPS.iter().enumerate() {
        for (i, &target) in targets.iter().enumerate() {
            let lo = target.saturating_sub(WINDOW_HALF_WIDTH);
            for d in lo..=(target + WINDOW_HALF_WIDTH) {
                window_table[d as usize].push((gi, i));
            }
        }
    }

    for _step in 0..steps {
        let (_size, duration) = system.update_with_avalanche_shape(&mut shape);
        if let Some(entries) = window_table.get(duration as usize) {
            for &(gi, i) in entries {
                let target = DURATION_GROUPS[gi].1[i];
                group_counts[gi][i] += 1;
                let used = (target as usize).min(duration as usize);
                for t in 0..used {
                    group_sums[gi][i][t] += shape[t] as f64;
                }
            }
        }
        for (gi, &(_, targets)) in DURATION_GROUPS.iter().enumerate() {
            if group_raw_samples[gi].len() >= 7 {
                continue;
            }
            if duration == targets[0] {
                let len = targets[0] as usize;
                group_raw_samples[gi].push(shape[..len].to_vec());
            }
        }
        pb.inc(1);
    }

    let summary = system.simplified_format();

    for (gi, &(label, targets)) in DURATION_GROUPS.iter().enumerate() {
        let sums = &group_sums[gi];
        let counts = &group_counts[gi];

        let avg_shapes: Vec<Vec<f64>> = sums
            .iter()
            .zip(counts.iter())
            .map(|(s, &c)| {
                if c == 0 {
                    vec![0.0; s.len()]
                } else {
                    let denom = c as f64;
                    s.iter().map(|x| x / denom).collect()
                }
            })
            .collect();

        for (i, &target) in targets.iter().enumerate() {
            if counts[i] == 0 {
                continue;
            }
            let pairs: Vec<(f64, f64)> = avg_shapes[i]
                .iter()
                .enumerate()
                .map(|(k, &s)| (k as f64 + 1.0, s))
                .collect();
            let dat_path = system.get_dat_file_path(
                &format!("shape_{}_T{}", label, target),
                init_steps,
                steps,
            );
            io::write_v_f64_pair(&dat_path, &pairs).unwrap();
        }

        let mut raw_plot = MultiLinePlot::new()
            .title(format!("Average avalanche shape [{}] ({})", label, summary))
            .x_label("t")
            .y_label("<s(t,T)>")
            .axis_scale(AxisScale::Linear)
            .line_width(2)
            .title_font_size(24);
        for (i, &target) in targets.iter().enumerate() {
            if counts[i] == 0 {
                continue;
            }
            let data: Vec<(f64, f64)> = avg_shapes[i]
                .iter()
                .enumerate()
                .map(|(k, &s)| (k as f64 + 1.0, s))
                .collect();
            let legend = format!("T = {} (N = {})", target, counts[i]);
            raw_plot = raw_plot.add_series(data, legend);
        }
        let raw_png = system.get_png_file_path(&format!("shape_raw_{}", label), init_steps, steps);
        raw_plot.save(&raw_png).unwrap();

        let samples = &group_raw_samples[gi];
        if samples.len() < 7 {
            eprintln!(
                "cal_shape [{}]: only {} raw sample(s) captured at T = {} (need 7); skipping samples-vs-average plot.",
                label,
                samples.len(),
                targets[0]
            );
        } else {
            let mut samples_plot = MultiLinePlot::new()
                .title(format!(
                    "Individual samples vs average at T = {} [{}] ({})",
                    targets[0], label, summary
                ))
                .x_label("t")
                .y_label("s(t,T)")
                .axis_scale(AxisScale::Linear)
                .line_width(2)
                .title_font_size(22);
            for (k, sample) in samples.iter().enumerate() {
                let data: Vec<(f64, f64)> = sample
                    .iter()
                    .enumerate()
                    .map(|(t, &v)| (t as f64 + 1.0, v as f64))
                    .collect();
                samples_plot = samples_plot.add_series(data, format!("sample {}", k + 1));
            }
            let avg_data: Vec<(f64, f64)> = avg_shapes[0]
                .iter()
                .enumerate()
                .map(|(t, &v)| (t as f64 + 1.0, v))
                .collect();
            samples_plot = samples_plot.add_series(avg_data, "average".to_string());
            let samples_png = system.get_png_file_path(
                &format!("shape_samples_vs_average_{}", label),
                init_steps,
                steps,
            );
            samples_plot.save(&samples_png).unwrap();
        }

        let populated = counts.iter().filter(|&&c| c > 0).count();
        if populated < 2 {
            eprintln!(
                "cal_shape [{}]: only {} populated window(s); skipping collapse/fit.",
                label, populated
            );
            continue;
        }

        let gamma = find_optimal_gamma(&avg_shapes, targets, counts);
        let (_c_amp, a_asym) = fit_amplitude_and_asymmetry(&avg_shapes, targets, counts, gamma);

        let mut collapse_plot = MultiLinePlot::new()
            .title(format!(
                "Collapsed avalanche shape [{}] ({}, gamma = {:.3}, A = {:.3})",
                label, summary, gamma, a_asym
            ))
            .x_label("t / T")
            .y_label("<s(t,T)> / T^(gamma-1)")
            .axis_scale(AxisScale::Linear)
            .line_width(2)
            .title_font_size(22);
        for (i, &target) in targets.iter().enumerate() {
            if counts[i] == 0 {
                continue;
            }
            let (xs, ys) = collapsed_curve(&avg_shapes[i], target, gamma);
            let data: Vec<(f64, f64)> = xs.into_iter().zip(ys).collect();
            collapse_plot = collapse_plot.add_series(data, format!("T = {}", target));
        }
        let collapse_png =
            system.get_png_file_path(&format!("shape_collapsed_{}", label), init_steps, steps);
        collapse_plot.save(&collapse_png).unwrap();
    }
}

pub fn calculate_shape(param: &Parameters, pb: &ProgressBar, init_steps: u64, steps: u64) {
    let mut system = System::new(param.clone());
    if init_steps == 0 {
        system.init();
    } else {
        system.load(init_steps).unwrap();
    }

    calculate(&mut system, pb, init_steps, steps);

    system.save(init_steps + steps).unwrap();
}
