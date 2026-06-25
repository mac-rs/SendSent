use std::time::{Duration, Instant};

pub struct SpeedMeter { samples: Vec<(Instant, u64)>, window: Duration, last_total: u64 }

impl SpeedMeter {
    pub fn new(window: Duration) -> Self { Self { samples: Vec::new(), window, last_total: 0 } }
    pub fn record(&mut self, now: Instant, bytes_done_total: u64) {
        if let Some(last) = self.samples.last() {
            if bytes_done_total < last.1 { self.samples.clear(); }
        }
        self.samples.push((now, bytes_done_total));
        let cutoff = now - self.window;
        while self.samples.len() > 2 && self.samples[0].0 < cutoff { self.samples.remove(0); }
        self.last_total = bytes_done_total;
    }
    pub fn bps(&self) -> u64 {
        if self.samples.len() < 2 { return 0; }
        let (t0, b0) = self.samples[0];
        let (t1, b1) = *self.samples.last().unwrap();
        let secs = (t1 - t0).as_secs_f64().max(1e-6);
        (((b1 - b0) as f64) / secs) as u64
    }
    pub fn total(&self) -> u64 { self.last_total }
}

pub struct Throttle { period: Duration, last: Option<Instant> }
impl Throttle {
    pub fn new(hz: u32) -> Self { Self { period: Duration::from_secs_f64(1.0 / hz as f64), last: None } }
    pub fn allow(&mut self, now: Instant, force: bool) -> bool {
        let send = force || self.last.map(|l| now - l >= self.period).unwrap_or(true);
        if send { self.last = Some(now); true } else { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn throttle_respects_rate() {
        let mut t = Throttle::new(8);
        let t0 = Instant::now();
        assert!(t.allow(t0, false));
        assert!(!t.allow(t0, false));
        assert!(t.allow(t0 + Duration::from_millis(126), false));
    }
    #[test]
    fn speed_meter_basic() {
        let mut m = SpeedMeter::new(Duration::from_secs(2));
        let t0 = Instant::now();
        m.record(t0, 0);
        m.record(t0 + Duration::from_secs(1), 1_000_000);
        let bps = m.bps();
        assert!(bps >= 900_000 && bps <= 1_100_000, "got {bps}");
    }
}
