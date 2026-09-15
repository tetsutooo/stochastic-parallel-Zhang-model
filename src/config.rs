use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::core::Parameters;

#[derive(Debug, Deserialize)]
struct RawConfig {
    init_steps: u64,
    steps: u64,
    parameters: Vec<RawParameters>,
}

#[derive(Debug, Deserialize)]
struct RawParameters {
    system_size: usize,
    sigma: f64,
    q: f64,
}

pub struct Config {
    pub init_steps: u64,
    pub steps: u64,
    pub parameters: Vec<Parameters>,
    //pub config_name: String,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("failed to read config '{}': {}", path, e))?;
        let raw: RawConfig =
            toml::from_str(&contents).map_err(|e| format!("failed to parse TOML: {}", e))?;

        let config_name = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("invalid config file path: {}", path))?
            .to_string();

        let parameters: Vec<Parameters> = raw
            .parameters
            .into_iter()
            .map(|p| Parameters {
                system_size: p.system_size,
                sigma: p.sigma,
                q: p.q,
                config_name: config_name.clone(),
            })
            .collect();

        let cfg = Config {
            init_steps: raw.init_steps,
            steps: raw.steps,
            parameters,
            //config_name,
        };
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), String> {
        if self.steps == 0 {
            return Err("steps must be greater than 0".to_string());
        }
        if self.parameters.is_empty() {
            return Err("at least one [[parameters]] block is required".to_string());
        }
        for (i, p) in self.parameters.iter().enumerate() {
            if p.system_size == 0 {
                return Err(format!("parameters[{}]: system_size must be > 0", i));
            }
            if !p.system_size.is_power_of_two() {
                return Err(format!(
                    "parameters[{}]: system_size = {} must be a power of 2",
                    i, p.system_size
                ));
            }
            if !p.sigma.is_finite() || p.sigma < 0.0 {
                return Err(format!(
                    "parameters[{}]: sigma must be a non-negative finite number",
                    i
                ));
            }
            if !p.q.is_finite() || !(0.0..1.0).contains(&p.q) {
                return Err(format!(
                    "parameters[{}]: q must be a finite number in [0.0, 1.0) (got {})",
                    i, p.q
                ));
            }
        }
        Ok(())
    }
}
