use std::{cell::RefCell, rc::Rc, sync::{atomic::{AtomicBool, Ordering}, Arc}, collections::HashMap};

use i_slint_core::{window::{WindowAdapter, WindowAdapterInternal}, renderer::Renderer, platform::{PlatformError, WindowEvent}};
use i_slint_renderer_skia::SkiaRenderer;
use nih_plug::prelude::{ParamPtr, GuiContext};
use plugin_canvas::dimensions::Scale;
use raw_window_handle::{HasWindowHandle, HasDisplayHandle};
use slint_interpreter::{ComponentInstance, ComponentDefinition, Value};

thread_local! {
    pub static WINDOW_TO_SLINT: RefCell<Option<Box<plugin_canvas::Window>>> = Default::default();
    pub static WINDOW_ADAPTER_FROM_SLINT: RefCell<Option<Rc<PluginCanvasWindowAdapter>>> = Default::default();
}

pub struct Context {
    pub component: ComponentInstance,
    pub component_definition: ComponentDefinition,
    pub param_map: Rc<HashMap<String, ParamPtr>>,
    pub gui_context: Arc<dyn GuiContext>,
}

pub struct PluginCanvasWindowAdapter {
    context: RefCell<Option<Context>>,

    plugin_canvas_window: plugin_canvas::Window,
    slint_window: slint::Window,
    renderer: SkiaRenderer,

    slint_size: slint::PhysicalSize,
    user_scale: Scale,

    pending_draw: AtomicBool,
}

impl PluginCanvasWindowAdapter {
    pub fn new() -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let plugin_canvas_window = *WINDOW_TO_SLINT.with(|window| window.borrow_mut().take().unwrap());
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
                context: Default::default(),

                plugin_canvas_window,
                slint_window,
                renderer,

                slint_size,
                user_scale,

                pending_draw: AtomicBool::new(true),
            }
        });

        self_rc.slint_window.dispatch_event(
            WindowEvent::ScaleFactorChanged { scale_factor: *user_scale as f32 }
        );

        WINDOW_ADAPTER_FROM_SLINT.with({
            let self_rc = self_rc.clone();
            move |window_adapter| *window_adapter.borrow_mut() = Some(self_rc)
        });

        Ok(self_rc as _)
    }

    pub fn set_context(&self, context: Context) {
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

        *self.context.borrow_mut() = Some(context);
    }

    // pub fn set_param_map(&self, param_map: Vec<(String, ParamPtr, String)>) {
    //     let param_map = param_map.iter()
    //         .map(|(name, param_ptr, _)| {
    //             (name.clone(), *param_ptr)
    //         })
    //         .collect();

    //     *self.param_map.borrow_mut() = Some(Rc::new(param_map));
    // }

    pub fn on_event(&self, event: plugin_canvas::Event) {
        match event {
            plugin_canvas::Event::Draw => {
                let context = self.context.borrow();
                let context = context.as_ref().unwrap();

                // Update all property values from plugin parameters
                if let Some(ui_plugin_parameters) = context.component_definition.global_properties("PluginParameters") {
                    for (name, _) in ui_plugin_parameters {
                        if let Some(param_ptr) = context.param_map.get(&name) {
                            let value = unsafe { param_ptr.unmodulated_normalized_value() };
                            context.component.set_global_property("PluginParameters", &name, Value::Number(value as f64)).unwrap();
                        }
                    }
                }

                i_slint_core::platform::update_timers_and_animations();
                
                if self.pending_draw.swap(false, Ordering::Relaxed) {
                    self.renderer.render().unwrap();
                }
            },

            plugin_canvas::Event::KeyDown { text } => {
                let text = text.into();
                self.slint_window.dispatch_event(WindowEvent::KeyPressed { text });
            },

            plugin_canvas::Event::KeyUp { text } => {
                let text = text.into();
                self.slint_window.dispatch_event(WindowEvent::KeyReleased { text });
            },

            plugin_canvas::Event::MouseButtonDown { button, position } => {
                let button = Self::convert_button(button);
                let position = self.convert_logical_position(position);

                self.slint_window.dispatch_event(WindowEvent::PointerPressed { position, button });
            },

            plugin_canvas::Event::MouseButtonUp { button, position } => {
                let button = Self::convert_button(button);
                let position = self.convert_logical_position(position);

                self.slint_window.dispatch_event(WindowEvent::PointerReleased { position, button });
            },

            plugin_canvas::Event::MouseExited => {
                self.slint_window.dispatch_event(WindowEvent::PointerExited);
            },

            plugin_canvas::Event::MouseMoved { position } => {
                let position = self.convert_logical_position(position);
                self.slint_window.dispatch_event(WindowEvent::PointerMoved { position });
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
