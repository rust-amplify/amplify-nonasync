use std::error::Error;
use std::fmt::Debug;

pub trait CloneNoPersistence: Sized {
    fn clone_no_persistence(&self) -> Self;
}

#[derive(Debug, Display, Error)]
#[display(inner)]
pub struct PersistenceError(pub Box<dyn Error + Send>);

impl PersistenceError {
    pub fn with<E: Error + Send + 'static>(e: E) -> Self { Self(Box::new(e)) }
}

pub trait PersistenceProvider<T>: Send + Sync + Debug {
    fn load(&self) -> Result<T, PersistenceError>;
    fn store(&self, object: &T) -> Result<(), PersistenceError>;
}

#[derive(Debug)]
pub struct Persistence<T: Persisting> {
    pub dirty: bool,
    pub autosave: bool,
    pub provider: Box<dyn PersistenceProvider<T>>,
}

impl<T: Persisting> Persistence<T> {
    pub fn load(
        provider: impl PersistenceProvider<T> + 'static,
        autosave: bool,
    ) -> Result<T, PersistenceError> {
        let mut obj: T = provider.load()?;
        let mut me = Self {
            dirty: false,
            autosave,
            provider: Box::new(provider),
        };
        obj.persistence_mut().replace(&mut me);
        Ok(obj)
    }
}

pub trait Persisting: Sized {
    #[inline]
    fn load(
        provider: impl PersistenceProvider<Self> + 'static,
        autosave: bool,
    ) -> Result<Self, PersistenceError> {
        Persistence::load(provider, autosave)
    }

    fn persistence(&self) -> Option<&Persistence<Self>>;

    fn persistence_mut(&mut self) -> Option<&mut Persistence<Self>>;

    fn is_persisted(&self) -> bool { self.persistence().is_some() }

    fn is_dirty(&self) -> bool { self.persistence().map(|p| p.autosave).unwrap_or(true) }

    fn mark_dirty(&mut self) {
        if let Some(p) = self.persistence_mut() {
            p.dirty = true;
        }
        #[cfg(feature = "log")]
        if let Some(p) = self.persistence() {
            if p.autosave {
                if let Err(e) = p.provider.store(self) {
                    log::error!(
                        "Unable to autosave a dirty object on Persisting::mark_dirty call. \
                         Details: {e}"
                    );
                }
            }
        }
    }

    fn is_autosave(&self) -> bool { self.persistence().map(|p| p.autosave).unwrap_or_default() }

    fn set_autosave(&mut self) {
        #[cfg(feature = "log")]
        if let Err(e) = self.store() {
            log::error!(
                "Unable to autosave a dirty object on Persisting::set_autosave call. Details: {e}"
            );
        }
    }

    /// Returns whether the object was persisting before this method.
    fn make_persistent(
        &mut self,
        provider: impl PersistenceProvider<Self> + 'static,
        autosave: bool,
    ) -> Result<bool, PersistenceError> {
        let was_persisted = self.is_persisted();
        let mut me = Persistence {
            dirty: false,
            autosave,
            provider: Box::new(provider),
        };
        self.persistence_mut().replace(&mut me);
        self.mark_dirty();
        Ok(was_persisted)
    }

    fn store(&mut self) -> Result<(), PersistenceError> {
        if self.is_dirty() {
            if let Some(p) = self.persistence() {
                p.provider.store(self)?;
            }
            if let Some(p) = self.persistence_mut() {
                p.dirty = false;
            }
        }
        Ok(())
    }
}
