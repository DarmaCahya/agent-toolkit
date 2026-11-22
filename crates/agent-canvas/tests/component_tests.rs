use agent_canvas::canvas::ThemeMode;
use agent_canvas::component::{DeltaTrend, EccLevel, KpiCard, QrCode, QrError, QrMode};

#[test]
fn qr_public_api_renders_vector_only_svg() {
    let qr = QrCode::encode("https://example.com/agent-canvas")
        .expect("payload fits")
        .with_size(320.0)
        .with_quiet_zone(2)
        .with_theme(ThemeMode::Dark);

    assert_eq!(qr.version(), 3);
    assert_eq!(qr.size(), 29);
    assert_eq!(qr.ecc_level(), EccLevel::M);
    assert_eq!(qr.mode(), QrMode::Byte);
    assert_eq!(qr.quiet_zone(), 2);
    assert!(qr.mask() < 8);
    assert_eq!(qr.modules().len(), 29 * 29);
    assert_eq!(qr.module(0, 0), Some(true));
    assert_eq!(qr.module(999, 0), None);

    let svg = qr.to_svg();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("width=\"320\""));
    assert!(svg.contains("viewBox=\"0 0 320 320\""));
    assert!(svg.contains("<path"));
    assert!(!svg.contains("<text"));
    assert!(!svg.contains("font-family"));

    let merged = qr.to_path();
    assert!(merged.starts_with("<path"));
    assert!(merged.contains("shape-rendering=\"crispEdges\""));

    let rects = qr.to_rects();
    let dark = qr.modules().iter().filter(|&&value| value).count();
    assert_eq!(rects.matches("<rect").count(), dark);
}

#[test]
fn qr_capacity_and_typed_errors() {
    assert_eq!(
        QrCode::capacity(1, EccLevel::Q, QrMode::Alphanumeric).expect("supported"),
        16
    );
    assert_eq!(
        QrCode::capacity(10, EccLevel::H, QrMode::Byte).expect("supported"),
        119
    );
    assert!(matches!(
        QrCode::capacity(11, EccLevel::L, QrMode::Byte),
        Err(QrError::UnsupportedVersion(11))
    ));
    assert_eq!(QrCode::encode(""), Err(QrError::EmptyInput));
    assert!(matches!(
        QrCode::encode_with_mask("payload", EccLevel::L, 9),
        Err(QrError::InvalidMask(9))
    ));
}

#[test]
fn qr_modes_are_detected() {
    let numeric = QrCode::encode("0123456789").expect("numeric");
    assert_eq!(numeric.mode(), QrMode::Numeric);
    let alphanumeric = QrCode::encode("HELLO WORLD").expect("alphanumeric");
    assert_eq!(alphanumeric.mode(), QrMode::Alphanumeric);
    let byte = QrCode::encode("Hello, World!").expect("byte");
    assert_eq!(byte.mode(), QrMode::Byte);
}

#[test]
fn kpi_card_exports_and_renders_every_sentiment() {
    let cases = [
        ("Revenue", "Rp 1.24B", DeltaTrend::Positive),
        ("Churn", "2.1%", DeltaTrend::Negative),
        ("Headcount", "128", DeltaTrend::Neutral),
    ];
    for (title, value, trend) in cases {
        let svg = KpiCard::new(title, value)
            .with_delta_trend("+0.4%", trend)
            .with_caption("vs. last month")
            .with_theme(ThemeMode::Light)
            .with_size(640.0, 320.0)
            .to_svg();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("width=\"640\""));
        assert!(svg.contains("viewBox=\"0 0 320 160\""));
        assert!(!svg.contains("<text"));
    }
}
