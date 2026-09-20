use std::any::TypeId;
use std::cell::RefCell;
use std::ffi::CStr;
use std::iter::zip;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use atomic_refcell::AtomicRefCell;
use plinth_core::signals::ptr_signal::{PtrSignal, PtrSignalMut};
use plinth_core::signals::signal::SignalMut;
use vst3::Steinberg::Vst::ControllerNumbers_::{kAfterTouch, kCtrlProgramChange, kPitchBend};
use vst3::Steinberg::Vst::{CtrlNumber, IMidiMapping, IMidiMappingTrait, INoteExpressionController, INoteExpressionPhysicalUIMapping};
use vst3::{ComPtr, ComRef};
use vst3::Steinberg::{int16, int32, kInvalidArgument, kNoInterface, kResultFalse, kResultOk, kResultTrue, tresult, uint32, FIDString, FUnknown, IBStream, IPlugView, IPluginBaseTrait, TBool, TUID};
use vst3::Steinberg::Vst::{kInfiniteTail, kNoParentUnitId, kNoProgramListId, kNoTail, BusDirection, BusDirections, BusDirections_, BusInfo, BusInfo_::BusFlags_, BusTypes_, CString, IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler, IComponentTrait, IEditController, IEditController2, IEditController2Trait, IEditControllerTrait, IHostApplication, IHostApplicationTrait, IProcessContextRequirements, IProcessContextRequirementsTrait, IProcessContextRequirements_, IUnitInfo, IUnitInfoTrait, IoMode, IoModes_, KnobMode, MediaType, MediaTypes, MediaTypes_, ParamID, ParamValue, ParameterInfo_, ProcessData, ProcessSetup, ProgramListID, ProgramListInfo, RoutingInfo, SpeakerArr, SpeakerArrangement, String128, SymbolicSampleSizes_, TChar, UnitID, UnitInfo, ViewType::kEditor};
use widestring::U16CStr;

use crate::formats::PluginFormat;
use crate::host::HostInfo;
use crate::vst3::parameters::{MidiParameter, MidiParameters};
use crate::{Event, Parameters, ProcessMode, ProcessState, Processor, ProcessorConfig};
use crate::editor::NoEditor;
use crate::parameters::{group::{self, ParameterGroupRef}, has_duplicates, info::ParameterInfo};
use crate::string::{char16_to_string, copy_str_to_char16};
use crate::vst3::{event::EventIterator, parameters::ParameterChangeIterator};
use crate::midi_capabilities::{MIDI_CHANNEL_COUNT, MIDI_CONTROLLER_COUNT};

use super::{plugin::Vst3Plugin, stream::Stream, view::View};

const ROOT_UNIT_NAME: &str  = "Root";
const ROOT_UNIT_ID: i32     = 0;
const FIRST_UNIT_ID: i32    = 1;

pub struct AudioThreadState<P: Vst3Plugin> {
    processor: parking_lot::Mutex<Option<P::Processor>>,
    aux_active: AtomicBool,
}

impl<P: Vst3Plugin> Default for AudioThreadState<P> {
    fn default() -> Self {
        Self {
            processor: Default::default(),
            aux_active: true.into(),
        }
    }
}

pub struct PluginComponent<P: Vst3Plugin> {
    plugin: Rc<RefCell<Option<P>>>,

    // NB: AtomicRefCell instead of RefCell because parameter info may be read concurrently (UI and audio)
    parameter_info: AtomicRefCell<Vec<ParameterInfo>>,
    parameter_groups: AtomicRefCell<Vec<ParameterGroupRef>>,
    midi_parameters: AtomicRefCell<MidiParameters>,

    process_mode: AtomicRefCell<ProcessMode>,
    processing: AtomicBool,
    tail_length: AtomicU32,
    latency: AtomicU32,

    component_handler: Rc<RefCell<Option<ComPtr<IComponentHandler>>>>,

    audio_thread_state: AudioThreadState<P>,
}

