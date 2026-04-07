// Strategy is defined in schema.rs and implemented in evaluator.rs.
// This module documents the algorithm for each strategy and re-exports
// the type for external consumers.

pub use crate::schema::Strategy;

// Strategy selection guide:
//
// - FirstMatch  — Stop at the first matching rule (ordered by priority DESC).
//                 Lowest latency for large rulesets where one match is expected.
//
// - AllMatch    — Evaluate every enabled rule; collect all matches.
//                 Use when multiple rules may fire simultaneously (e.g. promotions).
//
// - Scored      — Score each rule by the fraction of top-level condition nodes that
//                 resolved to true. Return all rules with score > 0, sorted by
//                 score DESC.  Rules with score 0 are excluded from results.
//                 (§5.4: each group node is a single virtual leaf whose value is
//                 the group's final resolved result.)
