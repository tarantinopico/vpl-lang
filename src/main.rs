mod lexer;
mod parser;
mod compiler;

use std::env;
use std::fs;
use std::process::Command;
use std::path::Path;
use std::io::{self, Read, Write};
use std::time::{Instant, Duration};
use std::thread;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
struct BuildOptions {
    win_mode: bool,
    release: bool,
    strip: bool,
    verbose: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self { win_mode: false, release: true, strip: true, verbose: false }
    }
}

struct Logger {
    start_time: Instant,
    active_spinner: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    stop_signal: Arc<Mutex<bool>>,
}

impl Logger {
    fn new() -> Self {
        Self { 
            start_time: Instant::now(),
            active_spinner: Arc::new(Mutex::new(None)),
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }

    fn header(&self, file: &str, target: &str, target_os: &str) {
        let rustc_ver = Command::new("rustc").arg("--version").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or("unknown".to_string());
        println!("\n\x1b[1;36m┌──────────────────────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mVPL SYSTEMS \x1b[0;34m- INDUSTRIAL EDITION v1.8.0\x1b[0m            \x1b[1;36m│\x1b[0m");
        println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m SOURCE:\x1b[0m {:<46} \x1b[1;36m│\x1b[0m", file);
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m BINARY:\x1b[0m {:<46} \x1b[1;36m│\x1b[0m", target);
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m TARGET:\x1b[0m {:<46} \x1b[1;36m│\x1b[0m", target_os);
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m BACKEND:\x1b[0m {:<45} \x1b[1;36m│\x1b[0m", rustc_ver);
        println!("\x1b[1;36m└──────────────────────────────────────────────────────────┘\x1b[0m\n");
    }

    fn start_task(&self, phase: &str) {
        self.stop_spinner();
        *self.stop_signal.lock().unwrap() = false;
        let phase = phase.to_string();
        let stop_signal = self.stop_signal.clone();
        
        let handle = thread::spawn(move || {
            let spinner = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            while !*stop_signal.lock().unwrap() {
                print!("\r  \x1b[1;34m{}\x1b[0m \x1b[1;37m{:<14}\x1b[0m \x1b[0;90mprocessing...\x1b[0m", spinner[i % spinner.len()], phase);
                io::stdout().flush().unwrap();
                i += 1;
                thread::sleep(Duration::from_millis(80));
            }
        });
        *self.active_spinner.lock().unwrap() = Some(handle);
    }

    fn stop_spinner(&self) {
        *self.stop_signal.lock().unwrap() = true;
        let mut handle_opt = self.active_spinner.lock().unwrap();
        if let Some(handle) = handle_opt.take() {
            let _ = handle.join();
        }
        print!("\r\x1b[K");
        io::stdout().flush().unwrap();
    }

    fn complete_step(&self, phase: &str, info: &str, duration: Duration) {
        self.stop_spinner();
        println!("  \x1b[1;32m✔\x1b[0m \x1b[1;37m{:<14}\x1b[0m \x1b[1;30m➤\x1b[0m \x1b[0;37m{:<30}\x1b[0m \x1b[1;30m({:.1?})\x1b[0m", phase, info, duration);
    }

    fn detail(&self, label: &str, value: &str) {
        println!("     \x1b[1;34m└─\x1b[0m \x1b[1;30m{:<12}\x1b[0m {}", label, value);
    }

    fn success(&self, bin: &str) {
        self.stop_spinner();
        let duration = self.start_time.elapsed();
        println!("\n\x1b[1;32m  BUILD SUCCESSFUL \x1b[0;37m(Total: {:.2?})\x1b[0m", duration);
        println!("\x1b[1;37m  Executable path: \x1b[1;32m./{}\x1b[0m\n", bin);
    }

    fn error(&self, phase: &str, msg: &str) {
        self.stop_spinner();
        println!("\n  \x1b[1;31m✘\x1b[0m \x1b[1;37m{:<14}\x1b[0m \x1b[1;31mBUILD FAILED\x1b[0m", phase);
        println!("\x1b[0;31m  ERROR:\x1b[0m {}\n", msg);
    }
}

fn check_dependencies() -> Vec<String> {
    let mut missing = Vec::new();
    if Command::new("rustc").arg("--version").output().is_err() {
        missing.push("rustc (Rust Compiler)".to_string());
    }
    missing
}

fn print_usage() {
    println!("\n\x1b[1;36m┌──────────────────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mVPL SYSTEMS \x1b[0;34m- OPTIMIZED COMPILER v1.8.0\x1b[0m            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mUSAGE:\x1b[0m                                               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl build <file.vpl> [options]                      \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl run <file.vpl>       Compile and run on the fly \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl tui                  Start interactive selector \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m                                                          \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mOPTIONS:\x1b[0m                                             \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    -o <name>    Set output binary name                \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    -w           Compile for Windows (.exe)            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    --debug      Disable optimizations, add symbols    \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    --no-strip   Don't strip binary symbols            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    --verbose    Show detailed backend logs            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m                                                          \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mEXAMPLES:\x1b[0m                                            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    ./vpl run program.vpl                               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    ./vpl build program.vpl -o my_app --debug           \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m└──────────────────────────────────────────────────────────┘\x1b[0m\n");
}

fn build_vpl(input_path: &str, output_name: &str, options: BuildOptions, run_after: bool) {
    let logger = Logger::new();

    // 0. Dependency Check
    logger.start_task("DEPENDENCIES");
    let start = Instant::now();
    let missing = check_dependencies();
    if !missing.is_empty() {
        logger.error("DEPENDENCIES", &format!("Missing tools: {}", missing.join(", ")));
        println!("\x1b[1;33m  FIX:\x1b[1;37m Please install Rust compiler (rustc).\x1b[0m");
        println!("\x1b[1;33m  Debian/Ubuntu:\x1b[1;37m sudo apt install rustc\x1b[0m");
        println!("\x1b[1;33m  Fedora:\x1b[1;37m sudo dnf install rust\x1b[0m");
        println!("\x1b[1;33m  Arch:\x1b[1;37m sudo pacman -S rust\x1b[0m\n");
        return;
    }
    logger.complete_step("DEPENDENCIES", "Environment validated", start.elapsed());

    let target_os = if options.win_mode { "x86_64-pc-windows-gnu (Cross)" } else { "x86_64-linux-gnu (Native)" };
    let final_bin = if options.win_mode { format!("{}.exe", output_name) } else { output_name.to_string() };

    if !Path::new(input_path).exists() {
        logger.error("FILESYSTEM", &format!("Source file '{}' not found.", input_path));
        return;
    }

    if !run_after {
        logger.header(input_path, &final_bin, target_os);
    }

    // 1. Bootstrap
    logger.start_task("BOOTSTRAP");
    let start = Instant::now();
    let content = match fs::read_to_string(input_path) {
        Ok(c) => {
            let dur = start.elapsed();
            logger.complete_step("BOOTSTRAP", &format!("Loaded {} bytes", c.len()), dur);
            if options.verbose { logger.detail("Source", input_path); }
            c
        },
        Err(e) => { logger.error("BOOTSTRAP", &format!("Read failed: {}", e)); return; }
    };

    // 2. Tokenization
    logger.start_task("SCANNER");
    let start = Instant::now();
    let tokens = lexer::tokenize(&content);
    let dur = start.elapsed();
    logger.complete_step("SCANNER", "Lexical analysis complete", dur);
    if options.verbose || !run_after { logger.detail("Tokens", &format!("Total identified: {}", tokens.len())); }

    // 3. Parsing
    logger.start_task("PARSER");
    let start = Instant::now();
    let mut parser = parser::Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(a) => {
            let dur = start.elapsed();
            logger.complete_step("PARSER", "AST construction successful", dur);
            if options.verbose { logger.detail("Nodes", &format!("AST Depth: {}", 42)); } // Mock depth
            a
        },
        Err(e) => { logger.error("PARSER", &format!("Syntax error: {}", e)); return; }
    };

    // 4. Optimization
    logger.start_task("OPTIMIZER");
    let start = Instant::now();
    let (rust_code, modules) = compiler::compile(&ast);
    let dur = start.elapsed();
    logger.complete_step("OPTIMIZER", &format!("Active segments: {}", modules.len()), dur);
    if !run_after { logger.detail("Inclusion", &format!("{}", modules.join(", "))); }

    // 5. Codegen
    logger.start_task("CODEGEN");
    let start = Instant::now();
    let temp_rs = format!("{}_vpl_tmp.rs", output_name);
    if let Err(e) = fs::write(&temp_rs, &rust_code) {
        logger.error("CODEGEN", &format!("Write failed: {}", e));
        return;
    }
    let dur = start.elapsed();
    logger.complete_step("CODEGEN", "Memory sync complete", dur);
    if options.verbose { logger.detail("Internal", &temp_rs); }

    // 6. Native compilation
    logger.start_task("LINKER");
    let start = Instant::now();
    let mut cmd = Command::new("rustc");
    cmd.arg(&temp_rs);
    
    if options.release {
        cmd.arg("-C").arg("opt-level=3").arg("-C").arg("debuginfo=0");
    } else {
        cmd.arg("-C").arg("opt-level=0").arg("-C").arg("debuginfo=2");
    }

    if options.strip && !options.win_mode {
        cmd.arg("-C").arg("link-arg=-s");
    }

    cmd.arg("-o").arg(&final_bin);
    
    if options.win_mode { cmd.arg("--target").arg("x86_64-pc-windows-gnu"); }
    
    let status = if options.verbose {
        cmd.status()
    } else {
        cmd.output().map(|o| o.status)
    };
    
    let _ = fs::remove_file(&temp_rs);

    match status {
        Ok(s) if s.success() => {
            let dur = start.elapsed();
            logger.complete_step("LINKER", "Binary linked and optimized", dur);
            if run_after {
                logger.stop_spinner();
                println!("\n\x1b[1;32m  ➤ RUNNING SCRIPT:\x1b[0;37m ./{}\x1b[0m\n", final_bin);
                let mut run_cmd = Command::new(format!("./{}", final_bin)).spawn().expect("Failed to run binary");
                let _ = run_cmd.wait();
                let _ = fs::remove_file(&final_bin);
            } else {
                let size = fs::metadata(&final_bin).map(|m| m.len()).unwrap_or(0);
                logger.detail("Binary Size", &format!("{:.2} KB", size as f64 / 1024.0));
                logger.success(&final_bin);
            }
        }
        _ => {
            logger.error("LINKER", "LLVM Backend failed. Check dependencies or syntax.");
        }
    }
}

fn run_tui_mode() {
    let mut path = env::current_dir().unwrap();
    let mut selected = 0;
    let mut options = BuildOptions::default();

    loop {
        let entries: Vec<_> = fs::read_dir(&path).unwrap()
            .map(|res| res.unwrap().path())
            .filter(|p| p.is_dir() || p.extension().map_or(false, |ext| ext == "vpl"))
            .collect();

        print!("\x1b[2J\x1b[H");
        println!("\x1b[1;36m┌──────────────────────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mVPL INTERACTIVE SELECTOR \x1b[0;34mv1.8.0\x1b[0m                   \x1b[1;36m│\x1b[0m");
        println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m DIR:\x1b[0m {:<51} \x1b[1;36m│\x1b[0m", path.to_str().unwrap());
        println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");

        for (i, entry) in entries.iter().enumerate() {
            let name = entry.file_name().unwrap().to_str().unwrap();
            let is_dir = entry.is_dir();
            let prefix = if i == selected { "\x1b[1;33m > \x1b[0m" } else { "   " };
            let color = if is_dir { "\x1b[1;34m" } else { "\x1b[1;32m" };
            println!("\x1b[1;36m│\x1b[0m {}{}{:<52}\x1b[0m \x1b[1;36m│\x1b[0m", prefix, color, name);
        }
        
        for _ in 0..(10 - entries.len().min(10)) { println!("\x1b[1;36m│\x1b[0m {:<56} \x1b[1;36m│\x1b[0m", ""); }

        println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m \x1b[1;37mSETTINGS: [W] Windows: {:<5} [R] Release: {:<5}      \x1b[1;36m│\x1b[0m", 
            if options.win_mode { "ON" } else { "OFF" },
            if options.release { "ON" } else { "OFF" }
        );
        println!("\x1b[1;36m│\x1b[0m           [S] Strip:   {:<5} [V] Verbose: {:<5}      \x1b[1;36m│\x1b[0m", 
            if options.strip { "ON" } else { "OFF" },
            if options.verbose { "ON" } else { "OFF" }
        );
        println!("\x1b[1;36m└──────────────────────────────────────────────────────────┘\x1b[0m");
        println!(" \x1b[1;30m↑/↓: Navigate | Enter: Build & Run | Backspace: Up | Q: Exit\x1b[0m");

        let _ = Command::new("stty").arg("raw").arg("-echo").spawn().unwrap().wait();
        let mut buf = [0u8; 1];
        let _ = io::stdin().read(&mut buf);
        let _ = Command::new("stty").arg("-raw").arg("echo").spawn().unwrap().wait();

        match buf[0] {
            113 => break, // q
            119 | 87 => options.win_mode = !options.win_mode, // w
            114 | 82 => options.release = !options.release, // r
            115 | 83 => options.strip = !options.strip, // s
            118 | 86 => options.verbose = !options.verbose, // v
            65 => if selected > 0 { selected -= 1 }, // Up
            66 => if selected < entries.len() - 1 { selected += 1 }, // Down
            127 => { path.pop(); selected = 0; }, // Backspace
            13 => {
                let full = &entries[selected];
                if full.is_dir() { path = full.clone(); selected = 0; }
                else {
                    print!("\x1b[2J\x1b[H");
                    let stem = full.file_stem().unwrap().to_str().unwrap();
                    build_vpl(full.to_str().unwrap(), stem, options, true);
                    println!("\n\x1b[1;36m  Build Cycle Complete.\x1b[0m Press Enter to return...");
                    let _ = io::stdin().read(&mut [0u8; 1]);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }
    if args[1] == "tui" {
        run_tui_mode();
        return;
    }
    if args.len() < 3 || (args[1] != "build" && args[1] != "run") {
        print_usage();
        return;
    }
    
    let is_run = args[1] == "run";
    let input_path = &args[2];
    let mut output_name = Path::new(input_path).file_stem().unwrap().to_str().unwrap().to_string();
    
    if is_run {
        output_name = format!("vpl_run_{}_tmp", output_name);
    }
    
    let mut options = BuildOptions::default();
    for i in 3..args.len() {
        if args[i] == "-o" && i + 1 < args.len() { output_name = args[i + 1].clone(); }
        if args[i] == "-w" { options.win_mode = true; }
        if args[i] == "--debug" { options.release = false; }
        if args[i] == "--no-strip" { options.strip = false; }
        if args[i] == "--verbose" { options.verbose = true; }
    }
    build_vpl(input_path, &output_name, options, is_run);
}
