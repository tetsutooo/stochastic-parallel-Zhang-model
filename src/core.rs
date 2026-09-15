use crate::io;
use crate::network;
use crate::rng::FastRng;

#[derive(Clone, PartialEq)]
pub struct Parameters {
    pub system_size: usize,
    pub sigma: f64,
    pub q: f64,

    pub config_name: String,
}

impl Parameters {
    pub fn display_format(&self) -> String {
        format!(
            "system_size: {}, sigma: {:.4}, q: {:.4}",
            self.system_size, self.sigma, self.q,
        )
    }
}

const E_TH: f64 = 1.0;
const DELTA_E_MAX: f64 = 0.25;

#[derive(Clone)]
pub struct System {
    energies: Vec<f64>,
    neighbors: Vec<Vec<usize>>,
    sigma: f64,
    q: f64,
    rng: FastRng,

    elapsed_time: f64,

    system_size: usize,
    num_of_sites: usize,
    idx_mask: usize,

    stack: Vec<usize>,
    visited: Vec<bool>,
    add: Vec<(usize, f64)>,

    eff_num_of_neighbors: Vec<u16>,

    eps_buf: Vec<f64>,
    eta_buf: Vec<f64>,

    threshold: f64,

    param: Parameters,
}

#[allow(dead_code)]
impl System {
    pub fn new(param: Parameters) -> Self {
        let system_size: usize = param.system_size;
        assert!(
            system_size.is_power_of_two(),
            "system_size must be a power of 2 (got {})",
            system_size,
        );
        let num_of_sites: usize = system_size * system_size;

        let rng: FastRng = FastRng::from_entropy();

        System {
            energies: Vec::with_capacity(num_of_sites),
            neighbors: Vec::with_capacity(num_of_sites),
            sigma: param.sigma,
            q: param.q,

            rng: rng,

            elapsed_time: 0.0,

            system_size: system_size,
            num_of_sites: num_of_sites,
            idx_mask: num_of_sites - 1,

            stack: Vec::with_capacity(1 << 5),
            visited: vec![false; num_of_sites],
            add: Vec::with_capacity(1 << 7),

            eff_num_of_neighbors: Vec::new(),
            eps_buf: Vec::with_capacity(1 << 4),
            eta_buf: Vec::with_capacity(1 << 4),

            threshold: E_TH,

            param: param,
        }
    }

    fn set_ramdom_energy(&mut self) {
        for _i in 0..self.num_of_sites {
            self.energies.push(self.rng.next_f64());
        }
    }

    fn set_regular_lattice(&mut self) {
        self.neighbors = network::make_regular_lattice(self.system_size);
        network::add_newman_watts_shortcuts(&mut self.neighbors, self.q, &mut self.rng);
    }

    pub fn init(&mut self) {
        self.set_ramdom_energy();
        self.set_regular_lattice();
        self.build_eff_num_table();
    }

    fn build_eff_num_table(&mut self) {
        self.eff_num_of_neighbors = (0..self.num_of_sites)
            .map(|idx| {
                let mut n = self.neighbors[idx].len();
                let (x, y) = (idx % self.system_size, idx / self.system_size);
                if x == 0 || x == self.system_size - 1 {
                    n += 1; // 端の場合は +1
                }
                if y == 0 || y == self.system_size - 1 {
                    n += 1; // 端の場合は +1
                }
                n as u16
            })
            .collect();
    }

    pub fn get_system_size(&self) -> usize {
        self.system_size
    }

    pub fn get_config_name(&self) -> &str {
        &self.param.config_name
    }

    pub fn get_num_of_sites(&self) -> usize {
        self.num_of_sites
    }

    pub fn get_neighbors(&self) -> &[Vec<usize>] {
        &self.neighbors
    }

    pub fn get_energies(&self) -> &[f64] {
        &self.energies
    }

    pub fn get_elapsed_time(&self) -> f64 {
        self.elapsed_time
    }

    pub fn get_sigma(&self) -> f64 {
        self.sigma
    }

    pub fn get_q(&self) -> f64 {
        self.q
    }

    #[inline]
    pub fn get_min_energy(&self) -> f64 {
        self.energies.iter().copied().fold(f64::INFINITY, f64::min)
    }