impl<P: Vst3Plugin + 'static> PluginComponent<P> {
    pub fn new() -> Self {
        Self {
            plugin: Default::default(),

            parameter_info: Default::default(),
            parameter_groups: Default::default(),
            midi_parameters: Default::default(),

            process_mode: ProcessMode::default().into(),
            processing: AtomicBool::new(false),
            tail_length: AtomicU32::new(0),
            latency: AtomicU32::new(0),

            component_handler: Default::default(),

            audio_thread_state: Default::default(),
        }
    }

    fn parameter_group_id(&self, parameter_info: &ParameterInfo) -> i32 {
        let parameter_path = parameter_info.path();
        if parameter_path.is_empty() {
            return ROOT_UNIT_ID;
        }

        let unit_index = self.parameter_groups.borrow().iter().position(|group| group.path == parameter_path).unwrap() as i32;
        FIRST_UNIT_ID + unit_index
    }
}

impl<P: Vst3Plugin> vst3::Class for PluginComponent<P> {
    type Interfaces = (IAudioProcessor, IComponent, IComponent, IEditController, IEditController2, IMidiMapping, IProcessContextRequirements, IUnitInfo, INoteExpressionController, INoteExpressionPhysicalUIMapping);
}

impl<P: Vst3Plugin> IPluginBaseTrait for PluginComponent<P> {
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        tracing::trace!("IPluginBase::initialize");

        if self.plugin.borrow().is_some() {
            return kResultOk;
        }

        // Get plugin name if available
        let mut host_name = None;

        if let Some(context) = unsafe { ComRef::from_raw(context) } && let Some(host_application) = context.cast::<IHostApplication>() {
            let mut name = [0; 128];

            if unsafe { host_application.getName(&mut name) == kResultOk } && let Some(name) = char16_to_string(&name) {
                host_name = Some(name);
            }
        }

        // Create plugin and find parameter info
        let host_info = HostInfo {
            name: host_name,
            format: PluginFormat::Vst3,
        };

        let mut plugin = P::new(host_info);
        assert!(plugin.with_parameters(|parameters| !has_duplicates(parameters.ids())));

        plugin.init();

        let mut parameter_infos = self.parameter_info.borrow_mut();

        // Create units based on parameter groups
        // Also verify parameters
        *self.parameter_groups.borrow_mut() = plugin.with_parameters(|parameters| {
            assert!(
                parameters.ids().iter()
                    .copied()
                    .filter(|&id| parameters.get(id).unwrap().info().is_bypass())
                    .count() <= 1,
                "You can only define one bypass parameter"
            );

            for &id in parameters.ids() {
                let info = parameters.get(id).unwrap().info();
                parameter_infos.push(info.clone());
            }

            group::from_parameters(parameters)
        });

        // Allocate hidden reserved VST3 parameters for each MIDI message type the plugin requires
        // via its MIDI_CAPABILITIES. MIDI needs a note input port, so allocate nothing without one.
        if P::HAS_NOTE_INPUT {
            let midi_parameters = plugin.with_parameters(|parameters| {
                MidiParameters::new(&P::MIDI_CAPABILITIES, parameters.ids(), &mut parameter_infos)
            });
            *self.midi_parameters.borrow_mut() = midi_parameters;
        }

        *self.plugin.borrow_mut() = Some(plugin);

        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        tracing::trace!("IPluginBase::terminate");

        *self.plugin.borrow_mut() = None;
        self.parameter_info.borrow_mut().clear();
        self.parameter_groups.borrow_mut().clear();
        self.midi_parameters.borrow_mut().clear();

        kResultOk
    }
}

