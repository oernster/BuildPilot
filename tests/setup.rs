//! The setup program's policy (INST-001 to INST-006): routes, plans and words, without a
//! window or a machine.

use buildpilot::setup::plan::{self, Choices, Step};
use buildpilot::setup::route::{Installed, Route, route};
use buildpilot::setup::version::Version;
use buildpilot::setup::wording::{self, FLOW_ARROW};

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

const FOLDER: &str = r"C:\Users\op\AppData\Local\Programs\BuildPilot";

#[test]
fn versions_parse_three_numbers_and_order_as_releases() {
    assert_eq!(v(" 1.2.3 ").to_string(), "1.2.3");
    for bad in ["1.2", "1.2.3.4", "1.x.3", "", "1..3"] {
        assert_eq!(Version::parse(bad), None, "{bad}");
    }
    assert!(v("0.9.9") < v("0.10.0"));
    assert!(v("1.0.0") > v("0.99.99"));
}

// INST-003: the route follows the version comparison.
#[test]
fn the_route_follows_what_is_installed() {
    let carried = v("0.2.0");
    assert_eq!(route(Installed::Nothing, carried), Route::Install);
    assert_eq!(
        route(Installed::Recorded(Some(v("0.1.0"))), carried),
        Route::Update {
            from: Some(v("0.1.0"))
        }
    );
    assert_eq!(
        route(Installed::Recorded(Some(v("0.3.0"))), carried),
        Route::Downgrade { from: v("0.3.0") }
    );
    assert_eq!(
        route(Installed::Recorded(Some(carried)), carried),
        Route::Manage
    );
    assert_eq!(
        route(Installed::Recorded(None), carried),
        Route::Update { from: None }
    );
}

// INST-002, INST-005: what each kind of run does, in order.
#[test]
fn plans_name_their_steps_in_order() {
    let chosen = Choices {
        desktop_shortcut: true,
        launch_after: false,
    };
    assert_eq!(
        plan::install(chosen),
        [
            Step::CopyFiles,
            Step::StartMenuShortcut,
            Step::AddDesktopShortcut,
            Step::Register
        ]
    );
    let declined = Choices {
        desktop_shortcut: false,
        ..chosen
    };
    assert_eq!(plan::install(declined)[2], Step::RemoveDesktopShortcut);
    assert_eq!(
        plan::repair(),
        [Step::CopyFiles, Step::StartMenuShortcut, Step::Register]
    );
    assert_eq!(
        plan::uninstall(true),
        [Step::RemoveShortcuts, Step::Unregister, Step::RemoveFiles]
    );
    assert_eq!(plan::uninstall(false).last(), Some(&Step::RemoveData));
}

// The heading names one version only where the route is about one; two go in the flow line.
#[test]
fn each_route_says_what_it_does() {
    let carried = v("0.2.0");
    let install = wording::route_words(Route::Install, carried, FOLDER);
    assert_eq!(install.heading, "Install BuildPilot 0.2.0");
    assert!(install.lead.contains(FOLDER) && install.flow.is_empty());
    assert_eq!(
        (install.go.as_str(), install.second.as_str()),
        ("Install", "")
    );

    let update = wording::route_words(
        Route::Update {
            from: Some(v("0.1.0")),
        },
        carried,
        FOLDER,
    );
    assert_eq!(update.heading, "Update BuildPilot");
    assert_eq!(update.flow, format!("v0.1.0 {FLOW_ARROW} v0.2.0"));
    assert_eq!(update.go, "Update");
    let unknown = wording::route_words(Route::Update { from: None }, carried, FOLDER);
    assert_eq!(
        unknown.flow,
        format!("an earlier version {FLOW_ARROW} v0.2.0")
    );

    let back = wording::route_words(Route::Downgrade { from: v("0.3.0") }, carried, FOLDER);
    assert_eq!(back.flow, format!("v0.3.0 {FLOW_ARROW} v0.2.0"));
    assert_eq!(back.go, "Go back");

    let manage = wording::route_words(Route::Manage, carried, FOLDER);
    assert_eq!(manage.heading, "BuildPilot 0.2.0 is installed");
    assert_eq!(
        (manage.go.as_str(), manage.second.as_str()),
        ("Repair", "Reinstall")
    );
}

#[test]
fn every_step_and_verdict_has_words() {
    let steps = [
        Step::CopyFiles,
        Step::StartMenuShortcut,
        Step::AddDesktopShortcut,
        Step::RemoveDesktopShortcut,
        Step::Register,
        Step::RemoveShortcuts,
        Step::Unregister,
        Step::RemoveFiles,
        Step::RemoveData,
    ];
    for step in steps {
        assert!(wording::step_words(step).len() > 10, "{step:?}");
    }
    assert_eq!(wording::progress_heading(false), "Installing BuildPilot");
    assert_eq!(wording::progress_heading(true), "Uninstalling BuildPilot");
    let weights: u32 = steps.iter().map(|step| plan::step_weight(*step)).sum();
    assert_eq!(plan::weight(&steps), weights);
    assert_eq!(
        plan::weight(&[]),
        1,
        "an empty plan still has a whole to divide"
    );
    let log = r"C:\Temp\BuildPilotSetup.log";
    assert_eq!(wording::verdict(None, false, log).0, "BuildPilot is ready");
    assert_eq!(
        wording::verdict(None, true, log).0,
        "BuildPilot is uninstalled"
    );
    let (title, line) = wording::verdict(Some((Step::Register, "access denied")), false, log);
    assert_eq!(title, "Setup did not finish");
    assert!(
        line.contains("access denied") && line.contains(log),
        "{line}"
    );
}
