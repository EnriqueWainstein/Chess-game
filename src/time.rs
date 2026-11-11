use std::time::Duration;

const SAFETY_MARGIN: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeBudget {
    pub soft: Duration,
    pub hard: Duration,
}

pub fn allocate_time(
    remaining: Duration,
    increment: Duration,
    moves_to_go: Option<u32>,
) -> TimeBudget {
    let usable = remaining.saturating_sub(SAFETY_MARGIN);
    if usable.is_zero() {
        return TimeBudget {
            soft: Duration::from_millis(1),
            hard: Duration::from_millis(1),
        };
    }

    let moves = moves_to_go.unwrap_or(30).max(1);
    let base = usable / moves;
    let soft = base
        .saturating_add(increment.mul_f64(0.8))
        .min(usable)
        .max(Duration::from_millis(1));
    let hard = soft
        .saturating_mul(3)
        .min(usable)
        .max(Duration::from_millis(1));
    TimeBudget { soft, hard }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_budget_preserves_safety_margin_and_respects_moves_to_go() {
        let budget = allocate_time(Duration::from_secs(2), Duration::ZERO, Some(20));
        assert_eq!(budget.soft, Duration::from_micros(97_500));
        assert_eq!(budget.hard, Duration::from_micros(292_500));
        assert!(budget.hard <= Duration::from_millis(1950));

        let low_time = allocate_time(Duration::from_millis(20), Duration::ZERO, None);
        assert_eq!(low_time.soft, Duration::from_millis(1));
        assert_eq!(low_time.hard, Duration::from_millis(1));
    }

    #[test]
    fn increment_is_used_without_exceeding_remaining_time_after_margin() {
        let budget = allocate_time(Duration::from_secs(1), Duration::from_millis(200), None);
        assert_eq!(budget.soft, Duration::from_nanos(191_666_666));
        assert_eq!(budget.hard, Duration::from_nanos(574_999_998));
        assert!(budget.hard <= Duration::from_millis(950));
    }
}
