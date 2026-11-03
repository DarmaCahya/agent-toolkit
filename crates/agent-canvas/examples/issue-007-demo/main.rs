use agent_canvas::{
    BarSeries, CartesianPlot, Color, DonutChart, FunnelChart, LineSeries,
    QuadrantMatrix, Sparkline, ThemeMode, WaterfallChart, WaterfallItem,
};
use std::fs;
use std::time::Instant;

fn main() {
    println!("============================================================");
    println!("   MINI-PROJECT USER PROBE: ISSUE #7 (EXECUTIVE CHARTS)     ");
    println!("   Perspective: Real-World Business Report & Financial CFO  ");
    println!("============================================================");

    let t0 = Instant::now();

    // 1. REAL CASE A: Financial Portfolio Split (Donut with Millions Rupiah)
    let donut_svg = DonutChart::new()
        .title("Ringkasan Alokasi Portofolio Finansial (Q4 2026)")
        .center_text("Rp 185 Juta", "Total Aset Likuid")
        .add_slice("Saham BBRI (55 Lot)", 105_000_000.0)
        .add_slice("Deposito Krom Bank (6% p.a.)", 65_000_000.0)
        .add_slice("Kas Operasional & GoPay", 15_000_000.0)
        .with_theme(ThemeMode::Dark)
        .to_svg()
        .expect("render portfolio donut");

    let p1 = "crates/agent-canvas/examples/issue-007-demo/probe_portfolio_donut.svg";
    fs::write(p1, &donut_svg).expect("write donut");
    println!("• [PROBE 1: Donut Portfolio] Generated: {} bytes -> {}", donut_svg.len(), p1);

    // 2. REAL CASE B: Corporate Financial Bridge (Waterfall Net Profit)
    let waterfall_svg = WaterfallChart::new()
        .title("Jembatan Arus Kas & Laba Bersih Operasional (IDR Juta)")
        .add_item(WaterfallItem::step("Gross Revenue Proyek", 240.0))
        .add_item(WaterfallItem::step("Biaya Server & Cloudflare", -28.0))
        .add_item(WaterfallItem::step("Gaji Tim & Engineer", -95.0))
        .add_item(WaterfallItem::step("Operasional & Marketing", -32.0))
        .add_item(WaterfallItem::step("Pajak & Retribusi", -15.0))
        .add_item(WaterfallItem::total("Net Profit Bersih"))
        .with_theme(ThemeMode::Dark)
        .to_svg()
        .expect("render waterfall");

    let p2 = "crates/agent-canvas/examples/issue-007-demo/probe_waterfall_bridge.svg";
    fs::write(p2, &waterfall_svg).expect("write waterfall");
    println!("• [PROBE 2: Waterfall Bridge] Generated: {} bytes -> {}", waterfall_svg.len(), p2);

    // 3. REAL CASE C: Multi-Project Prioritization Matrix (2x2 Quadrant)
    let quadrant_svg = QuadrantMatrix::new()
        .title("Matriks Keputusan Prioritas Proyek Darma (Effort vs Impact)")
        .x_axis("Tingkat Kesulitan / Effort")
        .y_axis("Dampak Bisnis / Impact")
        .quadrant_labels("Strategic Initiatives", "Quick Wins", "Fill-ins", "Re-evaluate")
        .add_item("Proyek Setneg (DDP)", 45.0, 92.0)
        .add_item("GitPaham Portal", 20.0, 85.0)
        .add_item("Darma Showcase", 15.0, 75.0)
        .add_item("Vizier PDF Addon Phase 2", 65.0, 95.0)
        .add_item("Refactor Legacy Script Lama", 75.0, 25.0)
        .with_theme(ThemeMode::Dark)
        .to_svg()
        .expect("render quadrant matrix");

    let p3 = "crates/agent-canvas/examples/issue-007-demo/probe_quadrant_priorities.svg";
    fs::write(p3, &quadrant_svg).expect("write quadrant");
    println!("• [PROBE 3: Quadrant Matrix] Generated: {} bytes -> {}", quadrant_svg.len(), p3);

    // 4. REAL CASE D: Composable Executive Cartesian Frame (Bar Actual + Line Target)
    let mut canvas = agent_canvas::canvas::SvgCanvas::new(800.0, 480.0)
        .with_background(Color::from_hex("#0F172A").unwrap());

    let plot = CartesianPlot::new(800.0, 480.0)
        .title("Kinerja Bisnis 2026: Realisasi Omzet vs Target RKAP (Juta IDR)")
        .with_theme(ThemeMode::Dark)
        .x_categories(&["Q1 2026", "Q2 2026", "Q3 2026", "Q4 2026"]);

    let (plot_rect, y_scale, cat_scale) = plot
        .render_frame(&mut canvas, 0.0, 320.0)
        .expect("render frame");

    let actual_bar = BarSeries::new("Realisasi Omzet", &[125.0, 190.0, 240.0, 310.0])
        .with_color(Color::from_hex("#059669").unwrap());
    actual_bar.render_on(&mut canvas, &plot_rect, &cat_scale, &y_scale, 0, 1, Color::from_hex("#059669").unwrap());

    let target_line = LineSeries::new("Target RKAP", &[110.0, 160.0, 220.0, 280.0])
        .with_color(Color::from_hex("#38BDF8").unwrap())
        .with_stroke_width(3.0);
    target_line.render_on(&mut canvas, &plot_rect, &cat_scale, &y_scale, Color::from_hex("#38BDF8").unwrap());

    let combo_svg = canvas.to_svg();
    let p4 = "crates/agent-canvas/examples/issue-007-demo/probe_combo_bar_line.svg";
    fs::write(p4, &combo_svg).expect("write combo");
    println!("• [PROBE 4: Combo Bar+Line] Generated: {} bytes -> {}", combo_svg.len(), p4);

    // 5. REAL CASE E: Inline Sparklines for Financial Table Cells
    let spark_green = Sparkline::new(&[14.0, 16.0, 15.5, 18.2, 22.0, 24.5])
        .with_color(Color::from_hex("#10B981").unwrap())
        .with_size(140.0, 32.0)
        .to_svg()
        .expect("render sparkline");
    let p5 = "crates/agent-canvas/examples/issue-007-demo/probe_table_sparkline.svg";
    fs::write(p5, &spark_green).expect("write sparkline");
    println!("• [PROBE 5: Table Sparkline] Generated: {} bytes -> {}", spark_green.len(), p5);

    // 6. REAL CASE F: Sales Conversion Pipeline (Funnel)
    let funnel_svg = FunnelChart::new()
        .title("Pipeline Akuisisi Pengguna Baru (Funnel Konversi)")
        .add_stage("Total Pengunjung Unik", 50_000.0)
        .add_stage("Pendaftaran Akun Baru", 12_500.0)
        .add_stage("Aktivasi Layanan / Trial", 4_200.0)
        .add_stage("Langganan Berbayar Pro", 980.0)
        .with_theme(ThemeMode::Dark)
        .to_svg()
        .expect("render funnel");
    let p6 = "crates/agent-canvas/examples/issue-007-demo/probe_sales_funnel.svg";
    fs::write(p6, &funnel_svg).expect("write funnel");
    println!("• [PROBE 6: Sales Funnel] Generated: {} bytes -> {}", funnel_svg.len(), p6);

    let dur = t0.elapsed();
    println!("------------------------------------------------------------");
    println!("ALL 6 REAL-WORLD USER PROBES GENERATED IN: {:.2} ms!", dur.as_secs_f64() * 1000.0);
    println!("============================================================");
}
