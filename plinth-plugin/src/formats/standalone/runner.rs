use std::{rc::Rc, sync::{Arc, mpsc}, time::{Duration, Instant}};

use cpal::{BufferSize, FromSample, I24, SizedSample, Stream, StreamConfig, traits::{DeviceTrait, StreamTrait}};
use midir::MidiInputConnection;
use raw_window_handle::HasWindowHandle;
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowAttributes, WindowId}};

use super::{parameters::StandaloneParameterEventMap, audio::AudioState, config::{AudioOutputConfig, MidiInputConfig}, host::StandaloneHost, midi, plugin::StandalonePlugin};

use crate::{Editor, Event, Host, HostInfo, ProcessMode, Processor, ProcessorConfig, formats::PluginFormat};

struct StandaloneRunner<P: StandalonePlugin> {
    plugin: P,
    editor: P::Editor,
    to_plugin_receiver: mpsc::Receiver<Event>,
    title: &'static str,
    size: (f64, f64),
    window: Option<Window>,
    last_frame: Instant,
    audio_stream: Stream,
    midi_connections: Vec<MidiInputConnection<()>>,
}

impl<P: StandalonePlugin> Drop for StandaloneRunner<P> {
    fn drop(&mut self) {
        let _ = self.audio_stream.pause();
        self.midi_connections.clear();
    }
}

impl<P: StandalonePlugin> ApplicationHandler for StandaloneRunner<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create new window
        let attrs = WindowAttributes::default()
            .with_title(self.title)
            .with_inner_size(winit::dpi::LogicalSize::new(self.size.0, self.size.1))
            .with_resizable(self.editor.can_resize());

        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(e) => {
                log::error!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        // Set initial scale and get initial size
        if !cfg!(target_os = "macos") {
            // On macOS the system's DPI scale already is applied in the plugin view
            self.editor.set_scale(window.scale_factor());
        }
        self.size = self.editor.window_size();

        // Attach editor to the window
        let handle = window
            .window_handle()
            .expect("Failed to get window's platform handle")
            .as_raw();
        self.editor.open(handle);
        self.window = Some(window);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.editor.close();
        self.window = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            self.editor.close();
            event_loop.exit();
        } else if let WindowEvent::ScaleFactorChanged {
            scale_factor,
            inner_size_writer: _,
        } = event
        {
            #[allow(clippy::collapsible_if)]
            if !cfg!(target_os = "macos") {
                // see `resumed`impl
                self.editor.set_scale(scale_factor);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        let now = Instant::now();
        let frame_interval = Duration::from_millis(16);

        if now >= self.last_frame + frame_interval {
            while let Ok(event) = self.to_plugin_receiver.try_recv() {
                self.plugin.process_event(&event);
            }
            self.editor.on_frame();
            self.last_frame = now;
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(self.last_frame + frame_interval));
    }
}

pub fn run_standalone<P: StandalonePlugin + 'static>(
    audio_config: AudioOutputConfig,
    midi_config: MidiInputConfig,
) {
    let host_info = HostInfo {
        name: Some("Standalone".to_string()),
        format: PluginFormat::Standalone,
    };

    let mut plugin = P::new(host_info);

    // Parameter event map (shared between host and audio thread)
    let parameter_event_map =
        plugin.with_parameters(|params| Arc::new(StandaloneParameterEventMap::new(params)));

    // Channels
    let (midi_sender, midi_receiver) = mpsc::channel::<Event>();
    let (to_plugin_sender, to_plugin_receiver) = mpsc::channel::<Event>();

    // Open MIDI connections if plugin accepts note inputs
    let midi_connections = if P::HAS_NOTE_INPUT {
        midi::connect_inputs(&midi_config, midi_sender)
    } else {
        vec![]
    };

    // Open audio device
    let mut audio_host = audio_config
        .open_host()
        .expect("Failed to open audio driver");
    let audio_device = audio_config
        .open_device(&mut audio_host)
        .expect("Failed to open audio device");
    let audio_stream_config = audio_config
        .select_stream_config(&audio_device)
        .expect("Failed to select audio stream config");

    // Create processor
    // NB: CPAL unfortunately has no getter for the real applied block size, so we need to ensure that the processor never gets called with more frames
    let processor_config = ProcessorConfig {
        sample_rate: audio_stream_config.sample_rate() as f64,
        min_block_size: 1,
        max_block_size: P::MAX_BLOCK_SIZE,
        process_mode: ProcessMode::Realtime,
    };
    let mut processor = plugin.create_processor(processor_config);
    processor.reset();

    // Create audio state
    let audio_state = AudioState::<P>::new(
        processor,
        audio_stream_config.channels() as usize,
        midi_receiver,
        parameter_event_map.clone(),
    );

    // Create and start the CPAL stream
    fn run_audio_stream<P, T>(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        mut audio_state: AudioState<P>,
    ) -> Result<Stream, Box<dyn std::error::Error>>
    where
        P: StandalonePlugin + 'static,
        T: SizedSample + FromSample<f32>,
        f32: FromSample<T>,
    {
        let channels = config.channels as usize;

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                audio_state.process(data, channels);
            },
            |err| {
                log::error!("An audio stream error occurred: {err}");
            },
            None,
        )?;
        stream.play()?;

        Ok(stream)
    }

    let stream_format = audio_stream_config.sample_format();
    let stream_config = StreamConfig {
        channels: audio_stream_config.channels(),
        sample_rate: audio_stream_config.sample_rate(),
        buffer_size: audio_config
            .buffer_size
            .map(BufferSize::Fixed)
            .unwrap_or(BufferSize::Default),
    };

    let audio_stream = match stream_format {
        cpal::SampleFormat::I8 => run_audio_stream::<P, i8>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::I16 => run_audio_stream::<P, i16>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::I24 => run_audio_stream::<P, I24>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::I32 => run_audio_stream::<P, i32>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::I64 => run_audio_stream::<P, i64>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::U8 => run_audio_stream::<P, u8>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::U16 => run_audio_stream::<P, u16>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::U32 => run_audio_stream::<P, u32>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::U64 => run_audio_stream::<P, u64>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::F32 => run_audio_stream::<P, f32>(&audio_device, stream_config, audio_state),
        cpal::SampleFormat::F64 => run_audio_stream::<P, f64>(&audio_device, stream_config, audio_state),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .expect("Failed to build audio output stream");

    // Create host and editor
    let host = Rc::new(StandaloneHost::new(parameter_event_map, to_plugin_sender));
    let editor = plugin.create_editor(host as Rc<dyn Host>);

    // Create winit event loop
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    // Run winit event loop (blocks until window is closed)
    let mut runner = StandaloneRunner {
        plugin,
        editor,
        to_plugin_receiver,
        title: P::NAME,
        size: P::Editor::DEFAULT_SIZE,
        window: None,
        last_frame: Instant::now(),
        audio_stream,
        midi_connections,
    };

    event_loop.run_app(&mut runner).expect("Event loop error");
}
