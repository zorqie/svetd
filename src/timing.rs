#![allow(unused)]
use crossbeam_channel::Sender;
use log::{debug, trace};
use rusty_link::{AblLink, SessionState};
use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering::Relaxed}},
    time::{Duration, Instant},
};

pub enum Timing {
    Tap,
    ChangeBpm(f32),
    ChangeFadeMode(FadeMode), // TODO do we even need this?
}

pub fn start_timer() -> Sender<Timing> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut t = Timer::default();
    
    tx
}

pub static CONNECTED_PEERS: AtomicU64 = AtomicU64::new(0);

pub struct Timer {
    link: AblLink,
    beats_per_minute: f32,
    previous_change_beats_per_minute: f32,
    pub change_beats_per_minute: f32,
    beat_progression: f32,
    avg_fps: Option<f32>,
    avg_fps_time: Instant,
    last_frame: Instant,
    frame_count: usize,
    taps: [Option<Instant>; 4],
    tap_count: usize,
    pub fade_mode: FadeMode,
}

impl Default for Timer {
    fn default() -> Self {
        let link = AblLink::new(120.0);
        link.enable(true);
        link.enable_start_stop_sync(true);

        Self {
            link,
            beats_per_minute: 120.0,
            previous_change_beats_per_minute: 120.0,
            change_beats_per_minute: 120.0,
            beat_progression: 0.0,
            avg_fps: None,
            avg_fps_time: Instant::now(),
            last_frame: Instant::now(),
            frame_count: 0,
            taps: [None; 4],
            tap_count: 0,
            fade_mode: Default::default(),
        }
    }
}

#[derive(Default, PartialEq, Eq)]
pub enum FadeMode {
    Instant,
    #[default]
    Beat,
    Beats4,
    Beats16,
}

impl Timer {
    pub fn beat_progression(&self) -> f32 {
        self.beat_progression
    }

    pub fn beats_per_minute(&self) -> f32 {
        self.beats_per_minute
    }

    pub fn framerate(&self) -> Option<f32> {
        self.avg_fps
    }

