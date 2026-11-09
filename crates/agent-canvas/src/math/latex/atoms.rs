//! TeX atoms: classes, styles, inter-atom spacing and AST-to-atom conversion.

use super::font::MathConsts;
use super::parser::{BigOpKind, Node, NodeKind};

/// TeX atom classes (the `Inner` class is omitted in v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtomClass {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
}

/// Maps an input character to the code point of the math glyph to draw.
///
/// Latin letters use the Mathematical Italic block (`h` is U+210E), hyphen-minus
/// becomes U+2212, everything else is unchanged.
pub(crate) fn math_char(c: char) -> char {
    match c {
        'h' => '\u{210E}',
        'a'..='z' => offset_char('\u{1D44E}', c, 'a'),
        'A'..='Z' => offset_char('\u{1D434}', c, 'A'),
        '-' => '\u{2212}',
        other => other,
    }
}

fn offset_char(base: char, c: char, first: char) -> char {
    let delta = u32::from(c).saturating_sub(u32::from(first));
    char::from_u32(u32::from(base).saturating_add(delta)).unwrap_or(c)
}

/// Math style size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StyleSize {
    D,
    T,
    S,
    SS,
}

/// TeX math style: size plus cramped flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Style {
    pub size: StyleSize,
    pub cramped: bool,
}

impl Style {
    pub(crate) const DISPLAY: Style = Style {
        size: StyleSize::D,
        cramped: false,
    };

    fn script_size(self) -> StyleSize {
        match self.size {
            StyleSize::D | StyleSize::T => StyleSize::S,
            StyleSize::S | StyleSize::SS => StyleSize::SS,
        }
    }

    pub(crate) fn sup(self) -> Style {
        Style {
            size: self.script_size(),
            cramped: self.cramped,
        }
    }

    pub(crate) fn sub(self) -> Style {
        Style {
            size: self.script_size(),
            cramped: true,
        }
    }

    pub(crate) fn frac_num(self) -> Style {
        Style {
            size: match self.size {
                StyleSize::D => StyleSize::T,
                StyleSize::T => StyleSize::S,
                StyleSize::S | StyleSize::SS => StyleSize::SS,
            },
            cramped: self.cramped,
        }
    }

    pub(crate) fn frac_den(self) -> Style {
        Style {
            cramped: true,
            ..self.frac_num()
        }
    }

    pub(crate) fn cramp(self) -> Style {
        Style {
            cramped: true,
            ..self
        }
    }

    /// Scriptscript, non-cramped (radical index).
    pub(crate) fn script_script(self) -> Style {
        Style {
            size: StyleSize::SS,
            cramped: false,
        }
    }

    pub(crate) fn is_script(self) -> bool {
        self.size >= StyleSize::S
    }

    pub(crate) fn is_display(self) -> bool {
        self.size == StyleSize::D
    }

    /// Size factor of this style relative to the base size, from the font's
    /// script scale-down constants.
    pub(crate) fn scale(self, c: &MathConsts) -> f64 {
        match self.size {
            StyleSize::D | StyleSize::T => 1.0,
            StyleSize::S => c.script_percent_scale_down / 100.0,
            StyleSize::SS => c.script_script_percent_scale_down / 100.0,
        }
    }

    /// One mu in em at this style (18 mu = 1 em).
    pub(crate) fn mu_scale(self, c: &MathConsts) -> f64 {
        self.scale(c) / 18.0
    }
}

/// Inter-atom space in mu from the TeX table. Entries that TeX gives in
/// parentheses apply to non-script styles only.
pub(crate) fn space(left: AtomClass, right: AtomClass, style: Style) -> f64 {
    use AtomClass::{Bin, Close, Op, Open, Ord, Punct, Rel};
    // Positive: always; negative: non-script only. Magnitude: 1 thin, 2 medium, 3 thick.
    let code: i8 = match (left, right) {
        (Ord, Op) | (Op, Ord | Op) | (Close, Op) => 1,
        (Ord, Bin) | (Bin, Ord | Op | Open) | (Close, Bin) => -2,
        (Ord, Rel) | (Op, Rel) | (Rel, Ord | Op | Open) | (Close, Rel) => -3,
        (Punct, Ord | Op | Rel | Open | Close | Punct) => -1,
        _ => 0,
    };
    let mu = match code.unsigned_abs() {
        1 => 3.0,
        2 => 4.0,
        3 => 5.0,
        _ => 0.0,
    };
    if code < 0 && style.is_script() {
        0.0
    } else {
        mu
    }
}

