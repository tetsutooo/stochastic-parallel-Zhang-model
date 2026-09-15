#![allow(dead_code)]

use std::error::Error;
use std::path::Path;

use plotters::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisScale {
    Linear,
    LogX,
    LogY,
    LogLog,
}

const PALETTE: [(u8, u8, u8); 5] = [
    (0x1b, 0x9e, 0x77),
    (0xd9, 0x5f, 0x02),
    (0x75, 0x70, 0xb3),
    (0xe7, 0x29, 0x8a),
    (0x66, 0xa6, 0x1e),
];

const IMG_WIDTH: u32 = 960;
const IMG_HEIGHT: u32 = 720;
const FONT_FAMILY: &str = "sans-serif";

pub struct LinePlot {
    data: Vec<(f64, f64)>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    line_width: u32,
    axis_scale: AxisScale,
    color: (u8, u8, u8),
    x_range: Option<(f64, f64)>,
    y_range: Option<(f64, f64)>,
    title_font_size: u32,
    label_font_size: u32,
    tick_font_size: u32,
    legend: Option<String>,
    power_law: Option<(f64, f64)>,
}

impl Default for LinePlot {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            title: None,
            x_label: None,
            y_label: None,
            line_width: 1,
            axis_scale: AxisScale::LogLog,
            color: PALETTE[0],
            x_range: None,
            y_range: None,
            title_font_size: 28,
            label_font_size: 20,
            tick_font_size: 14,
            legend: None,
            power_law: None,
        }
    }
}

