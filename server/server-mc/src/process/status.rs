use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProcessStatus {
    Starting(StartingStatus),
    Running,
    Terminating,
    Terminated,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum StartingStatus {
    DownloadServerJar,
    DownloadWorld,
    InitializeConfigFile,
    WaitingForServerReady,
}

#[derive(Debug, Clone, Default)]
pub struct StatusInfo {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
