use indicatif::MultiProgress;

mod animation;
mod calculate_shape;
mod calculate_size_and_duration;
mod config;
mod core;
mod heatmap_plot;
mod io;
mod line_plot;
mod make_animation;
mod multi_line_plot;
mod network;
mod prep;
mod progress;
mod rng;

#[derive(Clone, Copy)]
enum ParamKey {
    Sigma,
    Q,
    SystemSize,
}

impl ParamKey {
    const ALL: [ParamKey; 3] = [ParamKey::Sigma, ParamKey::Q, ParamKey::SystemSize];

    fn varies(self, params: &[core::Parameters]) -> bool {
        if params.len() < 2 {
            return false;
        }
        let p0 = &params[0];
        match self {
            ParamKey::Sigma => params.iter().any(|p| p.sigma != p0.sigma),
            ParamKey::Q => params.iter().any(|p| p.q != p0.q),
            ParamKey::SystemSize => params.iter().any(|p| p.system_size != p0.system_size),
        }
    }

    fn format_prefix(self, p: &core::Parameters) -> String {
        match self {
            ParamKey::Sigma => format!("sigma = {:.4}", p.sigma),
            ParamKey::Q => format!("q = {:.4}", p.q),
            ParamKey::SystemSize => format!("system_size = {}", p.system_size),
        }
    }
}

fn pick_representative(params: &[core::Parameters]) -> ParamKey {
    ParamKey::ALL
        .into_iter()
        .find(|k| k.varies(params))
        .unwrap_or(ParamKey::Sigma)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config.toml>", args[0]);
        eprintln!("Example: cargo run --release -- configs/example.toml");
        std::process::exit(1);
    }

    let cfg_path = &args[1];
    let cfg = match config::Config::load_from_file(cfg_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            std::process::exit(1);
        }
    };

    let representative = pick_representative(&cfg.parameters);

    let prefixes: Vec<String> = cfg
        .parameters
        .iter()
        .map(|p| representative.format_prefix(p))
        .collect();

    println!("init_steps = {:e}, steps = {:e}", cfg.init_steps, cfg.steps);
    println!("読み込んだパラメータ ({} 組):", cfg.parameters.len());
    for (i, (param, prefix)) in cfg.parameters.iter().zip(prefixes.iter()).enumerate() {
        println!("  [{}] {}", i, prefix);
        println!("       {}", param.display_format());
    }

    println!("mode を選択してください。");
    println!("0 => make_animation");
    println!("1 => prep_calculation");
    println!("2 => calculate_size_and_duration");
    println!("3 => calculate_shape");
    println!("12 => prep_calculation + calculate_size_and_duration");
    println!("13 => prep_calculation + calculate_shape");
    let mut mode = String::new();
    std::io::stdin().read_line(&mut mode).ok();
    let mode: u8 = mode.trim().parse().ok().unwrap();

    match mode {
        0 => {
            let multi = MultiProgress::new();
            std::thread::scope(|s| {
                for (param, prefix) in cfg.parameters.iter().zip(prefixes.into_iter()) {
                    let pb = progress::new_multi_progressbar(&multi, cfg.steps, prefix);
                    let init_steps = cfg.init_steps;
                    let steps = cfg.steps;
                    s.spawn(move || {
                        make_animation::calculate_anim(param, &pb, init_steps, steps);
                        pb.finish();
                    });
                }
            });
        }
        1 => {
            let multi = MultiProgress::new();
            std::thread::scope(|s| {
                for (param, prefix) in cfg.parameters.iter().zip(prefixes.into_iter()) {
                    let pb = progress::new_multi_progressbar(&multi, cfg.steps, prefix);
                    let init_steps = cfg.init_steps;
                    let steps = cfg.steps;
                    s.spawn(move || {
                        prep::prep_calculate(param, &pb, init_steps, steps);
                        pb.finish();
                    });
                }
            });
        }
        2 => {
            let multi = MultiProgress::new();
            std::thread::scope(|s| {
                for (param, prefix) in cfg.parameters.iter().zip(prefixes.into_iter()) {
                    let pb = progress::new_multi_progressbar(&multi, cfg.steps, prefix);
                    let init_steps = cfg.init_steps;
                    let steps = cfg.steps;
                    s.spawn(move || {
                        calculate_size_and_duration::calculate1(param, &pb, init_steps, steps);
                        pb.finish();
                    });
                }
            });
        }

        3 => {
            let multi = MultiProgress::new();
            std::thread::scope(|s| {
                for (param, prefix) in cfg.parameters.iter().zip(prefixes.into_iter()) {
                    let pb = progress::new_multi_progressbar(&multi, cfg.steps, prefix);
                    let init_steps = cfg.init_steps;
                    let steps = cfg.steps;
                    s.spawn(move || {
                        calculate_shape::calculate_shape(param, &pb, init_steps, steps);
                        pb.finish();
                    });
                }
            });
        }
        12 => {
            let multi = MultiProgress::new();
            std::thread::scope(|s| {
                for (param, prefix) in cfg.parameters.iter().zip(prefixes.into_iter()) {
                    let pb =
                        progress::new_multi_progressbar(&multi, cfg.init_steps + cfg.steps, prefix);
                    let init_steps = cfg.init_steps;
                    let steps = cfg.steps;
                    s.spawn(move || {
                        prep::prep_calculate(param, &pb, 0, init_steps);
                        calculate_size_and_duration::calculate1(param, &pb, init_steps, steps);
                        pb.finish();
                    });
                }
            });
        }
        13 => {
            let multi = MultiProgress::new();
            std::thread::scope(|s| {
                for (param, prefix) in cfg.parameters.iter().zip(prefixes.into_iter()) {
                    let pb =
                        progress::new_multi_progressbar(&multi, cfg.init_steps + cfg.steps, prefix);
                    let init_steps = cfg.init_steps;
                    let steps = cfg.steps;
                    s.spawn(move || {
                        prep::prep_calculate(param, &pb, 0, init_steps);
                        calculate_shape::calculate_shape(param, &pb, init_steps, steps);
                        pb.finish();
                    });
                }
            });
        }
        _ => (),
    }
}
