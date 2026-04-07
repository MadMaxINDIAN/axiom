/// Full Prometheus metric set (§11.2).
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec,
    CounterVec, Gauge, HistogramVec,
};
use lazy_static::lazy_static;

lazy_static! {
    // axiom_evaluations_total{strategy, ruleset}
    pub static ref EVAL_TOTAL: CounterVec = register_counter_vec!(
        "axiom_evaluations_total",
        "Total evaluation requests",
        &["strategy", "ruleset"]
    ).unwrap();

    // axiom_evaluation_duration_seconds{strategy}
    // Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms
    pub static ref EVAL_DURATION: HistogramVec = register_histogram_vec!(
        "axiom_evaluation_duration_seconds",
        "Evaluation latency",
        &["strategy"],
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500]
    ).unwrap();

    // axiom_rules_matched_total{rule_id}
    pub static ref RULES_MATCHED: CounterVec = register_counter_vec!(
        "axiom_rules_matched_total",
        "Total rule matches",
        &["rule_id"]
    ).unwrap();

    // axiom_rules_loaded (Gauge)
    pub static ref RULES_LOADED: Gauge = register_gauge!(
        "axiom_rules_loaded",
        "Current number of enabled rules in the registry"
    ).unwrap();

    // axiom_evaluation_timeouts_total
    pub static ref EVAL_TIMEOUTS: prometheus::Counter = {
        prometheus::register_counter!(
            "axiom_evaluation_timeouts_total",
            "Evaluations aborted by timeout budget"
        ).unwrap()
    };

    // axiom_store_query_duration_seconds
    pub static ref STORE_DURATION: HistogramVec = register_histogram_vec!(
        "axiom_store_query_duration_seconds",
        "Storage layer query latency",
        &["operation"],
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.500]
    ).unwrap();
}

/// Record a completed evaluation.
pub fn record_eval(
    strategy:      &str,
    ruleset:       &str,
    duration_us:   u64,
    matched_rules: &[String],
    timed_out:     bool,
) {
    EVAL_TOTAL.with_label_values(&[strategy, ruleset]).inc();
    EVAL_DURATION
        .with_label_values(&[strategy])
        .observe(duration_us as f64 / 1_000_000.0);
    for rule_id in matched_rules {
        RULES_MATCHED.with_label_values(&[rule_id]).inc();
    }
    if timed_out { EVAL_TIMEOUTS.inc(); }
}

/// Update the rules-loaded gauge.
pub fn set_rules_loaded(n: usize) {
    RULES_LOADED.set(n as f64);
}
