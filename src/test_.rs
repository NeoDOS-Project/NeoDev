use crate::config::Config;
use crate::discovery::Discovery;
use crate::image;
use crate::vmm;
use crate::vmm::{NetworkMode, StorageMode, VmConfig};
use anyhow::Result;
use colored::*;
use std::time::{Duration, Instant};

static SERIAL_LOG: &str = "/tmp/neodos_serial.log";

pub struct TestOptions {
    pub storage: StorageMode,
    pub kvm: bool,
    pub iterations: u32,
    pub timeout: u64,
    pub backend: String,
}

pub struct TestResult {
    pub kernel_tests_passed: bool,
    pub kernel_count: Option<u32>,
    pub command_tests_passed: bool,
    pub shell_tests_passed: bool,
    pub total_duration: Duration,
    pub panics: Vec<String>,
}

pub struct DhcpTestResult {
    pub dhcp_passed: bool,
    pub output_lines: Vec<String>,
    pub total_duration: Duration,
}

pub fn run_tests(cfg: &Config, opts: &TestOptions) -> Result<TestResult> {
    println!("{} NeoDOS Test Runner", "[*]".bold().cyan());
    println!("  Backend: {}", opts.backend);
    println!();

    for iter in 1..=opts.iterations {
        if opts.iterations > 1 {
            println!("{} Iteration {}/{}", "[*]".bold().cyan(), iter, opts.iterations);
        }
        let result = run_single_test(cfg, opts)?;
        if iter < opts.iterations { println!(); }
        if iter == opts.iterations || !result.kernel_tests_passed { return Ok(result); }
    }
    unreachable!()
}

fn run_single_test(cfg: &Config, opts: &TestOptions) -> Result<TestResult> {
    let start = Instant::now();
    let _ = std::fs::remove_file(SERIAL_LOG);

    let disk_image = cfg.neodos_root.join("disk_image.img");
    if !disk_image.exists() {
        anyhow::bail!("disk_image.img not found. Run 'neodev build --image' first.");
    }

    let backend = vmm::create_backend(&opts.backend)?;
    backend.check_prerequisites(cfg)?;

    let serial_path = std::path::PathBuf::from(SERIAL_LOG);
    let vmcfg = VmConfig {
        name: "NeoDOS".into(), memory_mb: cfg.vm_memory_mb, cpus: cfg.vm_cpus,
        efi: true, disk_image: disk_image.clone(), disk_vdi: cfg.neodos_root.join("disk_image.vdi"),
        serial_file: Some(serial_path), network: vmm::NetworkMode::User,
        headless: true, gdb: false, gdb_port: 1234, storage_mode: opts.storage,
    };

    let backend_ref: &dyn vmm::HypervisorBackend = backend.as_ref();
    let mut instance = backend_ref.start_headless(cfg, &vmcfg)
        .unwrap_or_else(|e| panic!("Failed to start headless VM (backend: {}): {}\n\
            Ensure the VM is properly created with 'neodev vm create --backend {}'", opts.backend, e, opts.backend));

    std::thread::sleep(Duration::from_secs(3));
    let result = wait_for_test_completion(&mut instance, opts.timeout)?;
    let _ = instance.kill();

    println!();
    print_test_result(&result, start.elapsed());
    Ok(result)
}

fn wait_for_test_completion(instance: &mut Box<dyn vmm::VmInstance>, timeout_secs: u64) -> Result<TestResult> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut output_lines: Vec<String> = vec![];
    let mut last_serial_len = 0u64;
    while Instant::now() < deadline {
        if let Some(exit_code) = instance.wait_timeout(Duration::from_millis(100))? {
            return Ok(TestResult { kernel_tests_passed: false, kernel_count: None, command_tests_passed: false, shell_tests_passed: false, total_duration: Duration::ZERO, panics: vec![format!("VM exited early: code {}", exit_code)] });
        }
        if let Ok(new_data) = read_serial_since(last_serial_len) {
            last_serial_len += new_data.len() as u64;
            for line in new_data.split('\n') {
                let clean = strip_ansi(line);
                if !clean.is_empty() { output_lines.push(clean.clone()); print_serial_line(&clean, ""); }
            }
        }
        let full_text = output_lines.join("\n");
        if full_text.contains("ALL_TESTS_COMPLETE") && full_text.contains("CMDTEST_COMPLETE") && full_text.contains("STRESSCMD_COMPLETE") { break; }
        std::thread::sleep(Duration::from_millis(300));
    }
    analyze_results(&output_lines)
}

