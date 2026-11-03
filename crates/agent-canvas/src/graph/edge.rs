#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    Solid,
    Dotted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from_id: String,
    pub to_id: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from_id: from.into(),
            to_id: to.into(),
            label: None,
            style: EdgeStyle::Solid,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_style(mut self, style: EdgeStyle) -> Self {
        self.style = style;
        self
    }
}
