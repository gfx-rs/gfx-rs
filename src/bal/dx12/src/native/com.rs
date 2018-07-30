use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::ptr;
use winapi::um::unknwnbase::IUnknown;
use winapi::Interface;
use D3DResult;

pub struct WeakPtr<T>(*mut T);

impl<T> WeakPtr<T> {
    pub fn null() -> Self {
        WeakPtr(ptr::null_mut())
    }

    pub fn from_raw(ptr: *mut T) -> WeakPtr<T> {
        WeakPtr(ptr)
    }

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn as_ptr(&self) -> *const T {
        self.0
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        self.0
    }
}

impl<T: Interface> WeakPtr<T> {
    pub fn as_unknown(&self) -> &IUnknown {
        debug_assert!(!self.is_null());
        unsafe { &*(self.0 as *mut IUnknown) }
    }

    // Cast creates a new WeakPtr requiring explicit destroy call.
    pub fn cast<U>(&self) -> D3DResult<WeakPtr<U>>
    where
        U: Interface,
    {
        let obj = WeakPtr::<U>::null();
        let hr = unsafe {
            self.as_unknown().QueryInterface(
                &U::uuidof(),
                &mut obj.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };
        (obj, hr)
    }

    // Destroying one instance of the WeakPtr will invalidate all
    // copies and clones.
    pub fn destroy(&self) {
        unsafe {
            self.as_unknown().Release();
        }
    }
}

impl<T> Clone for WeakPtr<T> {
    fn clone(&self) -> Self {
        WeakPtr(self.0)
    }
}

impl<T> Copy for WeakPtr<T> {}

impl<T> Deref for WeakPtr<T> {
    type Target = T;
    fn deref(&self) -> &T {
        debug_assert!(!self.is_null());
        unsafe { &*self.0 }
    }
}

impl<T> fmt::Debug for WeakPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WeakPtr( ptr: {:?} )", self.0)
    }
}

impl<T> PartialEq<*mut T> for WeakPtr<T> {
    fn eq(&self, other: &*mut T) -> bool {
        self.0 == *other
    }
}

impl<T> PartialEq for WeakPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Hash for WeakPtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}