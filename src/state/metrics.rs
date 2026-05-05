use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use std::time::Instant;

const MEMORY_REFRESH_INTERVAL_MS: u64 = 500;
const MAX_STEP_HISTORY: usize = 64;

#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub runtime_ms: f64,
    pub step_time_ms: f64,
    pub process_memory_bytes: u64,
    pub statevector_bytes: usize,
    pub peak_memory_bytes: u64,
    /// Per-step execution times in ms. Capped at MAX_STEP_HISTORY.
    pub step_times_ms: Vec<f64>,
    /// Per-step gate counts.
    pub step_gate_counts: Vec<usize>,
    /// Compression ratio (original / compressed). 1.0 means no compression.
    pub compression_ratio: f64,
    /// State fidelity after compress→decompress roundtrip. 1.0 = lossless.
    pub compression_fidelity: f64,
    /// Norm error after decompression. 0.0 = perfect.
    pub compression_norm_error: f64,
    /// Compressed payload size in bytes (quantized indices, bit-packed).
    pub compressed_payload_bytes: usize,
    /// Compressed metadata size in bytes (scale, norm, seed, etc.).
    pub compressed_metadata_bytes: usize,
    /// Quantization bit width (1–8). 0 = no compression was applied.
    pub compression_bits: u8,
}

impl Metrics {
    pub fn update_peak_memory(&mut self, current: u64) {
        if current > self.peak_memory_bytes {
            self.peak_memory_bytes = current;
        }
    }

    pub fn avg_step_time_ms(&self) -> f64 {
        if self.step_times_ms.is_empty() {
            return 0.0;
        }
        self.step_times_ms.iter().sum::<f64>() / self.step_times_ms.len() as f64
    }

    pub fn max_step_time_ms(&self) -> f64 {
        self.step_times_ms.iter().cloned().fold(0.0_f64, f64::max)
    }

    pub fn total_gates(&self) -> usize {
        self.step_gate_counts.iter().sum()
    }

    pub fn compressed_total_bytes(&self) -> usize {
        self.compressed_payload_bytes + self.compressed_metadata_bytes
    }

    pub fn is_compressed(&self) -> bool {
        self.compression_bits > 0
    }
}

/// Compression metadata when the TurboSpin/Spinoza CLI emits report lines on stdout.
/// Passed from `turbospin.rs` into `MetricsTracker::finalize_run`.
#[derive(Clone, Debug, Default)]
pub struct CompressionInfo {
    pub ratio: f64,
    pub fidelity: f64,
    pub norm_error: f64,
    pub payload_bytes: usize,
    pub metadata_bytes: usize,
    pub bits: u8,
}

pub struct MetricsTracker {
    system: System,
    pid: sysinfo::Pid,
    peak_memory_bytes: u64,
    start_time: Option<Instant>,
    last_memory_refresh: Option<Instant>,
    step_start: Option<Instant>,
    step_times_ms: Vec<f64>,
    step_gate_counts: Vec<usize>,
    current_step_gates: usize,
}

impl MetricsTracker {
    pub fn new() -> Self {
        let mut system = System::new();
        let pid = sysinfo::get_current_pid().unwrap_or_else(|_| sysinfo::Pid::from_u32(0));
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        let mem = system.process(pid).map(|p| p.memory()).unwrap_or(0);

        Self {
            system,
            pid,
            peak_memory_bytes: mem,
            start_time: None,
            last_memory_refresh: None,
            step_start: None,
            step_times_ms: Vec::with_capacity(MAX_STEP_HISTORY),
            step_gate_counts: Vec::with_capacity(MAX_STEP_HISTORY),
            current_step_gates: 0,
        }
    }

    pub fn start_run(&mut self) {
        self.start_time = Some(Instant::now());
        self.last_memory_refresh = None;
        self.step_times_ms.clear();
        self.step_gate_counts.clear();
        self.current_step_gates = 0;
        self.throttled_refresh_memory();
    }

    /// Begin timing a single step. Call before executing the step's gates.
    pub fn begin_step(&mut self) {
        self.step_start = Some(Instant::now());
        self.current_step_gates = 0;
    }

    /// Record that a gate was executed within the current step.
    /// Call once per gate applied during the step.
    pub fn record_gate(&mut self) {
        self.current_step_gates += 1;
    }

    /// Finish timing the current step and record its duration + gate count.
    pub fn end_step(&mut self) {
        if let Some(start) = self.step_start.take() {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            if self.step_times_ms.len() >= MAX_STEP_HISTORY {
                self.step_times_ms.remove(0);
                self.step_gate_counts.remove(0);
            }
            self.step_times_ms.push(elapsed);
            self.step_gate_counts.push(self.current_step_gates);
        }
        self.current_step_gates = 0;
        self.throttled_refresh_memory();
    }

    /// Refresh process memory only if enough time has elapsed since the last
    /// refresh. Avoids expensive syscalls in tight loops.
    pub fn throttled_refresh_memory(&mut self) {
        let now = Instant::now();
        let should_refresh = self
            .last_memory_refresh
            .map_or(true, |last| now.duration_since(last).as_millis() as u64 >= MEMORY_REFRESH_INTERVAL_MS);
        if !should_refresh {
            return;
        }
        self.last_memory_refresh = Some(now);
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        if let Some(process) = self.system.process(self.pid) {
            let mem = process.memory();
            if mem > self.peak_memory_bytes {
                self.peak_memory_bytes = mem;
            }
        }
    }

    pub fn refresh_memory(&mut self) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        if let Some(process) = self.system.process(self.pid) {
            let mem = process.memory();
            if mem > self.peak_memory_bytes {
                self.peak_memory_bytes = mem;
            }
        }
    }

    pub fn finalize_run(
        &mut self,
        _num_qubits: usize,
        total_steps: usize,
        compression: Option<CompressionInfo>,
        actual_statevector_bytes: usize,
    ) -> Metrics {
        self.refresh_memory();

        let runtime_ms = self.start_time.map(|t| t.elapsed().as_secs_f64() * 1000.0).unwrap_or(0.0);
        let step_time_ms = if total_steps > 0 { runtime_ms / (total_steps as f64) } else { 0.0 };

        let process_memory_bytes = self.system.process(self.pid).map(|p| p.memory()).unwrap_or(0);
        let statevector_bytes = actual_statevector_bytes;

        let (
            compression_ratio,
            compression_fidelity,
            compression_norm_error,
            compressed_payload_bytes,
            compressed_metadata_bytes,
            compression_bits,
        ) = match &compression {
            Some(c) => (c.ratio, c.fidelity, c.norm_error, c.payload_bytes, c.metadata_bytes, c.bits),
            None => (1.0, 1.0, 0.0, 0, 0, 0),
        };

        Metrics {
            runtime_ms,
            step_time_ms,
            process_memory_bytes,
            statevector_bytes,
            peak_memory_bytes: self.peak_memory_bytes,
            step_times_ms: self.step_times_ms.clone(),
            step_gate_counts: self.step_gate_counts.clone(),
            compression_ratio,
            compression_fidelity,
            compression_norm_error,
            compressed_payload_bytes,
            compressed_metadata_bytes,
            compression_bits,
        }
    }
}
