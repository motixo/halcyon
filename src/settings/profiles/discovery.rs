use super::*;
use profiles::packages::{Attribution, Revision};
pub struct SourceLink {
    pub attribution: Attribution,
    pub exact: Option<Selection>,
    pub latest: Option<Selection>,
}
impl ProfilesEditor {
    pub fn discovery_sources(&self) -> Vec<SourceLink> {
        let Some(origin) = self.profile().and_then(|p| p.package.as_ref()) else {
            return Vec::new();
        };
        let mut sources = origin.ancestors.clone();
        // A remix already lists its actual sources. Do not present its own
        // derived identity as an unavailable upstream source.
        if sources.is_empty() && matches!(self.selection, Selection::Saved(_)) {
            sources.push(Attribution {
                identity: origin.identity.clone(),
                credits: origin.credits.clone(),
            });
        }
        sources
            .into_iter()
            .map(|attribution| {
                let identity = &attribution.identity;
                let curated = self.presets.iter().find(|p| {
                    p.profile
                        .package
                        .as_ref()
                        .is_some_and(|o| o.identity.id == identity.id)
                });
                let exact = curated
                    .filter(|p| p.profile.package.as_ref().unwrap().identity == *identity)
                    .map(|p| Selection::Preset(p.id))
                    .or_else(|| {
                        self.library()
                            .ok()?
                            .iter()
                            .find(|entry| {
                                self.selection != Selection::Saved(entry.id.clone())
                                    && entry
                                        .profile
                                        .as_ref()
                                        .ok()
                                        .and_then(|p| p.package.as_ref())
                                        .is_some_and(|o| o.identity == *identity)
                            })
                            .map(|e| Selection::Saved(e.id.clone()))
                    });
                let latest = curated
                    .filter(|p| p.profile.package.as_ref().unwrap().identity != *identity)
                    .map(|p| Selection::Preset(p.id));
                SourceLink {
                    attribution,
                    exact,
                    latest,
                }
            })
            .collect()
    }
    pub fn revision_label(revision: &Revision) -> String {
        // Validated hex identities are safe to shorten for display. Full values
        // remain in exports; these labels are not a signature or trust badge.
        format!("{} · {}", &revision.id[..10], &revision.revision[..10])
    }
}
#[cfg(test)]
mod tests;