fn read_serial_since(offset: u64) -> Result<String> {
    let path = std::path::Path::new(SERIAL_LOG);
    if !path.exists() { return Ok(String::new()); }
    let data = std::fs::read(path)?;
    if data.len() as u64 <= offset { return Ok(String::new()); }
    Ok(String::from_utf8_lossy(&data[offset as usize..]).to_string())
}

fn print_serial_line(line: &str, _state: &str) {
    if line.contains("ALL_TESTS_COMPLETE") { println!("{} ALL TESTS COMPLETE", "[+]".bold().green()); }
    else if line.contains("Type HELP") { println!("{} Shell detected!", "[+]".bold().green()); }
    else if line.contains("NeoDOS Kernel v") || line.contains("Bootloader v") { println!("  {}", line); }
    else if line.contains("[✓]") || line.contains("[+]") { let clean = line.trim(); if clean.len() > 4 { println!("  {}", clean); } }
    else if line.contains("kernel tests") || line.contains("ALL TESTS COMPLETE") { println!("  [TEST] {}", line); }
    else if line.contains("[DHCPTEST]") { println!("  {}", line); }
    else if line.contains("PANIC") || line.contains("panic") { println!("  {} {}", "[!]".bold().red(), line); }
}

fn analyze_results(lines: &[String]) -> Result<TestResult> {
    let full_text = lines.join("\n");
    let kernel_ok = full_text.contains("kernel tests passed");
    let kernel_count = full_text.lines().find_map(|l| {
        if l.contains("kernel tests passed") {
            let parts: Vec<&str> = l.split_whitespace().collect();
            parts.get(1).and_then(|s| s.parse::<u32>().ok())
        } else { None }
    });
    let cmd_ok = full_text.contains("ALL_COMMAND_TESTS_PASSED");
    let sh_ok = full_text.contains("SHELL_TESTS_PASSED");
    let mut panics = vec![];
    for line in lines {
        let clean = strip_ansi(line);
        for kw in &["KERNEL PANIC", "DOUBLE FAULT", "GPF:", "panic!", "BUGCHECK"] {
            if clean.contains(kw) { panics.push(clean.clone()); }
        }
    }
    Ok(TestResult { kernel_tests_passed: kernel_ok, kernel_count, command_tests_passed: cmd_ok, shell_tests_passed: sh_ok, total_duration: Duration::ZERO, panics })
}

fn print_test_result(result: &TestResult, elapsed: Duration) {
    println!("{}", "=".repeat(60));
    println!("{}", "TEST RESULTS".bold());
    println!("{}", "=".repeat(60));
    if result.kernel_tests_passed {
        if let Some(count) = result.kernel_count { println!("  {} Kernel tests: {} passed", "[PASS]".bold().green(), count); }
        else { println!("  {} Kernel tests passed", "[PASS]".bold().green()); }
    } else { println!("  {} Kernel tests FAILED", "[FAIL]".bold().red()); }
    if result.command_tests_passed { println!("  {} Command tests passed", "[PASS]".bold().green()); }
    else { println!("  {} Command tests FAILED or not run", "[INFO]".bold().yellow()); }
    if result.shell_tests_passed { println!("  {} Shell tests passed", "[PASS]".bold().green()); }
    else { println!("  {} Shell tests FAILED or not run", "[INFO]".bold().yellow()); }
    if !result.panics.is_empty() {
        println!(); println!("  {} Panics detected:", "[!]".bold().red());
        for p in &result.panics { println!("    {}", p); }
    }
    println!();
    if result.kernel_tests_passed { println!("{} OVERALL: SUCCESS", "[✓]".bold().green()); }
    else { println!("{} OVERALL: FAILED", "[✗]".bold().red()); }
    println!("  Duration: {:.1}s", elapsed.as_secs_f64());
}

// DHCP Integration Test

