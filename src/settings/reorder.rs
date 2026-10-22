pub(super) struct Reorder<T> {
    pub source: Option<T>,
    pub target: Option<T>,
}

impl<T: Copy + PartialEq> Reorder<T> {
    pub fn new() -> Self {
        Self {
            source: None,
            target: None,
        }
    }
    pub fn start(&mut self, item: T) {
        self.source = Some(item);
        self.target = Some(item);
    }
    pub fn over(&mut self, item: T) {
        if self.source.is_some() {
            self.target = Some(item);
        }
    }
    pub fn leave(&mut self, item: T) {
        if self.target == Some(item) {
            self.target = None;
        }
    }
    pub fn cancel(&mut self) {
        self.source = None;
        self.target = None;
    }
    pub fn finish(&mut self, order: &mut Vec<T>) -> bool {
        let source = self.source.take();
        let target = self.target.take();
        let (Some(source), Some(target)) = (source, target) else {
            return false;
        };
        let (Some(from), Some(to)) = (
            order.iter().position(|item| *item == source),
            order.iter().position(|item| *item == target),
        ) else {
            return false;
        };
        if from == to {
            return false;
        }
        let item = order.remove(from);
        order.insert(to, item);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_an_item_to_a_later_target_preserves_the_other_items() {
        let mut drag = Reorder::new();
        let mut order = vec![1, 2, 3];
        drag.start(1);
        drag.over(3);
        drag.leave(1);
        assert_eq!(order, [1, 2, 3]);
        assert!(drag.finish(&mut order));
        assert_eq!(order, [2, 3, 1]);
        assert!(drag.source.is_none() && drag.target.is_none());
        drag.start(1);
        drag.over(2);
        assert!(drag.finish(&mut order));
        assert_eq!(order, [1, 2, 3]);
    }

    #[test]
    fn outside_drops_and_cancel_leave_order_untouched() {
        let mut drag = Reorder::new();
        let mut order = vec![1, 2, 3];
        drag.start(1);
        drag.over(3);
        assert_eq!(drag.target, Some(3));
        drag.leave(3);
        assert!(!drag.finish(&mut order));
        assert_eq!(order, [1, 2, 3]);
        drag.start(1);
        drag.over(3);
        drag.cancel();
        assert!(!drag.finish(&mut order));
        assert_eq!(order, [1, 2, 3]);
    }

    #[test]
    fn missing_or_unchanged_targets_do_not_write_a_new_order() {
        let mut drag = Reorder::new();
        let mut order = vec![1, 2, 3];
        drag.start(1);
        assert_eq!(drag.source, Some(1));
        assert!(!drag.finish(&mut order));
        drag.start(4);
        drag.over(2);
        assert!(!drag.finish(&mut order));
        assert_eq!(order, [1, 2, 3]);
    }
}