    // #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn tick(&mut self) {
        self.limit_fps();
        self.set_link_values();
        self.get_link_values();
        self.calculate_avg_fps();
        self.remove_old_taps();
    }

    // #[cfg_attr(feature = "profiling", profiling::function)]
    fn limit_fps(&mut self) {
        let fps_limit = 60.; //PersistantState::fps_limit();
        let target_frame_time_nanos = 1e+9f32 / fps_limit;
        while target_frame_time_nanos > (self.last_frame.elapsed().as_nanos() as f32) {
            std::thread::sleep(std::time::Duration::from_nanos(100));
        }
        self.last_frame = Instant::now();
    }

    // #[cfg_attr(feature = "profiling", profiling::function)]
    fn get_link_values(&mut self) {
        CONNECTED_PEERS.store(self.link.num_peers(), Relaxed);

        let mut session_state = SessionState::default();
        self.link.capture_app_session_state(&mut session_state);
        let now = self.link.clock_micros();
        self.beat_progression = 40.0 + session_state.beat_at_time(now, 4.0) as f32;
        self.beats_per_minute = session_state.tempo() as f32;
    }

    // #[cfg_attr(feature = "profiling", profiling::function)]
    fn set_link_values(&mut self) {
        if self.previous_change_beats_per_minute == self.change_beats_per_minute {
            self.previous_change_beats_per_minute = self.beats_per_minute;
            self.change_beats_per_minute = self.beats_per_minute;
            return;
        }

        let mut session_state = SessionState::default();
        self.link.capture_app_session_state(&mut session_state);
        let now = self.link.clock_micros();
        session_state.set_tempo(self.change_beats_per_minute as f64, now);
        self.link.commit_app_session_state(&session_state);
        self.previous_change_beats_per_minute = self.change_beats_per_minute;
    }

    #[inline]
    fn beat_duration_nanoseconds(&self) -> f64 {
        60e+9f64 / self.beats_per_minute as f64
    }

    pub fn fade_duration(&self) -> Duration {
        Duration::from_nanos(
            match self.fade_mode {
                FadeMode::Instant => 0.0,
                FadeMode::Beat => self.beat_duration_nanoseconds(),
                FadeMode::Beats4 => self.beat_duration_nanoseconds() * 4.0,
                FadeMode::Beats16 => self.beat_duration_nanoseconds() * 16.0,
            }
            .round() as u64,
        )
    }

    fn calculate_avg_fps(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.avg_fps_time).as_millis() > 1000 {
            let avg_frame_time = self.avg_fps_time.elapsed() / self.frame_count as u32;
            let fps = 1e+9f32 / (avg_frame_time.as_nanos() as f32);
            self.avg_fps = Some(fps);
            trace!("fps: {fps}");
            self.avg_fps_time = now;
            self.frame_count = 0;
        }
    }

    fn remove_old_taps(&mut self) {
        let mut reset_tap_count = true;
        for tap in self.taps.iter_mut() {
            if let Some(date) = tap {
                if date.elapsed() > Duration::from_secs(10) {
                    tap.take();
                } else {
                    reset_tap_count = false;
                }
            }
        }

        if reset_tap_count {
            self.tap_count = 0;
        }
    }

    // pub fn half_button(&mut self, ui: &mut Ui, tap_input: bool) {
    //     if ui.add(Button::new("x½")).clicked() || tap_input {
    //         self.multiply_speed(0.5);
    //     }
    // }

    // pub fn double_button(&mut self, ui: &mut Ui, tap_input: bool) {
    //     if ui.add(Button::new("x2")).clicked() || tap_input {
    //         self.multiply_speed(2.0);
    //     }
    // }

    pub fn multiply_speed(&mut self, multiplier: f32) {
        self.change_beats_per_minute *= multiplier;
    }

    pub fn add_speed(&mut self, delta: f32) {
        self.change_beats_per_minute += delta;
    }

    // pub fn tap_button(&mut self, ui: &mut Ui, menu_button_size: Vec2, tap_input: bool) {
    //     let underlined = TextFormat {
    //         underline: Stroke::new(1.0, Color32::GRAY),
    //         ..Default::default()
    //     };
    //     let mut tap_text = LayoutJob::default();
    //     tap_text.append("T", 0.0, underlined);
    //     tap_text.append(
    //         &format!(
    //             "ap{}",
    //             match (self.tap_count, self.tap_count % 4) {
    //                 (0, _) => "",
    //                 (_, 0) => "/",
    //                 (_, 1) => "–",
    //                 (_, 2) => "\\",
    //                 _ => "|",
    //             }
    //         ),
    //         0.0,
    //         TextFormat::default(),
    //     );
    //     let response = ui.add_sized(menu_button_size, Button::new(tap_text));

    //     let mut alpha = None;
    //     let bar_progression = self.beat_progression % 1.0;
    //     if bar_progression < 0.10 {
    //         alpha = Some(30.0);
    //     } else if bar_progression < 0.20 {
    //         alpha = Some(20.0 - ((bar_progression - 0.1) * 200.0));
    //     } else if bar_progression > 0.9 {
    //         alpha = Some((bar_progression - 0.9) * 200.0);
    //     }
    //     if let Some(alpha) = alpha {
    //         ui.painter().rect_filled(
    //             match self.beat_flank() {
    //                 0 =>
    //                 // top left
    //                 {
    //                     response
    //                         .rect
    //                         .split_left_right_at_fraction(0.5)
    //                         .0
    //                         .split_top_bottom_at_fraction(0.5)
    //                         .0
    //                 }
    //                 1 =>
    //                 // top right
    //                 {
    //                     response
    //                         .rect
    //                         .split_left_right_at_fraction(0.5)
    //                         .1
    //                         .split_top_bottom_at_fraction(0.5)
    //                         .0
    //                 }
    //                 2 =>
    //                 // bottom left
    //                 {
    //                     response
    //                         .rect
    //                         .split_left_right_at_fraction(0.5)
    //                         .0
    //                         .split_top_bottom_at_fraction(0.5)
    //                         .1
    //                 }
    //                 3 =>
    //                 // bottom right
    //                 {
    //                     response
    //                         .rect
    //                         .split_left_right_at_fraction(0.5)
    //                         .1
    //                         .split_top_bottom_at_fraction(0.5)
    //                         .1
    //                 }
    //                 _ => unreachable!(),
    //             }
    //             .shrink(1.0),
    //             CornerRadius::default(),
    //             Color32::from_white_alpha(alpha as u8),
    //         );
    //     }

    //     let tapped = response.clicked() || tap_input;

    //     if tapped {
    //         self.tap();
    //     }
    // }

    pub fn beat_flank(&self) -> u8 {
        match self.beat_progression % 4.0 {
            0.5..1.5 => 1,
            1.5..2.5 => 2,
            2.5..3.5 => 3,
            _ => 0,
        }
    }

    pub fn tap(&mut self) {
        let link_now = self.link.clock_micros();

        self.tap_count += 1;
        let now = Instant::now();
        self.taps[0] = Some(now);
        self.taps.sort();

        let first = self
            .taps
            .iter()
            .filter_map(|tap| *tap)
            .next()
            .expect("There must be at least one entry");
        if first == now {
            return;
        }

        let beat_time_first = now.duration_since(first);
        let avg_beat_time = (beat_time_first.as_nanos())
            / (self.taps.iter().filter(|tap| tap.is_some()).count() as u128 - 1);

        let new_beats_per_minute = (60e+9f64 / f64::from(avg_beat_time as u32)) as f32;

        // adjust beat progression timing to last tap
        let offset = self.beat_progression % 4.0;
        let goal_offset = (self.tap_count - 1) as f32 % 4.0;
        let mut session_state = SessionState::default();
        self.link.capture_app_session_state(&mut session_state);
        session_state.set_tempo(new_beats_per_minute as f64, link_now);
        session_state.request_beat_at_time(
            (self.beat_progression + goal_offset - offset) as f64,
            link_now,
            4.0,
        );
        self.link.commit_app_session_state(&session_state);
    }
}
