#![allow(missing_docs)]

#[path = "reconcile_smoke/suite.rs"]
mod suite;
#[path = "reconcile_smoke/summary.rs"]
mod summary;

use std::io::Error;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match require_scenario()? {
        Scenario::LifecycleSuite => {
            let counts = suite::run_lifecycle_suite()?;
            summary::write_summary(&counts)
        }
        Scenario::ZeroCountProbe => summary::write_summary(&summary::SmokeCounts::default()),
    }
}

enum Scenario {
    LifecycleSuite,
    ZeroCountProbe,
}

fn require_scenario() -> Result<Scenario, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut scenario = None;
    while let Some(arg) = args.next() {
        if arg == "--scenario" {
            scenario = args.next();
        }
    }
    match scenario.as_deref() {
        Some("lifecycle-suite") => Ok(Scenario::LifecycleSuite),
        Some("zero-count-probe") => Ok(Scenario::ZeroCountProbe),
        Some(_) | None => Err(Box::new(Error::other(
            "usage: reconcile_smoke --scenario lifecycle-suite",
        ))),
    }
}
