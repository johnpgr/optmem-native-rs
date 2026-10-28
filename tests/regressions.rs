//! Native contract checks for validation and recall boundaries changed by refactors.

mod common;

use common::{snapshot, Pair};

#[test]
fn oversized_block_ids_leave_saved_summaries_unchanged() {
    let pair = Pair::new("oversized-blocks");

    for args in [
        vec!["init"],
        vec!["note", "first memory"],
        vec!["note", "second memory"],
        vec!["nap", "0-1", "saved summary"],
    ] {
        assert_eq!(pair.rs.run(&args, &[]).status, 0);
    }

    let before = snapshot(&pair.rs.mem());

    for id in [
        "0-18446744073709551615",
        "18446744073709551616-18446744073709551617",
        "0-170141183460469231731687303715884105727",
    ] {
        for command in ["forget", "zoom", "nap"] {
            let mut args = vec![command, id];
            if command == "nap" {
                args.push("replacement summary");
            }

            let result = pair.rs.run(&args, &[]);

            assert_eq!(result.status, 1, "{} {}", command, id);
            assert!(!result.stderr.contains("panicked"), "{}", result.stderr);
            assert_eq!(snapshot(&pair.rs.mem()), before, "{} {}", command, id);
        }
    }
}

#[test]
fn recall_matches_individual_records_even_for_greedy_patterns() {
    let pair = Pair::new("recall-boundaries");
    assert_eq!(pair.rs.run(&["init"], &[]).status, 0);

    for text in ["alpha", "beta", "coração"] {
        assert_eq!(pair.rs.run(&["note", text], &[]).status, 0);
    }

    let broad = pair.rs.run(&["recall", "."], &[]);
    let greedy = pair.rs.run(&["recall", "(?s:.*)"], &[]);
    assert_eq!(broad.status, 0);
    assert_eq!(greedy, broad);

    let crossing = pair.rs.run(&["recall", "(?s)alpha.*beta"], &[]);
    assert_eq!(crossing.status, 0);
    assert_eq!(crossing.stdout, "No match.\n");

    assert_eq!(pair.rs.run(&["config", "PART_CHARS=1"], &[]).status, 0);
    let capped = pair.rs.run(&["recall", "(?s:.*)"], &[]);
    assert_eq!(capped.status, 0);
    assert_eq!(
        capped.stdout,
        "\nNewest 0 of 3 matches. Narrow the regex.\n"
    );
}