/// An atom: a class plus its content.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Atom {
    pub class: AtomClass,
    pub content: AtomContent,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AtomContent {
    /// A single glyph by final code point.
    Glyph(char),
    /// A braced group laid out as one Ord atom.
    List(Vec<Atom>),
    Frac {
        num: Vec<Atom>,
        den: Vec<Atom>,
    },
    Sqrt {
        index: Option<Vec<Atom>>,
        body: Vec<Atom>,
    },
    Scripts {
        nucleus: Box<Atom>,
        sup: Option<Vec<Atom>>,
        sub: Option<Vec<Atom>>,
    },
    BigOp {
        ch: char,
        /// Integral-like: scripts always at the side.
        side_scripts: bool,
        sup: Option<Vec<Atom>>,
        sub: Option<Vec<Atom>>,
    },
}

/// Converts a node list into atoms, applying Bin-to-Ord demotion.
pub(crate) fn build_atoms(nodes: &[Node]) -> Vec<Atom> {
    let mut atoms: Vec<Atom> = nodes.iter().map(node_to_atom).collect();
    demote_bins(&mut atoms);
    atoms
}

fn arg_atoms(node: &Node) -> Vec<Atom> {
    match &node.kind {
        NodeKind::Group(items) => build_atoms(items),
        _ => build_atoms(std::slice::from_ref(node)),
    }
}

fn node_to_atom(node: &Node) -> Atom {
    match &node.kind {
        NodeKind::Symbol(sym) => Atom {
            class: sym.class,
            content: AtomContent::Glyph(sym.ch),
        },
        NodeKind::Group(items) => Atom {
            class: AtomClass::Ord,
            content: AtomContent::List(build_atoms(items)),
        },
        NodeKind::Frac { num, den } => Atom {
            class: AtomClass::Ord,
            content: AtomContent::Frac {
                num: arg_atoms(num),
                den: arg_atoms(den),
            },
        },
        NodeKind::Sqrt { index, body } => Atom {
            class: AtomClass::Ord,
            content: AtomContent::Sqrt {
                index: index.as_deref().map(arg_atoms),
                body: arg_atoms(body),
            },
        },
        NodeKind::Scripts { base, sup, sub } => {
            let nucleus = node_to_atom(base);
            Atom {
                class: nucleus.class,
                content: AtomContent::Scripts {
                    nucleus: Box::new(nucleus),
                    sup: sup.as_deref().map(arg_atoms),
                    sub: sub.as_deref().map(arg_atoms),
                },
            }
        }
        NodeKind::BigOp { op, sup, sub } => Atom {
            class: AtomClass::Op,
            content: AtomContent::BigOp {
                ch: op.glyph_char(),
                side_scripts: matches!(op, BigOpKind::Int),
                sup: sup.as_deref().map(arg_atoms),
                sub: sub.as_deref().map(arg_atoms),
            },
        },
    }
}

