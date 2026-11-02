import subprocess
import os

CLAUDE_BIN = "/home/agent/.hermes/bin/claude"
SRC_DIR = "/srv/hermes/workspace/unified-crate/src"

files_to_review = ["lib.rs", "error.rs", "types.rs", "sandbox.rs", "reader.rs", "rasterizer.rs", "generator.rs"]
code_bundle = ""

for f in files_to_review:
    path = os.path.join(SRC_DIR, f)
    with open(path, "r", encoding="utf-8") as file:
        code_bundle += f"\n\n// ===== FILE: {f} =====\n" + file.read()

prompt = f"""Kamu adalah Principal Rust Systems Architect dan Reviewer Keamanan Tingkat Tinggi (Hostile Code Auditor).
Lakukan audit kode paling kritis dan kejam terhadap seluruh source code crate PDF Facade untuk AI Agent berikut ini:

{code_bundle}

FOKUS AUDIT KAMU:
1. Panic safety: Apakah ada unwrap(), expect(), panic!, atau potensi divide by zero, index out of bounds, slicing string non-char boundary?
2. Memory safety & DoS: Bisakah input jahat (malformed/zip-bomb/huge dimensions) menyebabkan unbounded memory allocation, integer overflow, atau process crash?
3. Agent Ergonomics: Apakah ada kasus di mana agen AI mendapatkan data yang salah atau error yang tidak bisa ditindaklanjuti (unactionable error)?
4. Type Leakage: Apakah ada tipe dari pdf_oxide, hayro, atau fulgur yang bocor ke public interface?
5. Concurrency & Send/Sync: Apakah tipe publik aman dikirim antar thread (Send + Sync)?

Tuliskan review terperinci dalam Bahasa Indonesia. Jika ada celah, sebutkan NAMA FILE, BARIS/KODE, dan SARAN PERBAIKAN SPESIFIKNYA. Jika kode sudah 100% sempurna tanpa celah, katakan 'ZERO VULNERABILITIES APPROVED'."""

cmd = [CLAUDE_BIN, "-p", prompt]
res = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
print("=== CLAUDE AUDIT REVIEW REPORT ===")
print(res.stdout)
if res.returncode != 0:
    print("Error:", res.stderr)
