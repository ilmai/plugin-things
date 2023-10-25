use std::{cell::RefCell, rc::Rc, sync::{atomic::{AtomicBool, Ordering, AtomicUsize}, Arc, mpsc}, collections::{HashMap, HashSet}};

use i_slint_core::{window::{WindowAdapter, WindowAdapterInternal}, renderer::Renderer, platform::{PlatformError, WindowEvent}};
use i_slint_renderer_skia::SkiaRenderer;
use nih_plug::prelude::{ParamPtr, GuiContext};
use plugin_canvas::{dimensions::Scale, event::EventResponse};
use raw_window_handle::{HasWindowHandle, HasDisplayHandle};
use slint_interpreter::{ComponentInstance, ComponentDefinition, Value};

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
    pub component: ComponentInstance,
    pub component_definition: ComponentDefinition,
    pub param_map: Rc<HashMap<String, ParamPtr>>,
    pub gui_context: Arc<dyn GuiContext>,
    pub parameter_change_receiver: ParameterChangeReceiver,
}

pub struct PluginCanvasWindowAdapter {
    plugin_canvas_window: plugin_canvas::Window,
    slint_window: slint::Window,
    renderer: SkiaRenderer,

    context: RefCell<Option<Context>>,
    ui_parameters: RefCell<HashSet<String>>,

    slint_size: slint::PhysicalSize,
    user_scale: Scale,

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
        let plugin_canvas_size = plugin_canvas::PhysicalSize::from_logical(&window_attributes.size, window_attributes.scale);
        let user_scale = window_attributes.scale;

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
                ui_parameters: Default::default(),

                slint_size,
                user_scale,

