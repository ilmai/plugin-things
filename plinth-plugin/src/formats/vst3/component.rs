use std::any::TypeId;
use std::cell::{OnceCell, RefCell};
use std::ffi::CStr;
use std::iter::zip;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use atomic_refcell::AtomicRefCell;
use plinth_core::signals::ptr_signal::{PtrSignal, PtrSignalMut};
use plinth_core::signals::signal::SignalMut;
use vst3::{ComPtr, ComRef};
use vst3::Steinberg::{int16, int32, kInvalidArgument, kNoInterface, kResultFalse, kResultOk, tresult, uint32, FIDString, FUnknown, IBStream, IPlugView, IPluginBaseTrait, TBool, TUID};
use vst3::Steinberg::Vst::{kInfiniteTail, kNoParentUnitId, kNoProgramListId, kNoTail, BusDirection, BusDirections_, BusInfo, BusInfo_::BusFlags_, BusTypes_, CString, IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler, IComponentTrait, IEditController, IEditController2, IEditController2Trait, IEditControllerTrait, IHostApplication, IHostApplicationTrait, IProcessContextRequirements, IProcessContextRequirementsTrait, IProcessContextRequirements_, IUnitInfo, IUnitInfoTrait, IoMode, IoModes_, KnobMode, MediaType, MediaTypes_, ParamID, ParamValue, ParameterInfo_, ProcessData, ProcessSetup, ProgramListID, ProgramListInfo, RoutingInfo, SpeakerArr, SpeakerArrangement, String128, SymbolicSampleSizes_, TChar, UnitID, UnitInfo, ViewType::kEditor};
use widestring::U16CStr;

use crate::{Event, Parameters, ProcessMode, ProcessState, Processor};
use crate::editor::NoEditor;
use crate::parameters::{group::{self, ParameterGroupRef}, has_duplicates, info::ParameterInfo};
use crate::processor::ProcessorConfig;
use crate::string::{char16_to_string, copy_str_to_char16};
use crate::vst3::{event::EventIterator, parameters::ParameterChangeIterator};

use super::{plugin::Vst3Plugin, stream::Stream, view::View};

const ROOT_UNIT_NAME: &str  = "Root";
const ROOT_UNIT_ID: i32     = 0;
const FIRST_UNIT_ID: i32    = 1;

pub struct AudioThreadState<P: Vst3Plugin> {
    processor: AtomicRefCell<Option<P::Processor>>,
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
    plugin: Rc<RefCell<P>>,

    parameter_info: Vec<ParameterInfo>,
    parameter_groups: Vec<ParameterGroupRef>,

    processor_config: RefCell<ProcessorConfig>,
    processing: AtomicBool,
    tail_length: AtomicU32,
    host_name: OnceCell<String>,
    component_handler: RefCell<Option<ComPtr<IComponentHandler>>>,

    audio_thread_state: AudioThreadState<P>,
}

impl<P: Vst3Plugin + 'static> PluginComponent<P> {
    pub fn new() -> Self {
        let plugin = P::default();
        assert!(plugin.with_parameters(|parameters| !has_duplicates(parameters.ids())));

        let mut parameter_info = Vec::new();

        // Create units based on parameter groups
        // Also verify parameters
        let groups = plugin.with_parameters(|parameters| {
            assert!(
                parameters.ids().iter()
                    .copied()
                    .filter(|&id| parameters.get(id).unwrap().info().is_bypass())
                    .count() <= 1,
                "You can only define one bypass parameter"
            );

            for &id in parameters.ids() {
                let info = parameters.get(id).unwrap().info();
                parameter_info.push(info.clone());
            }

            group::from_parameters(parameters)
        });
        
        Self {
            plugin: Rc::new(RefCell::new(plugin)),
            
            parameter_info,
            parameter_groups: groups,

            processor_config: Default::default(),
            processing: AtomicBool::new(false),
            tail_length: AtomicU32::new(0),
            host_name: Default::default(),
            component_handler: Default::default(),

            audio_thread_state: Default::default(),
        }
    }

    fn parameter_group_id(&self, parameter_info: &ParameterInfo) -> i32 {
        let parameter_path = parameter_info.path();
        if parameter_path.is_empty() {
            return ROOT_UNIT_ID;
        }

        let unit_index = self.parameter_groups.iter().position(|group| group.path == parameter_path).unwrap() as i32;
        FIRST_UNIT_ID + unit_index
    }
}

