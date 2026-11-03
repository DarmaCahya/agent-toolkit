use agent_canvas::{
    BarChart, BarSeries, CartesianPlot, Color, DonutChart, FunnelChart, LineChart, LineSeries,
    QuadrantMatrix, Sparkline, ThemeMode, WaterfallChart, WaterfallItem,
};

#[test]
fn test_all_7_charts_render_valid_svg() {
    // 1. Donut Chart
    let donut_svg = DonutChart::new()
        .title("Portfolio Allocation Q4")
        .center_text("Rp 150M", "Total Assets")
        .add_slice("BBRI", 60.0)
        .add_slice("Krom Bank", 30.0)
        .add_slice("Cash", 10.0)
        .with_theme(ThemeMode::Dark)
        .to_svg()
        .expect("render donut");
    assert!(donut_svg.starts_with("<svg"));
    assert!(donut_svg.contains("BBRI (60.0%)"));
    assert!(donut_svg.contains("Portfolio Allocation Q4"));

    // 2. Pie Chart (hole_ratio = 0)
    let pie_svg = DonutChart::pie()
        .add_slice("A", 50.0)
        .add_slice("B", 50.0)
        .to_svg()
        .expect("render pie");
    assert!(pie_svg.starts_with("<svg"));

    // 3. Bar Chart (Grouped)
    let bar_svg = BarChart::new()
        .title("Quarterly Revenue vs Target")
        .categories(&["Q1", "Q2", "Q3", "Q4"])
        .add_series(BarSeries::new("Target", &[100.0, 150.0, 200.0, 250.0]))
        .add_series(BarSeries::new("Actual", &[110.0, 140.0, 220.0, 280.0]))
        .to_svg()
        .expect("render bar");
    assert!(bar_svg.starts_with("<svg"));
    assert!(bar_svg.contains("Quarterly Revenue vs Target"));

    // 4. Line Chart with Area Fill
    let line_svg = LineChart::new()
        .title("Monthly Active Users")
        .categories(&["Jan", "Feb", "Mar", "Apr"])
        .add_series(
            LineSeries::new("MAU", &[12000.0, 18500.0, 24000.0, 31000.0])
                .with_area_fill(true),
        )
        .to_svg()
        .expect("render line");
    assert!(line_svg.starts_with("<svg"));
    assert!(line_svg.contains("fill-opacity=\"0.15\""));

    // 5. Waterfall Chart (Financial Bridge)
    let waterfall_svg = WaterfallChart::new()
        .title("Net Profit Bridge 2026")
        .add_item(WaterfallItem::step("Gross Revenue", 100.0))
        .add_item(WaterfallItem::step("COGS", -30.0))
        .add_item(WaterfallItem::step("Opex", -25.0))
        .add_item(WaterfallItem::step("Tax", -10.0))
        .add_item(WaterfallItem::total("Net Income"))
        .to_svg()
        .expect("render waterfall");
    assert!(waterfall_svg.starts_with("<svg"));
    assert!(waterfall_svg.contains("Net Profit Bridge 2026"));

    // 6. Sparkline
    let sparkline_svg = Sparkline::new(&[10.0, 25.0, 18.0, 30.0, 42.0])
        .with_color(Color::from_hex("#10B981").unwrap())
        .to_svg()
        .expect("render sparkline");
    assert!(sparkline_svg.starts_with("<svg"));
    assert!(sparkline_svg.contains("stroke=\"#10B981\""));

    // 7. Quadrant Matrix (2x2 Decision Matrix)
    let quad_svg = QuadrantMatrix::new()
        .title("Product Feature Prioritization")
        .x_axis("Engineering Effort")
        .y_axis("Customer Value")
        .add_item("Quick Win Feature", 20.0, 85.0)
        .add_item("Major Overhaul", 85.0, 90.0)
        .add_item("Minor Polish", 15.0, 20.0)
        .to_svg()
        .expect("render quadrant");
    assert!(quad_svg.starts_with("<svg"));
    assert!(quad_svg.contains("Quick Win Feature"));
    assert!(quad_svg.contains("Engineering Effort"));

    // Bonus: Funnel Chart
    let funnel_svg = FunnelChart::new()
        .title("Sales Conversion Funnel")
        .add_stage("Impressions", 10000.0)
        .add_stage("Signups", 2500.0)
        .add_stage("Purchases", 450.0)
        .to_svg()
        .expect("render funnel");
    assert!(funnel_svg.starts_with("<svg"));
    assert!(funnel_svg.contains("Impressions (10000)"));
    assert!(funnel_svg.contains("25.0% conv"));
}

#[test]
fn test_cartesian_plot_composability_bar_plus_line() {
    let mut canvas = agent_canvas::canvas::SvgCanvas::new(800.0, 450.0);
    let plot = CartesianPlot::new(800.0, 450.0)
        .title("Combined Bar + Line Plan")
        .x_categories(&["2024", "2025", "2026"]);

    let (plot_rect, y_scale, cat_scale) = plot
        .render_frame(&mut canvas, 0.0, 100.0)
        .expect("render frame");

    let bar = BarSeries::new("Revenue", &[40.0, 70.0, 95.0]);
    bar.render_on(&mut canvas, &plot_rect, &cat_scale, &y_scale, 0, 1, Color::from_hex("#2563EB").unwrap());

    let line = LineSeries::new("Target", &[50.0, 65.0, 80.0]);
    line.render_on(&mut canvas, &plot_rect, &cat_scale, &y_scale, Color::from_hex("#10B981").unwrap());

    let svg = canvas.to_svg();
    assert!(svg.contains("Combined Bar + Line Plan"));
    assert!(svg.contains("<rect x="));
    assert!(svg.contains("<path d="));
}
