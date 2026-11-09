//! Renders the quadratic formula (or a formula given as the first argument) to
//! `quadratic.svg` (or the path given as the second argument).

use agent_canvas::math::MathTypesetter;

fn main() {
    let mut args = std::env::args().skip(1);
    let latex = args
        .next()
        .unwrap_or_else(|| r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}".to_string());
    let path = args.next().unwrap_or_else(|| "quadratic.svg".to_string());
    match MathTypesetter::new().render_to_svg(&latex) {
        Ok(svg) => {
            if let Err(e) = std::fs::write(&path, svg) {
                eprintln!("cannot write {path}: {e}");
                std::process::exit(1);
            }
            println!("wrote {path}");
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
