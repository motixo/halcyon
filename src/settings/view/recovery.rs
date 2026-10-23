use super::*;
impl Settings {
    pub(super) fn addons_section(&self) -> Element<'_, Message> {
        use crate::desktop::islands::Installation;
        match &self.desktop.islands.installation {
            Installation::Absent => pages::section(vec![split(
                label("Add-ons", 14., FG),
                button::text("Get Islands ↗")
                    .id("get-islands".into())
                    .on_press(Message::OpenIslandsGuide)
                    .into(),
            )]),
            Installation::Installed { .. } => pages::section(vec![split(
                label("Add-ons", 14., FG),
                button::text("Islands")
                    .on_press(Message::Page(Page::DesktopDock))
                    .into(),
            )]),
            Installation::Pending => cosmic::widget::space().height(0).into(),
        }
    }
    pub(super) fn desktop_recovery_section(&self) -> Element<'_, Message> {
        let recovery = &self.desktop_recovery;
        let mut rows = vec![split(
            detail("Restore desktop", recovery.summary()),
            row::with_children(vec![
                button::text("Export…")
                    .id("export-recovery".into())
                    .on_press_maybe(
                        ((recovery.info.is_some() || recovery.problem.is_some()) && !recovery.busy)
                            .then_some(Message::ExportRecovery),
                    )
                    .into(),
                button::standard("Restore desktop…")
                    .id("restore-desktop".into())
                    .on_press_maybe(
                        (recovery.info.is_some() && !recovery.busy)
                            .then_some(Message::DesktopRestorePreview),
                    )
                    .into(),
            ])
            .spacing(8)
            .into(),
        )];
        if let Some(problem) = &recovery.problem {
            rows.push(label(problem, 12., MUTED));
        }
        if let Some(plan) = &recovery.plan {
            rows.push(label(
                format!(
                    "Restore {} {} in {} {}?",
                    plan.changes(),
                    if plan.changes() == 1 {
                        "setting"
                    } else {
                        "settings"
                    },
                    plan.domains.len(),
                    if plan.domains.len() == 1 {
                        "scope"
                    } else {
                        "scopes"
                    }
                ),
                14.,
                FG,
            ));
            rows.push(label("A recovery point is saved first. Undo stays available after restart. Schedules stay on; the next future event can apply. Automatic colors resume with the next wallpaper.", 12., MUTED));
            let later = plan
                .snapshot
                .domains
                .iter()
                .filter(|d| d.captured_at != plan.snapshot.captured_at)
                .map(|d| {
                    format!(
                        "{:?} · {}",
                        d.domain,
                        d.captured_at.split('T').next().unwrap_or(&d.captured_at)
                    )
                })
                .collect::<Vec<_>>();
            if !later.is_empty() {
                rows.push(label(
                    format!("Captured later: {}", later.join(", ")),
                    12.,
                    MUTED,
                ));
            }
            if !plan.skipped.is_empty() {
                rows.push(label(
                    format!(
                        "{} unsupported or unavailable settings kept; included in export.",
                        plan.skipped.len()
                    ),
                    12.,
                    MUTED,
                ));
            }
            if !plan.missing_assets.is_empty() {
                rows.push(label(
                    format!("Missing wallpaper: {}", plan.missing_assets.join(", ")),
                    12.,
                    MUTED,
                ));
            }
            if plan.snapshot.domains.iter().any(|d| d.motion.is_some()) {
                rows.push(label("App animations use a separate GSettings preference. Its saved raw override is exportable; Restore and Undo leave it unchanged.", 12., MUTED));
            }
            rows.push(
                row::with_children(vec![
                    button::standard("Cancel")
                        .id("restore-desktop-cancel".into())
                        .on_press(Message::DesktopRestoreConfirm(false))
                        .into(),
                    button::suggested("Restore desktop")
                        .id("restore-desktop-confirm".into())
                        .on_press(Message::DesktopRestoreConfirm(true))
                        .into(),
                ])
                .spacing(8)
                .into(),
            );
        }
        pages::section(rows)
    }
}
