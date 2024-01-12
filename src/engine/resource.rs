use std::{any::TypeId, collections::HashMap};

use downcast::{downcast, Any};

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

pub trait Resource: Any + Send + Sync {}
downcast!(dyn Resource);

pub(crate) type BoxedResource = Box<dyn Resource>;

pub type Res<'rb, R> = MappedRwLockReadGuard<'rb, R>;
pub type ResMut<'rb, R> = MappedRwLockWriteGuard<'rb, R>;

pub struct ResourceBank {
    resources: HashMap<TypeId, RwLock<BoxedResource>>,
}

impl ResourceBank {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn get_resource<R: Resource>(&self) -> Res<R> {
        RwLockReadGuard::map(
            self.resources
                .get(&TypeId::of::<R>())
                .expect(&format!(
                    "Failed to get resource: {}",
                    std::any::type_name::<R>()
                ))
                .read(),
            |r| r.downcast_ref().unwrap(),
        )
    }

    pub fn get_resource_mut<R: Resource>(&self) -> ResMut<R>
    where
        R: Resource,
    {
        RwLockWriteGuard::map(
            self.resources
                .get(&TypeId::of::<R>())
                .expect(&format!(
                    "Failed to get resource: {}",
                    std::any::type_name::<R>()
                ))
                .write(),
            |r| r.downcast_mut().unwrap(),
        )
    }

    pub fn insert<R: Resource>(&mut self, resource: R) {
        self.resources
            .insert(TypeId::of::<R>(), RwLock::new(Box::new(resource)));
    }
}
