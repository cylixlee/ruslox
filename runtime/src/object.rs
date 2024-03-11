use std::{
    mem,
    ops::{Deref, DerefMut},
    ptr,
};

macro_rules! register_object {
    ($($objtype:ident), *) => {
        #[derive(Clone, Copy)]
        pub(crate) enum ObjectType {
            $($objtype, )*
        }

        paste::paste! {
            $(
                impl Downcast<[<$objtype Object>]> for ManagedReference {
                    fn downcast(&self) -> Option<&[<$objtype Object>]> {
                        match self.typ {
                            ObjectType::$objtype => Some(unsafe { &*(self.data as *mut [<$objtype Object>]) }),
                            #[allow(unreachable_patterns)]
                            _ => None,
                        }
                    }

                    fn downcast_mut(&mut self) -> Option<&mut [<$objtype Object>]> {
                        match self.typ {
                            ObjectType::$objtype => Some(unsafe { &mut *(self.data as *mut [<$objtype Object>]) }),
                            #[allow(unreachable_patterns)]
                            _ => None,
                        }
                    }
                }

                impl FromUnmanaged<[<$objtype Object>]> for ManagedReference {
                    fn from_unmanaged<G: GarbageCollect>(value: [<$objtype Object>], gc: &mut G) -> Self {
                        let reference = ManagedReference {
                            data: Box::into_raw(Box::new(value)) as *mut (),
                            meta: Box::into_raw(Box::new(ObjectMeta::new(ObjectType::$objtype))),
                        };
                        gc.register(reference.clone());
                        reference
                    }
                }
            )*

            impl ManagedReference {
                pub unsafe fn finalize(self) {
                    match self.typ {
                        $(ObjectType::$objtype => mem::drop(Box::from_raw(self.data as *mut [<$objtype Object>])) ,)*
                    }
                    mem::drop(Box::from_raw(self.meta));
                }
            }
        }
    };
}

register_object!(String);

pub(crate) struct ObjectMeta {
    pub typ: ObjectType,
}

impl ObjectMeta {
    fn new(typ: ObjectType) -> Self {
        Self { typ }
    }
}

pub(crate) type StringObject = String;

pub(crate) struct ManagedReference {
    data: *mut (),
    meta: *mut ObjectMeta,
}

impl ManagedReference {
    pub fn ptr(&self) -> usize {
        self.data as usize
    }
}

impl Deref for ManagedReference {
    type Target = ObjectMeta;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.meta }
    }
}

impl DerefMut for ManagedReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.meta }
    }
}

impl Clone for ManagedReference {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            meta: self.meta,
        }
    }
}

impl PartialEq for ManagedReference {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.data, other.data)
    }
}

impl Eq for ManagedReference {}

pub(crate) trait Downcast<T> {
    fn downcast(&self) -> Option<&T>;
    fn downcast_mut(&mut self) -> Option<&mut T>;
}

pub(crate) trait GarbageCollect {
    fn register(&mut self, reference: ManagedReference);
}

pub(crate) trait FromUnmanaged<T> {
    fn from_unmanaged<G: GarbageCollect>(value: T, gc: &mut G) -> Self;
}
