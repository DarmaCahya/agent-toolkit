use agent_canvas::canvas::ThemeMode;
use agent_canvas::component::{EccLevel, KpiCard, QrCode, QrMode};
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("=== AGENT-CANVAS: ISSUE #10 QR CODE + KPI CARDS ===");

    let out_dir = std::env::temp_dir().join("agent-canvas-issue-010");
    fs::create_dir_all(&out_dir).expect("create example output directory");

    let payloads = [
        ("hello", "HELLO WORLD", EccLevel::Q),
        (
            "url",
            "https://github.com/DarmaCahya/agent-toolkit",
            EccLevel::M,
        ),
        (
            "invoice",
            "INV-2026-00184 TOTAL 42.500.000 IDR",
            EccLevel::H,
        ),
    ];

    for (slug, payload, ecc) in payloads {
        let qr = QrCode::encode_with_ecc(payload, ecc)
            .expect("payload fits the supported versions")
            .with_size(360.0);
        let svg = qr.to_svg();
        let path = out_dir.join(format!("qr-{slug}.svg"));
        fs::write(&path, &svg).expect("write QR svg");

        println!(
            "QR {slug:<8} v{:<2} {:<1} {:?} mask {} -> {} modules, {} bytes -> {}",
            qr.version(),
            qr.ecc_level().label(),
            qr.mode(),
            qr.mask(),
            qr.size(),
            svg.len(),
            path.display()
        );
        println!(
            "           capacity at this size: {} bytes / {} numeric / {} alphanumeric",
            QrCode::capacity(qr.version(), ecc, QrMode::Byte).unwrap_or(0),
            QrCode::capacity(qr.version(), ecc, QrMode::Numeric).unwrap_or(0),
            QrCode::capacity(qr.version(), ecc, QrMode::Alphanumeric).unwrap_or(0)
        );
        assert!(!svg.contains("<text"), "QR output must stay vector-only");
    }

    let dark_qr = QrCode::encode("https://example.com/agent-canvas")
        .expect("payload fits")
        .with_theme(ThemeMode::Dark)
        .with_size(360.0);
    let dark_path = out_dir.join("qr-dark.svg");
    fs::write(&dark_path, dark_qr.to_svg()).expect("write dark QR svg");
    println!("QR dark theme  -> {}", dark_path.display());

    let cards = [
        (
            "kpi-revenue-light.svg",
            KpiCard::new("Monthly Revenue", "Rp 1.24B")
                .with_delta("+12.4%", true)
                .with_caption("vs. last month")
                .with_theme(ThemeMode::Light),
        ),
        (
            "kpi-churn-dark.svg",
            KpiCard::new("Customer Churn", "2.1%")
                .with_delta("+0.4pp", false)
                .with_caption("vs. last month")
                .with_theme(ThemeMode::Dark),
        ),
        (
            "kpi-headcount-dark.svg",
            KpiCard::new("Headcount", "128")
                .with_neutral_delta("0.0%")
                .with_caption("unchanged")
                .with_theme(ThemeMode::Dark),
        ),
    ];

    for (file, card) in cards {
        let svg = card.to_svg();
        let path: PathBuf = out_dir.join(file);
        fs::write(&path, &svg).expect("write KPI svg");
        assert!(!svg.contains("<text"), "KPI output must stay vector-only");
        println!("KPI {file:<24} {} bytes -> {}", svg.len(), path.display());
    }

    println!("Artifacts written to {}", out_dir.display());
}