impl LinePlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data(mut self, data: Vec<(f64, f64)>) -> Self {
        self.data = data;
        self
    }

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn x_label<S: Into<String>>(mut self, x_label: S) -> Self {
        self.x_label = Some(x_label.into());
        self
    }

    pub fn y_label<S: Into<String>>(mut self, y_label: S) -> Self {
        self.y_label = Some(y_label.into());
        self
    }

    pub fn line_width(mut self, w: u32) -> Self {
        self.line_width = w;
        self
    }

    pub fn axis_scale(mut self, s: AxisScale) -> Self {
        self.axis_scale = s;
        self
    }

    pub fn color(mut self, color_idx: usize) -> Self {
        self.color = PALETTE[color_idx % 5];
        self
    }

    pub fn x_range(mut self, lo: f64, hi: f64) -> Self {
        self.x_range = Some((lo, hi));
        self
    }

    pub fn y_range(mut self, lo: f64, hi: f64) -> Self {
        self.y_range = Some((lo, hi));
        self
    }

    pub fn title_font_size(mut self, n: u32) -> Self {
        self.title_font_size = n;
        self
    }

    pub fn label_font_size(mut self, n: u32) -> Self {
        self.label_font_size = n;
        self
    }

    pub fn tick_font_size(mut self, n: u32) -> Self {
        self.tick_font_size = n;
        self
    }

    pub fn legend<S: Into<String>>(mut self, s: S) -> Self {
        self.legend = Some(s.into());
        self
    }

    pub fn power_law(mut self, a: f64, b: f64) -> Self {
        self.power_law = Some((a, b));
        self
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let need_x_pos = matches!(self.axis_scale, AxisScale::LogX | AxisScale::LogLog);
        let need_y_pos = matches!(self.axis_scale, AxisScale::LogY | AxisScale::LogLog);

        let filtered: Vec<(f64, f64)> = self
            .data
            .iter()
            .copied()
            .filter(|(x, y)| {
                x.is_finite()
                    && y.is_finite()
                    && (!need_x_pos || *x > 0.0)
                    && (!need_y_pos || *y > 0.0)
            })
            .collect();

        if filtered.is_empty() {
            return Err("LinePlot::save: no plottable points for this axis scale".into());
        }

        let (x_lo, x_hi) = resolve_range(
            self.x_range,
            filtered.iter().map(|(x, _)| *x),
            need_x_pos,
            "x",
        )?;
        let (y_lo, y_hi) = resolve_range(
            self.y_range,
            filtered.iter().map(|(_, y)| *y),
            need_y_pos,
            "y",
        )?;

        let root = BitMapBackend::new(p, (IMG_WIDTH, IMG_HEIGHT)).into_drawing_area();
        root.fill(&WHITE)?;

        let title = self.title.as_deref().unwrap_or("");
        let x_label = self.x_label.as_deref().unwrap_or("");
        let y_label = self.y_label.as_deref().unwrap_or("");
        let rgb = RGBColor(self.color.0, self.color.1, self.color.2);
        let lw = self.line_width;
        let stroke = ShapeStyle::from(&rgb).stroke_width(lw);

        macro_rules! finalize {
            ($chart:expr, $log_x:expr) => {{
                let chart = &mut $chart;

                let series =
                    chart.draw_series(LineSeries::new(filtered.iter().copied(), stroke))?;
                if let Some(text) = self.legend.clone() {
                    series.label(text).legend(move |(x, y)| {
                        PathElement::new(
                            vec![(x, y), (x + 20, y)],
                            ShapeStyle::from(&rgb).stroke_width(lw),
                        )
                    });
                }

                if let Some((a, b)) = self.power_law {
                    let samples = power_law_samples(a, b, x_lo, x_hi, $log_x, 250);
                    let pl_filtered: Vec<(f64, f64)> = samples
                        .into_iter()
                        .filter(|(x, y)| {
                            x.is_finite()
                                && y.is_finite()
                                && (!need_x_pos || *x > 0.0)
                                && (!need_y_pos || *y > 0.0)
                        })
                        .collect();
                    let segments: Vec<Vec<(f64, f64)>> = pl_filtered
                        .chunks(4)
                        .filter_map(|c| {
                            if c.len() >= 2 {
                                Some(vec![c[0], c[1]])
                            } else {
                                None
                            }
                        })
                        .collect();
                    let dashed_style = ShapeStyle::from(&BLACK).stroke_width(lw);
                    chart
                        .draw_series(
                            segments
                                .into_iter()
                                .map(move |seg| PathElement::new(seg, dashed_style)),
                        )?
                        .label(format!("y = {:.3} * x^{{{:.3}}}", a, b))
                        .legend(move |(x, y)| {
                            PathElement::new(
                                vec![(x, y), (x + 6, y)],
                                ShapeStyle::from(&BLACK).stroke_width(lw),
                            )
                        });
                }

                if self.legend.is_some() || self.power_law.is_some() {
                    chart
                        .configure_series_labels()
                        .position(SeriesLabelPosition::UpperRight)
                        .background_style(WHITE.mix(0.85))
                        .border_style(BLACK.mix(0.5))
                        .label_font((FONT_FAMILY, self.label_font_size))
                        .draw()?;
                }
            }};
        }

        match self.axis_scale {
            AxisScale::Linear => {
                let mut chart = ChartBuilder::on(&root)
                    .caption(title, (FONT_FAMILY, self.title_font_size))
                    .margin(20)
                    .x_label_area_size(60)
                    .y_label_area_size(80)
                    .build_cartesian_2d(x_lo..x_hi, y_lo..y_hi)?;
                chart
                    .configure_mesh()
                    .x_desc(x_label)
                    .y_desc(y_label)
                    .axis_desc_style((FONT_FAMILY, self.label_font_size))
                    .label_style((FONT_FAMILY, self.tick_font_size))
                    .draw()?;
                finalize!(chart, false);
            }
            AxisScale::LogX => {
                let mut chart = ChartBuilder::on(&root)
                    .caption(title, (FONT_FAMILY, self.title_font_size))
                    .margin(20)
                    .x_label_area_size(60)
                    .y_label_area_size(80)
                    .build_cartesian_2d((x_lo..x_hi).log_scale(), y_lo..y_hi)?;
                chart
                    .configure_mesh()
                    .x_desc(x_label)
                    .y_desc(y_label)
                    .axis_desc_style((FONT_FAMILY, self.label_font_size))
                    .label_style((FONT_FAMILY, self.tick_font_size))
                    .draw()?;
                finalize!(chart, true);
            }
            AxisScale::LogY => {
                let mut chart = ChartBuilder::on(&root)
                    .caption(title, (FONT_FAMILY, self.title_font_size))
                    .margin(20)
                    .x_label_area_size(60)
                    .y_label_area_size(80)
                    .build_cartesian_2d(x_lo..x_hi, (y_lo..y_hi).log_scale())?;
                chart
                    .configure_mesh()
                    .x_desc(x_label)
                    .y_desc(y_label)
                    .axis_desc_style((FONT_FAMILY, self.label_font_size))
                    .label_style((FONT_FAMILY, self.tick_font_size))
                    .draw()?;
                finalize!(chart, false);
            }
            AxisScale::LogLog => {
                let mut chart = ChartBuilder::on(&root)
                    .caption(title, (FONT_FAMILY, self.title_font_size))
                    .margin(20)
                    .x_label_area_size(60)
                    .y_label_area_size(80)
                    .build_cartesian_2d((x_lo..x_hi).log_scale(), (y_lo..y_hi).log_scale())?;
                chart
                    .configure_mesh()
                    .x_desc(x_label)
                    .y_desc(y_label)
                    .axis_desc_style((FONT_FAMILY, self.label_font_size))
                    .label_style((FONT_FAMILY, self.tick_font_size))
                    .draw()?;
                finalize!(chart, true);
            }
        }

        root.present()?;
        Ok(())
    }
}

fn power_law_samples(
    a: f64,
    b: f64,
    x_lo: f64,
    x_hi: f64,
    log_x: bool,
    n: usize,
) -> Vec<(f64, f64)> {
    let n = n.max(2);
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            let x = if log_x && x_lo > 0.0 && x_hi > 0.0 {
                x_lo * (x_hi / x_lo).powf(t)
            } else {
                x_lo + t * (x_hi - x_lo)
            };
            (x, a * x.powf(b))
        })
        .collect()
}

fn resolve_range<I: Iterator<Item = f64>>(
    user: Option<(f64, f64)>,
    values: I,
    log: bool,
    axis: &str,
) -> Result<(f64, f64), Box<dyn Error>> {
    if let Some((lo, hi)) = user {
        if !(lo < hi) {
            return Err(
                format!("LinePlot::save: {axis}_range lo must be < hi (got {lo}..{hi})").into(),
            );
        }
        if log && lo <= 0.0 {
            return Err(format!(
                "LinePlot::save: {axis}_range lo must be > 0 on a log axis (got {lo})"
            )
            .into());
        }
        return Ok((lo, hi));
    }
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in values {
        if v < lo {
            lo = v;
        }
        if v > hi {
            hi = v;
        }
    }
    if lo == hi {
        if log {
            lo *= 0.9;
            hi *= 1.1;
        } else {
            lo -= 1.0;
            hi += 1.0;
        }
    }
    Ok((lo, hi))
}
