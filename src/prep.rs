use indicatif::ProgressBar;

use crate::core::{Parameters, System};

fn calculate(system: &mut System, pb: &ProgressBar, steps: u64) {
    let n_chunks = 100000; // 10000 チャンクに分割
    let chunk_size = steps / n_chunks;

    for _i in 0..n_chunks {
        for _j in 0..chunk_size {
            system.prep_update();
        }
        pb.inc(chunk_size);
    }
}

pub fn prep_calculate(param: &Parameters, pb: &ProgressBar, init_steps: u64, steps: u64) {
    let mut system = System::new(param.clone());
    if init_steps == 0 {
        system.init();
    } else {
        system.load(init_steps).unwrap();
    }

    calculate(&mut system, pb, steps);

    system.save(init_steps + steps).unwrap();
}
