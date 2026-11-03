use crate::canvas::{Color, Point, Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    /// Standard process rectangle with rounded corners (rx = 6)
    Process,
    /// Terminal start/end pill/stadium (rx = height / 2)
    Terminal,
    /// Decision diamond rhombus
    Decision,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    pub color: Option<Color>,
    pub rect: Rect,
}

impl Node {
    pub fn new(id: impl Into<String>, label: impl Into<String>, shape: NodeShape) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shape,
            color: None,
            rect: Rect::new(Point::new(0.0, 0.0), Size::new(120.0, 45.0)),
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}