impl<P: Vst3Plugin> vst3::Class for PluginComponent<P> {
    type Interfaces = (IAudioProcessor, IComponent, IComponent, IEditController, IEditController2, IProcessContextRequirements, IUnitInfo);
}

impl<P: Vst3Plugin> IPluginBaseTrait for PluginComponent<P> {
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        log::trace!("IPluginBase::initialize");

        if let Some(context) = unsafe { ComRef::from_raw(context) } {
            if let Some(host_application) = context.cast::<IHostApplication>() {
                let mut name = [0; 128];
                if unsafe { host_application.getName(&mut name) == kResultOk } {
                    if let Some(name) = char16_to_string(&name) {
                        self.host_name.set(name).unwrap();
                    }
                }
            }
        }

        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        log::trace!("IPluginBase::terminate");
        kResultOk
    }
}

impl<P: Vst3Plugin> IAudioProcessorTrait for PluginComponent<P> {
    unsafe fn setBusArrangements(&self, inputs: *mut SpeakerArrangement, num_ins: int32, outputs: *mut SpeakerArrangement, num_outs: int32) -> tresult {
        log::trace!("IAudioProcessor::setBusArrangements");

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
        log::trace!("IAudioProcessor::getBusArrangements");

        // Only support stereo
        unsafe { *arr = SpeakerArr::kStereo; }
        kResultOk
    }

    unsafe fn canProcessSampleSize(&self, symbolic_sample_size: int32) -> tresult {
        log::trace!("IAudioProcessor::canProcessSampleSize");

        if symbolic_sample_size == SymbolicSampleSizes_::kSample32 as int32 {
            kResultOk
        } else {
            kResultFalse
        }
    }

    unsafe fn getLatencySamples(&self) -> uint32 {
        log::trace!("IAudioProcessor::getLatencySamples");
        self.plugin.borrow().latency() as _
    }

    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        log::trace!("IAudioProcessor::setupProcessing");

        let setup = unsafe { &*setup };
        assert!(setup.maxSamplesPerBlock > 0);

        let mut processor_config = self.processor_config.borrow_mut();
        processor_config.sample_rate = setup.sampleRate;
        processor_config.max_block_size = setup.maxSamplesPerBlock as usize;

