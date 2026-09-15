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

pub const MAX_SERIES: usize = 8;

const PALETTE: [(u8, u8, u8); MAX_SERIES] = [
    (0x1f, 0x77, 0xb4), // blue
    (0xff, 0x7f, 0x0e), // orange
    (0x2c, 0xa0, 0x2c), // green
    (0xd6, 0x27, 0x28), // red
    (0x94, 0x67, 0xbd), // purple
    (0x8c, 0x56, 0x4b), // brown
    (0xe3, 0x77, 0xc2), // pink
    (0x7f, 0x7f, 0x7f), // gray
];

const IMG_WIDTH: u32 = 960;
const IMG_HEIGHT: u32 = 720;
const FONT_FAMILY: &str = "sans-serif";

struct Series {
    data: Vec<(f64, f64)>,
    legend: String,
}

pub struct MultiLinePlot {
    series: Vec<Series>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    line_width: u32,
    axis_scale: AxisScale,
    x_range: Option<(f64, f64)>,
    y_range: Option<(f64, f64)>,
    title_font_size: u32,
    label_font_size: u32,
    tick_font_size: u32,
}

impl Default for MultiLinePlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            title: None,
            x_label: None,
            y_label: None,
            line_width: 1,
            axis_scale: AxisScale::LogLog,
            x_range: None,
            y_range: None,
            title_font_size: 28,
            label_font_size: 20,
            tick_font_size: 14,
        }
    }
}

impl MultiLinePlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_series<S: Into<String>>(mut self, data: Vec<(f64, f64)>, legend: S) -> Self {
        self.series.push(Series {
            data,
            legend: legend.into(),
        });
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

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        if self.series.is_empty() {
            return Err("MultiLinePlot::save: no series to plot".into());
        }
        if self.series.len() > MAX_SERIES {
            return Err(format!(
                "MultiLinePlot::save: at most {MAX_SERIES} series supported (got {})",
                self.series.len()
            )
            .into());
        }

        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let need_x_pos = matches!(self.axis_scale, AxisScale::LogX | AxisScale::LogLog);
        let need_y_pos = matches!(self.axis_scale, AxisScale::LogY | AxisScale::LogLog);

        let filtered: Vec<Vec<(f64, f64)>> = self
            .series
            .iter()
            .map(|s| {
                s.data
                    .iter()
                    .copied()
                    .filter(|(x, y)| {
                        x.is_finite()
                            && y.is_finite()
                            && (!need_x_pos || *x > 0.0)
                            && (!need_y_pos || *y > 0.0)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        if filtered.iter().all(|s| s.is_empty()) {
            return Err(
                "MultiLinePlot::save: no plottable points across any series for this axis scale"
                    .into(),
            );
        }

        let (x_lo, x_hi) = resolve_range(
            self.x_range,
            filtered.iter().flat_map(|s| s.iter().map(|(x, _)| *x)),
            need_x_pos,
            "x",
        )?;
        let (y_lo, y_hi) = resolve_range(
            self.y_range,
            filtered.iter().flat_map(|s| s.iter().map(|(_, y)| *y)),
            need_y_pos,
            "y",
        )?;

        let root = BitMapBackend::new(p, (IMG_WIDTH, IMG_HEIGHT)).into_drawing_area();
        root.fill(&WHITE)?;

        let title = self.title.as_deref().unwrap_or("");
        let x_label = self.x_label.as_deref().unwrap_or("");
        let y_label = self.y_label.as_deref().unwrap_or("");
        let lw = self.line_width;

        macro_rules! finalize {
            ($chart:expr) => {{
                let chart = &mut $chart;
                for (i, pts) in filtered.iter().enumerate() {
                    if pts.is_empty() {
                        continue;
                    }
                    let (r, g, b) = PALETTE[i];
                    let rgb = RGBColor(r, g, b);
                    let stroke = ShapeStyle::from(&rgb).stroke_width(lw);
                    let label = self.series[i].legend.clone();
                    chart
                        .draw_series(LineSeries::new(pts.iter().copied(), stroke))?
                        .label(label)
                        .legend(move |(x, y)| {
                            PathElement::new(
                                vec![(x, y), (x + 20, y)],
                                ShapeStyle::from(&rgb).stroke_width(lw),
                            )
                        });
                }

                chart
                    .configure_series_labels()
                    .position(SeriesLabelPosition::UpperRight)
                    .background_style(WHITE.mix(0.85))
                    .border_style(BLACK.mix(0.5))
                    .label_font((FONT_FAMILY, self.label_font_size))
                    .draw()?;
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
                finalize!(chart);
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
                finalize!(chart);
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
                finalize!(chart);
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
                finalize!(chart);
            }
        }

        root.present()?;
        Ok(())
    }
}

fn resolve_range<I: Iterator<Item = f64>>(
    user: Option<(f64, f64)>,
    values: I,
    log: bool,
    axis: &str,
) -> Result<(f64, f64), Box<dyn Error>> {
    if let Some((lo, hi)) = user {
        if !(lo < hi) {
            return Err(format!(
                "MultiLinePlot::save: {axis}_range lo must be < hi (got {lo}..{hi})"
            )
            .into());
        }
        if log && lo <= 0.0 {
            return Err(format!(
                "MultiLinePlot::save: {axis}_range lo must be > 0 on a log axis (got {lo})"
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
    if !lo.is_finite() || !hi.is_finite() {
        return Err(format!("MultiLinePlot::save: no finite values to infer {axis} range").into());
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
