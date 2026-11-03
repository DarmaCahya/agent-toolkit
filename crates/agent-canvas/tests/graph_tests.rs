use agent_canvas::{Direction, Edge, EdgeStyle, Flowchart, Node, NodeShape, ThemeMode};

#[test]
fn test_flowchart_from_dsl_linear_and_branching() {
    let dsl = r#"
        flowchart TD
        start(Start Pipeline) --> parse[Parse PDF Data]
        parse --> check{Valid?}
        check -->|Yes| render[Generate Vector SVG]
        check -->|No| error[Log Error & Alert]
        render --> endNode(Finish & Save)
    "#;

    let chart = Flowchart::from_dsl(dsl).expect("parse valid dsl");
    assert_eq!(chart.direction, Direction::TopToBottom);
    assert_eq!(chart.nodes.len(), 6);
    assert_eq!(chart.edges.len(), 5);

    let svg = chart.to_svg().expect("render svg");
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains(r#"id="arrowhead""#), "Must embed arrowhead marker");
    assert!(svg.contains("marker-end=\"url(#arrowhead)\""));
    assert!(svg.contains("Start Pipeline"));
    assert!(svg.contains("Generate Vector SVG"));
    assert!(svg.contains("Yes"), "Must render edge label badge");
    assert!(svg.contains("No"), "Must render edge label badge");
}

#[test]
fn test_flowchart_left_to_right_orientation() {
    let dsl = r#"
        graph LR
        A[Client] --> B[API Gateway] --> C[Service Worker]
    "#;

    let chart = Flowchart::from_dsl(dsl).expect("parse LR dsl");
    assert_eq!(chart.direction, Direction::LeftToRight);
    let svg = chart.to_svg().expect("render LR svg");
    assert!(svg.contains("API Gateway"));
    assert!(svg.contains("Service Worker"));
}

#[test]
fn test_dsl_fault_tolerance_and_resilience() {
    // Dirty, messy DSL from LLM hallucinations: trailing arrows, extra comments, empty brackets
    let dirty_dsl = r#"
        flowchart TD
        %% This is a comment
        [Valid Node 1] --> [Valid Node 2]
        # Another shell style comment
        broken_arrow ---> next
        random empty line

        trailing_arrow -->
        single_node
    "#;

    // Must not panic, must recover gracefully
    let chart = Flowchart::from_dsl(dirty_dsl).expect("resilient parse");
    assert!(!chart.nodes.is_empty());
    let svg = chart.to_svg().expect("render resilient svg");
    assert!(svg.contains("Valid Node 1"));
}

#[test]
fn test_programmatic_flowchart_api() {
    let chart = Flowchart::new(Direction::TopToBottom)
        .title("Programmatic Architecture")
        .with_theme(ThemeMode::Dark)
        .add_node(Node::new("1", "User Input", NodeShape::Terminal))
        .add_node(Node::new("2", "Process Worker", NodeShape::Process))
        .add_edge(Edge::new("1", "2").with_label("Submit").with_style(EdgeStyle::Solid));

    let svg = chart.to_svg().expect("render programmatic flowchart");
    assert!(svg.contains("Programmatic Architecture"));
    assert!(svg.contains("User Input"));
    assert!(svg.contains("Process Worker"));
    assert!(svg.contains("Submit"));
}
