use std::path::Path;
use std::process::Command;

const FIELD_SEPARATOR: &str = "\u{1f}";

pub fn sqlite_rows(db_path: &Path, sql: &str) -> Vec<Vec<String>> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg("-separator")
        .arg(FIELD_SEPARATOR)
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("run sqlite3");
    assert!(
        output.status.success(),
        "sqlite3 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split(FIELD_SEPARATOR).map(str::to_owned).collect())
        .collect()
}
