use chrono::{Datelike, NaiveDate, Weekday};

/// Florida Business Calendar for calculating notice periods per § 83.56 and § 83.57
pub struct FloridaBusinessCalendar;

impl FloridaBusinessCalendar {
    /// Add business days to a date, excluding weekends and Florida state holidays
    pub fn add_business_days(start_date: NaiveDate, days: u32) -> NaiveDate {
        let mut current_date = start_date;
        let mut remaining_days = days;

        while remaining_days > 0 {
            current_date = current_date.succ_opt().expect("Date overflow");

            if Self::is_business_day(current_date) {
                remaining_days -= 1;
            }
        }

        current_date
    }

    /// Check if a date is a business day (not weekend or holiday)
    pub fn is_business_day(date: NaiveDate) -> bool {
        !Self::is_weekend(date) && !Self::is_holiday(date)
    }

    /// Check if a date is a weekend
    fn is_weekend(date: NaiveDate) -> bool {
        matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }

    /// Check if a date is a Florida state holiday
    pub fn is_holiday(date: NaiveDate) -> bool {
        let year = date.year();
        let month = date.month();
        let day = date.day();

        // Fixed date holidays
        if Self::is_fixed_holiday(month, day) {
            return true;
        }

        // Floating holidays (calculated based on year)
        Self::is_floating_holiday(date, year)
    }

    /// Check if a date is a fixed Florida state holiday
    fn is_fixed_holiday(month: u32, day: u32) -> bool {
        matches!(
            (month, day),
            (1, 1) |   // New Year's Day
            (7, 4) |   // Independence Day
            (11, 11) | // Veterans Day
            (12, 25) // Christmas
        )
    }

    /// Check if a date is a floating Florida state holiday
    fn is_floating_holiday(date: NaiveDate, year: i32) -> bool {
        // Martin Luther King Jr. Day - 3rd Monday in January
        if Self::is_nth_weekday_of_month(date, year, 1, Weekday::Mon, 3) {
            return true;
        }

        // Memorial Day - Last Monday in May
        if Self::is_last_weekday_of_month(date, year, 5, Weekday::Mon) {
            return true;
        }

        // Labor Day - 1st Monday in September
        if Self::is_nth_weekday_of_month(date, year, 9, Weekday::Mon, 1) {
            return true;
        }

        // Thanksgiving - 4th Thursday in November
        if Self::is_nth_weekday_of_month(date, year, 11, Weekday::Thu, 4) {
            return true;
        }

        // Day after Thanksgiving - Friday after 4th Thursday in November
        if let Some(thanksgiving) = Self::get_nth_weekday_of_month(year, 11, Weekday::Thu, 4) {
            if let Some(day_after) = thanksgiving.succ_opt() {
                if date == day_after {
                    return true;
                }
            }
        }

        false
    }

    /// Check if date is the nth occurrence of a weekday in a month
    fn is_nth_weekday_of_month(
        date: NaiveDate,
        year: i32,
        month: u32,
        weekday: Weekday,
        n: u32,
    ) -> bool {
        if let Some(target) = Self::get_nth_weekday_of_month(year, month, weekday, n) {
            date == target
        } else {
            false
        }
    }

    /// Get the nth occurrence of a weekday in a month
    fn get_nth_weekday_of_month(
        year: i32,
        month: u32,
        weekday: Weekday,
        n: u32,
    ) -> Option<NaiveDate> {
        let first = NaiveDate::from_ymd_opt(year, month, 1)?;
        let mut current = first;
        let mut count = 0;

        // Search through the month
        while current.month() == month {
            if current.weekday() == weekday {
                count += 1;
                if count == n {
                    return Some(current);
                }
            }
            current = current.succ_opt()?;
        }

        None
    }

    /// Check if date is the last occurrence of a weekday in a month
    fn is_last_weekday_of_month(date: NaiveDate, year: i32, month: u32, weekday: Weekday) -> bool {
        if let Some(target) = Self::get_last_weekday_of_month(year, month, weekday) {
            date == target
        } else {
            false
        }
    }

    /// Get the last occurrence of a weekday in a month
    fn get_last_weekday_of_month(year: i32, month: u32, weekday: Weekday) -> Option<NaiveDate> {
        // Start from the last day of the month and work backwards
        let last_day = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)?.pred_opt()?
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)?.pred_opt()?
        };

        let mut current = last_day;
        while current.month() == month {
            if current.weekday() == weekday {
                return Some(current);
            }
            current = current.pred_opt()?;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_three_day_notice_excludes_weekends() {
        // Friday notice should have deadline on Wednesday
        let notice_date = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(); // Friday
        let deadline = FloridaBusinessCalendar::add_business_days(notice_date, 3);
        assert_eq!(deadline, NaiveDate::from_ymd_opt(2024, 1, 10).unwrap()); // Wednesday
    }

    #[test]
    fn test_excludes_florida_holidays() {
        // Memorial Day 2024 is May 27
        let notice_date = NaiveDate::from_ymd_opt(2024, 5, 24).unwrap(); // Friday before
        let deadline = FloridaBusinessCalendar::add_business_days(notice_date, 3);
        // Skip Sat, Sun, Memorial Day Monday = Tuesday May 28, Wed 29, Thu 30
        assert_eq!(deadline, NaiveDate::from_ymd_opt(2024, 5, 30).unwrap());
    }

    #[test]
    fn test_new_years_day() {
        let notice_date = NaiveDate::from_ymd_opt(2023, 12, 29).unwrap(); // Friday
        let deadline = FloridaBusinessCalendar::add_business_days(notice_date, 3);
        // Skip Sat 30, Sun 31, Mon Jan 1 (holiday) = Tue 2, Wed 3, Thu 4
        assert_eq!(deadline, NaiveDate::from_ymd_opt(2024, 1, 4).unwrap());
    }

    #[test]
    fn test_is_florida_holiday() {
        // New Year's Day 2024
        assert!(FloridaBusinessCalendar::is_holiday(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        ));
        // Regular day
        assert!(!FloridaBusinessCalendar::is_holiday(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()
        ));
    }

    #[test]
    fn test_seven_day_notice() {
        let notice_date = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(); // Friday
        let deadline = FloridaBusinessCalendar::add_business_days(notice_date, 7);
        // 7 business days from Friday March 1
        assert_eq!(deadline, NaiveDate::from_ymd_opt(2024, 3, 12).unwrap());
    }
}
