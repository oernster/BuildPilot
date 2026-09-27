//! Application tests: every use case driven through `App` against fake ports. Each test names
//! the SRS requirement it verifies.

mod application {
    mod deck;
    mod fakes;
    mod navigation;
    mod run;
    mod startup;
    mod support;
}
