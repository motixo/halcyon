//! Shared wording for Restore after an interrupted change, used by Dock &
//! Panel, System appearance and Profiles: one short line for the footer and
//! full details for a disclosure. Both name what was kept because someone
//! changed it afterwards: COSMIC settings, widget settings and the saved
//! Undo entry.
#[cfg(test)]
use crate::desktop::Owner;
use crate::desktop::store::RecoveryReport;
use crate::settings::system_appearance::describe;

/// Names shown in the short line before the rest is counted.
const SHORT: usize = 3;

/// The page that can undo a latest change made by `owner`.
#[cfg(test)]
pub fn owner_page(owner: Owner) -> super::Page {
    match owner {
        Owner::Preferences => super::Page::Widgets,
        Owner::DockPanel => super::Page::DesktopDock,
        Owner::Appearance => super::Page::DesktopAppearance,
        Owner::Profile => super::Page::DesktopProfiles,
        Owner::Windows => super::Page::DesktopWindows,
        Owner::Workspaces => super::Page::DesktopWorkspaces,
        Owner::WindowRules => super::Page::DesktopWindowRules,
        Owner::Shortcuts => super::Page::DesktopShortcuts,
        Owner::Wallpaper => super::Page::DesktopWallpaper,
        Owner::DesktopIcons => super::Page::DesktopIcons,
    }
}

fn kept(report: &RecoveryReport) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if !report.kept_external.is_empty() {
        names.extend(
            describe(&report.kept_external)
                .split(", ")
                .map(String::from),
        );
    }
    names.extend(report.kept_fields.iter().cloned());
    if report.kept_undo {
        names.push("the saved Undo entry".into());
    }
    names
}

fn collapse(names: &[String]) -> String {
    if names.len() <= SHORT {
        return names.join(", ");
    }
    format!(
        "{} and {} more",
        names[..SHORT].join(", "),
        names.len() - SHORT
    )
}

/// One line: “Interrupted change undone; kept newer Dock size, Card palette
/// and 2 more”.
pub fn summary(report: &RecoveryReport) -> String {
    let kept = kept(report);
    if kept.is_empty() {
        "Interrupted change undone".into()
    } else {
        format!("Interrupted change undone; kept newer {}", collapse(&kept))
    }
}

/// Every group in full, for a details disclosure. Empty groups are left out.
pub fn details(report: &RecoveryReport) -> Vec<String> {
    let mut lines = Vec::new();
    if !report.restored.is_empty() {
        lines.push(format!("Restored: {}", describe(&report.restored)));
    }
    if !report.restored_fields.is_empty() {
        lines.push(format!(
            "Widget settings restored: {}",
            report.restored_fields.join(", ")
        ));
    }
    if !report.kept_external.is_empty() {
        lines.push(format!(
            "Changed since and kept: {}",
            describe(&report.kept_external)
        ));
    }
    if !report.kept_fields.is_empty() {
        lines.push(format!(
            "Widget settings changed since and kept: {}",
            report.kept_fields.join(", ")
        ));
    }
    if report.kept_undo {
        lines.push("The saved Undo entry was changed since and kept as it is.".into());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::ConfigKey;

    #[test]
    fn notices_name_kept_widget_settings_and_the_undo_entry() {
        let dock = |name| ConfigKey::new("com.system76.CosmicPanel.Dock", 1, name).unwrap();
        let report = RecoveryReport {
            restored: vec![dock("anchor")],
            unchanged: Vec::new(),
            kept_external: vec![dock("size")],
            restored_fields: vec!["Card opacity".into()],
            kept_fields: vec!["Card palette".into(), "Visible widgets".into()],
            kept_undo: true,
        };
        let line = summary(&report);
        assert!(line.contains("Dock size"), "{line}");
        assert!(line.contains("Card palette"), "{line}");
        // Long lists collapse; details keep every name.
        assert!(line.ends_with("and 1 more"), "{line}");
        let details = details(&report).join("\n");
        for name in [
            "Dock position",
            "Card opacity",
            "Dock size",
            "Visible widgets",
            "Undo entry",
        ] {
            assert!(details.contains(name), "{name}: {details}");
        }
        let quiet = RecoveryReport {
            restored: vec![dock("anchor")],
            unchanged: Vec::new(),
            kept_external: Vec::new(),
            restored_fields: Vec::new(),
            kept_fields: Vec::new(),
            kept_undo: false,
        };
        assert_eq!(summary(&quiet), "Interrupted change undone");
    }
}
