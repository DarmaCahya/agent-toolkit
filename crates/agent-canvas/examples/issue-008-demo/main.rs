use agent_canvas::{Flowchart, ThemeMode};
use std::fs;
use std::time::Instant;

fn main() {
    println!("=== AGENT-CANVAS: ISSUE #8 FLOWCHART & MANHATTAN PROBE ===");

    let out_dir = "/srv/hermes/workspace/unified-crate/crates/agent-canvas/examples/issue-008-demo";
    fs::create_dir_all(out_dir).expect("create probe output dir");

    // Case 1: System Architecture (Top-to-Bottom)
    let dsl_arch = r#"
        flowchart TD
        client(Mobile & Web Client) --> gateway[API Gateway]
        gateway --> auth{Token Valid?}
        auth -->|No| reject[401 Unauthorized Response]
        auth -->|Yes| worker[Worker Pool Service]
        worker --> db[(PostgreSQL Database)]
        worker -.-> cache[(Redis Cache)]
        worker --> done(200 OK Response)
    "#;

    let start = Instant::now();
    let chart_arch = Flowchart::from_dsl(dsl_arch)
        .expect("parse dsl")
        .title("Arsitektur Backend & Otentikasi Token (Top-to-Bottom)")
        .with_theme(ThemeMode::Dark);
    let svg_arch = chart_arch.to_svg().expect("render svg");
    let dur_arch = start.elapsed();

    fs::write(
        format!("{}/probe_flowchart_arch_td.svg", out_dir),
        &svg_arch,
    )
    .unwrap();
    println!(
        "1. System Architecture Flowchart: {:?} (SVG size: {} bytes)",
        dur_arch,
        svg_arch.len()
    );

    // Case 2: Business Logic Approval Pipeline (Left-to-Right)
    let dsl_biz = r#"
        flowchart LR
        request(Pengajuan Budget Proyek) --> manager[Review Branch Manager]
        manager --> eval{Nominal > 50 Juta?}
        eval -->|Yes| director[Approval Direktur Keuangan]
        eval -->|No| finance[Pencairan Dana Otomatis]
        director --> finance
        finance --> notify(Notifikasi Berhasil)
    "#;

    let start_biz = Instant::now();
    let chart_biz = Flowchart::from_dsl(dsl_biz)
        .expect("parse dsl")
        .title("Alur Persetujuan Anggaran Proyek (Left-to-Right)")
        .with_theme(ThemeMode::Dark);
    let svg_biz = chart_biz.to_svg().expect("render svg");
    let dur_biz = start_biz.elapsed();

    fs::write(format!("{}/probe_flowchart_biz_lr.svg", out_dir), &svg_biz).unwrap();
    println!(
        "2. Business Approval Pipeline: {:?} (SVG size: {} bytes)",
        dur_biz,
        svg_biz.len()
    );

    println!("Mini-project benchmark artifacts generated in {}", out_dir);
}
