//! Multi-GB ACT streaming check over a randomly-incremented hex dataset.
//!
//! Dataset: a u64 advanced by random increments (deterministic LCG), each
//! value rendered as fixed-width 16-char lowercase hex. Fixed width makes
//! lexicographic order == numeric order, so keys are strictly increasing by
//! construction, and the whole sequence can be regenerated from the seed for
//! verification without ever storing it.
//!
//! Unlike dense sequential keys, the random gaps leave long unique suffixes,
//! so this shape exercises ACT line nodes (path compression) and the
//! FileDumper line-dedup cache, not just branch nodes.
//!
//! Ignored by default (writes a multi-GB file, takes minutes). Run with:
//!
//! ```sh
//! cargo test --release --test act_big_hex act_stream -- --ignored --nocapture
//! ```
//!
//! Environment overrides:
//! - `ACT_HEX_KEYS`: number of keys (default 200,000,000 — a multi-GB file)
//! - `ACT_HEX_PATH`: where to write the `.act` file (default
//!   `$CARGO_TARGET_TMPDIR/act_big_hex.act`; keep it off tmpfs for real sizes)
//! - `ACT_HEX_KEEP`: set to keep the file after the test instead of deleting

use std::path::PathBuf;
use std::time::Instant;

use pathmap::arena_compact::ACTOutputStream;
use pathmap::zipper::{ZipperIteration, ZipperMoving, ZipperReadOnlyValues};

const SEED: u64 = 0xdead_beef_1234_5678;
/// Random step in 1..=2^37; ~2^36 average keeps 200M keys inside u64 range
/// while spreading them across most of the 16-digit hex space.
const STEP_MASK: u64 = (1 << 37) - 1;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
}

/// The dataset: key i is hex16 of the i-th randomly incremented value.
struct KeyGen {
    rng: Lcg,
    v: u64,
}
impl KeyGen {
    fn new() -> Self {
        KeyGen { rng: Lcg(SEED), v: 0 }
    }
    fn next_key(&mut self) -> [u8; 16] {
        self.v += 1 + (self.rng.next() & STEP_MASK);
        hex16(self.v)
    }
}

fn hex16(v: u64) -> [u8; 16] {
    let mut b = [0u8; 16];
    for (i, out) in b.iter_mut().enumerate() {
        *out = b"0123456789abcdef"[((v >> ((15 - i) * 4)) & 0xf) as usize];
    }
    b
}

/// Universe generator for the formula test: each key comes with a 4-bit
/// membership mask assigning it to sources `a`..`d` (bit 0 = `a`, ...).
/// Both the key and its mask derive from one LCG draw, so the whole
/// universe regenerates from the seed.
struct UniverseGen {
    rng: Lcg,
    v: u64,
}
impl UniverseGen {
    fn new() -> Self {
        UniverseGen { rng: Lcg(SEED), v: 0 }
    }
    fn next(&mut self) -> ([u8; 16], u8) {
        let r = self.rng.next();
        self.v += 1 + (r & STEP_MASK);
        (hex16(self.v), ((r >> 37) & 0xf) as u8)
    }
}

const FORMULA: &str = "(a | b) & c - d";

/// Set-algebra oracle for [FORMULA] on a membership mask.
fn in_result(m: u8) -> bool {
    (m & 0b0011) != 0 && (m & 0b0100) != 0 && (m & 0b1000) == 0
}

