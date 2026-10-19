use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize};
use std::sync::Arc;

use crate::platform::CallbackQueue;

use cursor_icon::CursorIcon;
use i_slint_core::{window::{WindowAdapter, WindowAdapterInternal}, renderer::Renderer, platform::{PlatformError, WindowEvent}};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use keyboard_types::Code;
#[cfg(target_os="macos")]
use plugin_canvas::is_macos_version_at_least;
use plugin_canvas::keyboard::KeyboardModifiers;
use plugin_canvas::screen::screen_scale;
use plugin_canvas::{event::EventResponse, LogicalSize};
use portable_atomic::AtomicF64;

use crate::view::PluginView;

thread_local! {
    pub static WINDOW_TO_SLINT: RefCell<Option<Arc<plugin_canvas::Window>>> = Default::default();
    pub static WINDOW_ADAPTER_FROM_SLINT: RefCell<Option<Rc<PluginCanvasWindowAdapter>>> = Default::default();
}

pub struct PluginCanvasWindowAdapter {
    // renderer needs to be declared first so it's dropped first
    renderer: SkiaRenderer,
    plugin_canvas_window: Arc<plugin_canvas::Window>,
    slint_window: slint::Window,

    view: RefCell<Option<Box<dyn PluginView>>>,

    physical_size: RefCell<slint::PhysicalSize>,
    scale: AtomicF64,

    pending_draw: AtomicBool,
    buttons_down: AtomicUsize,
    pending_mouse_exit: AtomicBool,

    modifiers: RefCell<KeyboardModifiers>,
    callback_queue: CallbackQueue,

    input_focus: AtomicBool,
}

impl PluginCanvasWindowAdapter {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(callback_queue: CallbackQueue) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let plugin_canvas_window = WINDOW_TO_SLINT.take().unwrap();

        let window_attributes = plugin_canvas_window.attributes();

        let scale = window_attributes.scale();
        let mut combined_scale = scale;
        if cfg!(target_os = "macos") {
            combined_scale *= screen_scale();
        }

        let plugin_canvas_size = window_attributes.size() * combined_scale;

        let slint_size = slint::PhysicalSize {
            width: plugin_canvas_size.width as u32,
            height: plugin_canvas_size.height as u32,
        };

        let skia_context = SkiaSharedContext::default();

        #[cfg(target_os="linux")]
        let renderer = SkiaRenderer::default(&skia_context);

        #[cfg(target_os="macos")]
        let renderer = if is_macos_version_at_least(13, 0, 0) {
            SkiaRenderer::default_metal(&skia_context)
        } else {
            SkiaRenderer::default_opengl(&skia_context)
        };

        #[cfg(target_os="windows")]
        let renderer = SkiaRenderer::default_direct3d(&skia_context);

        renderer.set_window_handle(plugin_canvas_window.clone(), plugin_canvas_window.clone(), slint_size, None, false)?;

        let self_rc = Rc::new_cyclic(|self_weak| {
            let slint_window = slint::Window::new(self_weak.clone() as _);

            Self {
                plugin_canvas_window,
                slint_window,
                renderer,

                view: Default::default(),

                physical_size: slint_size.into(),
                scale: scale.into(),

                pending_draw: AtomicBool::new(true),
                buttons_down: Default::default(),
                pending_mouse_exit: Default::default(),

                modifiers: Default::default(),
                callback_queue,

                input_focus: false.into(),
            }
        });

