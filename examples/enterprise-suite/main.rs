use agent_canvas::{
    BarSeries, CartesianPlot, Color, DonutChart, Flowchart, FunnelChart,
    LineSeries, QuadrantMatrix, Sparkline, SvgCanvas, ThemeMode, WaterfallChart, WaterfallItem,
};
use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() {
    println!("===============================================================================");
    println!("🏛️  NUSANTARA ENTERPRISE INTELLIGENCE SUITE (NEIS) — COMPREHENSIVE BENCHMARK");
    println!("===============================================================================");

    let out_dir = "/srv/hermes/workspace/unified-crate/examples/enterprise-suite/output";
    fs::create_dir_all(out_dir).expect("create output directory");

    let t_all = Instant::now();

    // -------------------------------------------------------------------------
    // PILAR 1: KEUANGAN KORPORAT (CFO Bridge & Treasury Multi-Asset Allocation)
    // -------------------------------------------------------------------------
    println!("\n[1/6] 💰 Generating Corporate Treasury & Financial Bridge...");

    // 1A. Donut Multi-Slice Treasury Portfolio (8 Instrumen Keuangan Triliun Rupiah)
    let t0 = Instant::now();
    let treasury_donut = DonutChart::new()
        .title("Alokasi Portofolio Kas & Treasury Holding Q4 (Total IDR 12.8 T)")
        .with_theme(ThemeMode::Dark)
        .center_text("IDR 12.8 T", "Kas Konsolidasi")
        .add_slice("Surat Berharga Negara (SBN)", 4_800.0)
        .add_slice("Deposito On Call (BUMN)", 2_500.0)
        .add_slice("Obligasi Korporasi AAA", 1_800.0)
        .add_slice("Saham Bluechip LQ45", 1_400.0)
        .add_slice("Pasar Uang Syariah", 950.0)
        .add_slice("Valuta Asing (USD/SGD)", 750.0)
        .add_slice("Hedging Komoditas", 400.0)
        .add_slice("Cadangan Kas Darurat", 200.0);
    let svg_treasury = treasury_donut.to_svg().expect("render treasury donut");
    save_file(out_dir, "01_treasury_multi_asset_donut.svg", &svg_treasury);
    println!("  ✓ 1A. Treasury Donut (8 slices) generated in {:?} ({} bytes)", t0.elapsed(), svg_treasury.len());

    // 1B. Waterfall Chart: Konsolidasi EBITDA & Jembatan Laba Bersih Holding
    let t1 = Instant::now();
    let waterfall = WaterfallChart::new()
        .title("Jembatan Rekonsiliasi EBITDA ke Net Income Konsolidasi (IDR Miliar)")
        .with_theme(ThemeMode::Dark)
        .add_item(WaterfallItem::step("Pendapatan Bruto Konsolidasi", 1250.0))
        .add_item(WaterfallItem::step("Beban Pokok Pendapatan (COGS)", -480.0))
        .add_item(WaterfallItem::step("Beban Operasional & Logistik", -210.0))
        .add_item(WaterfallItem::step("Gaji Tim & Tunjangan", -195.0))
        .add_item(WaterfallItem::step("Pemasaran & Akuisisi Pengguna", -85.0))
        .add_item(WaterfallItem::step("Beban Bunga Pinjaman Bank", -45.0))
        .add_item(WaterfallItem::step("Pajak Penghasilan Badan (PPh)", -55.0))
        .add_item(WaterfallItem::total("Laba Bersih Akhir (Net Profit)"));
    let svg_waterfall = waterfall.to_svg().expect("render waterfall");
    save_file(out_dir, "02_corporate_ebitda_waterfall.svg", &svg_waterfall);
    println!("  ✓ 1B. Corporate Waterfall generated in {:?} ({} bytes)", t1.elapsed(), svg_waterfall.len());

    // -------------------------------------------------------------------------
    // PILAR 2: OPERASIONAL BISNIS & SALES (Funnel & 12-Month Grouped Performance)
    // -------------------------------------------------------------------------
    println!("\n[2/6] 📊 Generating Sales Operations & Multi-Month Performance...");

    // 2A. Sales Onboarding Conversion Funnel (5 Tahapan)
    let t2 = Instant::now();
    let funnel = FunnelChart::new()
        .title("Pipeline Akuisisi Merchant Enterprise Q4")
        .with_theme(ThemeMode::Dark)
        .add_stage("Leads Terkualifikasi", 14_200.0)
        .add_stage("Verifikasi Legal & Dokumen", 8_900.0)
        .add_stage("Demo Produk & Pilot Sandbox", 4_600.0)
        .add_stage("Negosiasi Kontrak & SLA", 2_150.0)
        .add_stage("Akad & Go-Live Produksi", 1_280.0);
    let svg_funnel = funnel.to_svg().expect("render funnel");
    save_file(out_dir, "03_enterprise_sales_funnel.svg", &svg_funnel);
    println!("  ✓ 2A. Enterprise Funnel generated in {:?} ({} bytes)", t2.elapsed(), svg_funnel.len());

    // 2B. 12-Bulan Grouped Bar + Target Overlay (Januari - Desember)
    let t3 = Instant::now();
    let months = [
        "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Ags", "Sep", "Okt", "Nov", "Des",
    ];
    let actual_sales = [
        42.0, 48.0, 55.0, 51.0, 62.0, 68.0, 74.0, 79.0, 85.0, 92.0, 98.0, 115.0,
    ];
    let target_sales = [
        40.0, 45.0, 50.0, 55.0, 60.0, 65.0, 70.0, 75.0, 80.0, 85.0, 90.0, 100.0,
    ];

    let mut combo_canvas = SvgCanvas::new(880.0, 500.0)
        .with_background(Color::from_rgb(0x0F, 0x17, 0x2A));

    let mut combo_plot = CartesianPlot::new(880.0, 500.0)
        .with_theme(ThemeMode::Dark)
        .title("Realisasi Penjualan 12 Bulan vs Target Rencana Kerja (IDR Miliar)")
        .x_categories(&months);
    combo_plot.margin_bottom = 65.0;

    let (plot_rect, y_scale, cat_scale) = combo_plot
        .render_frame(&mut combo_canvas, 0.0, 130.0)
        .expect("render combo frame");

    let bar_series = BarSeries::new("Realisasi Penjualan", &actual_sales)
        .with_color(Color::from_rgb(0x3B, 0x82, 0xF6));
    bar_series.render_on(&mut combo_canvas, &plot_rect, &cat_scale, &y_scale, 0, 1, Color::from_rgb(0x3B, 0x82, 0xF6));

    let line_series = LineSeries::new("Target RKAP", &target_sales)
        .with_color(Color::from_rgb(0x10, 0xB9, 0x81))
        .with_stroke_width(2.5);
    line_series.render_on(&mut combo_canvas, &plot_rect, &cat_scale, &y_scale, Color::from_rgb(0x10, 0xB9, 0x81));

    let svg_combo = combo_canvas.to_svg();
    save_file(out_dir, "04_twelve_month_performance_combo.svg", &svg_combo);
    println!("  ✓ 2B. 12-Month Performance Combo generated in {:?} ({} bytes)", t3.elapsed(), svg_combo.len());

    // -------------------------------------------------------------------------
    // PILAR 3: MANAJEMEN STRATEGIS & RISIKO (12 Inisiatif Strategis 2x2 Matrix)
    // -------------------------------------------------------------------------
    println!("\n[3/6] 🧭 Generating Strategic Decision Matrix (12 Initiatives)...");
    let t4 = Instant::now();
    let quadrant = QuadrantMatrix::new()
        .title("Matriks Prioritas Inisiatif Strategis Dekade Baru 2027")
        .with_theme(ThemeMode::Dark)
        .x_axis("Biaya & Kompleksitas Teknis (Effort)")
        .y_axis("Dampak Finansial & Valuasi (Impact)")
        .quadrant_labels("Strategic Initiatives", "Quick Wins", "Fill-ins", "Re-evaluate")
        // Quick Wins (Low Effort, High Impact)
        .add_item("API Monetization", 25.0, 85.0)
        .add_item("Automated Invoicing", 18.0, 72.0)
        .add_item("Fraud Detection ML", 32.0, 88.0)
        // Strategic Bets (High Effort, High Impact)
        .add_item("Autonomous Agent Engine", 82.0, 95.0)
        .add_item("Regional Multi-Region Cloud", 75.0, 84.0)
        .add_item("Core Banking Replacement", 92.0, 91.0)
        .add_item("Cross-Border Settlement", 68.0, 78.0)
        // Fill-ins (Low Effort, Low Impact)
        .add_item("Internal Dark Mode", 15.0, 22.0)
        .add_item("Slack Alert Webhook", 22.0, 35.0)
        .add_item("Email Redesign", 28.0, 29.0)
        // Re-evaluate (High Effort, Low Impact)
        .add_item("Bespoke CRM In-house", 88.0, 32.0)
        .add_item("On-Prem Tape Storage", 74.0, 18.0);
    let svg_quadrant = quadrant.to_svg().expect("render quadrant");
    save_file(out_dir, "05_twelve_initiatives_strategic_matrix.svg", &svg_quadrant);
    println!("  ✓ 3. Strategic Matrix (12 projects) generated in {:?} ({} bytes)", t4.elapsed(), svg_quadrant.len());

    // -------------------------------------------------------------------------
    // PILAR 4: MONITORING REAL-TIME KPI (Executive Sparkline Matrix)
    // -------------------------------------------------------------------------
    println!("\n[4/6] ⚡ Generating Executive Micro-Sparkline KPI Cards...");
    let t5 = Instant::now();
    let kpi_metrics = [
        ("API P99 Latency (ms)", [42.0, 39.0, 44.0, 52.0, 48.0, 41.0, 38.0, 36.0, 35.0, 34.0], Color::from_rgb(0x10, 0xB9, 0x81)),
        ("Error Rate (%)", [0.12, 0.15, 0.11, 0.45, 0.22, 0.08, 0.05, 0.04, 0.06, 0.03], Color::from_rgb(0xEF, 0x44, 0x44)),
        ("DB Connection Pool", [65.0, 70.0, 72.0, 85.0, 90.0, 78.0, 62.0, 58.0, 55.0, 52.0], Color::from_rgb(0xF5, 0x9E, 0x0B)),
        ("Payment Gateway TPS", [120.0, 145.0, 190.0, 240.0, 310.0, 380.0, 420.0, 460.0, 510.0, 580.0], Color::from_rgb(0x3B, 0x82, 0xF6)),
        ("Kas Masuk Harian (M)", [18.0, 22.0, 19.0, 25.0, 31.0, 28.0, 35.0, 33.0, 42.0, 49.0], Color::from_rgb(0x8B, 0x5C, 0xF6)),
    ];

    for (idx, (name, values, color)) in kpi_metrics.into_iter().enumerate() {
        let spark = Sparkline::new(&values)
            .with_color(color)
            .with_size(180.0, 35.0);
        let svg_spark = spark.to_svg().expect("render sparkline");
        save_file(out_dir, &format!("06_kpi_sparkline_{}.svg", idx + 1), &svg_spark);
        println!("  ✓ 4.{}. KPI Sparkline '{}' ({} bytes)", idx + 1, name, svg_spark.len());
    }
    println!("  ✓ 4. All KPI Sparklines generated in {:?}", t5.elapsed());

    // -------------------------------------------------------------------------
    // PILAR 5: ARSITEKTUR SISTEM & SOP LEGAL (Kompleks Flowchart TD & LR)
    // -------------------------------------------------------------------------
    println!("\n[5/6] 🗺️ Generating Enterprise Architecture & SOP Flowcharts...");

    // 5A. Core Payment System Architecture (Top-to-Bottom with 11 Nodes & Async Queues)
    let t6 = Instant::now();
    let dsl_core_banking = r#"
        flowchart TD
        user(Consumer & Merchant Apps) --> cdn[Global Cloudflare Edge]
        cdn --> gw[Central API Gateway]
        gw --> ratelimit{Rate Limit OK?}
        ratelimit -->|No| drop[429 Too Many Requests]
        ratelimit -->|Yes| auth[OAuth2 Token Validation]
        auth --> balance{Saldo Cukup?}
        balance -->|No| reject[Tolak Transaksi 402]
        balance -->|Yes| ledger[Core Immutable Ledger Engine]
        ledger --> audit[(PostgreSQL Audit Log)]
        ledger -.-> event[(Kafka Event Bus & CDC)]
        ledger --> notify(Kirim Struk & Webhook Sukses)
    "#;
    let fc_banking = Flowchart::from_dsl(dsl_core_banking)
        .expect("parse banking dsl")
        .title("Arsitektur Sistem Pembayaran Real-Time (Core Banking Flow)")
        .with_theme(ThemeMode::Dark);
    let svg_banking = fc_banking.to_svg().expect("render banking flowchart");
    save_file(out_dir, "07_core_payment_architecture_flowchart.svg", &svg_banking);
    println!("  ✓ 5A. Core Payment Flowchart (11 nodes, branches) generated in {:?} ({} bytes)", t6.elapsed(), svg_banking.len());

    // 5B. SOP Pengadaan Barang & Jasa Multi-Divisi (Left-to-Right dengan Obstacle Bypasses)
    let t7 = Instant::now();
    let dsl_sop_procurement = r#"
        flowchart LR
        request(Pengajuan PO oleh Divisi) --> verify[Pemeriksaan Dokumen Pengadaan]
        verify --> budget{Di Bawah Budget RKAP?}
        budget -->|Yes| fastlane[Persetujuan Cepat Kadiv]
        budget -->|No| bod[Review Khusus Direksi & Dewan Komisaris]
        fastlane --> vendor[Penerbitan Kontrak Vendor]
        bod --> vendor
        vendor --> legal{Review Legalitas Kontrak?}
        legal -->|Yes| pay[Pencairan Uang Muka via Finance]
        legal -->|No| cancel(Pembatalan & Revisi Klausul)
        pay --> delivery(Pengiriman Barang & BAST)
    "#;
    let fc_procurement = Flowchart::from_dsl(dsl_sop_procurement)
        .expect("parse procurement dsl")
        .title("SOP Tata Kelola Pengadaan Barang & Jasa Holding (Left-to-Right)")
        .with_theme(ThemeMode::Dark);
    let svg_procurement = fc_procurement.to_svg().expect("render procurement flowchart");
    save_file(out_dir, "08_sop_procurement_multidept_flowchart.svg", &svg_procurement);
    println!("  ✓ 5B. SOP Procurement Flowchart (Multi-Branch LR) generated in {:?} ({} bytes)", t7.elapsed(), svg_procurement.len());

    // -------------------------------------------------------------------------
    // PILAR 6: STRESS-LOAD & BATCH PRODUCTION (100 Charts Batch Concurrency)
    // -------------------------------------------------------------------------
    println!("\n[6/6] 🔥 Executing Extreme Stress Benchmark (100 Enterprise Charts)...");
    let t8 = Instant::now();
    let mut total_svg_bytes = 0;
    let batch_count = 100;

    for i in 0..batch_count {
        let sales_val = 1000.0 + (i as f32) * 15.0;
        let cogs_val = 400.0 + (i as f32) * 5.0;

        let wf = WaterfallChart::new()
            .title(format!("Simulasi Konsolidasi Batch #{} (IDR Miliar)", i + 1))
            .add_item(WaterfallItem::step("Pendapatan Bruto", sales_val))
            .add_item(WaterfallItem::step("COGS & Beban Pabrik", -cogs_val))
            .add_item(WaterfallItem::step("Operasional & Distribusi", -250.0))
            .add_item(WaterfallItem::total("Net Profit Bersih"));
        let svg = wf.to_svg().expect("batch render");
        total_svg_bytes += svg.len();
    }
    let dur_stress = t8.elapsed();
    let avg_per_chart = dur_stress / (batch_count as u32);
    println!("  ✓ 6. Rendered {} Enterprise Charts in {:?}", batch_count, dur_stress);
    println!("       Avg speed per chart: {:?}", avg_per_chart);
    println!("       Total payload: {:.2} MB", (total_svg_bytes as f64) / (1024.0 * 1024.0));

    println!("\n===============================================================================");
    println!("🎉 ALL 6 ENTERPRISE SUITE PILARS GENERATED SUCCESSFULLY in {:?}", t_all.elapsed());
    println!("📁 Output artifacts saved at: {}", out_dir);
    println!("===============================================================================");
}

fn save_file(dir: &str, name: &str, content: &str) {
    let p = Path::new(dir).join(name);
    fs::write(&p, content).unwrap_or_else(|e| panic!("Failed to write {}: {}", p.display(), e));
}