impl<P: Vst3Plugin> IAudioProcessorTrait for PluginComponent<P> {
    unsafe fn setBusArrangements(&self, inputs: *mut SpeakerArrangement, num_ins: int32, outputs: *mut SpeakerArrangement, num_outs: int32) -> tresult {
        tracing::trace!("IAudioProcessor::setBusArrangements");

        if inputs.is_null() || outputs.is_null() {
            return kInvalidArgument;
        }

        let expected_inputs = if P::HAS_AUX_INPUT { 2 } else { 1 };
        if num_ins != expected_inputs {
            return kResultFalse;
        }

        if num_outs != 1 {
            return kResultFalse;
        }

        let inputs = unsafe { std::slice::from_raw_parts(inputs, num_ins as _) };
        if inputs[0] != SpeakerArr::kStereo {
            return kResultFalse;
        }
        if P::HAS_AUX_INPUT && inputs[1] != SpeakerArr::kStereo {
            return kResultFalse;
        }

        let outputs = unsafe { std::slice::from_raw_parts(outputs, num_outs as _) };
        if outputs[0] != SpeakerArr::kStereo {
            return kResultFalse;
        }

        kResultOk
    }

    unsafe fn getBusArrangement(&self, _dir: BusDirection, _index: int32, arr: *mut SpeakerArrangement) -> tresult {
        tracing::trace!("IAudioProcessor::getBusArrangements");

        // Only support stereo
        unsafe { *arr = SpeakerArr::kStereo; }
        kResultOk
    }

    unsafe fn canProcessSampleSize(&self, symbolic_sample_size: int32) -> tresult {
        tracing::trace!("IAudioProcessor::canProcessSampleSize");

        if symbolic_sample_size == SymbolicSampleSizes_::kSample32 as int32 {
            kResultOk
        } else {
            kResultFalse
        }
    }

    unsafe fn getLatencySamples(&self) -> uint32 {
        tracing::trace!("IAudioProcessor::getLatencySamples");
        self.latency.load(Ordering::Acquire)
    }

    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        tracing::trace!("IAudioProcessor::setupProcessing");

        let setup = unsafe { &*setup };

        let processor_config = ProcessorConfig {
            sample_rate: setup.sampleRate,
            min_block_size: 0,
            max_block_size: setup.maxSamplesPerBlock as _,
            process_mode: *self.process_mode.borrow(),
        };

        let plugin = self.plugin.borrow();
        let Some(plugin) = plugin.as_ref() else {
            return kResultFalse;
        };

        let mut processor = self.audio_thread_state.processor.lock();
        *processor = Some(plugin.create_processor(processor_config));

        // Cache latency since it's not allowed to change during processing
        self.latency.store(plugin.latency(), Ordering::Release);

