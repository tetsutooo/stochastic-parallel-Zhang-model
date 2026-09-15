#![allow(dead_code)]

use crate::rng::FastRng;

pub fn make_regular_lattice(system_size: usize) -> Vec<Vec<usize>> {
    assert!(system_size > 0, "system_size must be positive");

    let n = system_size * system_size;
    let mut out: Vec<Vec<usize>> = vec![Vec::with_capacity(4); n];

    for y in 0..system_size {
        for x in 0..system_size {
            let idx = y * system_size + x;
            let nbrs = &mut out[idx];

            if y > 0 {
                nbrs.push((y - 1) * system_size + x);
            }
            if x > 0 {
                nbrs.push(y * system_size + (x - 1));
            }
            if x + 1 < system_size {
                nbrs.push(y * system_size + (x + 1));
            }
            if y + 1 < system_size {
                nbrs.push((y + 1) * system_size + x);
            }
        }
    }

    out
}

pub fn add_newman_watts_shortcuts(
    lattice: &mut Vec<Vec<usize>>,
    q: f64,
    rng: &mut FastRng,
) -> usize {
    assert!(!lattice.is_empty(), "lattice must be non-empty");
    assert!((0.0..1.0).contains(&q), "q must be in [0.0, 1.0)");

    let n = lattice.len();
    if n <= 1 {
        return 0;
    }

    let e_init: usize = 2 * n;
    let m_target = (q * e_init as f64).round() as usize;

    let e_actual: usize = lattice.iter().map(Vec::len).sum::<usize>() / 2;
    let max_edges = n.checked_mul(n - 1).expect("N*(N-1) overflow") / 2;
    assert!(
        e_actual + m_target <= max_edges,
        "requested too many shortcuts (E_actual={e_actual}, M={m_target}, max={max_edges})",
    );

    let mut added = 0usize;
    let mut attempts = 0usize;
    let attempt_limit = m_target.saturating_mul(1024).max(1);

    while added < m_target && attempts < attempt_limit {
        attempts += 1;

        let i = rng.gen_usize_range(0, n);
        let j = rng.gen_usize_range(0, n);
        if i == j {
            continue;
        }

        if lattice[i].binary_search(&j).is_ok() {
            continue;
        }

        let pos_i = lattice[i].partition_point(|&x| x < j);
        lattice[i].insert(pos_i, j);

        let pos_j = lattice[j].partition_point(|&x| x < i);
        lattice[j].insert(pos_j, i);

        added += 1;
    }

    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbor_counts_2x2() {
        let net = make_regular_lattice(2);
        assert_eq!(net.len(), 4);
        for nbrs in &net {
            assert_eq!(nbrs.len(), 2);
        }
    }

    #[test]
    fn neighbor_counts_4x4() {
        let l = 4;
        let net = make_regular_lattice(l);
        assert_eq!(net.len(), l * l);

        let mut counts = [0usize; 5];
        for nbrs in &net {
            counts[nbrs.len()] += 1;
        }
        assert_eq!(counts[2], 4);
        assert_eq!(counts[3], 4 * (l - 2));
        assert_eq!(counts[4], (l - 2) * (l - 2));
    }

    #[test]
    fn sorted_and_in_range() {
        let l = 8;
        let n = l * l;
        let net = make_regular_lattice(l);
        for (i, nbrs) in net.iter().enumerate() {
            for w in nbrs.windows(2) {
                assert!(w[0] < w[1], "out[{i}] not strictly increasing");
            }
            for &j in nbrs {
                assert!(j < n, "out-of-range neighbour {j} in out[{i}]");
                assert!(j != i, "self-loop at {i}");
            }
        }
    }

    #[test]
    fn symmetric() {
        let l = 8;
        let net = make_regular_lattice(l);
        for (i, nbrs) in net.iter().enumerate() {
            for &j in nbrs {
                assert!(
                    net[j].binary_search(&i).is_ok(),
                    "edge {i} -> {j} is not reciprocated"
                );
            }
        }
    }

    #[test]
    fn specific_neighbors_4x4() {
        let l = 4;
        let net = make_regular_lattice(l);
        // Corner (0, 0) -> idx 0: neighbors are (1, 0)=1 and (0, 1)=4.
        assert_eq!(net[0], vec![1, 4]);
        // Edge (1, 0) -> idx 1: neighbors are (0, 0)=0, (2, 0)=2, (1, 1)=5.
        assert_eq!(net[1], vec![0, 2, 5]);
        // Interior (1, 1) -> idx 5: neighbors are (1, 0)=1, (0, 1)=4, (2, 1)=6, (1, 2)=9.
        assert_eq!(net[5], vec![1, 4, 6, 9]);
        // Corner (3, 3) -> idx 15: neighbors are (3, 2)=11 and (2, 3)=14.
        assert_eq!(net[15], vec![11, 14]);
    }

    // ---- Newman-Watts shortcut tests ----

    fn count_edges(net: &[Vec<usize>]) -> usize {
        net.iter().map(Vec::len).sum::<usize>() / 2
    }

    fn assert_invariants(net: &[Vec<usize>]) {
        let n = net.len();
        for (i, nbrs) in net.iter().enumerate() {
            for w in nbrs.windows(2) {
                assert!(w[0] < w[1], "out[{i}] not strictly increasing");
            }
            for &j in nbrs {
                assert!(j < n, "out-of-range neighbour {j} in out[{i}]");
                assert!(j != i, "self-loop at {i}");
                assert!(
                    net[j].binary_search(&i).is_ok(),
                    "edge {i} -> {j} not reciprocated",
                );
            }
        }
    }

    #[test]
    fn nw_q_zero_leaves_lattice_unchanged() {
        let l = 8;
        let base = make_regular_lattice(l);
        let mut net = base.clone();
        let mut rng = FastRng::new(42);
        let added = add_newman_watts_shortcuts(&mut net, 0.0, &mut rng);
        assert_eq!(added, 0);
        assert_eq!(net, base);
    }

    #[test]
    fn nw_adds_exact_count_and_preserves_invariants() {
        let l = 16;
        let q = 0.1;
        let base = make_regular_lattice(l);
        let e_init = count_edges(&base);
        let m_expected = (q * (2 * l * l) as f64).round() as usize;

        let mut net = base.clone();
        let mut rng = FastRng::new(123);
        let added = add_newman_watts_shortcuts(&mut net, q, &mut rng);

        assert_eq!(added, m_expected);
        assert_eq!(count_edges(&net) - e_init, added);
        assert_invariants(&net);

        for (i, base_nbrs) in base.iter().enumerate() {
            for &j in base_nbrs {
                assert!(
                    net[i].binary_search(&j).is_ok(),
                    "original edge {i}-{j} lost"
                );
            }
        }
    }

    #[test]
    fn nw_target_uses_nominal_2n_base() {
        let l = 8;
        let mut net = make_regular_lattice(l);
        assert_eq!(count_edges(&net), 2 * l * (l - 1));

        let mut rng = FastRng::new(7);
        let added = add_newman_watts_shortcuts(&mut net, 0.5, &mut rng);
        assert_eq!(added, 64);
        assert_eq!(count_edges(&net), 2 * l * (l - 1) + 64);
        assert_invariants(&net);
    }

    #[test]
    fn nw_n_eq_1_returns_zero() {
        let mut net: Vec<Vec<usize>> = vec![Vec::new()];
        let mut rng = FastRng::new(0);
        assert_eq!(add_newman_watts_shortcuts(&mut net, 0.5, &mut rng), 0);
    }

    #[test]
    #[should_panic(expected = "q must be in [0.0, 1.0)")]
    fn nw_panics_on_negative_q() {
        let mut net = make_regular_lattice(4);
        let mut rng = FastRng::new(0);
        add_newman_watts_shortcuts(&mut net, -0.1, &mut rng);
    }

    #[test]
    #[should_panic(expected = "q must be in [0.0, 1.0)")]
    fn nw_panics_on_q_equals_one() {
        let mut net = make_regular_lattice(4);
        let mut rng = FastRng::new(0);
        add_newman_watts_shortcuts(&mut net, 1.0, &mut rng);
    }

    #[test]
    #[should_panic(expected = "lattice must be non-empty")]
    fn nw_panics_on_empty_lattice() {
        let mut net: Vec<Vec<usize>> = Vec::new();
        let mut rng = FastRng::new(0);
        add_newman_watts_shortcuts(&mut net, 0.1, &mut rng);
    }
}
