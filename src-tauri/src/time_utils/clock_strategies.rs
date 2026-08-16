#![allow(clippy::missing_const_for_fn)]
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimerState {
    Idle,
    Running { start_time_ms: u64, elapsed_ms: u64 },
    Paused { elapsed_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroTimer {
    pub state: TimerState,
    pub work_duration: Duration,
    pub break_duration: Duration,
    pub is_break: bool,
}

impl PomodoroTimer {
    #[must_use]
    pub fn new(work_mins: u64, break_mins: u64) -> Self {
        Self {
            state: TimerState::Idle,
            work_duration: Duration::from_secs(work_mins * 60),
            break_duration: Duration::from_secs(break_mins * 60),
            is_break: false,
        }
    }

    pub fn start(&mut self, now_ms: u64) {
        if matches!(self.state, TimerState::Idle | TimerState::Paused { .. }) {
            let elapsed = match self.state {
                TimerState::Paused { elapsed_ms } => elapsed_ms,
                _ => 0,
            };
            self.state = TimerState::Running {
                start_time_ms: now_ms,
                elapsed_ms: elapsed,
            };
        }
    }

    pub fn pause(&mut self, now_ms: u64) {
        if let TimerState::Running { start_time_ms, elapsed_ms } = self.state {
            let current_elapsed = elapsed_ms + now_ms.saturating_sub(start_time_ms);
            self.state = TimerState::Paused { elapsed_ms: current_elapsed };
        }
    }
    
    pub fn reset(&mut self) {
        self.state = TimerState::Idle;
        self.is_break = false;
    }

    pub fn tick(&mut self, now_ms: u64) -> Option<bool> {
        if let TimerState::Running { start_time_ms, elapsed_ms } = self.state {
            let total_elapsed = elapsed_ms + now_ms.saturating_sub(start_time_ms);
            let current_duration = if self.is_break {
                u64::try_from(self.break_duration.as_millis()).unwrap_or(u64::MAX)
            } else {
                u64::try_from(self.work_duration.as_millis()).unwrap_or(u64::MAX)
            };

            if total_elapsed >= current_duration {
                self.is_break = !self.is_break;
                self.state = TimerState::Idle;
                return Some(self.is_break);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pomodoro_flow() {
        let mut timer = PomodoroTimer::new(25, 5);
        assert_eq!(timer.state, TimerState::Idle);
        
        timer.start(1000);
        assert!(matches!(timer.state, TimerState::Running { .. }));
        
        // Advance 25 mins
        let res = timer.tick(1000 + 25 * 60 * 1000);
        assert_eq!(res, Some(true)); // transitioned to break
        assert_eq!(timer.state, TimerState::Idle);
        assert!(timer.is_break);
    }
}
