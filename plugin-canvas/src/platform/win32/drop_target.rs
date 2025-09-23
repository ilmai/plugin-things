use std::cell::RefCell;
use std::ffi::OsString;
use std::ops::Deref;
use std::os::windows::prelude::OsStringExt;
use std::ptr::null_mut;
use std::sync::Weak;

use windows::Win32::Foundation::{POINTL, POINT};
use windows::Win32::Graphics::Gdi::MapWindowPoints;
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::core::implement;
use windows::Win32::System::Ole::{IDropTarget, IDropTarget_Impl, DROPEFFECT, CF_HDROP, DROPEFFECT_NONE, DROPEFFECT_COPY, DROPEFFECT_MOVE, DROPEFFECT_LINK};
use windows::Win32::UI::WindowsAndMessaging::HWND_DESKTOP;

use crate::event::EventResponse;
use crate::platform::interface::OsWindowInterface;
use crate::thread_bound::ThreadBound;
use crate::{LogicalPosition, PhysicalPosition};
use crate::drag_drop::{DropData, DropOperation};
use super::window::OsWindow;

#[implement(IDropTarget)]
pub(super) struct DropTarget {
    window: Weak<ThreadBound<OsWindow>>,
    drop_data: RefCell<DropData>,
}

impl DropTarget {
    pub fn new(window: Weak<ThreadBound<OsWindow>>) -> Self {
        Self {
            window,
            drop_data: Default::default(),
        }
    }

    fn parse_drag_data(&self, pdataobj: windows_core::Ref<'_, IDataObject>) -> windows::core::Result<()> {
        let Some(data_object) = pdataobj.deref() else {
            *self.drop_data.borrow_mut() = DropData::None;
            return Ok(());
        };

        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };

        unsafe {
            let medium = data_object.GetData(&format)?;
            let hdrop = HDROP(medium.u.hGlobal.0);
       
            let item_count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
            if item_count == 0 {
                *self.drop_data.borrow_mut() = DropData::None;
                return Ok(());
            }

            let mut paths = Vec::with_capacity(item_count as usize);

            for i in 0..item_count {
                let characters = DragQueryFileW(hdrop, i, None);
                let buffer_size = characters as usize + 1;
                let mut buffer = vec![0; buffer_size];

                DragQueryFileW(hdrop, i, Some(&mut buffer));
                buffer.truncate(buffer_size);

                paths.push(OsString::from_wide(&buffer[..characters as usize]).into())
            }

            *self.drop_data.borrow_mut() = DropData::Files(paths);
        }

        Ok(())
    }

    fn convert_drag_operation(&self, response: EventResponse) -> DROPEFFECT {
        if let EventResponse::DropAccepted(operation) = response {
            match operation {
                DropOperation::None => DROPEFFECT_NONE,
                DropOperation::Copy => DROPEFFECT_COPY,
                DropOperation::Move => DROPEFFECT_MOVE,
                DropOperation::Link => DROPEFFECT_LINK,
            }
        } else {
            DROPEFFECT_NONE
        }
    }

    fn convert_coordinates(&self, point: &POINTL) -> LogicalPosition {
        let Some(window) = self.window.upgrade() else {
            return LogicalPosition::default();
        };

        let scale = window.os_scale();

        // It looks like MapWindowPoints isn't DPI aware (and neither is ScreenToClient),
        // so we need to pre-scale the point here?
        // TODO: Find out what's going on
        let mut points = [POINT { x: (point.x as f64 / scale) as i32, y: (point.y as f64 / scale) as i32 }];

        unsafe { MapWindowPoints(Some(HWND_DESKTOP), Some(window.hwnd()), &mut points); }

        PhysicalPosition {
            x: points[0].x,
            y: points[0].y,
        }.to_logical(1.0)
    }
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(&self, pdataobj: windows_core::Ref<'_, IDataObject>, _grfkeystate: MODIFIERKEYS_FLAGS, pt: &POINTL, pdweffect: *mut DROPEFFECT) -> windows_core::Result<()> {
        let Some(window) = self.window.upgrade() else {
            return Ok(());
        };

        self.parse_drag_data(pdataobj)?;

        let response = window.send_event(crate::Event::DragEntered {
            position: self.convert_coordinates(pt),
            data: self.drop_data.borrow().clone(),
        });
        
        unsafe { *pdweffect = self.convert_drag_operation(response) };
                
        Ok(())
    }

    fn DragOver(&self, _grfkeystate: MODIFIERKEYS_FLAGS, pt: &POINTL, pdweffect: *mut DROPEFFECT) -> windows::core::Result<()> {
        let Some(window) = self.window.upgrade() else {
            return Ok(());
        };

        let response = window.send_event(crate::Event::DragMoved {
            position: self.convert_coordinates(pt),
            data: self.drop_data.borrow().clone(),
        });
        
        unsafe { *pdweffect = self.convert_drag_operation(response) };

        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        let Some(window) = self.window.upgrade() else {
            return Ok(());
        };

        window.send_event(crate::Event::DragExited);

        Ok(())
    }

    fn Drop(&self, pdataobj: windows_core::Ref<'_, IDataObject>, _grfkeystate: MODIFIERKEYS_FLAGS, pt: &POINTL, pdweffect: *mut DROPEFFECT) -> windows_core::Result<()> {
        let Some(window) = self.window.upgrade() else {
            return Ok(());
        };

        self.parse_drag_data(pdataobj)?;

        let response = window.send_event(crate::Event::DragDropped {
            position: self.convert_coordinates(pt),
            data: self.drop_data.borrow().clone(),
        });
        
        unsafe { *pdweffect = self.convert_drag_operation(response) };

        Ok(())
    }
}