pub fn run_dhcp_test(cfg: &Config, disc: &Discovery, backend: &str, timeout_secs: u64) -> Result<DhcpTestResult> {
    let start = Instant::now();
    println!("{} NeoDOS DHCP Integration Test (bridge)", "[*]".bold().cyan());
    println!("  Backend: {}", backend);
    println!();

    let _ = std::fs::remove_file(SERIAL_LOG);
    if !cfg.neodos_root.join("disk_image.img").exists() {
        anyhow::bail!("disk_image.img not found. Run 'neodev build --image' first.");
    }

    println!("  Generating test registry hive...");
    image::generate_test_hive(cfg, true)?;
    let fs_image = cfg.neodos_root.join("data").join("neodos_image.img");
    image::build_ne2_image(cfg, disc, &fs_image, "NEODOS", 25600)?;
    let esp_image = image::create_esp_image(cfg)?;
    image::create_gpt_image(cfg, &esp_image, &fs_image, &cfg.neodos_root.join("disk_image.img"))?;
    let _ = std::fs::remove_file(&esp_image);

    let vbox_backend = vmm::create_backend(backend)?;
    vbox_backend.check_prerequisites(cfg)?;

    let serial_path = std::path::PathBuf::from(SERIAL_LOG);
    let disk_image = cfg.neodos_root.join("disk_image.img");
    let vmcfg = VmConfig {
        name: "NeoDOS".into(), memory_mb: cfg.vm_memory_mb, cpus: cfg.vm_cpus,
        efi: true, disk_image: disk_image.clone(), disk_vdi: cfg.neodos_root.join("disk_image.vdi"),
        serial_file: Some(serial_path), network: NetworkMode::Bridged,
        headless: true, gdb: false, gdb_port: 1234, storage_mode: StorageMode::Ahci,
    };

    vbox_backend.ensure_vm(cfg, &vmcfg)?;
    println!("  Starting VM...");
    let backend_ref: &dyn vmm::HypervisorBackend = vbox_backend.as_ref();
    let mut instance = backend_ref.start_headless(cfg, &vmcfg)?;

    let result = wait_for_dhcp_result(&mut instance, timeout_secs, &start);
    let _ = instance.kill();
    let _ = image::restore_hive(cfg);
    result
}

pub fn run_dhcp_test_qemu(cfg: &Config, disc: &Discovery, timeout_secs: u64) -> Result<DhcpTestResult> {
    let start = Instant::now();
    println!("{} NeoDOS DHCP Integration Test (QEMU)", "[*]".bold().cyan());
    println!("  Network: user-mode (SLiRP)");
    println!();

    let _ = std::fs::remove_file(SERIAL_LOG);
    if !cfg.neodos_root.join("disk_image.img").exists() {
        anyhow::bail!("disk_image.img not found. Run 'neodev build --image' first.");
    }

    println!("  Generating test registry hive...");
    image::generate_test_hive(cfg, true)?;
    let fs_image = cfg.neodos_root.join("data").join("neodos_image.img");
    image::build_ne2_image(cfg, disc, &fs_image, "NEODOS", 25600)?;
    let esp_image = image::create_esp_image(cfg)?;
    image::create_gpt_image(cfg, &esp_image, &fs_image, &cfg.neodos_root.join("disk_image.img"))?;
    let _ = std::fs::remove_file(&esp_image);

    let backend = vmm::create_backend("qemu")?;
    backend.check_prerequisites(cfg)?;

    let serial_path = std::path::PathBuf::from(SERIAL_LOG);
    let disk_image = cfg.neodos_root.join("disk_image.img");
    let vmcfg = VmConfig {
        name: "NeoDOS".into(), memory_mb: cfg.vm_memory_mb, cpus: cfg.vm_cpus,
        efi: true, disk_image: disk_image.clone(), disk_vdi: cfg.neodos_root.join("disk_image.vdi"),
        serial_file: Some(serial_path), network: NetworkMode::User,
        headless: true, gdb: false, gdb_port: 1234, storage_mode: StorageMode::Ahci,
    };

    println!("  Starting QEMU...");
    let backend_ref: &dyn vmm::HypervisorBackend = backend.as_ref();
    let mut instance = backend_ref.start_headless(cfg, &vmcfg)?;

    let result = wait_for_dhcp_result(&mut instance, timeout_secs, &start);
    let _ = instance.kill();
    let _ = image::restore_hive(cfg);
    result
}

