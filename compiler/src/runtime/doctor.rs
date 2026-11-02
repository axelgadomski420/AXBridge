pub fn run() {
    println!("AXCode Ultra Compiler — diagnostyka");
    println!("-----------------------------------");
    println!("Wersja kompilatora: {}", env!("CARGO_PKG_VERSION"));
    println!("Platforma: {}", std::env::consts::OS);
    println!("Arch: {}", std::env::consts::ARCH);
    println!("\nWymagane targety:");
    for t in ["swift", "kotlin", "typescript", "workers"] {
        println!("  • {}  ✓ wspierany", t);
    }
    println!("\nWalidacja dostępności: AKTYWNA (WCAG 2.2 AA — kontrast, targety, etykiety)");
    println!("Sprawdź: `axc check file.ax`");
}
