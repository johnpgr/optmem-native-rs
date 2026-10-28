//! The blocks `wake` prints. A BLOCK is an aligned power-of-two range of
//! memories, [lo,hi), compressed into one line. This is the reference
//! algorithm step for step, including its float bisection, so both
//! implementations pick the same blocks.

/// Half-open aligned range; user-facing IDs subtract one from the upper bound.
pub type Block = (u64, u64);

/// Tile [0,T) with aligned power-of-two blocks; keep a block whole iff its
/// size is at most `alpha` times its age.
fn cover_alpha(t: u64, alpha: f64) -> Vec<Block> {
    let mut root: u64 = 1;

    while root < t {
        root *= 2;
    }

    let (mut out, mut stack) = (Vec::new(), vec![(0u64, root)]);

    while let Some((lo, hi)) = stack.pop() {
        if lo >= t {
            continue;
        }

        let size = hi - lo;

        if size > 1 && (hi > t || size as f64 > alpha * (t - lo) as f64) {
            let mid = (lo + hi) / 2;
            stack.push((mid, hi));
            stack.push((lo, mid));
        } else {
            out.push((lo, hi));
        }
    }

    out.sort_unstable();
    out
}

/// At most `budget` blocks, finest near T. If everything fits, nothing is
/// compressed at all.
pub fn cover(t: u64, budget: u64) -> Vec<Block> {
    if t == 0 {
        return Vec::new();
    }

    if t <= budget {
        return (0..t).map(|i| (i, i + 1)).collect();
    }

    let (mut lo, mut hi) = (0.0f64, 1.0f64);

    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;

        if cover_alpha(t, mid).len() as u64 > budget {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let mut out = cover_alpha(t, hi);
    // Block sizes jump in powers of two, so alpha alone can undershoot the
    // budget. Spend what is left on the present, where detail is worth most.

    while (out.len() as u64) < budget {
        let Some(i) = out.iter().rposition(|b| b.1 - b.0 > 1) else {
            break;
        };

        let (a, b) = out[i];
        let mid = (a + b) / 2;
        out.splice(i..i + 1, [(a, mid), (mid, b)]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiles(t: u64, blocks: &[Block]) -> bool {
        let mut at = 0;

        for &(lo, hi) in blocks {
            let n = hi - lo;

            if lo != at || n == 0 || n & (n - 1) != 0 || lo % n != 0 {
                return false;
            }
            at = hi;
        }
        at == t
    }

    #[test]
    fn small_is_verbatim() {
        assert_eq!(cover(3, 96), vec![(0, 1), (1, 2), (2, 3)]);
        assert!(cover(0, 96).is_empty());
    }

    #[test]
    fn always_tiles() {
        for t in 1..2000 {
            for budget in [1, 2, 7, 96] {
                assert!(tiles(t, &cover(t, budget)), "t={} budget={}", t, budget);
            }
        }
    }
}

#[cfg(test)]
mod parity {
    use super::cover;
    use std::process::Command;

    /// Compare with the reference's own `cover` over many sizes and budgets.
    #[test]
    fn matches_reference() {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/reference/memo.py");
        let code = format!(
            "from importlib.machinery import SourceFileLoader as L\n\
             m = L('m', {:?}).load_module()\n\
             for b in (1, 3, 8, 96, 200):\n\
             \x20   for t in list(range(1, 1200)) + [4095, 4096, 4097, 99999, 1000000, 123456789]:\n\
             \x20       print(b, t, ' '.join('%d-%d' % x for x in m.cover(t, b)))",
            script
        );
        let py = std::env::var("OPTMEM_PYTHON").unwrap_or_else(|_| "python3".into());
        let Ok(out) = Command::new(py).args(["-c", &code]).output() else {
            eprintln!("skipped: python3 unavailable");
            return;
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8(out.stdout).unwrap();
        let mut n = 0;

        for line in text.lines() {
            let mut it = line.splitn(3, ' ');
            let b: u64 = it.next().unwrap().parse().unwrap();
            let t: u64 = it.next().unwrap().parse().unwrap();
            let want = it.next().unwrap_or("");
            let got = cover(t, b)
                .iter()
                .map(|(lo, hi)| format!("{}-{}", lo, hi))
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(got, want, "budget={} T={}", b, t);
            n += 1;
        }
        assert!(n > 5000);
    }
}
