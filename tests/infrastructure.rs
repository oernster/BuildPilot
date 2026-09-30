//! Infrastructure tests: real files in temporary folders, plus real processes on Windows. Each
//! test names the SRS requirement it verifies.

mod infrastructure {
    mod build_info;
    mod config_store;
    mod icons;
    mod identity;
    #[cfg(windows)]
    mod instance;
    #[cfg(windows)]
    mod log_file;
    #[cfg(windows)]
    mod process;
    #[cfg(windows)]
    mod reader_panic;
    #[cfg(windows)]
    mod releases;
    mod run_times_store;
    #[cfg(windows)]
    mod setup_machine;
    mod system;
}
