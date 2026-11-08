use agent_canvas::{
    Color, Fill, FontMetrics, Palette, Path, Point, Rect, Size, Stroke, SvgCanvas, ThemeMode,
};

#[test]
fn test_canvas_primitives_render_valid_svg() {
    let mut canvas = SvgCanvas::new(800.0, 600.0)
        .with_view_box(0.0, 0.0, 800.0, 600.0)
        .with_background(Color::from_hex("#0F172A").expect("dark background"));

    // Draw header text
    canvas.draw_text(
        Point::new(400.0, 50.0),
        "Architecture Overview & Status <Active>",
        24.0,
        Color::from_hex("#F8FAFC").expect("white text"),
        "middle",
    );

    // Draw box with rounded corners
    let rect = Rect::new(Point::new(100.0, 100.0), Size::new(250.0, 120.0)).with_radius(8.0, 8.0);
    let fill = Fill::new(Color::from_hex("#2563EB").expect("sapphire blue"), 0.9);
    let stroke = Stroke::new(Color::from_hex("#38BDF8").expect("cyan border"), 2.0);
    canvas.draw_rect(rect, Some(stroke), Some(fill));

    // Draw circle node
    canvas.draw_circle(
        Point::new(550.0, 160.0),
        45.0,
        Some(Stroke::new(Color::from_hex("#059669").expect("green"), 3.0)),
        Some(Fill::new(
            Color::from_hex("#10B981").expect("green fill"),
            0.85,
        )),
    );

    // Draw connecting bezier curve using builder chaining
    let path = Path::new().move_to(Point::new(350.0, 160.0)).cubic_to(
        Point::new(420.0, 120.0),
        Point::new(450.0, 200.0),
        Point::new(505.0, 160.0),
    );
    let arrow_stroke = Stroke::new(Color::from_hex("#94A3B8").expect("slate"), 2.5);
    canvas.draw_path(&path, Some(arrow_stroke), None);

    let svg = canvas.to_svg();

    assert!(svg.starts_with("<svg"), "Must start with <svg tag");
    assert!(
        svg.trim_end().ends_with("</svg>"),
        "Must end with </svg> tag"
    );
    assert!(svg.contains("viewBox=\"0 0 800 600\""));
    assert!(
        svg.contains("&lt;Active&gt;"),
        "XML entities must be escaped"
    );
    assert!(svg.contains("rx=\"8\""));
    assert!(svg.contains("d=\"M350 160 C420 120 450 200 505 160\""));
}

#[test]
fn test_font_metrics_and_theme_palette() {
    let text = "Quarterly Financial Analysis 2026";
    let width_16 = FontMetrics::estimate_text_width(text, 16.0);
    let width_24 = FontMetrics::estimate_text_width(text, 24.0);

    assert!(
        width_24 > width_16,
        "Larger font size must yield wider width"
    );
    assert_eq!(
        width_24 / 24.0,
        width_16 / 16.0,
        "Width scaling must be strictly proportional"
    );

    let cjk_text = "東京データ";
    let cjk_width = FontMetrics::estimate_text_width(cjk_text, 20.0);
    // 5 CJK characters at 1.0 em * 20.0 = 100.0
    assert_eq!(
        cjk_width, 100.0,
        "CJK characters must accurately calculate at 1.0 em"
    );

    let light_palette = Palette::new(ThemeMode::Light);
    let dark_palette = Palette::new(ThemeMode::Dark);

    assert_eq!(
        light_palette.get_categorical(0),
        Color::from_hex("#2563EB").unwrap()
    );
    assert_eq!(
        light_palette.get_categorical(8),
        Color::from_hex("#2563EB").unwrap(),
        "Index must cycle modulo 8"
    );
    assert_ne!(light_palette.background(), dark_palette.background());
}
