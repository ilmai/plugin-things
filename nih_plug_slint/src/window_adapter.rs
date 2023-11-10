use std::{cell::RefCell, rc::Rc, sync::{atomic::{AtomicBool, Ordering, AtomicUsize}, Arc, mpsc}};

use i_slint_core::{window::{WindowAdapter, WindowAdapterInternal}, renderer::Renderer, platform::{PlatformError, WindowEvent}};
use i_slint_renderer_skia::SkiaRenderer;
use nih_plug::prelude::GuiContext;
use plugin_canvas::event::EventResponse;
use raw_window_handle::{HasWindowHandle, HasDisplayHandle};

use crate::plugin_component_handle::PluginComponentHandle;

thread_local! {
    pub static WINDOW_TO_SLINT: RefCell<Option<Box<plugin_canvas::Window>>> = Default::default();
    pub static WINDOW_ADAPTER_FROM_SLINT: RefCell<Option<Rc<PluginCanvasWindowAdapter>>> = Default::default();
}

pub enum ParameterChange {
    ValueChanged { id: String },
    ModulationChanged { id: String },
    AllValuesChanged,
}

pub type ParameterChangeSender = mpsc::Sender<ParameterChange>;
pub type ParameterChangeReceiver = mpsc::Receiver<ParameterChange>;

pub struct Context {
    pub gui_context: Arc<dyn GuiContext>,
    pub parameter_change_receiver: ParameterChangeReceiver,
    pub component: Box<dyn PluginComponentHandle>,
}

impl Context {
    pub fn component<T: PluginComponentHandle + 'static>(&self) -> Option<&T> {
        self.component.as_any().downcast_ref()
    }
}

pub struct PluginCanvasWindowAdapter {
    plugin_canvas_window: plugin_canvas::Window,
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
        let plugin_canvas_window = *WINDOW_TO_SLINT.take().unwrap();
        let window_handle = plugin_canvas_window.window_handle().unwrap();
        let display_handle = plugin_canvas_window.display_handle().unwrap();
        
        let window_attributes = plugin_canvas_window.attributes();

        // TODO: Why is this needed on Linux?
        #[cfg(target_os = "linux")]
        let scale = window_attributes.user_scale() * plugin_canvas_window.os_scale();
        #[cfg(not(target_os = "linux"))]
        let scale = window_attributes.user_scale();

        let plugin_canvas_size = window_attributes.size() * scale;

        let slint_size = slint::PhysicalSize {
            width: plugin_canvas_size.width as u32,
            height: plugin_canvas_size.height as u32,
        };

        let renderer = SkiaRenderer::new(window_handle, display_handle, slint_size)?;

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
        // Initialize parameter values
        context.component.update_all_parameters();

        *self.context.borrow_mut() = Some(context);
    }

    pub fn on_event(&self, event: &plugin_canvas::Event) -> EventResponse {
        match event {
            plugin_canvas::Event::Draw => {
                let context = self.context.borrow();
                let context = context.as_ref().unwrap();

                // Update property values for all changed plugin parameters
                while let Ok(parameter_change) = context.parameter_change_receiver.try_recv() {
                    match parameter_change {
                        ParameterChange::ValueChanged { id } => {
                            context.component.update_parameter(&id, true, false);
                        }

                        ParameterChange::ModulationChanged { id } => {
                            context.component.update_parameter(&id, false, true);
                        }

                        ParameterChange::AllValuesChanged => {
                            context.component.update_all_parameters();
                        },
                    }
                }

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
                
                let buttons_down = self.buttons_down.fetch_sub(1, Ordering::Relaxed);
                if buttons_down == 1 && self.pending_mouse_exit.swap(false, Ordering::Relaxed) {
                    self.slint_window.dispatch_event(WindowEvent::PointerExited);
                }

                self.slint_window.dispatch_event(WindowEvent::PointerReleased { position, button });
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

            plugin_canvas::Event::DragMoved { .. } => {
                EventResponse::Ignored
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
            i_slint_core::items::MouseCursor::Pointer => plugin_canvas::cursor::Cursor::Pointer,
            _ => plugin_canvas::cursor::Cursor::Arrow,
        };

        self.plugin_canvas_window.set_cursor(cursor);
    }
}
