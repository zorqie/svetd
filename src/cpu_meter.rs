use std::sync::atomic::AtomicUsize;

pub static CPU_USAGE: AtomicUsize = AtomicUsize::new(0);

pub fn start_cpu_meter() {
    let mut sys = sysinfo::System::new_all();
    std::thread::spawn(move || {
        loop {

            // First we update all information of our `System` struct.
            sys.refresh_cpu_usage();

            let max_cpu = (sys.global_cpu_usage() * 100.).round() as usize;

            CPU_USAGE.store(
                max_cpu,
                std::sync::atomic::Ordering::Relaxed,
            );
            println!("CPU: {:5.2}%", max_cpu as f32/100.);
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    });
}

pub fn cpu_usage() -> f32 {
    let cpu = CPU_USAGE.load(std::sync::atomic::Ordering::Relaxed);
    cpu as f32/100.
}
