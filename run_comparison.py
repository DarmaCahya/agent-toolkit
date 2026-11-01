import subprocess
import time
import os
import json

CRATE_DIR = "/srv/hermes/workspace/unified-crate"
CORPUS_DIR = os.path.join(CRATE_DIR, "corpus")

print("=== RUNNING BEFORE vs AFTER COMPARISON ON UNIFIED CRATE ===")

# Test 1: Baseline Read (Before - all pages dumped into 1 giant string)
# vs Experiment Read (After - windowed 10 pages + max_chars cap + scanned check)
test_code = """
use vizier_pdf::{PdfReader, PdfReadOptions};
use std::fs;
use std::time::Instant;

fn main() {
    let bytes_500 = fs::read("corpus/500_pages.pdf").expect("read 500");
    
    // BEFORE: extract_markdown (Full dump)
    let t0 = Instant::now();
    let full_md = PdfReader::extract_markdown(&bytes_500).expect("full extract");
    let dur_before = t0.elapsed();
    
    // AFTER: read with agent options (Default: 10 pages window, 100k chars cap)
    let t1 = Instant::now();
    let res_after = PdfReader::read(&bytes_500, None).expect("agent read");
    let dur_after = t1.elapsed();
    
    // AFTER: Scanned PDF detection test
    let empty_pdf = vizier_pdf::PdfGenerator::from_html("<html><body><div style='height:100px;'></div></body></html>").unwrap();
    let scanned_res = PdfReader::read(&empty_pdf, None).unwrap();
    
    println!("--- DATA COMPARISON RESULTS ---");
    println!("BEFORE_FULL_CHARS: {}", full_md.chars().count());
    println!("BEFORE_EST_TOKENS: {}", full_md.chars().count() / 4);
    println!("BEFORE_TIME_MS: {:.2}", dur_before.as_secs_f64() * 1000.0);
    
    println!("AFTER_WINDOW_CHARS: {}", res_after.markdown.chars().count());
    println!("AFTER_EST_TOKENS: {}", res_after.markdown.chars().count() / 4);
    println!("AFTER_TIME_MS: {:.2}", dur_after.as_secs_f64() * 1000.0);
    println!("AFTER_PAGES: {} to {} of {}", res_after.page_start, res_after.page_end, res_after.total_pages);
    println!("AFTER_TRUNCATED: {}", res_after.truncated);
    println!("AFTER_NOTE: {}", res_after.note.unwrap_or_default());
    println!("SCANNED_LIKELY_EMPTY: {}", scanned_res.scanned_likely);
    println!("SCANNED_NOTE: {}", scanned_res.note.unwrap_or_default());
}
"""

with open(os.path.join(CRATE_DIR, "examples", "compare.rs"), "w") as f:
    f.write(test_code)

res = subprocess.run(["cargo", "run", "--example", "compare", "--release"], cwd=CRATE_DIR, capture_output=True, text=True)
print(res.stdout)
if res.returncode != 0:
    print("Error:", res.stderr)
