use agent_canvas::{
    Color, Fill, FontMetrics, Palette, Path, Point, Rect, Size, Stroke, SvgCanvas, ThemeMode,
};
use std::fs;

fn main() {
    println!("============================================================");
    println!("     MINI-PROJECT PROBE: ISSUE #6 (CORE CANVAS PRIMITIVES)  ");
    println!("============================================================");

    // 1. Dark Mode Executive Architecture Canvas
    let mut dark_canvas = SvgCanvas::new(900.0, 500.0)
        .with_view_box(0.0, 0.0, 900.0, 500.0)
        .with_background(Color::from_hex("#0F172A").unwrap());

    let dark_palette = Palette::new(ThemeMode::Dark);

    // Title
    let title = "Executive Data Pipeline — Real-World Canvas Probe";
    let title_w = FontMetrics::estimate_text_width(title, 20.0);
    println!("• Title font metric calculated width: {:.1}px", title_w);

    dark_canvas.draw_text(
        Point::new(450.0, 40.0),
        title,
        20.0,
        dark_palette.text_primary(),
        "middle",
    );

    // Node 1: Ingestion
    let node1_rect = Rect::new(Point::new(80.0, 150.0), Size::new(200.0, 120.0)).with_radius(10.0, 10.0);
    dark_canvas.draw_rect(
        node1_rect,
        Some(Stroke::new(dark_palette.get_categorical(0), 2.0)),
        Some(Fill::new(dark_palette.get_categorical(0), 0.15)),
    );
    dark_canvas.draw_text(Point::new(180.0, 200.0), "Stage 1: Ingestion", 15.0, dark_palette.text_primary(), "middle");
    dark_canvas.draw_text(Point::new(180.0, 225.0), "Kafka / EventHub", 13.0, dark_palette.text_secondary(), "middle");

    // Node 2: Processing (Center)
    let node2_rect = Rect::new(Point::new(350.0, 150.0), Size::new(200.0, 120.0)).with_radius(10.0, 10.0);
    dark_canvas.draw_rect(
        node2_rect,
        Some(Stroke::new(dark_palette.get_categorical(1), 2.0)),
        Some(Fill::new(dark_palette.get_categorical(1), 0.15)),
    );
    dark_canvas.draw_text(Point::new(450.0, 200.0), "Stage 2: Processing", 15.0, dark_palette.text_primary(), "middle");
    dark_canvas.draw_text(Point::new(450.0, 225.0), "Agent Engine Rust", 13.0, dark_palette.text_secondary(), "middle");

    // Node 3: Storage (Right)
    let node3_rect = Rect::new(Point::new(620.0, 150.0), Size::new(200.0, 120.0)).with_radius(10.0, 10.0);
    dark_canvas.draw_rect(
        node3_rect,
        Some(Stroke::new(dark_palette.get_categorical(4), 2.0)),
        Some(Fill::new(dark_palette.get_categorical(4), 0.15)),
    );
    dark_canvas.draw_text(Point::new(720.0, 200.0), "Stage 3: Vault / Lake", 15.0, dark_palette.text_primary(), "middle");
    dark_canvas.draw_text(Point::new(720.0, 225.0), "Obsidian & Postgres", 13.0, dark_palette.text_secondary(), "middle");

    // Connecting Bezier curves with arrows
    let curve1 = Path::new()
        .move_to(Point::new(280.0, 210.0))
        .cubic_to(Point::new(315.0, 190.0), Point::new(315.0, 230.0), Point::new(350.0, 210.0));
    dark_canvas.draw_path(&curve1, Some(Stroke::new(dark_palette.get_categorical(2), 2.5)), None);

    let curve2 = Path::new()
        .move_to(Point::new(550.0, 210.0))
        .cubic_to(Point::new(585.0, 190.0), Point::new(585.0, 230.0), Point::new(620.0, 210.0));
    dark_canvas.draw_path(&curve2, Some(Stroke::new(dark_palette.get_categorical(2), 2.5)), None);

    // Status metric card at bottom
    let status_rect = Rect::new(Point::new(80.0, 340.0), Size::new(740.0, 80.0)).with_radius(8.0, 8.0);
    dark_canvas.draw_rect(
        status_rect,
        Some(Stroke::new(dark_palette.border(), 1.0)),
        Some(Fill::new(Color::from_hex("#1E293B").unwrap(), 0.8)),
    );
    dark_canvas.draw_text(
        Point::new(120.0, 385.0),
        "Pipeline Telemetry: 100% Deterministic • 0ms Runtime Lag • Memory Safe",
        14.0,
        Color::from_hex("#10B981").unwrap(),
        "start",
    );

    let output_svg = dark_canvas.to_svg();
    let out_path = "crates/agent-canvas/examples/issue-006-demo/output_probe.svg";
    fs::write(out_path, &output_svg).expect("write output probe svg");

    println!("• Output SVG generated successfully: {} bytes", output_svg.len());
    println!("• Saved to: {}", out_path);
    println!("============================================================");
}
