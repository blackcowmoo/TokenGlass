pub(crate) fn current_unix_timestamp() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "현재 시간을 확인할 수 없습니다.".to_string())
        .map(|duration| duration.as_secs() as i64)
}

pub(crate) fn calculate_bounds_with_offset(now: i64, offset: time::UtcOffset) -> (i64, i64) {
    let local_now = time::OffsetDateTime::from_unix_timestamp(now)
        .map(|dt| dt.to_offset(offset))
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());

    let period_start = time::Date::from_calendar_date(local_now.year(), local_now.month(), 1)
        .map(|date| {
            date.with_time(time::Time::MIDNIGHT)
                .assume_offset(offset)
                .unix_timestamp()
        })
        .unwrap_or_else(|_| fallback_period_start(now));
    let today_start =
        time::Date::from_calendar_date(local_now.year(), local_now.month(), local_now.day())
            .map(|date| {
                date.with_time(time::Time::MIDNIGHT)
                    .assume_offset(offset)
                    .unix_timestamp()
            })
            .unwrap_or_else(|_| fallback_today_start(now));

    (period_start, today_start)
}

pub(crate) fn calculate_period_and_today_bounds(now: i64) -> (i64, i64) {
    let local_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    calculate_bounds_with_offset(now, local_offset)
}

fn fallback_period_start(now: i64) -> i64 {
    let utc_now = time::OffsetDateTime::from_unix_timestamp(now)
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    time::Date::from_calendar_date(utc_now.year(), utc_now.month(), 1)
        .map(|date| {
            date.with_time(time::Time::MIDNIGHT)
                .assume_utc()
                .unix_timestamp()
        })
        .unwrap_or(now - 30 * 86400)
}

fn fallback_today_start(now: i64) -> i64 {
    let utc_now = time::OffsetDateTime::from_unix_timestamp(now)
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    time::Date::from_calendar_date(utc_now.year(), utc_now.month(), utc_now.day())
        .map(|date| {
            date.with_time(time::Time::MIDNIGHT)
                .assume_utc()
                .unix_timestamp()
        })
        .unwrap_or(now - 86400)
}

#[cfg(test)]
mod tests {
    use super::calculate_bounds_with_offset;

    #[test]
    fn timezone_bounds_calculation_respects_offset() {
        let now_kst = 1_786_550_400_i64;
        let kst_offset = time::UtcOffset::from_hms(9, 0, 0).unwrap();
        let (period_start, today_start) = calculate_bounds_with_offset(now_kst, kst_offset);

        assert_eq!(today_start, 1_786_546_800);
        assert_eq!(period_start, 1_785_510_000);
    }
}
