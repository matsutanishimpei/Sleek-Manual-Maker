use std::path::PathBuf;

const APP_DIR_NAME: &str = "SleekManualMaker";

pub fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            return PathBuf::from(app_data).join(APP_DIR_NAME);
        }
    }

    PathBuf::from(".")
}

pub fn records_dir() -> PathBuf {
    app_data_dir().join("records")
}

pub fn log_dir() -> PathBuf {
    app_data_dir().join("log")
}

pub fn application_log_path() -> PathBuf {
    log_dir().join("application.log")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_paths_are_under_app_data_dir() {
        let base = app_data_dir();
        assert_eq!(records_dir(), base.join("records"));
        assert_eq!(log_dir(), base.join("log"));
        assert_eq!(
            application_log_path(),
            base.join("log").join("application.log")
        );
    }
}