        kResultOk
    }

    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        tracing::trace!("IAudioProcessor::setProcessing: {state}");

        let processing = state != 0;
        self.processing.store(processing, Ordering::Release);

        let mut processor = self.audio_thread_state.processor.lock();
        if let Some(processor) = processor.as_mut() && !processing {
            processor.reset();
        }

        kResultOk
    }

    // Called from the audio thread
    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        let data = unsafe { &mut *data };

        let midi_parameters = self.midi_parameters.borrow();
        let parameter_change_iterator = ParameterChangeIterator::new(data.inputParameterChanges, &midi_parameters);
        let event_iterator = EventIterator::new(data.inputEvents, P::NOTE_EXPRESSIONS);
        let all_events = event_iterator.chain(parameter_change_iterator);
        let is_data_dump = data.inputs.is_null() || data.outputs.is_null() || data.numInputs == 0 || data.numSamples == 0;

        // On some platforms, this cast is needed
        #[allow(clippy::unnecessary_cast)]
        if !is_data_dump && data.symbolicSampleSize != SymbolicSampleSizes_::kSample32 as i32 {
            return kResultFalse;
        }

        // Prepare inputs & outputs
        let (main_input, main_output, aux_input) = if is_data_dump {
            (None, None, None)
        } else {
            let inputs = unsafe { std::slice::from_raw_parts(data.inputs, data.numInputs as _) };
            let outputs = unsafe { std::slice::from_raw_parts(data.outputs, data.numOutputs as _) };
            let main_input = inputs[0];
            let main_output = outputs[0];
            assert_eq!(main_input.numChannels, main_output.numChannels);

            let aux_input = if P::HAS_AUX_INPUT && self.audio_thread_state.aux_active.load(Ordering::Acquire) {
                assert_eq!(data.numInputs, 2);
                let aux_input = inputs[1];
                Some(unsafe { PtrSignal::from_pointers(aux_input.numChannels as usize, data.numSamples as usize, aux_input.__field0.channelBuffers32 as _) })
            } else {
                None
            };

            let main_input = unsafe { PtrSignal::from_pointers(main_input.numChannels as usize, data.numSamples as usize, main_input.__field0.channelBuffers32 as _) };
            let main_output = unsafe { PtrSignalMut::from_pointers(main_output.numChannels as usize, data.numSamples as usize, main_output.__field0.channelBuffers32) };

            (Some(main_input), Some(main_output), aux_input)
        };

        // Real-time safety: parking_lot Mutex is guaranteed to not do syscalls when uncontented
        // contention can only occur if we're setting up or tearing down the processor while process is called
        // In that case, we will simply output silence
        let Some(mut processor) = self.audio_thread_state.processor.try_lock() else {
            if let Some(mut main_output) = main_output {
                main_output.fill(0.0);
            }

            return kResultOk;
        };

        let Some(processor) = processor.as_mut() else {
            return kResultFalse;
        };

        if is_data_dump {
            processor.process_events(all_events);
            return kResultOk;
        }

        let main_input = main_input.unwrap();
        let mut main_output = main_output.unwrap();

        // If processing out-of-place, copy input to output
        if zip(main_input.pointers().iter(), main_output.pointers().iter())
            .any(|(&input_ptr, &output_ptr)| input_ptr != unsafe { &*output_ptr })
        {
            main_output.copy_from_signal(&main_input);
        }

        let transport = if data.processContext.is_null() {
            None
        } else {
            Some(unsafe { &*data.processContext }.into())
        };

        let process_state = processor.process(&mut main_output, aux_input.as_ref(), transport, all_events);

        let tail_length = match process_state {
            ProcessState::Error => {
                tracing::error!("Processing error!");
                return kResultFalse;
            },

            ProcessState::Normal | ProcessState::Tail(0) => kNoTail,
            ProcessState::Tail(tail) => tail as _,
            ProcessState::KeepAlive => kInfiniteTail,
        };

        self.tail_length.store(tail_length, Ordering::Release);

        kResultOk
    }

    unsafe fn getTailSamples(&self) -> uint32 {
        self.tail_length.load(Ordering::Acquire)
    }
}

impl<P: Vst3Plugin> IComponentTrait for PluginComponent<P> {
    unsafe fn getControllerClassId(&self, _class_id: *mut TUID) -> tresult {
        tracing::trace!("IComponent::getControllerClassId");
        kNoInterface
    }

    unsafe fn setIoMode(&self, mode: IoMode) -> tresult {
        tracing::trace!("IComponent::setIoMode");

        let mode = match mode as _ {
            IoModes_::kSimple | IoModes_::kAdvanced => ProcessMode::Realtime,
            IoModes_::kOfflineProcessing => ProcessMode::Offline,
            _ => {
                return kInvalidArgument;
            }
        };

        *self.process_mode.borrow_mut() = mode;

        kResultOk
    }

    unsafe fn getBusCount(&self, media_type: MediaType, dir: BusDirection) -> int32 {
        tracing::trace!("IComponent::getBusCount");

        let is_input = dir as BusDirections == BusDirections_::kInput;

        match media_type as MediaTypes {
            MediaTypes_::kAudio => {
                if is_input && P::HAS_AUX_INPUT {
                    2
                } else {
                    1
                }
            }
            MediaTypes_::kEvent => {
                if is_input {
                    P::HAS_NOTE_INPUT as int32
                } else {
                    P::HAS_NOTE_OUTPUT as int32
                }
            }
            _ => 0
        }
    }

