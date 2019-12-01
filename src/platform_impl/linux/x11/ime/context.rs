use std::{
    os::raw::{c_short, c_void},
    ptr,
    sync::Arc,
};

use super::{ffi, util, XConnection};

use winit_types::error::Error;

#[derive(Debug)]
pub enum ImeContextCreationError {
    Error(Error),
    Null,
}

unsafe fn create_pre_edit_attr<'a>(ic_spot: &'a ffi::XPoint) -> util::XSmartPointer<c_void> {
    let xlib = syms!(XLIB);
    util::XSmartPointer::new((xlib.XVaCreateNestedList)(
        0,
        ffi::XNSpotLocation_0.as_ptr() as *const _,
        ic_spot,
        ptr::null_mut::<()>(),
    ))
    .expect("XVaCreateNestedList returned NULL")
}

// WARNING: this struct doesn't destroy its XIC resource when dropped.
// This is intentional, as it doesn't have enough information to know whether or not the context
// still exists on the server. Since `ImeInner` has that awareness, destruction must be handled
// through `ImeInner`.
#[derive(Debug)]
pub struct ImeContext {
    pub ic: ffi::XIC,
    pub ic_spot: ffi::XPoint,
}

impl ImeContext {
    pub unsafe fn new(
        xconn: &Arc<XConnection>,
        im: ffi::XIM,
        window: ffi::Window,
        ic_spot: Option<ffi::XPoint>,
    ) -> Result<Self, ImeContextCreationError> {
        let ic = if let Some(ic_spot) = ic_spot {
            ImeContext::create_ic_with_spot(im, window, ic_spot)
        } else {
            ImeContext::create_ic(im, window)
        };

        let ic = ic.ok_or(ImeContextCreationError::Null)?;
        xconn
            .check_errors()
            .map_err(ImeContextCreationError::Error)?;

        Ok(ImeContext {
            ic,
            ic_spot: ic_spot.unwrap_or_else(|| ffi::XPoint { x: 0, y: 0 }),
        })
    }

    unsafe fn create_ic(im: ffi::XIM, window: ffi::Window) -> Option<ffi::XIC> {
        let xlib = syms!(XLIB);
        let ic = (xlib.XCreateIC)(
            im,
            ffi::XNInputStyle_0.as_ptr() as *const _,
            ffi::XIMPreeditNothing | ffi::XIMStatusNothing,
            ffi::XNClientWindow_0.as_ptr() as *const _,
            window,
            ptr::null_mut::<()>(),
        );
        if ic.is_null() {
            None
        } else {
            Some(ic)
        }
    }

    unsafe fn create_ic_with_spot(
        im: ffi::XIM,
        window: ffi::Window,
        ic_spot: ffi::XPoint,
    ) -> Option<ffi::XIC> {
        let xlib = syms!(XLIB);
        let pre_edit_attr = create_pre_edit_attr(&ic_spot);
        let ic = (xlib.XCreateIC)(
            im,
            ffi::XNInputStyle_0.as_ptr() as *const _,
            ffi::XIMPreeditNothing | ffi::XIMStatusNothing,
            ffi::XNClientWindow_0.as_ptr() as *const _,
            window,
            ffi::XNPreeditAttributes_0.as_ptr() as *const _,
            pre_edit_attr.ptr,
            ptr::null_mut::<()>(),
        );
        if ic.is_null() {
            None
        } else {
            Some(ic)
        }
    }

    pub fn focus(&self, xconn: &Arc<XConnection>) -> Result<(), Error> {
        let xlib = syms!(XLIB);
        unsafe {
            (xlib.XSetICFocus)(self.ic);
        }
        xconn.check_errors()
    }

    pub fn unfocus(&self, xconn: &Arc<XConnection>) -> Result<(), Error> {
        let xlib = syms!(XLIB);
        unsafe {
            (xlib.XUnsetICFocus)(self.ic);
        }
        xconn.check_errors()
    }

    pub fn set_spot(&mut self, x: c_short, y: c_short) {
        let xlib = syms!(XLIB);
        if self.ic_spot.x == x && self.ic_spot.y == y {
            return;
        }
        self.ic_spot = ffi::XPoint { x, y };

        unsafe {
            let pre_edit_attr = create_pre_edit_attr(&self.ic_spot);
            (xlib.XSetICValues)(
                self.ic,
                ffi::XNPreeditAttributes_0.as_ptr() as *const _,
                pre_edit_attr.ptr,
                ptr::null_mut::<()>(),
            );
        }
    }
}