        self_rc.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: combined_scale as f32 }
        );
        self_rc.set_size(slint_size.into());

        WINDOW_ADAPTER_FROM_SLINT.set(Some(self_rc.clone()));

        Ok(self_rc as _)
    }

    pub fn set_view(&self, view: Box<dyn PluginView>) {
        *self.view.borrow_mut() = Some(view);
    }

    pub fn scale(&self) -> f64 {
        self.scale.load(Ordering::Acquire)
    }

    pub fn set_scale(&self, scale: f64) {
        self.scale.store(scale, Ordering::Release);

        let mut combined_scale = scale;
        if cfg!(target_os = "macos") {
            combined_scale *= screen_scale();
        }

        self.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: combined_scale as f32 }
        );
    }

    pub fn has_input_focus(&self) -> bool {
        self.input_focus.load(Ordering::Acquire)
    }

    pub fn close(&self) {
        // Remove component to unravel the cyclic reference
        self.view.borrow_mut().take();
        self.slint_window.dispatch_event(WindowEvent::CloseRequested);
    }

    pub fn on_event(&self, event: &plugin_canvas::Event) -> EventResponse {
        let view_response = if let Some(view) = self.view.borrow().as_ref() {
            view.on_event(event)
        } else {
            EventResponse::Ignored
        };

        let built_in_response = match event {
            plugin_canvas::Event::Draw => {
                // Note: this may invoke callbacks from other windows as well, not just ours, because the callback queue is owned by PluginCanvasPlatform. Events are added via `slint::invoke_from_event_loop` which has no window context.
                let callbacks: Vec<_> = self.callback_queue.lock().unwrap().drain(..).collect();
                for callback in callbacks { callback(); }

                match self.plugin_canvas_window.poll_events() {
                    Ok(_) => {},
                    Err(e) => {
                        tracing::error!("Error polling events: {e:?}");
                    }
                }

                i_slint_core::platform::update_timers_and_animations();

                if self.pending_draw.swap(false, Ordering::Relaxed) {
                    // Ignore the draw outcome as we're constantly drawing anyway
                    let _ = self.renderer.render().unwrap();
                }

                EventResponse::Handled
            },

            plugin_canvas::Event::KeyDown { key_code, text } => {
                if let Some(text) = Self::convert_key(*key_code, text) {
                    self.slint_window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
                }

                EventResponse::Handled
            },

            plugin_canvas::Event::KeyUp { key_code, text } => {
                if let Some(text) = Self::convert_key(*key_code, text) {
                    self.slint_window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
                }

                EventResponse::Handled
            },

            plugin_canvas::Event::KeyboardModifiers { modifiers } => {
                for modifier in [
                    KeyboardModifiers::Alt,
                    KeyboardModifiers::Control,
                    KeyboardModifiers::Meta,
                    KeyboardModifiers::Shift
                ] {
                    let was_pressed = self.modifiers.borrow().contains(modifier);
                    let pressed = modifiers.contains(modifier);

                    let text = match modifier {
                        KeyboardModifiers::Alt => '\u{0012}',
                        KeyboardModifiers::Control if cfg!(target_os="macos") => '\u{0017}',
                        KeyboardModifiers::Control => '\u{0011}',
                        KeyboardModifiers::Meta if cfg!(target_os="macos") => '\u{0011}',
                        KeyboardModifiers::Meta => '\u{0017}',
                        KeyboardModifiers::Shift => '\u{0010}',
                        _ => unimplemented!()
                    };

                    if !was_pressed && pressed {
                        self.slint_window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
                    }
                    if was_pressed && !pressed {
                        self.slint_window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
                    }
                }

                *self.modifiers.borrow_mut() = *modifiers;

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
        };

        if view_response != EventResponse::Ignored {
            view_response
        } else {
            built_in_response
        }
    }

    pub(crate) fn plugin_canvas_window(&self) -> &plugin_canvas::Window {
        &self.plugin_canvas_window
    }

    fn convert_button(button: &plugin_canvas::MouseButton) -> i_slint_core::platform::PointerEventButton {
        match button {
            plugin_canvas::MouseButton::Left => i_slint_core::platform::PointerEventButton::Left,
            plugin_canvas::MouseButton::Right => i_slint_core::platform::PointerEventButton::Right,
            plugin_canvas::MouseButton::Middle => i_slint_core::platform::PointerEventButton::Middle,
        }
    }

    fn convert_key(key_code: Code, text: &Option<String>) -> Option<String> {
        // Slint is using the deprecated keyCode standard, we'll have to convert some control keys
        // to its text representation
        match key_code {
            Code::Backspace => Some("\u{0008}".into()),
            Code::Enter => Some("\u{000A}".into()),
            Code::Delete => Some("\u{007F}".into()),
            Code::ArrowUp => Some("\u{F700}".into()),
            Code::ArrowDown => Some("\u{F701}".into()),
            Code::ArrowLeft => Some("\u{F702}".into()),
            Code::ArrowRight => Some("\u{F703}".into()),
            _ => text.clone()
        }
    }

    fn convert_logical_position(&self, position: &plugin_canvas::LogicalPosition) -> slint::LogicalPosition {
        slint::LogicalPosition {
            x: position.x as _,
            y: position.y as _,
        }
    }
}

