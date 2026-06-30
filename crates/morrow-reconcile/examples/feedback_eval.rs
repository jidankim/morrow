//! Local feedback eval replay command.

use morrow_reconcile::feedback_eval::{
    run_feedback_eval, synthetic_path_from_env, FeedbackEvalPaths,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = FeedbackEvalPaths {
        db_path: synthetic_path_from_env("MORROW_EVAL_DB")?,
        report_path: synthetic_path_from_env("MORROW_EVAL_REPORT")?,
    };
    let report = run_feedback_eval(&paths)?;
    println!("feedback_eval_ready=true");
    println!("status={}", report.status);
    println!("cases_evaluated={}", report.cases_evaluated);
    println!("cases_skipped={}", report.cases_skipped);
    for reason in report.skip_reasons.keys() {
        println!("skip_reason={reason}");
    }
    println!("eval_report={}", paths.report_path.display());
    Ok(())
}
