use chrono::{Datelike, Local, NaiveDate};

/// Data di calendario (locale) usata per il nome dei file di journal, in
/// formato ISO 8601 (`YYYY-MM-DD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct JournalDate(NaiveDate);

impl JournalDate {
    pub fn today() -> Self {
        Self(Local::now().date_naive())
    }

    pub fn succ(&self) -> Self {
        Self(self.0.succ_opt().unwrap_or(self.0))
    }

    pub fn pred(&self) -> Self {
        Self(self.0.pred_opt().unwrap_or(self.0))
    }

    pub fn to_file_stem(self) -> String {
        format!(
            "{:04}-{:02}-{:02}",
            self.0.year(),
            self.0.month(),
            self.0.day()
        )
    }

    /// Interpreta una stringa `YYYY-MM-DD` come data di journal, oppure
    /// `None` se non è una data ISO 8601 valida.
    pub fn parse(text: &str) -> Option<Self> {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").ok().map(Self)
    }
}

impl std::fmt::Display for JournalDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_file_stem())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_as_iso_8601() {
        let date = JournalDate(NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());
        assert_eq!(date.to_file_stem(), "2026-09-02");
    }

    #[test]
    fn succ_and_pred_are_inverse() {
        let date = JournalDate(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(date.succ().pred(), date);
    }

    #[test]
    fn parse_round_trips_with_to_file_stem() {
        let date = JournalDate(NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());
        assert_eq!(JournalDate::parse(&date.to_file_stem()), Some(date));
    }

    #[test]
    fn parse_rejects_invalid_strings() {
        assert_eq!(JournalDate::parse("not-a-date"), None);
        assert_eq!(JournalDate::parse("2026-13-40"), None);
    }

    #[test]
    fn ordering_is_chronological() {
        let earlier = JournalDate(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        let later = JournalDate(NaiveDate::from_ymd_opt(2026, 1, 2).unwrap());
        assert!(earlier < later);
    }
}
