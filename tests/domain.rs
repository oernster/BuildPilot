//! Domain tests: pure, no I/O. One module per domain module; each test names the SRS
//! requirement it verifies.

mod domain {
    mod auto_scroll;
    mod credits;
    mod deck;
    mod elapsed;
    mod environment;
    mod follow;
    mod host;
    mod installer;
    mod launch_plan;
    mod lifecycle;
    mod line_assembler;
    mod operation;
    mod output;
    mod preferences;
    mod run_times;
    mod scan;
    mod selection;
    mod step;
    mod support;
    mod text;
}
