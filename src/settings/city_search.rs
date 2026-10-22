use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub(super) struct CitySearch {
    pub query: String,
    pub generation: u64,
    pub deadline: Option<Instant>,
    pub searching: bool,
    pub searched: bool,
    pub error: Option<String>,
}

impl CitySearch {
    pub fn edit(&mut self, query: String, now: Instant) {
        self.query = query;
        self.generation = self.generation.wrapping_add(1);
        self.searching = false;
        self.searched = false;
        self.error = None;
        self.deadline =
            (self.query.trim().chars().count() >= 2).then_some(now + Duration::from_millis(350));
    }

    pub fn begin(&mut self, generation: u64) -> Option<String> {
        if generation != self.generation || self.searching || self.query.trim().chars().count() < 2
        {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.deadline = None;
        self.searching = true;
        self.searched = false;
        self.error = None;
        Some(self.query.trim().to_string())
    }

    pub fn finish(&mut self, generation: u64, error: Option<String>) -> bool {
        if generation != self.generation || !self.searching {
            return false;
        }
        self.searching = false;
        self.searched = error.is_none();
        self.error = error;
        true
    }

    pub fn select(&mut self, name: String) {
        self.edit(name, Instant::now());
        self.deadline = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_waits_and_only_the_latest_query_can_start() {
        let now = Instant::now();
        let mut search = CitySearch::default();
        search.edit("Za".into(), now);
        let first = search.generation;
        assert!(search.deadline.is_some_and(|deadline| deadline > now));
        search.edit("  Zagreb  ".into(), now + Duration::from_millis(100));
        assert!(search.begin(first).is_none());
        assert_eq!(search.begin(search.generation).as_deref(), Some("Zagreb"));
        assert!(search.searching);
        assert!(search.deadline.is_none());
        assert!(search.begin(search.generation).is_none());
    }

    #[test]
    fn short_queries_cancel_pending_search_and_ignore_old_results() {
        let mut search = CitySearch::default();
        search.edit("Paris".into(), Instant::now());
        let old = search.generation;
        assert_eq!(search.begin(old).as_deref(), Some("Paris"));
        let old = search.generation;
        search.edit("P".into(), Instant::now());
        assert!(search.deadline.is_none());
        assert!(!search.searching);
        assert!(!search.finish(old, None));
        assert!(!search.searched);
        assert!(search.begin(search.generation).is_none());
    }

    #[test]
    fn failed_search_can_be_retried_without_accepting_the_old_response() {
        let mut search = CitySearch::default();
        search.edit("Paris".into(), Instant::now());
        assert_eq!(search.begin(search.generation).as_deref(), Some("Paris"));
        let old = search.generation;
        assert!(search.finish(old, Some("Offline".into())));
        assert_eq!(search.error.as_deref(), Some("Offline"));
        assert!(!search.searched);
        assert_eq!(search.begin(search.generation).as_deref(), Some("Paris"));
        assert_ne!(search.generation, old);
        assert!(!search.finish(old, None));
        assert!(search.finish(search.generation, None));
        assert!(search.searched);
        assert!(search.error.is_none());
    }

    #[test]
    fn selecting_a_city_invalidates_pending_results() {
        let mut search = CitySearch::default();
        search.edit("Spring".into(), Instant::now());
        assert_eq!(search.begin(search.generation).as_deref(), Some("Spring"));
        let request = search.generation;
        search.select("Springfield".into());
        assert_eq!(search.query, "Springfield");
        assert!(!search.searching && !search.searched);
        assert!(search.deadline.is_none());
        assert!(!search.finish(request, None));
    }
}
