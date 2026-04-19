mod lexer;
mod parser;
mod compiler;

use std::env;
use std::fs;
use std::process::Command;
use std::path::{Path, PathBuf};
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
    silent: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self { win_mode: false, release: true, strip: true, verbose: true, silent: false }
    }
}

struct Logger {
    start_time: Instant,
    active_spinner: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    stop_signal: Arc<Mutex<bool>>,
    silent: bool,
}

impl Logger {
    fn new(silent: bool) -> Self {
        Self { 
            start_time: Instant::now(),
            active_spinner: Arc::new(Mutex::new(None)),
            stop_signal: Arc::new(Mutex::new(false)),
            silent,
        }
    }

    fn header(&self, file: &str, target: &str, target_os: &str) {
        if self.silent { return; }
        let rustc_ver = Command::new("rustc").arg("--version").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or("unknown".to_string());
        println!("\n\x1b[1;36m┌──────────────────────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mVPL SYSTEMS \x1b[0;34m- INDUSTRIAL EDITION v1.9.0\x1b[0m            \x1b[1;36m│\x1b[0m");
        println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m SOURCE:\x1b[0m {:<46} \x1b[1;36m│\x1b[0m", file);
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m BINARY:\x1b[0m {:<46} \x1b[1;36m│\x1b[0m", target);
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m TARGET:\x1b[0m {:<46} \x1b[1;36m│\x1b[0m", target_os);
        println!("\x1b[1;36m│\x1b[0m \x1b[1;32m BACKEND:\x1b[0m {:<45} \x1b[1;36m│\x1b[0m", rustc_ver);
        println!("\x1b[1;36m└──────────────────────────────────────────────────────────┘\x1b[0m\n");
    }

    fn start_task(&self, phase: &str) {
        if self.silent { return; }
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
        if self.silent { return; }
        *self.stop_signal.lock().unwrap() = true;
        let mut handle_opt = self.active_spinner.lock().unwrap();
        if let Some(handle) = handle_opt.take() {
            let _ = handle.join();
        }
        print!("\r\x1b[K");
        io::stdout().flush().unwrap();
    }

    fn complete_step(&self, phase: &str, info: &str, duration: Duration) {
        if self.silent { return; }
        self.stop_spinner();
        println!("  \x1b[1;32m✔\x1b[0m \x1b[1;37m{:<14}\x1b[0m \x1b[1;30m➤\x1b[0m \x1b[0;37m{:<30}\x1b[0m \x1b[1;30m({:.1?})\x1b[0m", phase, info, duration);
    }

    fn detail(&self, label: &str, value: &str) {
        if self.silent { return; }
        println!("     \x1b[1;34m└─\x1b[0m \x1b[1;30m{:<12}\x1b[0m {}", label, value);
    }

    fn success(&self, bin: &str) {
        if self.silent { return; }
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
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mVPL SYSTEMS \x1b[0;34m- OPTIMIZED COMPILER v1.9.0\x1b[0m            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mUSAGE:\x1b[0m                                               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl build <file.vpl> [options]                      \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl run <file.vpl>       Compile and run on the fly \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl add <lib.vplib>      Add library to repository  \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl tui                  Start interactive selector \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m                                                          \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mOPTIONS:\x1b[0m                                             \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    -o <name>    Set output binary name                \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    -w           Compile for Windows (.exe)            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    -silent      Suppress all logs                     \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    --debug      Disable optimizations, add symbols    \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    --no-strip   Don't strip binary symbols            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    --verbose    Show detailed backend logs            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m                                                          \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mEXAMPLES:\x1b[0m                                            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    ./vpl run program.vpl                               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    ./vpl add mylib.vplib                               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m└──────────────────────────────────────────────────────────┘\x1b[0m\n");
}

fn get_lib_dir() -> PathBuf {
    let mut p = env::current_dir().unwrap();
    p.push(".vpl");
    p.push("lib");
    if !p.exists() { fs::create_dir_all(&p).ok(); }
    p
}

fn build_vpl(input_path: &str, output_name: &str, options: BuildOptions, run_after: bool) {
    let logger = Logger::new(options.silent);

    // 0. Dependency Check
    logger.start_task("DEPENDENCIES");
    let start = Instant::now();
    let missing = check_dependencies();
    if !missing.is_empty() {
        logger.error("DEPENDENCIES", &format!("Missing tools: {}", missing.join(", ")));
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
    let mut content = match fs::read_to_string(input_path) {
        Ok(c) => c,
        Err(e) => { logger.error("BOOTSTRAP", &format!("Read failed: {}", e)); return; }
    };
    
    // Load libraries intelligently
    let mut available_libs = std::collections::HashMap::new();
    let mut dirs_to_check = vec![env::current_dir().unwrap(), get_lib_dir()];
    
    // Also check for .vpl and .vplib subdirectories in current dir
    let vpl_dir = env::current_dir().unwrap().join(".vpl");
    if vpl_dir.is_dir() { dirs_to_check.push(vpl_dir); }
    let vplib_dir = env::current_dir().unwrap().join(".vplib");
    if vplib_dir.is_dir() { dirs_to_check.push(vplib_dir); }
    
    for dir in dirs_to_check {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map_or(false, |e| e == "vplib") {
                    if let Ok(lib_content) = fs::read_to_string(entry.path()) {
                        let mut defined_funcs = Vec::new();
                        for line in lib_content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("func ") {
                                if let Some(paren) = trimmed.find('(') {
                                    let func_name = trimmed[5..paren].trim().to_string();
                                    if !func_name.is_empty() {
                                        defined_funcs.push(func_name);
                                    }
                                }
                            }
                        }
                        available_libs.insert(entry.path().clone(), (defined_funcs, lib_content));
                    }
                }
            }
        }
    }

    let mut included_files = std::collections::HashSet::new();
    let mut libs_loaded = 0;
    let mut added_something = true;
    
    while added_something {
        added_something = false;
        let mut to_include = Vec::new();
        
        for (path, (funcs, _)) in &available_libs {
            if !included_files.contains(path) {
                let mut is_needed = false;
                for func in funcs {
                    let call_pattern = format!("{}(", func);
                    let call_pattern_space = format!("{} (", func);
                    if content.contains(&call_pattern) || content.contains(&call_pattern_space) {
                        is_needed = true;
                        break;
                    }
                }
                if is_needed {
                    to_include.push(path.clone());
                }
            }
        }
        
        for path in to_include {
            if let Some((funcs, lib_content)) = available_libs.get(&path) {
                content.push_str("\n");
                content.push_str(lib_content);
                included_files.insert(path.clone());
                libs_loaded += 1;
                added_something = true;
                if options.verbose && !options.silent {
                    logger.detail("Library", &format!("{:<20} (provides: {})", path.file_name().unwrap().to_str().unwrap(), funcs.join(", ")));
                }
            }
        }
    }
    
    let dur = start.elapsed();
    logger.complete_step("BOOTSTRAP", &format!("Loaded {} bytes", content.len()), dur);
    if libs_loaded > 0 && !options.silent { 
        logger.detail("Status", &format!("Injected {} .vplib files intelligently", libs_loaded)); 
    }

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
    let mut parser = parser::Parser::new(tokens, content.clone());
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(e) => { logger.error("PARSER", &format!("Syntax error: {}", e)); return; }
    };
    let dur = start.elapsed();
    logger.complete_step("PARSER", "AST construction successful", dur);

    // 4. Optimization
    logger.start_task("OPTIMIZER");
    let start = Instant::now();
    let (rust_code, modules) = compiler::compile(&ast);
    let dur = start.elapsed();
    logger.complete_step("OPTIMIZER", &format!("Active segments: {}", modules.len()), dur);
    if (!run_after || options.verbose) && !options.silent { 
        for m in &modules {
            logger.detail("Segment", m);
        }
    }

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

    // 6. Native compilation
    logger.start_task("LINKER");
    let start = Instant::now();
    let mut cmd = Command::new("rustc");
    cmd.arg(&temp_rs);
    if options.release { cmd.arg("-C").arg("opt-level=3").arg("-C").arg("debuginfo=0"); }
    else { cmd.arg("-C").arg("opt-level=0").arg("-C").arg("debuginfo=2"); }
    if options.strip && !options.win_mode { cmd.arg("-C").arg("link-arg=-s"); }
    cmd.arg("-o").arg(&final_bin);
    if options.win_mode { cmd.arg("--target").arg("x86_64-pc-windows-gnu"); }
    
    let output = if options.verbose { 
        cmd.status().map(|s| (s, String::new()))
    } else { 
        cmd.output().map(|o| (o.status, String::from_utf8_lossy(&o.stderr).to_string()))
    };
    
    let _ = fs::remove_file(&temp_rs);

    match output {
        Ok((s, _stderr)) if s.success() => {
            let dur = start.elapsed();
            logger.complete_step("LINKER", "Binary linked and optimized", dur);
            if run_after {
                logger.stop_spinner();
                if !options.silent {
                    println!("\n\x1b[1;32m  ➤ RUNNING SCRIPT:\x1b[0;37m ./{}\x1b[0m\n", final_bin);
                }
                let mut run_cmd = Command::new(format!("./{}", final_bin)).spawn().expect("Failed to run binary");
                let _ = run_cmd.wait();
                let _ = fs::remove_file(&final_bin);
            } else {
                let size = fs::metadata(&final_bin).map(|m| m.len()).unwrap_or(0);
                logger.detail("Binary Size", &format!("{:.2} KB", size as f64 / 1024.0));
                logger.success(&final_bin);
            }
        }
        Ok((_, stderr)) => { 
            logger.error("LINKER", "LLVM Backend (rustc) failed to compile the generated code.");
            if !stderr.is_empty() {
                println!("\x1b[0;31m  RUSTC ERROR:\x1b[0m\n{}", stderr);
            }
            println!("\x1b[0;90m  (Hint: Use --verbose to see full command output or check generated code logic)\x1b[0m");
        }
        Err(e) => { logger.error("LINKER", &format!("Failed to execute compiler: {}", e)); }
    }
}

fn add_library(path: &str) {
    let src = Path::new(path);
    if !src.exists() { println!("\x1b[1;31mError:\x1b[0m Library file '{}' not found.", path); return; }
    let mut dest = get_lib_dir();
    dest.push(src.file_name().unwrap());
    if fs::copy(src, &dest).is_ok() {
        println!("\x1b[1;32mSuccess:\x1b[0m Library '{}' added to VPL repository.", src.file_name().unwrap().to_str().unwrap());
    } else {
        println!("\x1b[1;31mError:\x1b[0m Could not copy library.");
    }
}

fn run_tui_mode() {
    let mut path = env::current_dir().unwrap();
    let mut selected = 0;
    let mut options = BuildOptions::default();

    loop {
        let mut entries: Vec<_> = fs::read_dir(&path).unwrap()
            .map(|res| res.unwrap().path())
            .filter(|p| p.is_dir() || p.extension().map_or(false, |ext| ext == "vpl" || ext == "vplib"))
            .collect();
        entries.sort_by(|a, b| a.is_file().cmp(&b.is_file()).then(a.cmp(b)));

        print!("\x1b[2J\x1b[H");
        println!("\x1b[1;34m┌──────────────────────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[1;34m│\x1b[0m  \x1b[1;37mVPL PROFESSIONAL NAVIGATOR \x1b[0;34mv1.9.0\x1b[0m                \x1b[1;34m│\x1b[0m");
        println!("\x1b[1;34m├──────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[1;34m│\x1b[0m \x1b[1;32m PATH:\x1b[0m {:<50} \x1b[1;34m│\x1b[0m", path.to_str().unwrap().chars().take(50).collect::<String>());
        println!("\x1b[1;34m├──────────────────────────────────────────────────────────┤\x1b[0m");

        let view_size = 12;
        let start_idx = if selected >= view_size { selected - view_size + 1 } else { 0 };
        
        for i in 0..view_size {
            let idx = start_idx + i;
            if idx < entries.len() {
                let entry = &entries[idx];
                let name = entry.file_name().unwrap().to_str().unwrap();
                let is_dir = entry.is_dir();
                let prefix = if idx == selected { "\x1b[1;33m >> \x1b[0m" } else { "    " };
                let color = if is_dir { "\x1b[1;34m" } else if name.ends_with(".vplib") { "\x1b[1;35m" } else { "\x1b[1;32m" };
                let icon = if is_dir { "📁" } else if name.ends_with(".vplib") { "📚" } else { "📄" };
                println!("\x1b[1;34m│\x1b[0m {}{} {} {:<47}\x1b[0m \x1b[1;34m│\x1b[0m", prefix, color, icon, name.chars().take(47).collect::<String>());
            } else {
                println!("\x1b[1;34m│\x1b[0m {:<56} \x1b[1;34m│\x1b[0m", "");
            }
        }

        println!("\x1b[1;34m├──────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[1;34m│\x1b[0m \x1b[1;37m[W] Windows: {:<3} [R] Release: {:<3} [S] Strip: {:<3} [V] Verb: {:<3} \x1b[1;34m│\x1b[0m", 
            if options.win_mode { "ON" } else { "OFF" }, if options.release { "ON" } else { "OFF" },
            if options.strip { "ON" } else { "OFF" }, if options.verbose { "ON" } else { "OFF" }
        );
        println!("\x1b[1;34m└──────────────────────────────────────────────────────────┘\x1b[0m");
        println!(" \x1b[1;30mENTER: Run/Open | A: Add Lib | BACKSPACE: Up | Q: Quit\x1b[0m");

        let _ = Command::new("stty").arg("raw").arg("-echo").spawn().unwrap().wait();
        let mut buf = [0u8; 3];
        let n = io::stdin().read(&mut buf).unwrap_or(0);
        let _ = Command::new("stty").arg("-raw").arg("echo").spawn().unwrap().wait();

        if n == 0 { continue; }

        match buf[0] {
            113 => break, // q
            97 | 65 if n == 1 => { // a
                let entry = &entries[selected];
                if entry.extension().map_or(false, |e| e == "vplib") {
                    add_library(entry.to_str().unwrap());
                    thread::sleep(Duration::from_secs(1));
                }
            },
            119 | 87 => options.win_mode = !options.win_mode,
            114 | 82 => options.release = !options.release,
            115 | 83 => options.strip = !options.strip,
            118 | 86 => options.verbose = !options.verbose,
            27 if n == 3 && buf[1] == 91 => {
                match buf[2] {
                    65 => if selected > 0 { selected -= 1 }, // Up
                    66 => if selected < entries.len() - 1 { selected += 1 }, // Down
                    _ => {}
                }
            }
            127 => { if path.parent().is_some() { path.pop(); } selected = 0; },
            13 => {
                let full = &entries[selected];
                if full.is_dir() { path = full.clone(); selected = 0; }
                else {
                    print!("\x1b[2J\x1b[H");
                    let stem = full.file_stem().unwrap().to_str().unwrap();
                    build_vpl(full.to_str().unwrap(), stem, options, true);
                    println!("\nPress Enter to return...");
                    let _ = io::stdin().read(&mut [0u8; 1]);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { print_usage(); return; }
    if args[1] == "tui" { run_tui_mode(); return; }
    if args[1] == "add" && args.len() >= 3 { add_library(&args[2]); return; }
    if args.len() < 3 || (args[1] != "build" && args[1] != "run") { print_usage(); return; }
    
    let is_run = args[1] == "run";
    let input_path = &args[2];
    let mut output_name = Path::new(input_path).file_stem().unwrap().to_str().unwrap().to_string();
    if is_run { output_name = format!("vpl_run_{}_tmp", output_name); }
    
    let mut options = BuildOptions::default();
    for i in 3..args.len() {
        if args[i] == "-o" && i + 1 < args.len() { output_name = args[i + 1].clone(); }
        if args[i] == "-w" { options.win_mode = true; }
        if args[i] == "-silent" || args[i] == "--silent" { options.silent = true; }
        if args[i] == "--debug" { options.release = false; }
        if args[i] == "--no-strip" { options.strip = false; }
        if args[i] == "--verbose" { options.verbose = true; }
        if args[i] == "--quiet" { options.verbose = false; }
    }
    build_vpl(input_path, &output_name, options, is_run);
}