        kResultOk
    }

    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        log::trace!("IAudioProcessor::setProcessing");

        let processing = state != 0;
        self.processing.store(processing, Ordering::Release);

        let mut processor = self.audio_thread_state.processor.borrow_mut();
        if let Some(processor) = processor.as_mut() {
            if !processing {
                processor.reset();
            }
        }

        kResultOk
    }

    // Called from the audio thread
    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        if !self.processing.load(Ordering::Acquire) {
            // KLUDGE: Ableton Live can call process() while plugin isn't active
            return kResultFalse;
        }        

        let data = unsafe { &mut *data };

        let parameter_change_iterator = ParameterChangeIterator::new(data.inputParameterChanges);
        let event_iterator = EventIterator::new(data.inputEvents);
        let all_events = event_iterator.chain(parameter_change_iterator);

        let mut processor = self.audio_thread_state.processor.borrow_mut();
        let Some(processor) = processor.as_mut() else {
            return kResultFalse;
        };

        let aux_active = self.audio_thread_state.aux_active.load(Ordering::Acquire);

        // Empty input: this is a parameter dump
        if data.inputs.is_null() || data.outputs.is_null() || data.numInputs == 0 || data.numSamples == 0 {
            processor.process_events(all_events);
            return kResultOk;
        }

        // On some platforms, this cast is needed
        #[allow(clippy::unnecessary_cast)]
        if data.symbolicSampleSize != SymbolicSampleSizes_::kSample32 as i32 {
            return kResultFalse;
        }

        let inputs = unsafe { std::slice::from_raw_parts(data.inputs, data.numInputs as _) };
        let outputs = unsafe { std::slice::from_raw_parts(data.outputs, data.numOutputs as _) };
        let main_input = inputs[0];
        let main_output = outputs[0];
        assert_eq!(main_input.numChannels, main_output.numChannels);

        let aux_input = if P::HAS_AUX_INPUT && aux_active {
            assert_eq!(data.numInputs, 2);
            let aux_input = inputs[1];
            Some(unsafe { PtrSignal::from_pointers(aux_input.numChannels as usize, data.numSamples as usize, aux_input.__field0.channelBuffers32 as _) })
        } else {
            None
        };

        let main_input = unsafe { PtrSignal::from_pointers(main_input.numChannels as usize, data.numSamples as usize, main_input.__field0.channelBuffers32 as _) };
        let mut main_output = unsafe { PtrSignalMut::from_pointers(main_output.numChannels as usize, data.numSamples as usize, main_output.__field0.channelBuffers32) };

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
                println!("Processing error!");
                return kResultFalse;
            },

            ProcessState::Normal => kNoTail,
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
        log::trace!("IComponent::getControllerClassId");
        kNoInterface
    }

    unsafe fn setIoMode(&self, mode: IoMode) -> tresult {
        log::trace!("IComponent::setIoMode");

        let mode = match mode as _ {
            IoModes_::kSimple | IoModes_::kAdvanced => ProcessMode::Realtime,
            IoModes_::kOfflineProcessing => ProcessMode::Offline,
            _ => {
                return kInvalidArgument;
            }
        };

        self.processor_config.borrow_mut().process_mode = mode;

        kResultOk
    }

    unsafe fn getBusCount(&self, media_type: MediaType, dir: BusDirection) -> int32 {
        log::trace!("IComponent::getBusCount");

        // On some platforms, these casts are needed
        #[allow(clippy::unnecessary_cast)]
        if P::HAS_AUX_INPUT && media_type == MediaTypes_::kAudio as i32 && dir == BusDirections_::kInput as i32 {
            2
        } else {
            1
        }
    }

    unsafe fn getBusInfo(&self, media_type: MediaType, dir: BusDirection, index: int32, bus: *mut BusInfo) -> tresult {
        log::trace!("IComponent::getBusInfo");

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
        log::trace!("IComponent::getRoutingInfo");

        let in_info = unsafe { &*in_info };
        let out_info = unsafe { &mut *out_info };
        
        out_info.mediaType = in_info.mediaType;
        out_info.busIndex = in_info.busIndex;
        out_info.channel = in_info.channel;

        kResultOk
    }

    unsafe fn activateBus(&self, media_type: MediaType, dir: BusDirection, index: int32, state: TBool) -> tresult {
        log::trace!("IComponent::activateBus");

        // On some platforms, these casts are needed
        #[allow(clippy::unnecessary_cast)]
        if P::HAS_AUX_INPUT && media_type == MediaTypes_::kAudio as i32 && dir == BusDirections_::kInput as i32 && index == 1 {
            self.audio_thread_state.aux_active.store(state != 0, Ordering::Release);
        }

        // TODO: Support disabling other buses
        kResultOk
    }

    unsafe fn setActive(&self, state: TBool) -> tresult {
        log::trace!("IComponent::setActive");

        let active = state > 0;

        if self.processing.load(Ordering::Acquire) && !active {
            // KLUDGE: Ableton Live calls setActive(0) without calling setProcessing(0) first
            unsafe { self.setProcessing(0) };
        }

        let mut processor = self.audio_thread_state.processor.borrow_mut();

        if active {
            let mut plugin = self.plugin.borrow_mut();
            *processor = Some(plugin.create_processor(&self.processor_config.borrow()));
        } else {
            *processor = None;
        }

        kResultOk
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        log::trace!("IComponent::setState");

        let mut plugin = self.plugin.borrow_mut();
        let Some(mut stream) = Stream::new(state) else {
            return kInvalidArgument;
        };

        match plugin.load_state(&mut stream) {
            Ok(_) => kResultOk,
            Err(_) => kInvalidArgument, // TODO: Extract actual error code
        }
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        log::trace!("IComponent::getState");

        let plugin = self.plugin.borrow();
        let Some(mut stream) = Stream::new(state) else {
            return kInvalidArgument;
        };

        match plugin.save_state(&mut stream) {
            Ok(_) => kResultOk,
            Err(_) => kInvalidArgument, // TODO: Extract actual error code
        }
    }
}

