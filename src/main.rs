mod lexer;
mod parser;
mod compiler;

use std::env;
use std::fs;
use std::process::Command;
use std::path::Path;
use std::io::{self, Write};
use std::time::{Instant, Duration};
use std::thread;
use std::sync::{Arc, Mutex};

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
        println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mVPL SYSTEMS \x1b[0;34m- INDUSTRIAL EDITION v1.4.0\x1b[0m            \x1b[1;36m│\x1b[0m");
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
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mVPL SYSTEMS \x1b[0;34m- OPTIMIZED COMPILER v1.5.0\x1b[0m            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m├──────────────────────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mUSAGE:\x1b[0m                                               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl build <file.vpl> [options]                      \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl run <file.vpl>       Compile and run on the fly \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    vpl tui                  Start interactive selector \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m                                                          \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mOPTIONS:\x1b[0m                                             \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    -o <name>    Set output binary name                \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    -w           Compile for Windows (.exe)            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m                                                          \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;32mEXAMPLES:\x1b[0m                                            \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    ./vpl run program.vpl                               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m    ./vpl build program.vpl -o my_app -w                \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m└──────────────────────────────────────────────────────────┘\x1b[0m\n");
}

fn build_vpl(input_path: &str, output_name: &str, win_mode: bool, run_after: bool) {
    let logger = Logger::new();

    // 0. Dependency Check
    logger.start_task("DEPENDENCIES");
    let start = Instant::now();
    let missing = check_dependencies();
    if !missing.is_empty() {
        logger.error("DEPENDENCIES", &format!("Missing tools: {}", missing.join(", ")));
        println!("\x1b[1;33m  FIX:\x1b[1;37m sudo eopkg install rust\x1b[0m\n");
        return;
    }
    logger.complete_step("DEPENDENCIES", "Environment validated", start.elapsed());

    let target_os = if win_mode { "x86_64-pc-windows-gnu (Cross)" } else { "x86_64-linux-gnu (Native)" };
    let final_bin = if win_mode { format!("{}.exe", output_name) } else { output_name.to_string() };

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
    thread::sleep(Duration::from_millis(50));
    let content = match fs::read_to_string(input_path) {
        Ok(c) => {
            let dur = start.elapsed();
            logger.complete_step("BOOTSTRAP", &format!("Loaded {} bytes", c.len()), dur);
            if !run_after { logger.detail("Status", "I/O Stream synchronized"); }
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
    if !run_after { logger.detail("Tokens", &format!("Total identified: {}", tokens.len())); }

    // 3. Parsing
    logger.start_task("PARSER");
    let start = Instant::now();
    let mut parser = parser::Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(a) => {
            let dur = start.elapsed();
            logger.complete_step("PARSER", "AST construction successful", dur);
            if !run_after { logger.detail("Model", "High-fidelity syntax tree"); }
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
    if !run_after { logger.detail("Output", &format!("{}_vpl_tmp.rs", output_name)); }

    // 6. Native compilation
    logger.start_task("LINKER");
    let start = Instant::now();
    let mut cmd = Command::new("rustc");
    cmd.arg(&temp_rs).arg("-C").arg("opt-level=3").arg("-C").arg("debuginfo=0").arg("-o").arg(&final_bin);
    if win_mode { cmd.arg("--target").arg("x86_64-pc-windows-gnu"); }
    let status = cmd.output();
    let _ = fs::remove_file(&temp_rs);

    match status {
        Ok(out) if out.status.success() => {
            let dur = start.elapsed();
            logger.complete_step("LINKER", "Binary linked and optimized", dur);
            if run_after {
                logger.stop_spinner();
                println!("\n\x1b[1;32m  ➤ RUNNING SCRIPT:\x1b[0;37m ./{}\x1b[0m\n", final_bin);
                let mut run_cmd = Command::new(format!("./{}", final_bin)).spawn().expect("Failed to run binary");
                let _ = run_cmd.wait();
                let _ = fs::remove_file(&final_bin);
            } else {
                logger.success(&final_bin);
            }
        }
        Ok(out) => {
            logger.error("LINKER", "LLVM Backend failed.");
            println!("\x1b[1;30m  ──[ DIAGNOSTIC DATA ]─────────────────────────────────────\x1b[0m\n{}\n", String::from_utf8_lossy(&out.stderr));
        }
        Err(e) => logger.error("LINKER", &format!("Execution failed: {}", e)),
    }
}

fn run_tui_mode() {
    use std::io::{Read};
    let mut path = env::current_dir().unwrap();
    let mut selected = 0;
    let _ = Command::new("stty").arg("raw").arg("-echo").spawn().unwrap().wait();
    loop {
        let mut entries = Vec::new();
        if path.parent().is_some() { entries.push("..".to_string()); }
        if let Ok(list) = fs::read_dir(&path) {
            let mut files: Vec<_> = list.flatten().filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if e.path().is_dir() || name.ends_with(".vpl") { Some(name) } else { None }
            }).collect();
            files.sort();
            entries.extend(files);
        }
        if selected >= entries.len() { selected = 0; }
        print!("\x1b[2J\x1b[H\x1b[1;36m VPL SELECTOR \x1b[0m | \x1b[1;30mFolder: {}\x1b[0m\r\n", path.display());
        print!("\x1b[1;30m ──────────────────────────────────────────────────────────\x1b[0m\r\n");
        for (i, entry) in entries.iter().enumerate() {
            if i == selected { print!(" \x1b[1;32m> {}\x1b[0m\r\n", entry); }
            else { if fs::metadata(path.join(entry)).map(|m| m.is_dir()).unwrap_or(entry == "..") { print!(" \x1b[1;34m  {}\x1b[0m\r\n", entry); } else { print!("   {}\r\n", entry); } }
        }
        print!("\x1b[1;30m ──────────────────────────────────────────────────────────\x1b[0m\r\n");
        print!(" [Arrows] Move | [Enter] Select/Build | [Q] Quit\r\n");
        io::stdout().flush().unwrap();
        let mut buf = [0; 3];
        let _ = io::stdin().read(&mut buf);
        if buf[0] == b'q' || buf[0] == 3 { break; }
        if buf[0] == 27 && buf[1] == 91 { match buf[2] { 65 => if selected > 0 { selected -= 1; }, 66 => if selected < entries.len() - 1 { selected += 1; }, _ => {} } }
        if buf[0] == 13 || buf[0] == 10 {
            let name = &entries[selected];
            if name == ".." { path = path.parent().unwrap_or(&path).to_path_buf(); selected = 0; }
            else {
                let full = path.join(name);
                if full.is_dir() { path = full; selected = 0; }
                else {
                    let _ = Command::new("stty").arg("-raw").arg("echo").spawn().unwrap().wait();
                    print!("\x1b[2J\x1b[H");
                    let stem = full.file_stem().unwrap().to_str().unwrap();
                    
                    build_vpl(full.to_str().unwrap(), stem, false, true);

                    println!("\n\x1b[1;36m  Build Cycle Complete.\x1b[0m Press Enter to return...");
                    let _ = io::stdin().read(&mut [0u8; 1]);
                    let _ = Command::new("stty").arg("raw").arg("-echo").spawn().unwrap().wait();
                }
            }
        }
    }
    let _ = Command::new("stty").arg("-raw").arg("echo").spawn().unwrap().wait();
    print!("\x1b[2J\x1b[H");
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
    
    let mut win_mode = false;
    for i in 3..args.len() {
        if args[i] == "-o" && i + 1 < args.len() { output_name = args[i + 1].clone(); }
        if args[i] == "-w" { win_mode = true; }
    }
    build_vpl(input_path, &output_name, win_mode, is_run);
}