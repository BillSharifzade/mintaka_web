//! When to order a certificate: about 30 days before the current one expires, a third of the
//! lifetime before it for shorter-lived certificates, and with growing pauses after failures.
//! Pure functions of the wall clock, so the whole policy is unit-tested.

use std::time::{Duration, SystemTime};

const DAY: Duration = Duration::from_secs(86_400);

/// Renew this long before expiry (Let's Encrypt's own advice for 90-day certificates).
pub(crate) const RENEW_BEFORE: Duration = Duration::from_secs(30 * 86_400);
/// The first retry after a failed order.
pub(crate) const FIRST_RETRY: Duration = Duration::from_secs(5 * 60);
/// Retries double up to this while nothing usable is served, or the certificate expires within
/// a week. Let's Encrypt allows 5 failed validations per name per hour.
pub(crate) const MAX_RETRY_URGENT: Duration = Duration::from_secs(3600);
/// …and up to this while a good certificate is still being served.
pub(crate) const MAX_RETRY_RELAXED: Duration = Duration::from_secs(6 * 3600);

/// The validity period of a certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Validity {
    pub not_before: SystemTime,
    pub not_after: SystemTime,
}

/// The moment to renew: [`RENEW_BEFORE`] ahead of expiry, or a third of the lifetime ahead of
/// it when that is shorter (Let's Encrypt's 45- and 6-day profiles), so a fresh short-lived
/// certificate is never "due" the moment it arrives.
pub(crate) fn renew_at(v: Validity) -> SystemTime {
    let lifetime = v.not_after.duration_since(v.not_before).unwrap_or(Duration::ZERO);
    v.not_after.checked_sub(RENEW_BEFORE.min(lifetime / 3)).unwrap_or(v.not_before)
}

/// How long to wait before the next order, given the certificate being served (if any) and
/// the number of consecutive failed orders.
pub(crate) fn next_attempt(now: SystemTime, current: Option<Validity>, failures: u32) -> Duration {
    let usable = current.filter(|v| now < v.not_after);
    if failures > 0 {
        let relaxed = usable.is_some_and(|v| v.not_after.duration_since(now).unwrap_or(Duration::ZERO) > 7 * DAY);
        let cap = if relaxed { MAX_RETRY_RELAXED } else { MAX_RETRY_URGENT };
        return FIRST_RETRY.saturating_mul(1 << (failures - 1).min(16)).min(cap);
    }
    match usable {
        Some(v) => renew_at(v).duration_since(now).unwrap_or(Duration::ZERO),
        None => Duration::ZERO,
    }
}

/// `59 d 23 h`, `4 h 10 min`, `5 min` — for logs.
pub(crate) fn human(d: Duration) -> String {
    let s = d.as_secs();
    match (s / 86_400, s % 86_400 / 3600, s % 3600 / 60) {
        (0, 0, m) => format!("{m} min"),
        (0, h, m) => format!("{h} h {m} min"),
        (d, h, _) => format!("{d} d {h} h"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    fn at(days: f64) -> SystemTime {
        let base = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let offset = Duration::from_secs_f64(days.abs() * 86_400.0);
        if days < 0.0 { base - offset } else { base + offset }
    }

    fn cert(from: f64, until: f64) -> Validity {
        Validity { not_before: at(from), not_after: at(until) }
    }

    #[test]
    fn renews_thirty_days_before_expiry_or_a_third_of_short_lifetimes() {
        assert_eq!(renew_at(cert(0.0, 90.0)), at(60.0), "90-day certificate: day 60");
        assert_eq!(renew_at(cert(0.0, 45.0)), at(30.0), "45-day: 15 days ahead, not at once");
        assert_eq!(renew_at(cert(0.0, 6.0)), at(4.0), "6-day: 2 days ahead");
        assert_eq!(renew_at(cert(0.0, 397.0)), at(367.0), "long-lived: still 30 days");
        assert_eq!(renew_at(cert(5.0, 5.0)), at(5.0), "degenerate");
    }

    #[test]
    fn waits_until_due_and_orders_at_once_without_a_usable_certificate() {
        let c = cert(0.0, 90.0);
        assert_eq!(next_attempt(at(0.0), Some(c), 0), Duration::from_secs(60 * 86_400));
        assert_eq!(next_attempt(at(59.5), Some(c), 0), Duration::from_secs(43_200));
        assert_eq!(next_attempt(at(75.0), Some(c), 0), Duration::ZERO, "overdue");
        assert_eq!(next_attempt(at(91.0), Some(c), 0), Duration::ZERO, "expired");
        assert_eq!(next_attempt(at(0.0), None, 0), Duration::ZERO, "nothing cached");
    }

    #[test]
    fn backs_off_after_failures_more_patiently_while_a_good_certificate_is_served() {
        let none: Vec<_> = (1..=6).map(|f| next_attempt(at(0.0), None, f).as_secs() / 60).collect();
        assert_eq!(none, [5, 10, 20, 40, 60, 60], "minutes, capped at an hour");
        let fine = cert(-60.0, 30.0);
        let relaxed: Vec<_> = (1..=8).map(|f| next_attempt(at(0.0), Some(fine), f).as_secs() / 60).collect();
        assert_eq!(relaxed, [5, 10, 20, 40, 80, 160, 320, 360], "capped at six hours");
        let expiring = cert(-85.0, 5.0);
        assert_eq!(next_attempt(at(0.0), Some(expiring), 8), MAX_RETRY_URGENT, "a week from expiry: hourly");
        assert_eq!(next_attempt(at(0.0), None, u32::MAX), MAX_RETRY_URGENT, "no overflow");
    }

    #[test]
    fn human_durations() {
        assert_eq!(human(Duration::from_secs(300)), "5 min");
        assert_eq!(human(Duration::from_secs(4 * 3600 + 600)), "4 h 10 min");
        assert_eq!(human(Duration::from_secs(60 * 86_400 - 3600)), "59 d 23 h");
    }
}