fn demote_bins(atoms: &mut [Atom]) {
    use AtomClass::{Bin, Close, Op, Open, Ord, Punct, Rel};
    let mut prev: Option<AtomClass> = None;
    for i in 0..atoms.len() {
        let cur = atoms.get(i).map(|a| a.class);
        match (prev, cur) {
            (None | Some(Bin | Op | Rel | Open | Punct), Some(Bin)) => {
                if let Some(a) = atoms.get_mut(i) {
                    a.class = Ord;
                }
            }
            (Some(Bin), Some(Rel | Close | Punct)) => {
                if let Some(p) = i.checked_sub(1).and_then(|j| atoms.get_mut(j)) {
                    p.class = Ord;
                }
            }
            _ => {}
        }
        prev = atoms.get(i).map(|a| a.class);
    }
    if let Some(last) = atoms.last_mut() {
        if last.class == Bin {
            last.class = Ord;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::math::latex::lexer::lex;
    use crate::math::latex::parser::parse;

    fn classes(s: &str) -> Vec<AtomClass> {
        let toks = lex(s).unwrap();
        let root = parse(&toks, s.chars().count()).unwrap();
        let NodeKind::Group(items) = &root.kind else {
            panic!("root is a group")
        };
        build_atoms(items).iter().map(|a| a.class).collect()
    }

    #[test]
    fn class_assignment() {
        use AtomClass::*;
        assert_eq!(
            classes(r"a1+b\pm c\cdot d\times e=<>\leq\geq\neq(),\sum\int\prod"),
            vec![
                Ord, Ord, Bin, Ord, Bin, Ord, Bin, Ord, Bin, Ord, Rel, Rel, Rel, Rel, Rel, Rel,
                Open, Close, Punct, Op, Op, Op
            ]
        );
    }

    #[test]
    fn bin_demotion() {
        use AtomClass::*;
        assert_eq!(classes("-b"), vec![Ord, Ord]); // start
        assert_eq!(classes("a=-b"), vec![Ord, Rel, Ord, Ord]); // after Rel
        assert_eq!(classes("a+"), vec![Ord, Ord]); // at end
        assert_eq!(classes("a+-b"), vec![Ord, Bin, Ord, Ord]); // after Bin
        assert_eq!(classes("(-b"), vec![Open, Ord, Ord]); // after Open
        assert_eq!(classes("a,-b"), vec![Ord, Punct, Ord, Ord]); // after Punct
        assert_eq!(classes(r"\sum-b"), vec![Op, Ord, Ord]); // after Op
        assert_eq!(classes("a+=b"), vec![Ord, Ord, Rel, Ord]); // before Rel
        assert_eq!(classes("(a+)"), vec![Open, Ord, Ord, Close]); // before Close
        assert_eq!(classes("a+,b"), vec![Ord, Ord, Punct, Ord]); // before Punct
        assert_eq!(classes("a-b"), vec![Ord, Bin, Ord]);
    }

    #[test]
    fn spacing_table() {
        use AtomClass::*;
        let t = Style {
            size: StyleSize::T,
            cramped: false,
        };
        let s = Style {
            size: StyleSize::S,
            cramped: false,
        };
        assert_eq!(space(Ord, Bin, t), 4.0);
        assert_eq!(space(Bin, Ord, t), 4.0);
        assert_eq!(space(Ord, Rel, t), 5.0);
        assert_eq!(space(Ord, Op, t), 3.0);
        assert_eq!(space(Ord, Ord, t), 0.0);
        assert_eq!(space(Punct, Ord, t), 3.0);
        assert_eq!(space(Open, Ord, t), 0.0);
        assert_eq!(space(Ord, Close, t), 0.0);
        // Medium and thick spaces vanish in script styles; Ord-Op thin stays.
        assert_eq!(space(Ord, Bin, s), 0.0);
        assert_eq!(space(Ord, Rel, s), 0.0);
        assert_eq!(space(Ord, Op, s), 3.0);
        assert_eq!(space(Punct, Ord, s), 0.0);
    }

    #[test]
    fn style_transitions() {
        use StyleSize::*;
        let d = Style::DISPLAY;
        assert_eq!(d.sup().size, S);
        assert!(!d.sup().cramped);
        assert!(d.sub().cramped);
        assert_eq!(d.frac_num().size, T);
        assert_eq!(d.frac_den().size, T);
        assert!(d.frac_den().cramped);
        assert!(!d.frac_num().cramped);
        let t = Style {
            size: T,
            cramped: false,
        };
        assert_eq!(t.frac_num().size, S);
        assert_eq!(d.sup().sup().size, SS);
        assert_eq!(d.sup().sup().sup().size, SS);
        assert!(d.cramp().cramped);
        assert!(d.sup().is_script() && !d.is_script() && d.is_display());
    }

    #[test]
    fn style_scale_from_font() {
        let font = crate::math::latex::font::MathFont::load().unwrap();
        let c = font.consts();
        let d = Style::DISPLAY;
        assert!((d.scale(c) - 1.0).abs() < 1e-9);
        assert!((d.sup().scale(c) - c.script_percent_scale_down / 100.0).abs() < 1e-9);
        assert!((d.sup().sup().scale(c) - c.script_script_percent_scale_down / 100.0).abs() < 1e-9);
        assert!((d.mu_scale(c) - 1.0 / 18.0).abs() < 1e-9);
    }

    #[test]
    fn letter_mapping() {
        assert_eq!(math_char('a'), '\u{1D44E}');
        assert_eq!(math_char('z'), '\u{1D467}');
        assert_eq!(math_char('A'), '\u{1D434}');
        assert_eq!(math_char('h'), '\u{210E}');
        assert_eq!(math_char('-'), '\u{2212}');
        assert_eq!(math_char('7'), '7');
        assert_eq!(math_char('+'), '+');
    }
}
