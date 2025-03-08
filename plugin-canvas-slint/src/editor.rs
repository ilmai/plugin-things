use std::cell::OnceCell;
use std::rc::{Rc, Weak};

use raw_window_handle::RawWindowHandle;
use plugin_canvas::{event::EventResponse, window::WindowAttributes, Event};
use slint::platform::WindowAdapter;

use crate::{platform::PluginCanvasPlatform, plugin_component_handle::PluginComponentHandle, window_adapter::{PluginCanvasWindowAdapter, WINDOW_ADAPTER_FROM_SLINT, WINDOW_TO_SLINT}};

pub struct SlintEditor;

impl SlintEditor {
    pub fn open<C, B>(
        parent: RawWindowHandle,
        window_attributes: WindowAttributes,
        component_builder: B
    ) -> Rc<EditorHandle>
    where
        C: PluginComponentHandle + 'static,
        B: Fn(Rc<plugin_canvas::Window>) -> C + 'static,
    {
        let editor_handle = Rc::new(EditorHandle::new());

        let window = plugin_canvas::Window::open(
            parent,
            window_attributes.clone(),
            {
                let editor_weak_ptr = Rc::downgrade(&editor_handle).into_raw();
                let editor_thread = std::thread::current().id();

                Box::new(move |event| {
                    if std::thread::current().id() != editor_thread {
                        log::warn!("Tried to call event callback from non-editor thread");
                        return EventResponse::Ignored;
                    }

                    let editor_weak = unsafe { Weak::from_raw(editor_weak_ptr) };                    
                    if let Some(editor_handle) = editor_weak.upgrade() {
                        editor_handle.on_event(&event);
                    }

                    // Leak the weak reference to avoid dropping it
                    let _ = editor_weak.into_raw();
                    EventResponse::Ignored
                })
            },
        ).unwrap();

        // It's ok if this fails as it just means it has already been set
        slint::platform::set_platform(Box::new(PluginCanvasPlatform)).ok();

        let window = Rc::new(window);
        WINDOW_TO_SLINT.set(Some(window.clone()));

        let component = component_builder(window);
        component.window().show().unwrap();

        let window_adapter = WINDOW_ADAPTER_FROM_SLINT.take().unwrap();
        window_adapter.set_component(Box::new(component));

        editor_handle.set_window_adapter(window_adapter);
        editor_handle
    }
}

pub struct EditorHandle {
    window_adapter: OnceCell<Rc<PluginCanvasWindowAdapter>>,
}

impl EditorHandle {
    pub fn on_frame(&self) {
        self.on_event(&Event::Draw);
    }

    pub fn set_window_size(&self, width: f64, height: f64) {
        let size = slint::LogicalSize {
            width: width as _,
            height: height as _,
        };

        if let Some(window_adapter) = self.window_adapter() {
            window_adapter.set_size(size.into());
        }
    }

    pub fn set_scale(&self, scale: f64) {
        if let Some(window_adapter) = self.window_adapter() {
            window_adapter.set_scale(scale);
        }
    }

    fn new() -> Self {
        Self {
            window_adapter: Default::default(),
        }
    }

    fn window_adapter(&self) -> Option<&PluginCanvasWindowAdapter> {
        self.window_adapter.get().map(|adapter| &**adapter)
    }

    fn set_window_adapter(&self, window_adapter: Rc<PluginCanvasWindowAdapter>) {
        self.window_adapter.set(window_adapter).unwrap();
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        if let Some(window_adapter) = self.window_adapter() {
            window_adapter.on_event(event)
        } else {
            EventResponse::Ignored
        }
    } 
}