    unsafe fn getBusInfo(&self, media_type: MediaType, dir: BusDirection, index: int32, bus: *mut BusInfo) -> tresult {
        tracing::trace!("IComponent::getBusInfo");

        if index >= unsafe { self.getBusCount(media_type, dir) } {
            return kInvalidArgument;
        }

        let bus = unsafe { &mut *bus };
        bus.mediaType = media_type;
        bus.direction = dir;
        bus.flags = BusFlags_::kDefaultActive as _;

        if index == 0 {
            copy_str_to_char16("Main", &mut bus.name);
            bus.busType = BusTypes_::kMain as _;
        } else {
            copy_str_to_char16("Aux", &mut bus.name);
            bus.busType = BusTypes_::kAux as _;
        }

        bus.channelCount = match media_type as _ {
            MediaTypes_::kAudio => 2,
            MediaTypes_::kEvent => 16,
            _ => { return kInvalidArgument }
        };

        kResultOk
    }

    unsafe fn getRoutingInfo(&self, in_info: *mut RoutingInfo, out_info: *mut RoutingInfo) -> tresult {
        tracing::trace!("IComponent::getRoutingInfo");

        let in_info = unsafe { &*in_info };
        let out_info = unsafe { &mut *out_info };

        out_info.mediaType = in_info.mediaType;
        out_info.busIndex = in_info.busIndex;
        out_info.channel = in_info.channel;

        kResultOk
    }

    unsafe fn activateBus(&self, media_type: MediaType, dir: BusDirection, index: int32, state: TBool) -> tresult {
        tracing::trace!("IComponent::activateBus");

        // On some platforms, these casts are needed
        #[allow(clippy::unnecessary_cast)]
        if P::HAS_AUX_INPUT && media_type == MediaTypes_::kAudio as i32 && dir == BusDirections_::kInput as i32 && index == 1 {
            self.audio_thread_state.aux_active.store(state != 0, Ordering::Release);
        }

        // TODO: Support disabling other buses
        kResultOk
    }

    unsafe fn setActive(&self, _state: TBool) -> tresult {
        tracing::trace!("IComponent::setActive: {_state}");
        kResultOk
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        tracing::trace!("IComponent::setState");

        let mut plugin = self.plugin.borrow_mut();
        let Some(plugin) = plugin.as_mut() else {
            return kResultFalse;
        };

        let Some(mut stream) = Stream::new(state) else {
            return kResultFalse;
        };

        match plugin.load_state(&mut stream) {
            Ok(_) => kResultOk,
            Err(_) => kResultFalse, // TODO: Extract actual error code
        }
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        tracing::trace!("IComponent::getState");

        let plugin = self.plugin.borrow();
        let Some(plugin) = plugin.as_ref() else {
            return kResultFalse;
        };
        let Some(mut stream) = Stream::new(state) else {
            return kResultFalse;
        };

        match plugin.save_state(&mut stream) {
            Ok(_) => kResultOk,
            Err(_) => kResultFalse, // TODO: Extract actual error code
        }
    }
}

