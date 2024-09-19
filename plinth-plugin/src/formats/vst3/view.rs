use std::{cell::RefCell, ffi::{c_void, CStr}, rc::Rc};

use vst3::{ComPtr, ComRef, Steinberg::{char16, int16, kInvalidArgument, kResultFalse, kResultOk, tresult, FIDString, IPlugFrame, IPlugView, IPlugViewContentScaleSupport, IPlugViewContentScaleSupportTrait, IPlugViewContentScaleSupport_::ScaleFactor, IPlugViewTrait, TBool, ViewRect}};

use crate::editor::Editor;

use super::{component::UiThreadState, host::Vst3Host, Vst3Plugin};

pub struct ViewContext {
    frame: Option<ComPtr<IPlugFrame>>,
    scale_factor: ScaleFactor,

    #[cfg(target_os="linux")]
    timer_handler: Option<ComPtr<vst3::Steinberg::Linux::ITimerHandler>>,
}

pub struct View<P: Vst3Plugin> {
    plugin: Rc<RefCell<P>>,
    ui_thread_state: Rc<UiThreadState<P>>,
    context: RefCell<ViewContext>,
}

impl<P: Vst3Plugin> View<P> {
    pub fn new(
        plugin: Rc<RefCell<P>>,
        ui_thread_state: Rc<UiThreadState<P>>,
    ) -> Self {
        let context = ViewContext {
            frame: None,
            scale_factor: 1.0,

            #[cfg(target_os="linux")]            
            timer_handler: None,
        };

        Self {
            plugin,
            ui_thread_state,
            context: context.into(),
        }
    }
}

impl<P: Vst3Plugin> vst3::Class for View<P> {
    type Interfaces = (IPlugView, IPlugViewContentScaleSupport);
}

impl<P: Vst3Plugin + 'static> IPlugViewTrait for View<P> {
    unsafe fn isPlatformTypeSupported(&self, platform_type: FIDString) -> tresult {
        let platform_type = CStr::from_ptr(platform_type);

        #[cfg(target_os="windows")]
        let supported = platform_type == CStr::from_ptr(vst3::Steinberg::kPlatformTypeHWND);
        
        #[cfg(target_os="macos")]
        let supported = platform_type == CStr::from_ptr(vst3::Steinberg::kPlatformTypeNSView);
        
        #[cfg(target_os="linux")]
        let supported = platform_type == CStr::from_ptr(vst3::Steinberg::kPlatformTypeX11EmbedWindowID);

        if supported {
            kResultOk
        } else {
            kResultFalse
        }
    }

    unsafe fn attached(&self, parent: *mut c_void, platform_type: FIDString) -> tresult {
        if parent.is_null() {
            return kInvalidArgument;
        }
        if self.isPlatformTypeSupported(platform_type) != kResultOk {
            return kInvalidArgument;
        }
        
        let context = self.context.borrow();
        let mut editor = self.ui_thread_state.editor.borrow_mut();
        assert!(editor.is_none());

        let parent = crate::window_handle::from_ptr(parent);
        let host = Rc::new(Vst3Host::new(self.ui_thread_state.handler.borrow().clone().unwrap()));
        *editor = Some(self.plugin.borrow().open_editor(parent, host, context.scale_factor as f64));

        kResultOk
    }

    unsafe fn removed(&self) -> tresult {
        #[cfg(target_os="linux")]
        {
            use vst3::Steinberg::Linux::IRunLoopTrait;

            let mut context = self.context.borrow_mut();
            let frame = context.frame.as_mut().unwrap();

            if let Some(run_loop) = frame.cast::<vst3::Steinberg::Linux::IRunLoop>() {
                if let Some(timer_handler) = context.timer_handler.take() {
                    run_loop.unregisterTimer(timer_handler.as_ptr());
                }
            }
        }

        let mut editor = self.ui_thread_state.editor.borrow_mut();
        assert!(editor.is_some());
        *editor = None;

        kResultOk
    }

    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultOk
    }

    unsafe fn onKeyDown(&self, _key: char16, _key_code: int16, _modifiers: int16) -> tresult {
        kResultOk
    }

    unsafe fn onKeyUp(&self, _key: char16, _key_code: int16, _modifiers: int16) -> tresult {
        kResultOk
    }

    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        if size.is_null() {
            return kInvalidArgument;
        }

        let context = self.context.borrow();
        let editor_size = P::Editor::SIZE;
        let scale_factor = context.scale_factor as f64;

        let size = unsafe { &mut *size };
        size.left = 0;
        size.top = 0;
        size.right = (editor_size.0 * scale_factor) as i32;
        size.bottom = (editor_size.1 * scale_factor) as i32;

        kResultOk
    }

    unsafe fn onSize(&self, _new_size: *mut ViewRect) -> tresult {
        kResultOk
    }

    unsafe fn onFocus(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setFrame(&self, frame: *mut IPlugFrame) -> tresult {
        if frame.is_null() {
            return kInvalidArgument;
        }

        let mut context = self.context.borrow_mut();
        context.frame = ComRef::from_raw(frame)
            .map(|frame| frame.to_com_ptr());

        #[cfg(target_os="linux")]
        {
            use vst3::Steinberg::Linux::IRunLoopTrait;

            let frame = context.frame.as_mut().unwrap();
            if let Some(run_loop) = frame.cast::<vst3::Steinberg::Linux::IRunLoop>() {
                let timer_handler = vst3::ComWrapper::new(TimerHandler {
                    ui_thread_state: self.ui_thread_state.clone(),
                });

                context.timer_handler = timer_handler.to_com_ptr();
                run_loop.registerTimer(context.timer_handler.as_mut().unwrap().as_ptr(), crate::editor::FRAME_TIMER_MILLISECONDS);
            }
        }

        kResultOk
    }

    unsafe fn canResize(&self) -> tresult {
        kResultOk
    }

    unsafe fn checkSizeConstraint(&self, _rect: *mut ViewRect) -> tresult {
        kResultOk
    }
}

impl<P: Vst3Plugin + 'static> IPlugViewContentScaleSupportTrait for View<P> {
    unsafe fn setContentScaleFactor(&self, factor: ScaleFactor) -> tresult {
        // MacOS does its own scaling
        #[cfg(not(target_os="macos"))]
        {
            self.context.borrow_mut().scale_factor = factor;
        }

        kResultOk
    }
}

#[cfg(target_os="linux")]
struct TimerHandler<P: Vst3Plugin> {
    ui_thread_state: Rc<UiThreadState<P>>,
}

#[cfg(target_os="linux")]
impl<P: Vst3Plugin> vst3::Class for TimerHandler<P> {
    type Interfaces = (vst3::Steinberg::Linux::ITimerHandler,);
}

#[cfg(target_os="linux")]
impl<P: Vst3Plugin> vst3::Steinberg::Linux::ITimerHandlerTrait for TimerHandler<P> {
    unsafe fn onTimer(&self) {
        if let Some(editor) = self.ui_thread_state.editor.borrow_mut().as_mut() {
            editor.on_frame();
        }
    }
}