impl<P: Vst3Plugin + 'static> IEditControllerTrait for PluginComponent<P> {
    unsafe fn setComponentState(&self, _state: *mut IBStream) -> tresult {
        log::trace!("IEditController::setComponentState");
        kResultOk
    }

    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        log::trace!("IEditController::setState");
        kResultOk
    }

    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        log::trace!("IEditController::getState");
        kResultOk
    }

    unsafe fn getParameterCount(&self) -> int32 {
        log::trace!("IEditController::getParameterCount");
        self.plugin.borrow().with_parameters(|parameters| parameters.ids().len() as _)
    }

    unsafe fn getParameterInfo(&self, param_index: int32, info: *mut vst3::Steinberg::Vst::ParameterInfo) -> tresult {
        log::trace!("IEditController::getParameterInfo");

        if param_index < 0 {
            return kInvalidArgument;
        }

        let Some(parameter_info) = self.parameter_info.get(param_index as usize) else {
            return kInvalidArgument;
        };

        let vst3_info = unsafe { &mut *info };

        vst3_info.id = parameter_info.id();
        copy_str_to_char16(parameter_info.name(), &mut vst3_info.title);
        // TODO: info.shortTitle
        vst3_info.stepCount = parameter_info.steps() as _;
        vst3_info.defaultNormalizedValue = parameter_info.default_normalized_value();
        vst3_info.unitId = self.parameter_group_id(parameter_info);

        // On some platforms, this cast is needed
        #[allow(clippy::unnecessary_cast)]
        {
            vst3_info.flags = ParameterInfo_::ParameterFlags_::kCanAutomate as i32;
        }

        // On some platforms, this cast is needed
        #[allow(clippy::unnecessary_cast)]
        if parameter_info.is_bypass() {
            vst3_info.flags |= ParameterInfo_::ParameterFlags_::kIsBypass as i32;
        }

        kResultOk
    }

    unsafe fn getParamStringByValue(&self, id: ParamID, value_normalized: ParamValue, string: *mut String128) -> tresult {
        log::trace!("IEditController::getParamStringByValue");

        self.plugin.borrow().with_parameters(|parameters| {
            let Some(parameter) = parameters.get(id) else {
                return kInvalidArgument;
            };

            let formatted = parameter.normalized_to_string(value_normalized);
            copy_str_to_char16(&formatted, unsafe { &mut *string });
    
            kResultOk    
        })
    }

    unsafe fn getParamValueByString(&self, id: ParamID, string: *mut TChar, value_normalized: *mut ParamValue) -> tresult {
        log::trace!("IEditController::getParamValueByString");

        if string.is_null() {
            return kInvalidArgument;
        }

        let string = unsafe { U16CStr::from_ptr_str(string as _) };
        let Ok(string) = string.to_string() else {
            return kInvalidArgument;
        };

        self.plugin.borrow().with_parameters(|parameters| {
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
        let plugin = self.plugin.borrow();
        plugin.with_parameters(|parameters| {
            let Some(parameter) = parameters.get(id) else {
                return 0.0;
            };

            parameter.normalized_value()
        })
    }

    unsafe fn setParamNormalized(&self, id: ParamID, normalized: ParamValue) -> tresult {
        let mut plugin = self.plugin.borrow_mut();
        plugin.process_event(&Event::ParameterValue {
            sample_offset: 0,
            id,
            value: normalized,
        });

        kResultOk
    }

    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        log::trace!("IEditController::setComponentHandler");

        let Some(handler) = (unsafe { ComRef::from_raw(handler) }) else {
            return kInvalidArgument;
        };
        
        *self.component_handler.borrow_mut() = Some(handler.to_com_ptr());

        kResultOk
    }

    unsafe fn createView(&self, name: FIDString) -> *mut IPlugView {
        log::trace!("IEditController::createView");

        if name.is_null() {
            return null_mut();
        }

        if unsafe { CStr::from_ptr(name) != CStr::from_ptr(kEditor) } {
            return null_mut();
        }

        if TypeId::of::<P::Editor>() == TypeId::of::<NoEditor>() {
            return null_mut();
        }

        let Some(component_handler) = self.component_handler.borrow().clone() else {
            return null_mut();
        };

        let view = View::<P>::new(
            self.plugin.clone(),
            self.host_name.get().cloned(),
            component_handler,
        );

        view.to_com_ptr::<IPlugView>().unwrap().into_raw()
    }
}

