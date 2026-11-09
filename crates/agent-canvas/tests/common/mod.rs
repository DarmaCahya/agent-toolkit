//! Helpers shared by the LaTeX integration tests.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

const FORBIDDEN: [&str; 8] = [
    "<text",
    "<tspan",
    "<style",
    "<script",
    "<foreignObject",
    "<image",
    "font-family",
    "javascript:",
];

const ALLOWED_ELEMENTS: [&str; 5] = ["svg", "defs", "path", "use", "rect"];

/// Splits the document into tag bodies (text between `<` and `>`).
fn tags(svg: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        let end = after.find('>').expect("unterminated tag");
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

/// Minimal well-formedness check: balanced tags, allowed elements only.
pub fn assert_well_formed(svg: &str) {
    let mut stack: Vec<&str> = Vec::new();
    for tag in tags(svg) {
        if let Some(name) = tag.strip_prefix('/') {
            assert_eq!(
                stack.pop(),
                Some(name.trim()),
                "unbalanced close tag in {svg}"
            );
            continue;
        }
        let self_closing = tag.ends_with('/');
        let name = tag
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap();
        assert!(
            ALLOWED_ELEMENTS.contains(&name),
            "element <{name}> not allowed"
        );
        if !self_closing {
            stack.push(name);
        }
    }
    assert!(stack.is_empty(), "unclosed tags: {stack:?}");
}

/// Forbidden-content scan from the SVG output contract.
pub fn assert_safe_svg(svg: &str) {
    for f in FORBIDDEN {
        assert!(!svg.contains(f), "forbidden content {f:?}");
    }
    // `on*=` event attributes and non-local hrefs.
    for tag in tags(svg) {
        for attr in tag.split_whitespace().skip(1) {
            let name = attr.split('=').next().unwrap();
            assert!(
                !(name.starts_with("on") && attr.contains('=')),
                "event attribute {attr}"
            );
        }
    }
    let mut rest = svg;
    while let Some(i) = rest.find("href=\"") {
        let v = &rest[i + 6..];
        assert!(v.starts_with('#'), "non-local href");
        rest = v;
    }
    assert!(!svg.contains("href='"), "single-quoted href");
    assert_finite_numbers(svg);
}

/// No NaN / inf / exponent notation in attribute values.
pub fn assert_finite_numbers(svg: &str) {
    assert!(
        !svg.contains("NaN") && !svg.contains("inf"),
        "non-finite number"
    );
    let mut rest = svg;
    while let Some(i) = rest.find("=\"") {
        let v = &rest[i + 2..];
        let end = v.find('"').unwrap();
        let val = &v[..end];
        let numeric = val.chars().all(|c| {
            c.is_ascii_digit() || " .-,()translatescalMLQCZ#".contains(c) || c.is_ascii_alphabetic()
        });
        if numeric && val.chars().any(|c| c.is_ascii_digit()) {
            assert!(
                !val.contains("e+")
                    && !val.contains("e-")
                    && !val.contains("E+")
                    && !val.contains("E-"),
                "exponent in {val}"
            );
        }
        rest = &v[end + 1..];
    }
}

/// `x` offsets of every `<use>` element, in document order.
pub fn use_xs(svg: &str) -> Vec<f64> {
    svg.lines()
        .filter(|l| l.starts_with("<use"))
        .map(|l| {
            let t = l.split("translate(").nth(1).unwrap();
            t.split([' ', ')']).next().unwrap().parse::<f64>().unwrap()
        })
        .collect()
}

/// Parses `viewBox="a b c d"`.
pub fn view_box(svg: &str) -> [f64; 4] {
    let v = svg
        .split("viewBox=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let n: Vec<f64> = v.split(' ').map(|x| x.parse().unwrap()).collect();
    [n[0], n[1], n[2], n[3]]
}