impl Debug for PluginCanvasWindowAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginCanvasWindowAdapter")
            .field("physical_size", &self.physical_size)
            .field("scale", &self.scale)
            .field("pending_draw", &self.pending_draw)
            .field("buttons_down", &self.buttons_down)
            .field("pending_mouse_exit", &self.pending_mouse_exit)
            .finish()
    }
}

impl WindowAdapter for PluginCanvasWindowAdapter {
    fn window(&self) -> &slint::Window {
        &self.slint_window
    }

    fn size(&self) -> slint::PhysicalSize {
        *self.physical_size.borrow()
    }

    fn set_size(&self, size: slint::WindowSize) {
        let scale = self.scale.load(Ordering::Acquire);
        let screen_scale = if cfg!(target_os = "macos") {
            screen_scale()
        } else {
            1.0
        };

        let physical_size = size.to_physical(screen_scale as _);
        let mut logical_size = size.to_logical(screen_scale as _);

        *self.physical_size.borrow_mut() = physical_size;
        self.plugin_canvas_window.resized(LogicalSize::new(logical_size.width as _, logical_size.height as _), scale);

        logical_size.width /= scale as f32;
        logical_size.height /= scale as f32;

        self.slint_window.dispatch_event(
            WindowEvent::Resized { size: logical_size },
        );
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
            i_slint_core::window::InputMethodRequest::Disable => false,
            _ => { return; }
        };

        self.input_focus.store(input_focus, Ordering::Release);
        self.plugin_canvas_window.set_input_focus(input_focus);
    }

    fn set_mouse_cursor(&self, cursor: i_slint_core::cursor::MouseCursorInner) {
        use i_slint_core::cursor::MouseCursorInner;
        use i_slint_core::items::BuiltInMouseCursor;

        let cursor = match cursor {
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Default) => Some(CursorIcon::Default),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::None) => None,
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Help) => Some(CursorIcon::Help),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Pointer) => Some(CursorIcon::Pointer),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Progress) => Some(CursorIcon::Progress),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Wait) => Some(CursorIcon::Wait),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Crosshair) => Some(CursorIcon::Crosshair),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Text) => Some(CursorIcon::Text),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Alias) => Some(CursorIcon::Alias),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Copy) => Some(CursorIcon::Copy),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Move) => Some(CursorIcon::Move),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NoDrop) => Some(CursorIcon::NoDrop),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NotAllowed) => Some(CursorIcon::NotAllowed),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Grab) => Some(CursorIcon::Grab),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::Grabbing) => Some(CursorIcon::Grabbing),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::ColResize) => Some(CursorIcon::ColResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::RowResize) => Some(CursorIcon::RowResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NResize) => Some(CursorIcon::NResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::EResize) => Some(CursorIcon::EResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::SResize) => Some(CursorIcon::SResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::WResize) => Some(CursorIcon::WResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NeResize) => Some(CursorIcon::NeResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NwResize) => Some(CursorIcon::NwResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::SeResize) => Some(CursorIcon::SeResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::SwResize) => Some(CursorIcon::SwResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::EwResize) => Some(CursorIcon::EwResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NsResize) => Some(CursorIcon::NsResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NeswResize) => Some(CursorIcon::NeswResize),
            MouseCursorInner::BuiltIn(BuiltInMouseCursor::NwseResize) => Some(CursorIcon::NwseResize),
            _ => None,
        };

        self.plugin_canvas_window.set_cursor(cursor);
    }
}
