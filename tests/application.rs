//! Application tests: every use case driven through `App` against fake ports. Each test names
//! the SRS requirement it verifies.

mod application {
    mod concurrency;
    mod deck;
    mod fakes;
    mod hosts;
    mod installer;
    mod log;
    mod navigation;
    mod run;
    mod run_times;
    mod scan;
    mod startup;
    mod steps;
    mod support;
    mod updates;
}
