//! Infrastructure tests: real files in temporary folders, plus real processes on Windows. Each
//! test names the SRS requirement it verifies.

mod infrastructure {
    mod config_store;
    mod icons;
    #[cfg(windows)]
    mod process;
    mod system;
}