/// Stream a formula from multiple on-disk ACT sources into an ACT output.
///
/// The state machine emits paths in strictly increasing order exactly once —
/// precisely `ACTOutputStream`'s input contract — so the result trie is
/// built on disk directly from the sink closure, with no intermediate
/// buffering. `ACTOutputStream` would error on any ordering violation, so
/// the push itself doubles as an assertion of the ordered-unique guarantee.
///
/// Sized via `ACT_FORMULA_KEYS` (default 100M universe keys, ~3 GiB of
/// source tries + output).
#[test]
#[ignore = "writes multiple GB of files and takes minutes; run explicitly in release"]
fn act_formula_act_to_act() -> Result<(), std::io::Error> {
    let n: u64 = std::env::var("ACT_FORMULA_KEYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000_000);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));

    let graph = synth_jit::formula::build_graph(FORMULA).expect("valid formula");
    assert_eq!(graph.source_names, ["a", "b", "c", "d"], "bit order assumption");

    // --- Build the four source ACTs in one pass over the universe ---
    let t0 = Instant::now();
    let src_paths: Vec<PathBuf> = graph
        .source_names
        .iter()
        .map(|name| dir.join(format!("act_formula_{name}.act")))
        .collect();
    let mut sources: Vec<ACTOutputStream> = src_paths
        .iter()
        .map(ACTOutputStream::new)
        .collect::<Result<_, _>>()?;
    let mut universe = UniverseGen::new();
    let mut src_counts = [0u64; 4];
    for _ in 0..n {
        let (key, mask) = universe.next();
        for (b, source) in sources.iter_mut().enumerate() {
            if mask & (1 << b) != 0 {
                source.push(key)?;
                src_counts[b] += 1;
            }
        }
    }
    let trees: Vec<_> = sources
        .into_iter()
        .map(|s| s.finish())
        .collect::<Result<Vec<_>, _>>()?;
    let src_bytes: u64 = src_paths
        .iter()
        .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();
    eprintln!(
        "built 4 sources ({src_counts:?} keys) in {:.1}s, {:.2} GiB total",
        t0.elapsed().as_secs_f64(),
        src_bytes as f64 / (1u64 << 30) as f64,
    );
    if n >= 100_000_000 {
        assert!(src_bytes > 2 * (1u64 << 30), "expected multi-GB sources, got {src_bytes}");
    }

    // --- Stream `FORMULA` from ACT zippers into an ACT output ---
    let out_path = dir.join("act_formula_out.act");
    let t1 = Instant::now();
    let mut out = ACTOutputStream::new(&out_path)?;
    let mut emitted: u64 = 0;
    {
        let mut zippers: Vec<_> = trees.iter().map(|t| t.read_zipper()).collect();
        let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();
        let mut sink = |path: &[u8]| {
            out.push(path).expect("machine output must be strictly increasing");
            emitted += 1;
        };
        unsafe { graph.run(&ptrs, &mut sink) };
    }
    let result = out.finish()?;
    let out_bytes = std::fs::metadata(&out_path)?.len();
    eprintln!(
        "`{FORMULA}` streamed {emitted} keys into {:.2} GiB ACT in {:.1}s",
        out_bytes as f64 / (1u64 << 30) as f64,
        t1.elapsed().as_secs_f64(),
    );

    // --- Verify the output trie against regenerated membership ---
    let t2 = Instant::now();
    let mut universe = UniverseGen::new();
    let mut expected: u64 = 0;
    let mut z = result.read_zipper();
    for _ in 0..n {
        let (key, mask) = universe.next();
        if in_result(mask) {
            expected += 1;
            assert!(z.to_next_val(), "output ended early after {} keys", expected - 1);
            assert_eq!(z.path(), key, "output mismatch at result index {}", expected - 1);
        }
    }
    assert!(!z.to_next_val(), "output has more than {expected} values");
    assert_eq!(emitted, expected, "emitted count disagrees with oracle");
    eprintln!(
        "verified {expected} result keys in {:.1}s",
        t2.elapsed().as_secs_f64()
    );

    drop(result);
    if std::env::var_os("ACT_HEX_KEEP").is_none() {
        for p in src_paths.iter().chain([&out_path]) {
            std::fs::remove_file(p)?;
        }
    } else {
        eprintln!("keeping files in {}", dir.display());
    }
    Ok(())
}

#[test]
#[ignore = "writes a multi-GB file and takes minutes; run explicitly in release"]
fn act_stream_multi_gb_hex() -> Result<(), std::io::Error> {
    let n: u64 = std::env::var("ACT_HEX_KEYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000_000);
    let path: PathBuf = std::env::var_os("ACT_HEX_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("act_big_hex.act"));

    // Show what the dataset looks like.
    let mut sample = KeyGen::new();
    eprint!("dataset sample:");
    for _ in 0..4 {
        eprint!(" {}", std::str::from_utf8(&sample.next_key()).unwrap());
    }
    eprintln!(" ...");

    // --- Build ---
    eprintln!("building {n} hex keys -> {}", path.display());
    let t0 = Instant::now();
    let mut keys = KeyGen::new();
    let mut out = ACTOutputStream::new(&path)?;
    for i in 0..n {
        out.push_val(keys.next_key(), i)?;
        if (i + 1) % 50_000_000 == 0 {
            eprintln!(
                "  pushed {:>4}M keys in {:.1}s",
                (i + 1) / 1_000_000,
                t0.elapsed().as_secs_f64()
            );
        }
    }
    let tree = out.finish()?;
    let build_secs = t0.elapsed().as_secs_f64();
    let size = std::fs::metadata(&path)?.len();
    eprintln!(
        "built in {build_secs:.1}s ({:.1}M keys/s), file size {:.2} GiB ({:.1} B/key)",
        n as f64 / build_secs / 1e6,
        size as f64 / (1u64 << 30) as f64,
        size as f64 / n as f64,
    );
    if n >= 200_000_000 {
        assert!(
            size > 2 * (1u64 << 30),
            "expected a multi-GB file at {n} keys, got {size} bytes"
        );
    }

    // --- Sampled point lookups (regenerate the sequence) ---
    let t1 = Instant::now();
    let step = (n / 100_000).max(1);
    let mut keys = KeyGen::new();
    for i in 0..n {
        let v_prev = keys.v;
        let key = keys.next_key();
        if i % step == 0 {
            assert_eq!(tree.get_val_at(key), Some(i), "lookup mismatch at index {i}");
            // The -1 neighbor must be absent, unless the step was exactly 1
            // and it is the previous member.
            if keys.v - 1 > v_prev {
                let absent = hex16(keys.v - 1);
                assert_eq!(tree.get_val_at(absent), None, "false positive at index {i}");
            }
            assert_eq!(tree.get_val_at(&key[..15]), None, "prefix hit at index {i}");
        }
    }
    eprintln!("point lookups ok in {:.1}s", t1.elapsed().as_secs_f64());

    // --- Full ordered walk regenerates every key exactly once ---
    let t2 = Instant::now();
    let mut keys = KeyGen::new();
    let mut z = tree.read_zipper_u64();
    let mut i: u64 = 0;
    while z.to_next_val() {
        assert!(i < n, "walk produced more than {n} values");
        assert_eq!(z.path(), keys.next_key(), "walk path mismatch at index {i}");
        assert_eq!(z.get_val().copied(), Some(i), "walk value mismatch at index {i}");
        i += 1;
    }
    assert_eq!(i, n, "walk produced {i} values, expected {n}");
    eprintln!(
        "full walk of {n} values ok in {:.1}s ({:.1}M vals/s)",
        t2.elapsed().as_secs_f64(),
        n as f64 / t2.elapsed().as_secs_f64() / 1e6,
    );

    drop(tree);
    if std::env::var_os("ACT_HEX_KEEP").is_none() {
        std::fs::remove_file(&path)?;
    } else {
        eprintln!("keeping {}", path.display());
    }
    Ok(())
}