fn wait_for_dhcp_result(instance: &mut Box<dyn vmm::VmInstance>, timeout_secs: u64, start: &Instant) -> Result<DhcpTestResult> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut output_lines: Vec<String> = vec![];
    let mut last_serial_len = 0u64;
    let mut dhcp_done = false;
    let mut dhcp_passed = false;

    println!("  Waiting for DHCP test to complete...");
    std::thread::sleep(Duration::from_secs(3));

    while Instant::now() < deadline {
        if let Some(exit_code) = instance.wait_timeout(Duration::from_millis(100))? {
            println!("  {} VM exited early (code {})", "[!]".bold().red(), exit_code); break;
        }
        if let Ok(new_data) = read_serial_since(last_serial_len) {
            last_serial_len += new_data.len() as u64;
            for line in new_data.split('\n') {
                let clean = strip_ansi(line);
                if !clean.is_empty() { output_lines.push(clean.clone()); print_dhcp_serial_line(&clean); }
            }
        }
        let full_text = output_lines.join("\n");
        if full_text.contains("DHCPTEST_COMPLETE") {
            dhcp_done = true;
            dhcp_passed = full_text.contains("DHCPTEST_PASSED");
            println!("{} DHCP test {}", "[*]".bold().cyan(), if dhcp_passed { "PASSED" } else { "FAILED" });
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    let elapsed = start.elapsed();
    if !dhcp_done { println!("  {} DHCP test timed out after {:.0}s", "[!]".bold().red(), elapsed.as_secs_f64()); }
    println!();
    print_dhcp_test_result(&output_lines, dhcp_passed, dhcp_done, elapsed);

    Ok(DhcpTestResult { dhcp_passed, output_lines, total_duration: elapsed })
}

fn print_dhcp_serial_line(line: &str) {
    if line.contains("DHCPTEST_COMPLETE") { println!("{} DHCP TEST COMPLETE", "[+]".bold().green()); }
    else if line.contains("DHCPTEST_PASSED") { println!("{} DHCP TEST PASSED", "[+]".bold().green()); }
    else if line.contains("DHCPTEST_FAILED") { println!("{} DHCP TEST FAILED", "[-]".bold().red()); }
    else if line.contains("[DHCPTEST]") {
        if line.contains("[PASS]") { println!("  {} {}", "[PASS]".bold().green(), line.trim()); }
        else if line.contains("[FAIL]") { println!("  {} {}", "[FAIL]".bold().red(), line.trim()); }
        else if line.contains("[WARN]") || line.contains("[INFO]") { println!("  {} {}", "[INFO]".bold().yellow(), line.trim()); }
        else { println!("  {}", line.trim()); }
    } else if line.contains("NeoDOS Kernel v") || line.contains("Bootloader v") { println!("  {}", line); }
    else if line.contains("[DHCPTEST] ERROR") || line.contains("PANIC") || line.contains("panic") { println!("  {} {}", "[!]".bold().red(), line); }
}

fn print_dhcp_test_result(lines: &[String], dhcp_passed: bool, dhcp_done: bool, elapsed: Duration) {
    println!("{}", "=".repeat(60));
    println!("{}", "DHCP TEST RESULTS".bold());
    println!("{}", "=".repeat(60));
    if dhcp_done {
        if dhcp_passed { println!("  {} DHCP test: PASSED", "[PASS]".bold().green()); }
        else { println!("  {} DHCP test: FAILED", "[FAIL]".bold().red()); }
    } else { println!("  {} DHCP test: TIMEOUT", "[!]".bold().red()); }
    println!("  Duration: {:.1}s", elapsed.as_secs_f64());
    println!();
    println!("{}", "SERIAL OUTPUT SUMMARY".bold());
    println!("{}", "-".repeat(60));
    for line in lines { if line.contains("[DHCPTEST]") { println!("  {}", line.trim()); } }
    println!();
    if dhcp_done && dhcp_passed { println!("{} DHCP INTEGRATION TEST: SUCCESS", "[✓]".bold().green()); }
    else { println!("{} DHCP INTEGRATION TEST: FAILED", "[✗]".bold().red()); }
    println!("  Duration: {:.1}s", elapsed.as_secs_f64());
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.next() == Some('[') { for ch in chars.by_ref() { if ch.is_ascii_alphabetic() { break; } } }
        } else { result.push(c); }
    }
    result
}
