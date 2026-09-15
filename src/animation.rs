#![allow(dead_code)]

use std::error::Error;
use std::path::Path;

use plotters::coord::Shift;
use plotters::prelude::*;

const IMG_WIDTH: u32 = 800;
const IMG_HEIGHT: u32 = 800;
const FONT_FAMILY: &str = "sans-serif";

pub struct Heatmap {
    frames: Vec<Vec<f64>>,
    system_size: usize,
    title: Option<String>,
    title_font_size: u32,
    frame_delay_ms: u32,
    value_range: (f64, f64),
    step_info: Option<(u64, u64)>,
}

impl Default for Heatmap {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            system_size: 0,
            title: None,
            title_font_size: 28,
            frame_delay_ms: 100,
            value_range: (0.0, 1.0),
            step_info: None,
        }
    }
}

impl Heatmap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn frames(mut self, frames: Vec<Vec<f64>>) -> Self {
        self.frames = frames;
        self
    }

    pub fn add_frame(mut self, frame: Vec<f64>) -> Self {
        self.frames.push(frame);
        self
    }

    pub fn system_size(mut self, n: usize) -> Self {
        self.system_size = n;
        self
    }

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn title_font_size(mut self, n: u32) -> Self {
        self.title_font_size = n;
        self
    }

    pub fn frame_delay_ms(mut self, ms: u32) -> Self {
        self.frame_delay_ms = ms;
        self
    }

    pub fn value_range(mut self, lo: f64, hi: f64) -> Self {
        self.value_range = (lo, hi);
        self
    }

    pub fn step_info(mut self, init_steps: u64, step_per_frame: u64) -> Self {
        self.step_info = Some((init_steps, step_per_frame));
        self
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        if self.frames.is_empty() {
            return Err("Heatmap::save: no frames provided".into());
        }
        if self.system_size == 0 {
            return Err("Heatmap::save: system_size must be > 0".into());
        }
        let n = self.system_size;
        let expected = n * n;
        for (i, frame) in self.frames.iter().enumerate() {
            if frame.len() != expected {
                return Err(format!(
                    "Heatmap::save: frame {} has {} elements; expected system_size^2 = {}",
                    i,
                    frame.len(),
                    expected
                )
                .into());
            }
        }

        let (vmin, vmax) = self.value_range;
        if !(vmin < vmax) {
            return Err(
                format!("Heatmap::save: value_range lo must be < hi (got {vmin}..{vmax})").into(),
            );
        }

        let base_title = self.title.as_deref().unwrap_or("");

        let root = BitMapBackend::gif(p, (IMG_WIDTH, IMG_HEIGHT), self.frame_delay_ms)?
            .into_drawing_area();

        for (frame_idx, frame) in self.frames.iter().enumerate() {
            let caption = match self.step_info {
                Some((init_steps, step_per_frame)) => {
                    let cur = init_steps + (frame_idx as u64) * step_per_frame;
                    format!("{} step={}", base_title, cur)
                }
                None => base_title.to_string(),
            };

            draw_frame(
                &root,
                frame,
                &caption,
                n,
                (vmin, vmax),
                self.title_font_size,
            )?;
        }

        Ok(())
    }

    pub fn open_stream<'a, P: AsRef<Path> + ?Sized>(
        &self,
        path: &'a P,
    ) -> Result<HeatmapStream<'a>, Box<dyn Error>> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        if self.system_size == 0 {
            return Err("Heatmap::open_stream: system_size must be > 0".into());
        }
        let (vmin, vmax) = self.value_range;
        if !(vmin < vmax) {
            return Err(format!(
                "Heatmap::open_stream: value_range lo must be < hi (got {vmin}..{vmax})"
            )
            .into());
        }

        let root = BitMapBackend::gif(p, (IMG_WIDTH, IMG_HEIGHT), self.frame_delay_ms)?
            .into_drawing_area();

        Ok(HeatmapStream {
            root,
            system_size: self.system_size,
            base_title: self.title.clone().unwrap_or_default(),
            title_font_size: self.title_font_size,
            value_range: self.value_range,
            step_info: self.step_info,
            frame_idx: 0,
        })
    }
}

pub struct HeatmapStream<'a> {
    root: DrawingArea<BitMapBackend<'a>, Shift>,
    system_size: usize,
    base_title: String,
    title_font_size: u32,
    value_range: (f64, f64),
    step_info: Option<(u64, u64)>,
    frame_idx: usize,
}

impl HeatmapStream<'_> {
    pub fn push_frame(&mut self, frame: &[f64]) -> Result<(), Box<dyn Error>> {
        let n = self.system_size;
        let expected = n * n;
        if frame.len() != expected {
            return Err(format!(
                "HeatmapStream::push_frame: frame {} has {} elements; expected system_size^2 = {}",
                self.frame_idx,
                frame.len(),
                expected
            )
            .into());
        }

        let caption = match self.step_info {
            Some((init_steps, step_per_frame)) => {
                let cur = init_steps + (self.frame_idx as u64) * step_per_frame;
                format!("{} step={}", self.base_title, cur)
            }
            None => self.base_title.clone(),
        };

        draw_frame(
            &self.root,
            frame,
            &caption,
            n,
            self.value_range,
            self.title_font_size,
        )?;
        self.frame_idx += 1;
        Ok(())
    }
}

fn draw_frame(
    root: &DrawingArea<BitMapBackend<'_>, Shift>,
    frame: &[f64],
    caption: &str,
    n: usize,
    (vmin, vmax): (f64, f64),
    title_font_size: u32,
) -> Result<(), Box<dyn Error>> {
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(root)
        .caption(caption, (FONT_FAMILY, title_font_size))
        .margin(20)
        .build_cartesian_2d(0i32..(n as i32), (n as i32)..0i32)?;

    chart.draw_series(frame.iter().enumerate().map(|(idx, &v)| {
        let x = (idx % n) as i32;
        let y = (idx / n) as i32;
        let t = ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0);
        let (r, g, b) = viridis(t);
        Rectangle::new([(x, y), (x + 1, y + 1)], RGBColor(r, g, b).filled())
    }))?;

    root.present()?;
    Ok(())
}

/// Approximate viridis colormap. Input `t` in `[0, 1]` -> `(R, G, B)`.
fn viridis(t: f64) -> (u8, u8, u8) {
    const STOPS: [(f64, f64, f64); 9] = [
        (1.000000, 0.968627, 0.952941),
        (0.992157, 0.878431, 0.866667),
        (0.988235, 0.772549, 0.752941),
        (0.980392, 0.623529, 0.709804),
        (0.968627, 0.407843, 0.631373),
        (0.866667, 0.203922, 0.592157),
        (0.682353, 0.003922, 0.494118),
        (0.478431, 0.003922, 0.466667),
        (0.286275, 0.000000, 0.415686),
    ];
    let t = t.clamp(0.0, 1.0);
    let last = STOPS.len() - 1;
    let scaled = t * last as f64;
    let i = (scaled as usize).min(last - 1);
    let f = scaled - i as f64;
    let (r0, g0, b0) = STOPS[i];
    let (r1, g1, b1) = STOPS[i + 1];
    let r = r0 + f * (r1 - r0);
    let g = g0 + f * (g1 - g0);
    let b = b0 + f * (b1 - b0);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