impl<P: Vst3Plugin + 'static> IEditControllerTrait for PluginComponent<P> {
    unsafe fn setComponentState(&self, _state: *mut IBStream) -> tresult {
        tracing::trace!("IEditController::setComponentState");
        kResultOk
    }

    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        tracing::trace!("IEditController::setState");
        kResultOk
    }

    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        tracing::trace!("IEditController::getState");
        kResultOk
    }

    unsafe fn getParameterCount(&self) -> int32 {
        tracing::trace!("IEditController::getParameterCount");
        self.parameter_info.borrow().len() as _
    }

    unsafe fn getParameterInfo(&self, param_index: int32, info: *mut vst3::Steinberg::Vst::ParameterInfo) -> tresult {
        tracing::trace!("IEditController::getParameterInfo");

        if param_index < 0 {
            return kInvalidArgument;
        }

        let parameter_info = self.parameter_info.borrow();
        let Some(parameter_info) = parameter_info.get(param_index as usize) else {
            return kInvalidArgument;
        };

        let vst3_info = unsafe { &mut *info };

        vst3_info.id = parameter_info.id();
        copy_str_to_char16(parameter_info.name(), &mut vst3_info.title);
        // TODO: info.shortTitle
        vst3_info.stepCount = parameter_info.steps() as _;
        vst3_info.defaultNormalizedValue = parameter_info.default_normalized_value();
        vst3_info.unitId = self.parameter_group_id(parameter_info);

        #[allow(clippy::unnecessary_cast)]
        if parameter_info.is_bypass() {
            vst3_info.flags = ParameterInfo_::ParameterFlags_::kIsBypass as i32;
            vst3_info.flags |= ParameterInfo_::ParameterFlags_::kCanAutomate as i32;
        } else if !parameter_info.visible() {
            vst3_info.flags = ParameterInfo_::ParameterFlags_::kIsHidden as i32;
        } else {
            vst3_info.flags = ParameterInfo_::ParameterFlags_::kCanAutomate as i32;
        }

        kResultOk
    }

    unsafe fn getParamStringByValue(&self, id: ParamID, value_normalized: ParamValue, string: *mut String128) -> tresult {
        tracing::trace!("IEditController::getParamStringByValue");

        let plugin = self.plugin.borrow();
        let Some(plugin) = plugin.as_ref() else {
            return kResultFalse;
        };

        plugin.with_parameters(|parameters| {
            let Some(parameter) = parameters.get(id) else {
                return kInvalidArgument;
            };

            let formatted = parameter.normalized_to_string(value_normalized);
            copy_str_to_char16(&formatted, unsafe { &mut *string });

            kResultOk
        })
    }

    unsafe fn getParamValueByString(&self, id: ParamID, string: *mut TChar, value_normalized: *mut ParamValue) -> tresult {
        tracing::trace!("IEditController::getParamValueByString");

        if string.is_null() {
            return kInvalidArgument;
        }

        let plugin = self.plugin.borrow();
        let Some(plugin) = plugin.as_ref() else {
            return kResultFalse;
        };

        let string = unsafe { U16CStr::from_ptr_str(string as _) };
        let Ok(string) = string.to_string() else {
            return kInvalidArgument;
        };

        plugin.with_parameters(|parameters| {
            let Some(parameter) = parameters.get(id) else {
                return kInvalidArgument;
            };

            let Some(value) = parameter.string_to_normalized(&string) else {
                return kInvalidArgument;
            };

            unsafe { *value_normalized = value };

            kResultOk
        })
    }

    unsafe fn normalizedParamToPlain(&self, _id: ParamID, value_normalized: ParamValue) -> ParamValue {
        value_normalized
    }

    unsafe fn plainParamToNormalized(&self, _id: ParamID, plain_value: ParamValue) -> ParamValue {
        plain_value
    }

    unsafe fn getParamNormalized(&self, id: ParamID) -> ParamValue {
        tracing::trace!("IEditController::getParamNormalized");

        let plugin = self.plugin.borrow();
        let Some(plugin) = plugin.as_ref() else {
            return 0.0;
        };

        plugin.with_parameters(|parameters| {
            let Some(parameter) = parameters.get(id) else {
                return 0.0;
            };

            parameter.normalized_value()
        })
    }

    unsafe fn setParamNormalized(&self, id: ParamID, value: ParamValue) -> tresult {
        tracing::trace!("IEditController::setParamNormalized");

        let mut plugin = self.plugin.borrow_mut();
        let Some(plugin) = plugin.as_mut() else {
            return kResultFalse;
        };

        let event = self.midi_parameters.borrow().parameter_change_to_event(id, value, 0);

        if let Event::ParameterValue { id, value, .. } = event {
            plugin.with_parameters(|parameters| parameters.get(id).unwrap().set_normalized_value(value));
        }

        kResultOk
    }

    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        tracing::trace!("IEditController::setComponentHandler: {:x}", handler as usize);

        if handler.is_null() {
            *self.component_handler.borrow_mut() = None;
        } else {
            let Some(handler) = (unsafe { ComRef::from_raw(handler) }) else {
                return kInvalidArgument;
            };

            *self.component_handler.borrow_mut() = Some(handler.to_com_ptr());
        }

        kResultOk
    }

    unsafe fn createView(&self, name: FIDString) -> *mut IPlugView {
        tracing::trace!("IEditController::createView");

        if name.is_null() {
            return null_mut();
        }

        if unsafe { CStr::from_ptr(name) != CStr::from_ptr(kEditor) } {
            return null_mut();
        }

        if TypeId::of::<P::Editor>() == TypeId::of::<NoEditor>() {
            return null_mut();
        }

        let view = View::<P>::new(
            self.plugin.clone(),
            self.component_handler.clone(),
        );

        view.to_com_ptr::<IPlugView>().unwrap().into_raw()
    }
}

