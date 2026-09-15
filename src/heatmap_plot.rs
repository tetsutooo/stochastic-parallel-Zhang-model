#![allow(dead_code)]

use std::error::Error;
use std::path::Path;

use plotters::coord::Shift;
use plotters::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisScale {
    Linear,
    LogX,
    LogY,
    LogLog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueScale {
    Linear,
    Log,
}

const IMG_WIDTH: u32 = 960;
const IMG_HEIGHT: u32 = 720;
const FONT_FAMILY: &str = "sans-serif";
const CURVE_RGB: (u8, u8, u8) = (0, 230, 230);
const CBAR_AREA_WIDTH: u32 = 110;

pub struct HeatmapPlot {
    data: Vec<(f64, f64, f64)>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    axis_scale: AxisScale,
    x_range: Option<(f64, f64)>,
    y_range: Option<(f64, f64)>,
    value_range: Option<(f64, f64)>,
    value_scale: ValueScale,
    title_font_size: u32,
    label_font_size: u32,
    tick_font_size: u32,
    curve: Option<(f64, f64)>,
    curve_width: u32,
    legend: Option<String>,
}

impl Default for HeatmapPlot {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            title: None,
            x_label: None,
            y_label: None,
            axis_scale: AxisScale::Linear,
            x_range: None,
            y_range: None,
            value_range: None,
            value_scale: ValueScale::Linear,
            title_font_size: 28,
            label_font_size: 20,
            tick_font_size: 14,
            curve: None,
            curve_width: 1,
            legend: None,
        }
    }
}

impl HeatmapPlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data(mut self, data: Vec<(f64, f64, f64)>) -> Self {
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

    pub fn value_range(mut self, lo: f64, hi: f64) -> Self {
        self.value_range = Some((lo, hi));
        self
    }