impl<P: Vst3Plugin> IEditController2Trait for PluginComponent<P> {
    unsafe fn setKnobMode(&self, _mode: KnobMode) -> tresult {
        log::trace!("IEditController2::setKnobMode");
        kResultFalse
    }

    unsafe fn openHelp(&self, _only_check: TBool) -> tresult {
        log::trace!("IEditController2::openHelp");
        kResultFalse
    }

    unsafe fn openAboutBox(&self, _only_check: TBool) -> tresult {
        log::trace!("IEditController2::openAboutBox");
        kResultFalse
    }
}

impl<P: Vst3Plugin> IProcessContextRequirementsTrait for PluginComponent<P> {
    unsafe fn getProcessContextRequirements(&self) -> uint32 {
        log::trace!("IProcessContextRequirements::getProcessContextRequirements");
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
        log::trace!("IUnitInfo::getUnitCount");
        self.parameter_groups.len() as int32 + 1 // +1 for the root unit
    }

    unsafe fn getUnitInfo(&self, unit_index: int32, info: *mut UnitInfo) -> tresult {
        log::trace!("IUnitInfo::getUnitInfo");

        let unit_count = self.parameter_groups.len() + 1; // +1 for the root unit

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
            let group = &self.parameter_groups[unit_index as usize];
            copy_str_to_char16(&group.name, &mut info.name);

            if let Some(parent) = &group.parent {
                info.parentUnitId = FIRST_UNIT_ID + self.parameter_groups.iter().position(|group| group == parent).unwrap() as i32;
            } else {
                info.parentUnitId = ROOT_UNIT_ID;
            }
        }

        kResultOk
    }

    unsafe fn getProgramListCount(&self) -> int32 {
        log::trace!("IUnitInfo::getProgramListCount");
        0
    }

    unsafe fn getProgramListInfo(&self, _list_index: int32, _info: *mut ProgramListInfo) -> tresult {
        log::trace!("IUnitInfo::getProgramListInfo");
        kInvalidArgument
    }

    unsafe fn getProgramName(&self, _list_id: ProgramListID, _program_index: int32, _name: *mut String128) -> tresult {
        log::trace!("IUnitInfo::getProgramName");
        kInvalidArgument
    }

    unsafe fn getProgramInfo(&self, _list_id: ProgramListID, _program_index: int32, _attribute_id: CString, _attribute_value: *mut String128) -> tresult {
        log::trace!("IUnitInfo::getProgramInfo");
        kInvalidArgument
    }

    unsafe fn hasProgramPitchNames(&self, _list_id: ProgramListID, _program_index: int32) -> tresult {
        log::trace!("IUnitInfo::hasProgramPitchNames");
        kInvalidArgument
    }

    unsafe fn getProgramPitchName(&self, _list_id: ProgramListID, _program_index: int32, _midi_pitch: int16, _name: *mut String128) -> tresult {
        log::trace!("IUnitInfo::getProgramPitchName");
        kInvalidArgument
    }

    unsafe fn getSelectedUnit(&self) -> UnitID {
        log::trace!("IUnitInfo::getSelectedUnit");
        0
    }

    unsafe fn selectUnit(&self, _unit_id: UnitID) -> tresult {
        log::trace!("IUnitInfo::selectUnit");
        kInvalidArgument
    }

    unsafe fn getUnitByBus(&self, _media_type: MediaType, _dir: BusDirection, _bus_index: int32, _channel: int32, _unit_id: *mut UnitID) -> tresult {
        log::trace!("IUnitInfo::getUnitByBus");
        kInvalidArgument
    }

    unsafe fn setUnitProgramData(&self, _list_or_unit_id: int32, _program_index: int32, _data: *mut IBStream) -> tresult {
        log::trace!("IUnitInfo::setUnitProgramData");
        kInvalidArgument
    }
}