impl<P: Vst3Plugin> IEditController2Trait for PluginComponent<P> {
    unsafe fn setKnobMode(&self, _mode: KnobMode) -> tresult {
        tracing::trace!("IEditController2::setKnobMode");
        kResultFalse
    }

    unsafe fn openHelp(&self, _only_check: TBool) -> tresult {
        tracing::trace!("IEditController2::openHelp");
        kResultFalse
    }

    unsafe fn openAboutBox(&self, _only_check: TBool) -> tresult {
        tracing::trace!("IEditController2::openAboutBox");
        kResultFalse
    }
}

impl<P: Vst3Plugin> IMidiMappingTrait for PluginComponent<P> {
    unsafe fn getMidiControllerAssignment(
        &self,
        bus_index: int32,
        channel: int16,
        midi_controller_number: CtrlNumber,
        id: *mut ParamID) -> tresult
    {
        if bus_index != 0 {
            return kResultFalse;
        }
        if !(0..MIDI_CHANNEL_COUNT as i16).contains(&channel) {
            return kInvalidArgument;
        }

        let channel = channel as u8;

        let midi_parameter = if midi_controller_number == kPitchBend as i16 {
            MidiParameter::PitchBend { channel }
        } else if midi_controller_number == kAfterTouch as i16 {
            MidiParameter::ChannelPressure { channel }
        } else if midi_controller_number == kCtrlProgramChange as i16 {
            MidiParameter::ProgramChange { channel }
        } else if (0..MIDI_CONTROLLER_COUNT as i16).contains(&midi_controller_number) {
            MidiParameter::ControlChange { channel, controller: midi_controller_number as u8 }
        } else {
            return kResultFalse;
        };

        if let Some(parameter_id) = self.midi_parameters.borrow().parameter_id(&midi_parameter) {
            unsafe { *id = parameter_id as _ };
            kResultTrue
        } else {
            kResultFalse
        }
    }
}

impl<P: Vst3Plugin> IProcessContextRequirementsTrait for PluginComponent<P> {
    unsafe fn getProcessContextRequirements(&self) -> uint32 {
        tracing::trace!("IProcessContextRequirements::getProcessContextRequirements");
        IProcessContextRequirements_::Flags_::kNeedContinousTimeSamples as uint32 |
        IProcessContextRequirements_::Flags_::kNeedProjectTimeMusic as uint32 |
        IProcessContextRequirements_::Flags_::kNeedBarPositionMusic as uint32 |
        IProcessContextRequirements_::Flags_::kNeedCycleMusic as uint32 |
        IProcessContextRequirements_::Flags_::kNeedTempo as uint32 |
        IProcessContextRequirements_::Flags_::kNeedTimeSignature as uint32 |
        IProcessContextRequirements_::Flags_::kNeedTransportState as uint32
    }
}