                pending_draw: AtomicBool::new(true),
                buttons_down: Default::default(),
                pending_mouse_exit: Default::default(),
            }
        });

        self_rc.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: *user_scale as f32 }
        );

        WINDOW_ADAPTER_FROM_SLINT.set(Some(self_rc.clone()));

        Ok(self_rc as _)
    }

    pub fn set_context(&self, context: Context) {
        // Save parameter names that are used by the UI
        let mut ui_parameters = self.ui_parameters.borrow_mut();
        for (name, _) in context.component_definition.global_properties("PluginParameters").unwrap() {
            ui_parameters.insert(name);
        }
        drop(ui_parameters);

        // Set callbacks
        let param_map = context.param_map.clone();
        let gui_context = context.gui_context.clone();
        context.component.set_global_callback("PluginParameters", "start-change", move |values| {
            if let Value::String(name) = &values[0] {
                let param_ptr = param_map.get(name.as_str()).unwrap();
                unsafe { gui_context.raw_begin_set_parameter(param_ptr.clone()) };
            }

            Value::Void
        }).unwrap();

        let param_map = context.param_map.clone();
        let gui_context = context.gui_context.clone();
        context.component.set_global_callback("PluginParameters", "changed", move |values| {
            if let (Value::String(name), Value::Number(value)) = (&values[0], &values[1]) {                
                let param_ptr = param_map.get(name.as_str()).unwrap();
                unsafe { gui_context.raw_set_parameter_normalized(param_ptr.clone(), *value as f32) };
            }

            Value::Void
        }).unwrap();

        let param_map = context.param_map.clone();
        let gui_context = context.gui_context.clone();
        context.component.set_global_callback("PluginParameters", "end-change", move |values| {
            if let Value::String(name) = &values[0] {
                let param_ptr = param_map.get(name.as_str()).unwrap();
                unsafe { gui_context.raw_end_set_parameter(param_ptr.clone()) };
            }

            Value::Void
        }).unwrap();

        let param_map = context.param_map.clone();
        let gui_context = context.gui_context.clone();
        context.component.set_global_callback("PluginParameters", "set-string", move |values| {
            if let (Value::String(name), Value::String(string)) = (&values[0], &values[1]) {
                let param_ptr = param_map.get(name.as_str()).unwrap();
                unsafe {
                    if let Some(value) = param_ptr.string_to_normalized_value(string) {
                        gui_context.raw_begin_set_parameter(param_ptr.clone());
                        gui_context.raw_set_parameter_normalized(param_ptr.clone(), value);
                        gui_context.raw_end_set_parameter(param_ptr.clone());
                    }
                }
            }

            Value::Void
        }).unwrap();

        // Set default values for parameters
        if let Some(ui_plugin_parameters) = context.component_definition.global_properties("PluginParameters") {
            for (name, _) in ui_plugin_parameters {
                if let Some(param_ptr) = context.param_map.get(&name) {
                    let default_value = unsafe { param_ptr.default_normalized_value() };

                    if let Ok(Value::Struct(mut plugin_parameter)) = context.component.get_global_property("PluginParameters", &name) {
                        plugin_parameter.set_field("default-value".into(), Value::Number(default_value as f64));
                        context.component.set_global_property("PluginParameters", &name, Value::Struct(plugin_parameter)).unwrap();
                    }
                }
            }
        }

        *self.context.borrow_mut() = Some(context);

        // Initialize parameter values
        self.update_all_parameters();
    }

    pub fn on_event(&self, event: plugin_canvas::Event) -> EventResponse {
        match event {
            plugin_canvas::Event::Draw => {
                let context = self.context.borrow();
                let context = context.as_ref().unwrap();

                // Update property values for all changed plugin parameters
                while let Ok(parameter_change) = context.parameter_change_receiver.try_recv() {
                    match parameter_change {
                        ParameterChange::ValueChanged { id } => {
                            if self.ui_parameters.borrow().contains(&id) {
                                self.update_parameter(&id, true, false);
                            }
                        }

                        ParameterChange::ModulationChanged { id } => {
                            if self.ui_parameters.borrow().contains(&id) {
                                self.update_parameter(&id, false, true);
                            }
                        }

                        ParameterChange::AllValuesChanged => {
                            self.update_all_parameters();
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
                        delta_x: delta_x as f32,
                        delta_y: delta_y as f32,
                    }
                );
                EventResponse::Handled
            },
            
            plugin_canvas::Event::DragEntered { position, data } => {
                EventResponse::Handled
            },

            plugin_canvas::Event::DragExited => {
                EventResponse::Handled
            },

            plugin_canvas::Event::DragMoved { position, data } => {
                EventResponse::Handled
            },

            plugin_canvas::Event::DragDropped { position, data } => {
                EventResponse::Handled
            },
        }
    }

    fn convert_button(button: plugin_canvas::MouseButton) -> i_slint_core::platform::PointerEventButton {
        match button {
            plugin_canvas::MouseButton::Left => i_slint_core::platform::PointerEventButton::Left,
            plugin_canvas::MouseButton::Right => i_slint_core::platform::PointerEventButton::Right,
            plugin_canvas::MouseButton::Middle => i_slint_core::platform::PointerEventButton::Middle,
        }
    }

    fn convert_logical_position(&self, position: plugin_canvas::LogicalPosition) -> slint::LogicalPosition {
        slint::LogicalPosition {
            x: (position.x / *self.user_scale) as f32,
            y: (position.y / *self.user_scale) as f32,
        }
    }

    fn update_parameter(&self, id: &str, update_value: bool, update_modulation: bool) {
        let context = self.context.borrow();
        let context = context.as_ref().unwrap();

        if let Some(param_ptr) = context.param_map.get(id) {
            if let Ok(Value::Struct(mut plugin_parameter)) = context.component.get_global_property("PluginParameters", &id) {
                let value = unsafe { param_ptr.unmodulated_normalized_value() };
                let modulation = unsafe { param_ptr.modulated_normalized_value() - value };

                if update_value {
                    let display_value = unsafe { param_ptr.normalized_value_to_string(value, true) };

                    plugin_parameter.set_field("value".into(), Value::Number(value as f64));
                    plugin_parameter.set_field("display-value".into(), Value::String(display_value.into()));    
                    plugin_parameter.set_field("modulation".into(), Value::Number(modulation as f64));
                } else if update_modulation {
                    plugin_parameter.set_field("modulation".into(), Value::Number(modulation as f64));
                }

                context.component.set_global_property("PluginParameters", id, Value::Struct(plugin_parameter)).unwrap();
            }
        }
    }

    fn update_all_parameters(&self) {
        for id in self.ui_parameters.borrow().iter() {
            self.update_parameter(id, true, true);
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
            i_slint_core::window::InputMethodRequest::SetPosition { .. } => true,
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
