use std::{cell::RefCell, rc::Rc, sync::atomic::{AtomicBool, Ordering, AtomicUsize}};

use cursor_icon::CursorIcon;
use i_slint_core::{window::{WindowAdapter, WindowAdapterInternal}, renderer::Renderer, platform::{PlatformError, WindowEvent}};
use i_slint_renderer_skia::SkiaRenderer;
use plugin_canvas::event::EventResponse;

use crate::plugin_component_handle::PluginComponentHandle;

thread_local! {
    pub static WINDOW_TO_SLINT: RefCell<Option<Rc<plugin_canvas::Window>>> = Default::default();
    pub static WINDOW_ADAPTER_FROM_SLINT: RefCell<Option<Rc<PluginCanvasWindowAdapter>>> = Default::default();
}

pub struct Context {
    pub component: Box<dyn PluginComponentHandle>,
}

pub struct PluginCanvasWindowAdapter {
    plugin_canvas_window: Rc<plugin_canvas::Window>,
    slint_window: slint::Window,
    renderer: SkiaRenderer,

    context: RefCell<Option<Context>>,

    slint_size: slint::PhysicalSize,

    pending_draw: AtomicBool,
    buttons_down: AtomicUsize,
    pending_mouse_exit: AtomicBool,
}

impl PluginCanvasWindowAdapter {
    pub fn new() -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let plugin_canvas_window = WINDOW_TO_SLINT.take().unwrap();
        
        let window_attributes = plugin_canvas_window.attributes();

        let scale = window_attributes.scale() * plugin_canvas_window.os_scale();
        let plugin_canvas_size = window_attributes.size() * scale;

        let slint_size = slint::PhysicalSize {
            width: plugin_canvas_size.width as u32,
            height: plugin_canvas_size.height as u32,
        };

        let renderer = SkiaRenderer::new(plugin_canvas_window.clone(), plugin_canvas_window.clone(), slint_size)?;

        let self_rc = Rc::new_cyclic(|self_weak| {
            let slint_window = slint::Window::new(self_weak.clone() as _);
            
            Self {
                plugin_canvas_window,
                slint_window,
                renderer,

                context: Default::default(),

                slint_size,

                pending_draw: AtomicBool::new(true),
                buttons_down: Default::default(),
                pending_mouse_exit: Default::default(),
            }
        });

        self_rc.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: scale as f32 }
        );

        WINDOW_ADAPTER_FROM_SLINT.set(Some(self_rc.clone()));

        Ok(self_rc as _)
    }

    pub fn with_context<T>(&self, f: impl Fn(&Context) -> T) -> T {
        let context = self.context.borrow();
        f(context.as_ref().unwrap())
    }

    pub fn set_context(&self, context: Context) {
        *self.context.borrow_mut() = Some(context);
    }

    pub fn set_scale(&self, scale: f64) {
        let scale = scale * self.plugin_canvas_window.os_scale();
        
        self.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: scale as f32 }
        );
    }

    pub fn on_event(&self, event: &plugin_canvas::Event) -> EventResponse {
        if let Some(context) = self.context.borrow().as_ref() {
            context.component.on_event(&event);
        }

        match event {
            plugin_canvas::Event::Close => {
                self.slint_window.dispatch_event(WindowEvent::CloseRequested);

                // Delete context when close is requested to unravel the cyclic reference
                self.context.borrow_mut().take();
                EventResponse::Handled
            },

            plugin_canvas::Event::Draw => {
                // TODO: Error handling
                self.plugin_canvas_window.poll_events().unwrap();

                i_slint_core::platform::update_timers_and_animations();
                
                if self.pending_draw.swap(false, Ordering::Relaxed) {
                    self.renderer.render().unwrap();
                }

                EventResponse::Handled
            },

            plugin_canvas::Event::KeyDown { text } => {
                let text = text.into();
                self.slint_window.dispatch_event(WindowEvent::KeyPressed { text });
                EventResponse::Handled
            },

            plugin_canvas::Event::KeyUp { text } => {
                let text = text.into();
                self.slint_window.dispatch_event(WindowEvent::KeyReleased { text });
                EventResponse::Handled
            },

            plugin_canvas::Event::MouseButtonDown { button, position } => {
                let button = Self::convert_button(button);
                let position = self.convert_logical_position(position);
                self.buttons_down.fetch_add(1, Ordering::Relaxed);

                self.slint_window.dispatch_event(WindowEvent::PointerPressed { position, button });
                EventResponse::Handled
            },

            plugin_canvas::Event::MouseButtonUp { button, position } => {
                let button = Self::convert_button(button);
                let position = self.convert_logical_position(position);
                
                self.slint_window.dispatch_event(WindowEvent::PointerReleased { position, button });

                let buttons_down = self.buttons_down.fetch_sub(1, Ordering::Relaxed);
                if buttons_down == 1 && self.pending_mouse_exit.swap(false, Ordering::Relaxed) {
                    self.slint_window.dispatch_event(WindowEvent::PointerExited);
                }

                EventResponse::Handled
            },

            plugin_canvas::Event::MouseExited => {
                if self.buttons_down.load(Ordering::Relaxed) > 0 {
                    // Don't report mouse exit while we're dragging with the mouse
                    self.pending_mouse_exit.store(true, Ordering::Relaxed);
                } else {
                    self.slint_window.dispatch_event(WindowEvent::PointerExited);
                }

                EventResponse::Handled
            },

            plugin_canvas::Event::MouseMoved { position } => {
                let position = self.convert_logical_position(position);
                self.slint_window.dispatch_event(WindowEvent::PointerMoved { position });
                EventResponse::Handled
            },
            
            plugin_canvas::Event::MouseWheel { position, delta_x, delta_y } => {
                let position = self.convert_logical_position(position);
                self.slint_window.dispatch_event(
                    WindowEvent::PointerScrolled {
                        position,
                        delta_x: *delta_x as f32,
                        delta_y: *delta_y as f32,
                    }
                );
                EventResponse::Handled
            },
            
            plugin_canvas::Event::DragEntered { .. } => {
                EventResponse::Ignored
            },

            plugin_canvas::Event::DragExited => {
                EventResponse::Ignored
            },

            plugin_canvas::Event::DragMoved { position, .. } => {
                let position = self.convert_logical_position(position);
                self.slint_window.dispatch_event(WindowEvent::PointerMoved { position });
                EventResponse::Handled
            },

            plugin_canvas::Event::DragDropped { .. } => {
                EventResponse::Ignored
            },
        }
    }

    fn convert_button(button: &plugin_canvas::MouseButton) -> i_slint_core::platform::PointerEventButton {
        match button {
            plugin_canvas::MouseButton::Left => i_slint_core::platform::PointerEventButton::Left,
            plugin_canvas::MouseButton::Right => i_slint_core::platform::PointerEventButton::Right,
            plugin_canvas::MouseButton::Middle => i_slint_core::platform::PointerEventButton::Middle,
        }
    }

    fn convert_logical_position(&self, position: &plugin_canvas::LogicalPosition) -> slint::LogicalPosition {
        slint::LogicalPosition {
            x: position.x as f32,
            y: position.y as f32,
        }
    }
}