impl<P: Vst3Plugin> IUnitInfoTrait for PluginComponent<P> {
    unsafe fn getUnitCount(&self) -> int32 {
        tracing::trace!("IUnitInfo::getUnitCount");
        let parameter_groups = self.parameter_groups.borrow();
        parameter_groups.len() as int32 + 1 // +1 for the root unit
    }

    unsafe fn getUnitInfo(&self, unit_index: int32, info: *mut UnitInfo) -> tresult {
        tracing::trace!("IUnitInfo::getUnitInfo");

        let parameter_groups = self.parameter_groups.borrow();
        let unit_count = parameter_groups.len() + 1; // +1 for the root unit

        if unit_index < 0 {
            return kInvalidArgument;
        }
        if unit_index as usize >= unit_count {
            return kInvalidArgument;
        }

        let info = unsafe { &mut *info };
        info.id = unit_index;
        info.programListId = kNoProgramListId;
        info.parentUnitId = kNoParentUnitId;

        // Special case root unit
        if unit_index == ROOT_UNIT_ID {
            copy_str_to_char16(ROOT_UNIT_NAME, &mut info.name);
        } else {
            let unit_index = unit_index - FIRST_UNIT_ID;
            let group = &parameter_groups[unit_index as usize];
            copy_str_to_char16(&group.name, &mut info.name);

            if let Some(parent) = &group.parent {
                info.parentUnitId = FIRST_UNIT_ID + parameter_groups.iter().position(|group| group == parent).unwrap() as i32;
            } else {
                info.parentUnitId = ROOT_UNIT_ID;
            }
        }

        kResultOk
    }

    unsafe fn getProgramListCount(&self) -> int32 {
        tracing::trace!("IUnitInfo::getProgramListCount");
        0
    }

    unsafe fn getProgramListInfo(&self, _list_index: int32, _info: *mut ProgramListInfo) -> tresult {
        tracing::trace!("IUnitInfo::getProgramListInfo");
        kInvalidArgument
    }

    unsafe fn getProgramName(&self, _list_id: ProgramListID, _program_index: int32, _name: *mut String128) -> tresult {
        tracing::trace!("IUnitInfo::getProgramName");
        kInvalidArgument
    }

    unsafe fn getProgramInfo(&self, _list_id: ProgramListID, _program_index: int32, _attribute_id: CString, _attribute_value: *mut String128) -> tresult {
        tracing::trace!("IUnitInfo::getProgramInfo");
        kInvalidArgument
    }

    unsafe fn hasProgramPitchNames(&self, _list_id: ProgramListID, _program_index: int32) -> tresult {
        tracing::trace!("IUnitInfo::hasProgramPitchNames");
        kInvalidArgument
    }

    unsafe fn getProgramPitchName(&self, _list_id: ProgramListID, _program_index: int32, _midi_pitch: int16, _name: *mut String128) -> tresult {
        tracing::trace!("IUnitInfo::getProgramPitchName");
        kInvalidArgument
    }

    unsafe fn getSelectedUnit(&self) -> UnitID {
        tracing::trace!("IUnitInfo::getSelectedUnit");
        0
    }

    unsafe fn selectUnit(&self, _unit_id: UnitID) -> tresult {
        tracing::trace!("IUnitInfo::selectUnit");
        kInvalidArgument
    }

    unsafe fn getUnitByBus(&self, _media_type: MediaType, _dir: BusDirection, _bus_index: int32, _channel: int32, _unit_id: *mut UnitID) -> tresult {
        tracing::trace!("IUnitInfo::getUnitByBus");
        kInvalidArgument
    }

    unsafe fn setUnitProgramData(&self, _list_or_unit_id: int32, _program_index: int32, _data: *mut IBStream) -> tresult {
        tracing::trace!("IUnitInfo::setUnitProgramData");
        kInvalidArgument
    }
}
