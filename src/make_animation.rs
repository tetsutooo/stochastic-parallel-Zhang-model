use indicatif::ProgressBar;

use crate::animation::Heatmap;
use crate::core::{Parameters, System};
const FRAME_INTERVAL: u64 = 1;

const FRAME_DELAY_MS: u32 = 33;

fn calculate(system: &mut System, pb: &ProgressBar, init_steps: u64, steps: u64) {
    let summary = system.simplified_format();
    let system_size = system.get_system_size();
    let gif_path = system.get_gif_file_path("energies", init_steps, steps);

    let mut stream = Heatmap::new()
        .system_size(system_size)
        .title(format!("Energies ({})", summary))
        .step_info(init_steps, FRAME_INTERVAL)
        .value_range(0.0, 1.0)
        .frame_delay_ms(FRAME_DELAY_MS)
        .open_stream(gif_path.as_str())
        .unwrap();

    for step in 0..steps {
        let _ = system.update();
        if step % FRAME_INTERVAL == 0 {
            stream.push_frame(system.get_energies()).unwrap();
        }
        pb.inc(1);
    }
}

pub fn calculate_anim(param: &Parameters, pb: &ProgressBar, init_steps: u64, steps: u64) {
    let mut system = System::new(param.clone());
    if init_steps == 0 {
        system.init();
    } else {
        system.load(init_steps).unwrap();
    }

    calculate(&mut system, pb, init_steps, steps);

    system.save(init_steps + steps).unwrap();
}