    pub fn value_scale(mut self, s: ValueScale) -> Self {
        self.value_scale = s;
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

    /// Overlay `y = a * x^b`. If unset, no curve is drawn.
    pub fn power_curve(mut self, a: f64, b: f64) -> Self {
        self.curve = Some((a, b));
        self
    }

    pub fn curve_width(mut self, w: u32) -> Self {
        self.curve_width = w;
        self
    }

    pub fn legend<S: Into<String>>(mut self, s: S) -> Self {
        self.legend = Some(s.into());
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

        let filtered: Vec<(f64, f64, f64)> = self
            .data
            .iter()
            .copied()
            .filter(|(x, y, v)| {
                x.is_finite()
                    && y.is_finite()
                    && v.is_finite()
                    && (!need_x_pos || *x - 0.5 > 0.0)
                    && (!need_y_pos || *y - 0.5 > 0.0)
            })
            .collect();

        if filtered.is_empty() {
            return Err("HeatmapPlot::save: no plottable cells for this axis scale".into());
        }

        let (x_lo, x_hi) = resolve_axis_range(
            self.x_range,
            filtered.iter().map(|&(x, _, _)| x),
            need_x_pos,
            "x",
        )?;
        let (y_lo, y_hi) = resolve_axis_range(
            self.y_range,
            filtered.iter().map(|&(_, y, _)| y),
            need_y_pos,
            "y",
        )?;

        let value_log = self.value_scale == ValueScale::Log;

        let (vmin, vmax) = match self.value_range {
            Some((lo, hi)) => {
                if !(lo < hi) {
                    return Err(format!(
                        "HeatmapPlot::save: value_range lo must be < hi (got {lo}..{hi})"
                    )
                    .into());
                }
                if value_log && lo <= 0.0 {
                    return Err(format!(
                        "HeatmapPlot::save: value_range lo must be > 0 on a log value scale (got {lo})"
                    )
                    .into());
                }
                (lo, hi)
            }
            None if value_log => {
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for &(_, _, v) in &filtered {
                    if v > 0.0 {
                        if v < lo {
                            lo = v;
                        }
                        if v > hi {
                            hi = v;
                        }
                    }
                }
                if !lo.is_finite() || !hi.is_finite() {
                    return Err(
                        "HeatmapPlot::save: no positive cell values for log value scale".into(),
                    );
                }
                if !(lo < hi) {
                    hi = lo * 10.0;
                }
                (lo, hi)
            }
            None => {
                let mut lo = 0.0_f64;
                let mut hi = f64::NEG_INFINITY;
                for &(_, _, v) in &filtered {
                    if v < lo {
                        lo = v;
                    }
                    if v > hi {
                        hi = v;
                    }
                }
                if !(lo < hi) {
                    hi = lo + 1.0;
                }
                (lo, hi)
            }
        };

        let root = BitMapBackend::new(p, (IMG_WIDTH, IMG_HEIGHT)).into_drawing_area();
        root.fill(&WHITE)?;
        let (chart_area, cbar_area) = root.split_horizontally(IMG_WIDTH - CBAR_AREA_WIDTH);

        let title = self.title.as_deref().unwrap_or("");
        let x_label = self.x_label.as_deref().unwrap_or("");
        let y_label = self.y_label.as_deref().unwrap_or("");
        let curve_color = RGBColor(CURVE_RGB.0, CURVE_RGB.1, CURVE_RGB.2);
        let curve_w = self.curve_width;

        macro_rules! finalize {
            ($chart:expr, $log_x:expr) => {{
                let chart = &mut $chart;

                {
                    let (bg_r, bg_g, bg_b) = black_red_yellow(0.0);
                    chart.draw_series(std::iter::once(Rectangle::new(
                        [(x_lo, y_lo), (x_hi, y_hi)],
                        RGBColor(bg_r, bg_g, bg_b).filled(),
                    )))?;
                }

                chart.draw_series(filtered.iter().map(|&(x, y, v)| {
                    let t = if vmax > vmin {
                        if value_log {
                            if v > 0.0 {
                                ((v.ln() - vmin.ln()) / (vmax.ln() - vmin.ln())).clamp(0.0, 1.0)
                            } else {
                                0.0
                            }
                        } else {
                            ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
                        }
                    } else {
                        0.0
                    };
                    let (r, g, b) = black_red_yellow(t);
                    Rectangle::new(
                        [(x - 0.5, y - 0.5), (x + 0.5, y + 0.5)],
                        RGBColor(r, g, b).filled(),
                    )
                }))?;

                if let Some((a, b)) = self.curve {
                    let samples = power_curve_samples(a, b, x_lo, x_hi, $log_x, 400);
                    let pl_filtered: Vec<(f64, f64)> = samples
                        .into_iter()
                        .filter(|(x, y)| {
                            x.is_finite()
                                && y.is_finite()
                                && (!need_x_pos || *x > 0.0)
                                && (!need_y_pos || *y > 0.0)
                                && *x >= x_lo
                                && *x <= x_hi
                                && *y >= y_lo
                                && *y <= y_hi
                        })
                        .collect();
                    let line_style = ShapeStyle::from(&curve_color).stroke_width(curve_w);
                    let series = chart.draw_series(LineSeries::new(pl_filtered, line_style))?;
                    let label = self
                        .legend
                        .clone()
                        .unwrap_or_else(|| format!("y = {:.3} * x^{{{:.3}}}", a, b));
                    series.label(label).legend(move |(x, y)| {
                        PathElement::new(
                            vec![(x, y), (x + 20, y)],
                            ShapeStyle::from(&curve_color).stroke_width(curve_w),
                        )
                    });

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
                let mut chart = ChartBuilder::on(&chart_area)
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
                let mut chart = ChartBuilder::on(&chart_area)
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
                let mut chart = ChartBuilder::on(&chart_area)
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
                let mut chart = ChartBuilder::on(&chart_area)
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

        draw_colorbar(
            &cbar_area,
            vmin,
            vmax,
            value_log,
            self.title_font_size,
            self.tick_font_size,
        )?;

        root.present()?;
        Ok(())
    }
}

fn draw_colorbar<DB>(
    cbar_area: &DrawingArea<DB, Shift>,
    vmin: f64,
    vmax: f64,
    log: bool,
    title_font_size: u32,
    tick_font_size: u32,
) -> Result<(), Box<dyn Error>>
where
    DB: DrawingBackend,
    DB::ErrorType: 'static,
{
    let (_, cb_h) = cbar_area.dim_in_pixel();

    let strip_x0: i32 = 10;
    let strip_x1: i32 = 40;
    let strip_y0: i32 = (20 + title_font_size as i32) + 10;
    let strip_y1: i32 = cb_h as i32 - (20 + 60);
    let strip_height = (strip_y1 - strip_y0).max(1);

    let n_strips: i32 = 200;
    for i in 0..n_strips {
        let y_lo = strip_y0 + i * strip_height / n_strips;
        let y_hi = strip_y0 + (i + 1) * strip_height / n_strips;
        let t = 1.0 - (i as f64 + 0.5) / n_strips as f64;
        let (r, g, b) = black_red_yellow(t);
        cbar_area.draw(&Rectangle::new(
            [(strip_x0, y_lo), (strip_x1, y_hi)],
            RGBColor(r, g, b).filled(),
        ))?;
    }

    cbar_area.draw(&Rectangle::new(
        [(strip_x0, strip_y0), (strip_x1, strip_y1)],
        ShapeStyle::from(&BLACK).stroke_width(1),
    ))?;

    let n_ticks: i32 = 5;
    let label_style: TextStyle =
        TextStyle::from((FONT_FAMILY, tick_font_size).into_font()).color(&BLACK);
    for i in 0..=n_ticks {
        let t = i as f64 / n_ticks as f64;

        let value = if log {
            vmax * (vmin / vmax).powf(t)
        } else {
            vmax - t * (vmax - vmin)
        };
        let y = strip_y0 + (t * strip_height as f64).round() as i32;

        cbar_area.draw(&PathElement::new(
            vec![(strip_x1, y), (strip_x1 + 4, y)],
            ShapeStyle::from(&BLACK).stroke_width(1),
        ))?;

        let label = if log {
            format!("{:.1e}", value)
        } else {
            format!("{:.1}", value)
        };
        cbar_area.draw_text(
            &label,
            &label_style,
            (strip_x1 + 8, y - (tick_font_size as i32) / 2),
        )?;
    }

    Ok(())
}

fn black_red_yellow(t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        let s = t * 2.0;
        let r = (s * 255.0).round() as u8;
        (r, 0, 0)
    } else {
        let s = (t - 0.5) * 2.0;
        let g = (s * 255.0).round() as u8;
        (255, g, 0)
    }
}

fn power_curve_samples(
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

fn resolve_axis_range<I: Iterator<Item = f64>>(
    user: Option<(f64, f64)>,
    values: I,
    log: bool,
    axis: &str,
) -> Result<(f64, f64), Box<dyn Error>> {
    if let Some((lo, hi)) = user {
        if !(lo < hi) {
            return Err(format!(
                "HeatmapPlot::save: {axis}_range lo must be < hi (got {lo}..{hi})"
            )
            .into());
        }
        if log && lo <= 0.0 {
            return Err(format!(
                "HeatmapPlot::save: {axis}_range lo must be > 0 on a log axis (got {lo})"
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

    if log {
        let lo_pad = (lo - 0.5).max(lo * 0.5).max(1e-9);
        let hi_pad = hi + 0.5;
        Ok((lo_pad, hi_pad))
    } else {
        Ok((lo - 0.5, hi + 0.5))
    }
}
