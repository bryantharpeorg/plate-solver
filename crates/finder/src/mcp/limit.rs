//! One sliding second shared by every client's moving tools.
use std::collections::VecDeque;

#[derive(Default)]
pub(crate) struct MoveLimit {
    accepted: VecDeque<f64>,
}
impl MoveLimit {
    pub(crate) fn accept(&mut self, now: f64) -> Result<(), String> {
        while self.accepted.front().is_some_and(|at| now - at >= 1.0) {
            self.accepted.pop_front();
        }
        if self.accepted.len() == 2 {
            return Err("the rate limit of 2 moving calls a second was reached".into());
        }
        self.accepted.push_back(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_slides_and_excess_calls_do_not_extend_it() {
        let mut limit = MoveLimit::default();
        assert!(limit.accept(0.0).is_ok());
        assert!(limit.accept(0.6).is_ok());
        for time in [0.7, 0.9, 0.99] {
            assert_eq!(
                limit.accept(time).unwrap_err(),
                "the rate limit of 2 moving calls a second was reached"
            );
        }
        assert!(limit.accept(1.0).is_ok());
        assert!(limit.accept(1.1).is_err());
        assert!(limit.accept(1.6).is_ok());
        assert!(limit.accept(2.6).is_ok());
        assert!(limit.accept(2.6).is_ok());
        assert!(limit.accept(2.6).is_err());
    }
}
