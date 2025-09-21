pub use starbase_id::{ALNUM as ID_CHARS, Id, IdError, SYMBOLS as ID_SYMBOLS};

pub trait IdExt {
    fn stable<S: AsRef<str>>(id: S) -> Id;
    fn stable_and_unstable<S: AsRef<str>>(id: S) -> (Id, Id);
    fn unstable<S: AsRef<str>>(id: S) -> Id;
}

impl IdExt for Id {
    fn stable<S: AsRef<str>>(id: S) -> Id {
        let id = id.as_ref();

        if let Some(suffix) = id.strip_prefix("unstable_") {
            Id::raw(suffix)
        } else {
            Id::raw(id)
        }
    }

    fn stable_and_unstable<S: AsRef<str>>(id: S) -> (Id, Id) {
        let id = id.as_ref();

        (Id::stable(id), Id::unstable(id))
    }

    fn unstable<S: AsRef<str>>(id: S) -> Id {
        let id = id.as_ref();

        if id.starts_with("unstable_") {
            Id::raw(id)
        } else {
            Id::raw(format!("unstable_{id}"))
        }
    }
}