impl WindowAdapter for PluginCanvasWindowAdapter {
    fn window(&self) -> &slint::Window {
        &self.slint_window
    }

    fn size(&self) -> slint::PhysicalSize {
        self.slint_size
    }

    fn request_redraw(&self) {
        self.pending_draw.store(true, Ordering::Relaxed);
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn internal(&self, _: i_slint_core::InternalToken) -> Option<&dyn WindowAdapterInternal> {
        Some(self)
    }
}

impl WindowAdapterInternal for PluginCanvasWindowAdapter {
    fn input_method_request(&self, request: i_slint_core::window::InputMethodRequest) {
        let input_focus = match request {
            i_slint_core::window::InputMethodRequest::Enable { .. } => true,
            i_slint_core::window::InputMethodRequest::Disable { .. } => false,
            _ => { return; }
        };

        self.plugin_canvas_window.set_input_focus(input_focus);
    }

    fn set_mouse_cursor(&self, cursor: i_slint_core::items::MouseCursor) {
        let cursor = match cursor {
            i_slint_core::items::MouseCursor::Default => Some(CursorIcon::Default),
            i_slint_core::items::MouseCursor::None => None,
            i_slint_core::items::MouseCursor::Help => Some(CursorIcon::Help),
            i_slint_core::items::MouseCursor::Pointer => Some(CursorIcon::Pointer),
            i_slint_core::items::MouseCursor::Progress => Some(CursorIcon::Progress),
            i_slint_core::items::MouseCursor::Wait => Some(CursorIcon::Wait),
            i_slint_core::items::MouseCursor::Crosshair => Some(CursorIcon::Crosshair),
            i_slint_core::items::MouseCursor::Text => Some(CursorIcon::Text),
            i_slint_core::items::MouseCursor::Alias => Some(CursorIcon::Alias),
            i_slint_core::items::MouseCursor::Copy => Some(CursorIcon::Copy),
            i_slint_core::items::MouseCursor::Move => Some(CursorIcon::Move),
            i_slint_core::items::MouseCursor::NoDrop => Some(CursorIcon::NoDrop),
            i_slint_core::items::MouseCursor::NotAllowed => Some(CursorIcon::NotAllowed),
            i_slint_core::items::MouseCursor::Grab => Some(CursorIcon::Grab),
            i_slint_core::items::MouseCursor::Grabbing => Some(CursorIcon::Grabbing),
            i_slint_core::items::MouseCursor::ColResize => Some(CursorIcon::ColResize),
            i_slint_core::items::MouseCursor::RowResize => Some(CursorIcon::RowResize),
            i_slint_core::items::MouseCursor::NResize => Some(CursorIcon::NResize),
            i_slint_core::items::MouseCursor::EResize => Some(CursorIcon::EResize),
            i_slint_core::items::MouseCursor::SResize => Some(CursorIcon::SResize),
            i_slint_core::items::MouseCursor::WResize => Some(CursorIcon::WResize),
            i_slint_core::items::MouseCursor::NeResize => Some(CursorIcon::NeResize),
            i_slint_core::items::MouseCursor::NwResize => Some(CursorIcon::NwResize),
            i_slint_core::items::MouseCursor::SeResize => Some(CursorIcon::SeResize),
            i_slint_core::items::MouseCursor::SwResize => Some(CursorIcon::SwResize),
            i_slint_core::items::MouseCursor::EwResize => Some(CursorIcon::EwResize),
            i_slint_core::items::MouseCursor::NsResize => Some(CursorIcon::NsResize),
            i_slint_core::items::MouseCursor::NeswResize => Some(CursorIcon::NeswResize),
            i_slint_core::items::MouseCursor::NwseResize => Some(CursorIcon::NwseResize),
        };

        self.plugin_canvas_window.set_cursor(cursor);
    }
}
