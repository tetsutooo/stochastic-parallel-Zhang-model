# Noisy Stochastic Parallel Zhang (SPZ) Sandpile on a Small-World Lattice

Numerical simulation code for the noisy stochastic parallel Zhang (SPZ) sandpile model
with Newman–Watts shortcuts.


## How to run

```sh
cargo run --release -- configs/<name>.toml
```

The binary then prompts for a run mode on stdin. For a non-interactive run:

```sh
echo 12 | cargo run --release -- configs/L512_deg_for_q.toml
```

| Mode | Action |
|---|---|
| `0` | Energy-field animation (GIF) |
| `1` | Warm-up only, saves a checkpoint |
| `2` | Size / duration statistics |
| `3` | Average avalanche shapes |
| `12` | `1` followed by `2` |
| `13` | `1` followed by `3` |

Modes `2` and `3` load the checkpoint at `init_steps` and therefore require it to exist.
Modes `12` and `13` start from a fresh state, create the checkpoint, then run the
measurement. Use `12` or `13` for a new configuration.

## Configuration files

Two configurations for L = 512 are provided in `configs/`:

| File | Sweep |
|---|---|
| `L512_deg_for_q.toml` | q varied at σ = 0 |
| `L512_deg_for_sigma.toml` | σ varied at q = 0 |

Excerpt from `configs/L512_deg_for_q.toml`:

```toml
init_steps  = 100000000
steps       = 100000000

[[parameters]]
system_size = 512
sigma       = 0.00
q           = 0.00

[[parameters]]
system_size = 512
sigma       = 0.00
q           = 0.0001

# ... further [[parameters]] blocks with other q values
```

Each `[[parameters]]` block runs in its own thread. `system_size` must be a power of two.
The file stem (e.g. `L512_deg_for_q`) names the output directories.

## Output

```
data/energies/<config>/    checkpoints (site energies)
data/neighbors/<config>/   checkpoints (neighbour lists)
data/outputs/<config>/     raw measurement data (.dat)
results/<config>/          quick-look plots (PNG / GIF)
```

The checkpoints used for the paper's runs are included, so modes `2` and `3` can be
re-run without repeating the warm-up.