    #[inline]
    pub fn get_max_energy(&self) -> f64 {
        self.energies
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    #[inline]
    pub fn get_ave_energy(&self) -> f64 {
        self.energies.iter().sum::<f64>() / self.num_of_sites as f64
    }

    pub fn format(&self) -> String {
        format!(
            "system_size: {}, sigma: {:.4}, q: {:.4}",
            self.system_size, self.sigma, self.q,
        )
    }

    pub fn file_path_format(&self) -> String {
        format!(
            "L{}_sigma{:.4}_q{:.4}",
            self.system_size, self.sigma, self.q,
        )
    }

    pub fn simplified_format(&self) -> String {
        format!(
            "L = {}, sigma = {:.4}, q = {:.4}",
            self.system_size, self.sigma, self.q,
        )
    }

    fn get_eng_file_path(&self, total_steps: u64) -> String {
        format!(
            "./data/energies/{}/energies_{}_step{:e}.dat",
            self.get_config_name(),
            self.file_path_format(),
            total_steps,
        )
    }

    fn get_neighbors_file_path(&self) -> String {
        format!(
            "./data/neighbors/{}/neighbors_{}.dat",
            self.get_config_name(),
            self.file_path_format(),
        )
    }

    pub fn get_png_file_path(&self, dir: &str, init_steps: u64, steps: u64) -> String {
        format!(
            "./results/{}/{}/{}_{}_step{:e}+{:e}.png",
            self.get_config_name(),
            dir,
            dir,
            self.file_path_format(),
            init_steps,
            steps
        )
    }

    pub fn get_gif_file_path(&self, dir: &str, init_steps: u64, steps: u64) -> String {
        format!(
            "./results/{}/{}/{}_{}_step{:e}+{:e}.gif",
            self.get_config_name(),
            dir,
            dir,
            self.file_path_format(),
            init_steps,
            steps
        )
    }

    pub fn get_dat_file_path(&self, dir: &str, init_steps: u64, steps: u64) -> String {
        format!(
            "./data/outputs/{}/{}/{}_{}_step{:e}+{:e}.dat",
            self.get_config_name(),
            dir,
            dir,
            self.file_path_format(),
            init_steps,
            steps
        )
    }

    pub fn get_csv_file_path(&self, name: &str, init_steps: u64, steps: u64) -> String {
        format!(
            "./results/{}/{}_step{:e}+{:e}.csv",
            self.get_config_name(),
            name,
            init_steps,
            steps
        )
    }

    fn save_energies(&self, total_steps: u64) -> io::IoResult<()> {
        let file_path = self.get_eng_file_path(total_steps);
        io::write_v_f64(file_path, &self.energies)?;
        Ok(())
    }

    fn save_neighbors(&self) -> io::IoResult<()> {
        let file_path = self.get_neighbors_file_path();
        io::write_vv_usize(file_path, &self.neighbors)?;
        Ok(())
    }

    pub fn save(&mut self, total_steps: u64) -> io::IoResult<()> {
        self.save_energies(total_steps)?;
        self.save_neighbors()?;
        Ok(())
    }

    fn load_energies(&mut self, total_steps: u64) -> io::IoResult<()> {
        let file_path: String = self.get_eng_file_path(total_steps);
        self.energies = io::read_v_f64(file_path)?;
        Ok(())
    }

    fn load_neighbors(&mut self) -> io::IoResult<()> {
        let file_path: String = self.get_neighbors_file_path();
        self.neighbors = io::read_vv_usize(file_path)?;
        Ok(())
    }

    pub fn load(&mut self, total_steps: u64) -> io::IoResult<()> {
        self.load_energies(total_steps)?;
        self.load_neighbors()?;
        self.build_eff_num_table();
        Ok(())
    }

    #[inline(always)]
    fn drive_1step(&mut self) -> usize {
        let idx: usize = (self.rng.next_u64() as usize) & self.idx_mask;
        let delta: f64 = self.rng.gen_f64_upto(DELTA_E_MAX);

        unsafe {
            *self.energies.get_unchecked_mut(idx) += delta;
        }

        idx
    }

    fn drive(&mut self) -> usize {
        loop {
            let idx: usize = self.drive_1step();

            unsafe {
                if *self.energies.get_unchecked(idx) >= self.threshold {
                    return idx;
                }
            }
        }
    }

    fn add_redist_energy(&mut self) {
        unsafe {
            self.add.iter().for_each(|&(idx, redist_energy)| {
                let energy = self.energies.get_unchecked_mut(idx);
                *energy += redist_energy;

                if *energy >= self.threshold && !*self.visited.get_unchecked(idx) {
                    *self.visited.get_unchecked_mut(idx) = true;
                    self.stack.push(idx);
                }
            });
        }
        self.add.clear();
        for &idx in &self.stack {
            unsafe {
                *self.visited.get_unchecked_mut(idx) = false;
            }
        }
    }

    #[inline(always)]
    fn topple_site(&mut self, idx: usize) {
        let toppling_energy: f64;
        unsafe {
            toppling_energy = *self.energies.get_unchecked(idx);
            *self.energies.get_unchecked_mut(idx) = 0f64;
        }

        let effective_num_of_neighbors =
            unsafe { *self.eff_num_of_neighbors.get_unchecked(idx) } as usize;

        self.eps_buf.clear();
        let mut sum: f64 = 0.0;
        for _ in 0..effective_num_of_neighbors {
            let r: f64 = self.rng.next_f64();
            self.eps_buf.push(r);
            sum += r;
        }
        let scale: f64 = toppling_energy / sum;

        let nbrs = unsafe { self.neighbors.get_unchecked(idx) };
        if self.sigma == 0.0 {
            for (k, &jdx) in nbrs.iter().enumerate() {
                let redist_energy = unsafe { *self.eps_buf.get_unchecked(k) } * scale;
                self.add.push((jdx, redist_energy));
            }
        } else {
            self.eta_buf.clear();
            for _ in 0..nbrs.len() {
                let r: f64 = self.rng.next_f64();
                self.eta_buf.push((2.0 * r - 1.0) * self.sigma);
            }
            for (k, &jdx) in nbrs.iter().enumerate() {
                let redist_energy = unsafe {
                    *self.eps_buf.get_unchecked(k) * scale + *self.eta_buf.get_unchecked(k)
                };
                self.add.push((jdx, redist_energy));
            }
        }
    }

    fn generate_avalanche_for_prep_update(&mut self) {
        loop {
            while let Some(idx) = self.stack.pop() {
                self.topple_site(idx);
            }

            self.add_redist_energy();

            if self.stack.is_empty() {
                break;
            }
        }
    }

    pub fn prep_update(&mut self) {
        let idx: usize = self.drive();

        self.stack.push(idx);

        self.generate_avalanche_for_prep_update();
    }

    fn generate_avalanche_for_update(&mut self) -> (u64, u64) {
        let mut size: u64 = 0u64;
        let mut duration: u64 = 0u64;

        loop {
            while let Some(idx) = self.stack.pop() {
                self.topple_site(idx);

                size += 1;
            }

            duration += 1;

            self.add_redist_energy();

            if self.stack.is_empty() {
                break;
            }
        }

        (size, duration)
    }

    pub fn update(&mut self) -> (u64, u64) {
        let idx: usize = self.drive();

        self.stack.push(idx);

        let (size, duration) = self.generate_avalanche_for_update();

        (size, duration)
    }

    fn generate_avalanche_for_update_with_avalanche_shape(
        &mut self,
        avalanche_shape: &mut Vec<u64>,
    ) -> (u64, u64) {
        let mut size: u64 = 0u64;
        let mut duration: u64 = 0u64;

        avalanche_shape.clear();

        loop {
            avalanche_shape.push(0);
            while let Some(idx) = self.stack.pop() {
                self.topple_site(idx);

                size += 1;
                avalanche_shape[duration as usize] += 1;
            }

            duration += 1;

            self.add_redist_energy();

            if self.stack.is_empty() {
                break;
            }
        }

        (size, duration)
    }

    pub fn update_with_avalanche_shape(&mut self, avalanche_shape: &mut Vec<u64>) -> (u64, u64) {
        let idx: usize = self.drive();

        self.stack.push(idx);

        self.generate_avalanche_for_update_with_avalanche_shape(avalanche_shape)
    }
}
