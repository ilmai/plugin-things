use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize};
use std::sync::Arc;

use cursor_icon::CursorIcon;
use i_slint_core::{window::{WindowAdapter, WindowAdapterInternal}, renderer::Renderer, platform::{PlatformError, WindowEvent}};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use keyboard_types::Code;
use plugin_canvas::keyboard::KeyboardModifiers;
use plugin_canvas::{event::EventResponse, LogicalSize};
use portable_atomic::AtomicF64;

use crate::view::PluginView;

thread_local! {
    pub static WINDOW_TO_SLINT: RefCell<Option<Arc<plugin_canvas::Window>>> = Default::default();
    pub static WINDOW_ADAPTER_FROM_SLINT: RefCell<Option<Rc<PluginCanvasWindowAdapter>>> = Default::default();
}

pub struct PluginCanvasWindowAdapter {
    plugin_canvas_window: Arc<plugin_canvas::Window>,
    slint_window: slint::Window,
    renderer: SkiaRenderer,

    view: RefCell<Option<Box<dyn PluginView>>>,

    physical_size: RefCell<slint::PhysicalSize>,
    scale: AtomicF64,

    pending_draw: AtomicBool,
    buttons_down: AtomicUsize,
    pending_mouse_exit: AtomicBool,

    modifiers: RefCell<KeyboardModifiers>,
}

impl PluginCanvasWindowAdapter {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let plugin_canvas_window = WINDOW_TO_SLINT.take().unwrap();
        
        let window_attributes = plugin_canvas_window.attributes();

        let scale = window_attributes.scale();
        let combined_scale = scale * plugin_canvas_window.os_scale();
        let plugin_canvas_size = window_attributes.size() * combined_scale;

        let slint_size = slint::PhysicalSize {
            width: plugin_canvas_size.width as u32,
            height: plugin_canvas_size.height as u32,
        };

        let skia_context = SkiaSharedContext::default();
        let renderer = SkiaRenderer::new(&skia_context, plugin_canvas_window.clone(), plugin_canvas_window.clone(), slint_size)?;

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
            }
        });

        self_rc.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: combined_scale as f32 }
        );

        WINDOW_ADAPTER_FROM_SLINT.set(Some(self_rc.clone()));

        Ok(self_rc as _)
    }

    pub fn set_view(&self, view: Box<dyn PluginView>) {
        *self.view.borrow_mut() = Some(view);
    }

    pub fn set_scale(&self, scale: f64) {
        self.scale.store(scale, Ordering::Release);

        let combined_scale = scale * self.plugin_canvas_window.os_scale();
        
        self.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: combined_scale as f32 }
        );
    }

    pub fn close(&self) {
        // Remove component to unravel the cyclic reference
        self.view.borrow_mut().take();
        self.slint_window.dispatch_event(WindowEvent::CloseRequested);
    }

    pub fn on_event(&self, event: &plugin_canvas::Event) -> EventResponse {
        let component_response = if let Some(component) = self.view.borrow().as_ref() {
            component.on_event(event)
        } else {
            EventResponse::Ignored
        };

        let built_in_response = match event {
            plugin_canvas::Event::Draw => {
                match self.plugin_canvas_window.poll_events() {
                    Ok(_) => {},
                    Err(e) => {
                        log::error!("Error polling events: {e:?}");
                    }
                }

                i_slint_core::platform::update_timers_and_animations();
                
                if self.pending_draw.swap(false, Ordering::Relaxed) {
                    self.renderer.render().unwrap();
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
                let mut my_modifiers = self.modifiers.borrow_mut();

                for modifier in [
                    KeyboardModifiers::Alt,
                    KeyboardModifiers::Control,
                    KeyboardModifiers::Meta,
                    KeyboardModifiers::Shift
                ] {
                    // TODO: This is mildly janky, could we clean it up?
                    macro_rules! modifier_to_char {
                        ($($char:literal # $name:ident # $($_qt:ident)|* # $($_winit:ident $(($_pos:ident))?)|* # $($_xkb:ident)|*;)*) => {
                            {
                                if false { unimplemented!() }

                                $(
                                    else if modifier == KeyboardModifiers::Alt && stringify!($name) == "Alt" {
                                        $char
                                    } else if modifier == KeyboardModifiers::Shift && stringify!($name) == "Shift" {
                                        $char
                                    } else if cfg!(target_os="macos") && modifier == KeyboardModifiers::Meta && stringify!($name) == "Control" {
                                        $char
                                    } else if cfg!(target_os="macos") && modifier == KeyboardModifiers::Control && stringify!($name) == "Meta" {
                                            $char
                                    } else if !cfg!(target_os="macos") && modifier == KeyboardModifiers::Control && stringify!($name) == "Control" {
                                            $char
                                    } else if !cfg!(target_os="macos") && modifier == KeyboardModifiers::Meta && stringify!($name) == "Meta" {
                                        $char
                                    }
                                )*

                                else {
                                    unimplemented!()
                                }
                            }
                        }
                    }

                    let was_pressed = my_modifiers.contains(modifier);
                    let pressed = modifiers.contains(modifier);

                    let text = i_slint_common::for_each_special_keys!(modifier_to_char);

                    if !was_pressed && pressed {
                        self.slint_window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
                    }
                    if was_pressed && !pressed {
                        self.slint_window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
                    }
                }

                *my_modifiers = *modifiers;

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

        if component_response != EventResponse::Ignored {
            component_response
        } else {
            built_in_response
        }
    }

    fn convert_button(button: &plugin_canvas::MouseButton) -> i_slint_core::platform::PointerEventButton {
        match button {
            plugin_canvas::MouseButton::Left => i_slint_core::platform::PointerEventButton::Left,
            plugin_canvas::MouseButton::Right => i_slint_core::platform::PointerEventButton::Right,
            plugin_canvas::MouseButton::Middle => i_slint_core::platform::PointerEventButton::Middle,
        }
    }

    fn convert_key(key_code: Code, text: &Option<String>) -> Option<String> {
        // Slint is using the deprecate keyCode standard, we'll have to convert some control keys
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
        let scale = self.scale.load(Ordering::Acquire);

        slint::LogicalPosition {
            x: (position.x / scale) as _,
            y: (position.y / scale) as _,
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
        let os_scale = self.plugin_canvas_window.os_scale();

        let physical_size = size.to_physical(os_scale as _);
        let logical_size = size.to_logical(os_scale as _);

        *self.physical_size.borrow_mut() = physical_size;
        self.plugin_canvas_window.resized(LogicalSize::new(logical_size.width as _, logical_size.height as _));

        let mut logical_size = size.to_logical(os_scale as _);
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
