#![allow(dead_code)]
use cocoa::base::{id, nil};
use std::ops::Deref;

pub struct IdRef(id);

impl IdRef {
    pub fn new(i: id) -> IdRef {
        info!("IdRef wrapping: {:?}", i);
        IdRef(i)
    }

    pub fn retain(i: id) -> IdRef {
        if i != nil {
            info!("IdRef retaining: {:?}", i);
            let _: id = unsafe { msg_send![i, retain] };
        }
        IdRef(i)
    }

    pub fn non_nil(self) -> Option<IdRef> {
        if self.0 == nil { None } else { Some(self) }
    }

    pub fn get_id(self) -> id {
        self.0
    }
}

impl Drop for IdRef {
    fn drop(&mut self) {
        if self.0 != nil {
            info!("IdRef dropping: {:?}", self.0);
            unsafe { msg_send![self.0, release] };
            self.0 = nil;
        }
    }
}

impl Deref for IdRef {
    type Target = id;
    fn deref<'a>(&'a self) -> &'a id {
        &self.0
    }
}

impl Clone for IdRef {
    fn clone(&self) -> IdRef {
        if self.0 != nil {
            let _: id = unsafe { msg_send![self.0, retain] };
        }
        IdRef(self.0)
    }
}
