use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc, offset::TimeZone};
use crate::{Result, ZKPNL_CONFIG};

pub fn now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&time_zone())
}

pub enum TimeRange {
    Range(DateTime<FixedOffset>, DateTime<FixedOffset>),
    UpToLastSince(DateTime<FixedOffset>),
    UpToNowSince(DateTime<FixedOffset>),
    UpTo(DateTime<FixedOffset>),
    UpToNow,
    UpToLast,
}

impl TimeRange {
    pub fn new(arg1: Option<&&str>, arg2: Option<&&str>, arg3: Option<&&str>, arg4: Option<&&str>) -> Result<TimeRange> {
        match (arg1, arg2, arg3, arg4) {
            (Some(&"from"), Some(start), Some(&"to"), Some(&"now")) => {
                let naive_start = NaiveDateTime::parse_from_str(start, "%Y%m%d%H%M")?;
                let start = time_zone().from_local_datetime(&naive_start).unwrap();
                Ok(TimeRange::UpToNowSince(start))
            },
            (Some(&"from"), Some(start), Some(&"to"), Some(end)) | (Some(&"to"), Some(end), Some(&"from"), Some(start)) => {
                let naive_start = NaiveDateTime::parse_from_str(start, "%Y%m%d%H%M")?;
                let start = time_zone().from_local_datetime(&naive_start).unwrap();
                let naive_end = NaiveDateTime::parse_from_str(end, "%Y%m%d%H%M")?;
                let end = time_zone().from_local_datetime(&naive_end).unwrap();
                Ok(TimeRange::Range(start, end))
            },
            (Some(&"from"), Some(start), _, _) => {
                let naive_start = NaiveDateTime::parse_from_str(start, "%Y%m%d%H%M")?;
                let start = time_zone().from_local_datetime(&naive_start).unwrap();
                Ok(TimeRange::UpToLastSince(start))
            },
            (Some(&"to"), Some(&"now"), _, _) => Ok(TimeRange::UpToNow),
            (Some(&"to"), Some(end), _, _) => {
                let naive_end = NaiveDateTime::parse_from_str(end, "%Y%m%d%H%M")?;
                let end = time_zone().from_local_datetime(&naive_end).unwrap();
                Ok(TimeRange::UpTo(end))
            },
            (None, None, None, None) => Ok(TimeRange::UpToLast),
            _ => {
                println!("{}", "command error, please follow time range format:");
                println!("{}", "[from <start>] [to (<end> | now)]");
                println!("{}", "ignore error and continue");
                Ok(TimeRange::UpToLast)
            }
        }
    }
}

fn time_zone() -> FixedOffset {
    FixedOffset::east(ZKPNL_CONFIG.time_zone * 3600)
}