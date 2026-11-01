use std::env;
use std::fs;
use std::process;
use std::time::Instant;
use vizier_pdf::{PdfGenerator, PdfRasterizer, PdfReader};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: vizier_pdf_cli <read|render|generate|cycle|benchmark> [args...]"
        );
        process::exit(1);
    }

    match args[1].as_str() {
        "read" => {
            if args.len() < 3 {
                eprintln!("Usage: vizier_pdf_cli read <input.pdf>");
                process::exit(1);
            }
            let path = &args[2];
            let start = Instant::now();
            let bytes = fs::read(path).unwrap_or_else(|e| {
                eprintln!("Failed to read file {}: {}", path, e);
                process::exit(1);
            });
            let page_count = PdfReader::page_count(&bytes).unwrap_or(0);
            let md = PdfReader::extract_markdown(&bytes).unwrap_or_else(|e| {
                eprintln!("Read error: {}", e);
                process::exit(1);
            });
            let elapsed = start.elapsed();
            println!(
                "Successfully extracted Markdown ({} pages, {} chars) in {:.4}s:\n--- PREVIEW ---\n{}\n--- END PREVIEW ---",
                page_count,
                md.len(),
                elapsed.as_secs_f64(),
                &md[..md.len().min(600)]
            );
        }
        "render" => {
            if args.len() < 4 {
                eprintln!("Usage: vizier_pdf_cli render <input.pdf> <out.png> [scale]");
                process::exit(1);
            }
            let input_path = &args[2];
            let output_path = &args[3];
            let scale: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2.0);

            let bytes = fs::read(input_path).unwrap_or_else(|e| {
                eprintln!("Failed to read file {}: {}", input_path, e);
                process::exit(1);
            });

            let start = Instant::now();
            let image = PdfRasterizer::render_page(&bytes, 0, scale).unwrap_or_else(|e| {
                eprintln!("Render error: {}", e);
                process::exit(1);
            });
            image.save_png(output_path).unwrap_or_else(|e| {
                eprintln!("Failed to save PNG: {}", e);
                process::exit(1);
            });
            let elapsed = start.elapsed();
            println!(
                "Rendered page 0 at {}x scale ({}x{} px) to {} in {:.4}s",
                scale,
                image.width,
                image.height,
                output_path,
                elapsed.as_secs_f64()
            );
        }
        "generate" => {
            if args.len() < 4 {
                eprintln!("Usage: vizier_pdf_cli generate <input.html> <out.pdf>");
                process::exit(1);
            }
            let html_path = &args[2];
            let out_pdf = &args[3];
            let html = fs::read_to_string(html_path).unwrap_or_else(|e| {
                eprintln!("Failed to read HTML {}: {}", html_path, e);
                process::exit(1);
            });

            let start = Instant::now();
            let pdf_bytes = PdfGenerator::from_html(&html).unwrap_or_else(|e| {
                eprintln!("Generation error: {}", e);
                process::exit(1);
            });
            fs::write(out_pdf, &pdf_bytes).unwrap_or_else(|e| {
                eprintln!("Failed to write PDF: {}", e);
                process::exit(1);
            });
            let elapsed = start.elapsed();
            println!(
                "Generated PDF from HTML ({} bytes) to {} in {:.4}s",
                pdf_bytes.len(),
                out_pdf,
                elapsed.as_secs_f64()
            );
        }
        "cycle" => {
            if args.len() < 5 {
                eprintln!("Usage: vizier_pdf_cli cycle <input.html> <out.pdf> <out.png>");
                process::exit(1);
            }
            let html_path = &args[2];
            let out_pdf = &args[3];
            let out_png = &args[4];

            let html = fs::read_to_string(html_path).expect("Read HTML");

            // 1. Generate HTML -> PDF
            let t0 = Instant::now();
            let pdf_bytes = PdfGenerator::from_html(&html).expect("Generate PDF");
            fs::write(out_pdf, &pdf_bytes).expect("Save PDF");
            let gen_dur = t0.elapsed();

            // 2. Read PDF -> Markdown
            let t1 = Instant::now();
            let md = PdfReader::extract_markdown(&pdf_bytes).expect("Extract Markdown");
            let read_dur = t1.elapsed();

            // 3. Rasterize PDF -> PNG
            let t2 = Instant::now();
            let img = PdfRasterizer::render_page(&pdf_bytes, 0, 2.0).expect("Rasterize Page");
            img.save_png(out_png).expect("Save PNG");
            let render_dur = t2.elapsed();

            println!("Cycle Test Completed:");
            println!("- Generation:  {:.4}s ({} bytes PDF)", gen_dur.as_secs_f64(), pdf_bytes.len());
            println!("- Read / MD:   {:.4}s ({} chars extracted)", read_dur.as_secs_f64(), md.len());
            println!("- Rasterize:   {:.4}s ({}x{} PNG)", render_dur.as_secs_f64(), img.width, img.height);
        }
        "benchmark" => {
            let corpus_dir = args.get(2).map(|s| s.as_str()).unwrap_or("/srv/hermes/workspace/pdf-benchmarks/corpus");
            println!("Running automated benchmark on corpus: {}", corpus_dir);

            let entries = match fs::read_dir(corpus_dir) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Cannot open corpus dir {}: {}", corpus_dir, e);
                    process::exit(1);
                }
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                    let file_name = path.file_name().unwrap().to_string_lossy();
                    let bytes = fs::read(&path).unwrap();
                    let size_kb = bytes.len() as f64 / 1024.0;

                    let t0 = Instant::now();
                    let count = PdfReader::page_count(&bytes).unwrap_or(0);
                    let md = PdfReader::extract_markdown(&bytes).unwrap_or_default();
                    let read_time = t0.elapsed();

                    let t1 = Instant::now();
                    let render_res = PdfRasterizer::render_page(&bytes, 0, 1.0);
                    let render_time = t1.elapsed();

                    println!(
                        "File: {:<20} | Size: {:>7.1} KB | Pages: {:>3} | Read: {:>7.3}s ({:>7} chars) | Render P1: {:>7.3}s (ok: {})",
                        file_name,
                        size_kb,
                        count,
                        read_time.as_secs_f64(),
                        md.len(),
                        render_time.as_secs_f64(),
                        render_res.is_ok()
                    );
                }
            }
        }
        other => {
            eprintln!("Unknown command: {}", other);
            process::exit(1);
        }
    }
}
