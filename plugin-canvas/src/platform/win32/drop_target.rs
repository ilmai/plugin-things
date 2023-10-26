use std::cell::RefCell;
use std::ffi::OsString;
use std::os::windows::prelude::OsStringExt;
use std::ptr::null_mut;
use std::rc::Rc;

use windows::Win32::Foundation::POINTL;
use windows::Win32::System::Com::{IDataObject, FORMATETC, DVASPECT_CONTENT, TYMED_HGLOBAL};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::core::implement;
use windows::Win32::System::Ole::{IDropTarget, IDropTarget_Impl, DROPEFFECT, CF_HDROP, DROPEFFECT_NONE, DROPEFFECT_COPY, DROPEFFECT_MOVE, DROPEFFECT_LINK};

use crate::event::EventResponse;
use crate::LogicalPosition;
use crate::drag_drop::{DropData, DropOperation};
use super::window::OsWindow;

#[implement(IDropTarget)]
pub(super) struct DropTarget {
    window: Rc<OsWindow>,
    drop_data: RefCell<DropData>,
}

impl DropTarget {
    pub fn new(window: Rc<OsWindow>) -> Self {
        Self {
            window,
            drop_data: Default::default(),
        }
    }

    fn parse_drag_data(&self, pdataobj: Option<&IDataObject>) -> windows::core::Result<()> {
        let Some(data_object) = pdataobj else {
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
            let hdrop = HDROP(medium.u.hGlobal.0 as isize);
       
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
        if let EventResponse::DragAccepted(operation) = response {
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
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for DropTarget {
    fn DragEnter(&self, pdataobj: Option<&IDataObject>, _grfkeystate: MODIFIERKEYS_FLAGS, pt: &POINTL, pdweffect: *mut DROPEFFECT) -> windows::core::Result<()> {
        self.parse_drag_data(pdataobj)?;

        let response = self.window.send_event(crate::Event::DragEntered {
            position: LogicalPosition { x: pt.x as f64, y: pt.y as f64 },
            data: self.drop_data.borrow().clone(),
        });
        
        unsafe { *pdweffect = self.convert_drag_operation(response) };
                
        Ok(())
    }

    fn DragOver(&self, _grfkeystate: MODIFIERKEYS_FLAGS, pt: &POINTL, pdweffect: *mut DROPEFFECT) -> windows::core::Result<()> {
        let response = self.window.send_event(crate::Event::DragMoved {
            position: LogicalPosition { x: pt.x as f64, y: pt.y as f64 },
            data: self.drop_data.borrow().clone(),
        });
        
        unsafe { *pdweffect = self.convert_drag_operation(response) };

        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        self.window.send_event(crate::Event::DragExited);
        Ok(())
    }

    fn Drop(&self, pdataobj: Option<&IDataObject>, _grfkeystate: MODIFIERKEYS_FLAGS, pt: &POINTL, pdweffect: *mut DROPEFFECT) -> windows::core::Result<()> {
        self.parse_drag_data(pdataobj)?;

        let response = self.window.send_event(crate::Event::DragDropped {
            position: LogicalPosition { x: pt.x as f64, y: pt.y as f64 },
            data: self.drop_data.borrow().clone(),
        });
        
        unsafe { *pdweffect = self.convert_drag_operation(response) };

        Ok(())
    }
}
