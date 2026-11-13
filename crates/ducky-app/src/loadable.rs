#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Loadable<T> {
    #[default]
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Loadable<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Loadable::Ready(value) => Some(value),
            _ => None,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Loadable::Loading)
    }

    pub(crate) fn from_result(result: Result<T, String>) -> Loadable<T> {
        match result {
            Ok(value) => Loadable::Ready(value),
            Err(message) => Loadable::Failed(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_idle() {
        assert_eq!(Loadable::<u8>::default(), Loadable::Idle);
    }

    #[test]
    fn ready_returns_value_only_when_ready() {
        assert_eq!(Loadable::Ready(3).ready(), Some(&3));
        assert_eq!(Loadable::<u8>::Loading.ready(), None);
        assert_eq!(Loadable::<u8>::Failed("x".to_owned()).ready(), None);
    }

    #[test]
    fn is_loading_matches_loading_only() {
        assert!(Loadable::<u8>::Loading.is_loading());
        assert!(!Loadable::<u8>::Idle.is_loading());
        assert!(!Loadable::Ready(1).is_loading());
    }
}
