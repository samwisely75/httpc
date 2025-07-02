pub type Result<T> = anyhow::Result<T>;

pub type Merger<T> = fn(T, T) -> T;

#[allow(dead_code)]
pub fn merge_opt<T>(o1: Option<T>, o2: Option<T>, merger: Merger<T>) -> Option<T> {
    match (o1, o2) {
        (Some(p), Some(s)) => Some(merger(p, s)),
        (Some(p), None) => Some(p),
        (None, Some(s)) => Some(s),
        (None, None) => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn merge_opt_should_return_first_when_second_is_none() {
        assert_eq!(merge_opt(Some(1), None, |a, b| a + b), Some(1));
    }

    #[test]
    fn merge_opt_should_return_second_when_first_is_none() {
        assert_eq!(merge_opt(None, Some(2), |a, b| a + b), Some(2));
    }

    #[test]
    fn merge_opt_should_return_merged_value_when_both_are_not_none() {
        assert_eq!(merge_opt(Some(1), Some(2), |a, b| a + b), Some(3));
    }
}
