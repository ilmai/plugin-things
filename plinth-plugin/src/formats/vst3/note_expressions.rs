use vst3::Steinberg::Vst::NoteExpressionTypeIDs_::{kBrightnessTypeID, kExpressionTypeID, kInvalidTypeID, kPanTypeID, kTuningTypeID, kVibratoTypeID, kVolumeTypeID};
use vst3::Steinberg::Vst::PhysicalUITypeIDs_::{kPUIPressure, kPUIXMovement, kPUIYMovement};
use vst3::Steinberg::Vst::{INoteExpressionControllerTrait, INoteExpressionPhysicalUIMappingTrait, NoteExpressionTypeID, NoteExpressionTypeInfo, NoteExpressionValue, PhysicalUIMapList, PhysicalUITypeIDs, String128, TChar};
use vst3::Steinberg::{int16, int32, kInvalidArgument, kResultFalse, kResultOk, tresult};
use widestring::U16CStr;

use crate::NoteExpressions;
use crate::string::copy_str_to_char16;
use crate::midi_capabilities::MIDI_CHANNEL_COUNT;

use super::{component::PluginComponent, plugin::Vst3Plugin};

/// Same range as CLAP_NOTE_EXPRESSION_TUNING. See also [Event::PolyTuning]'s documented range.
const TUNING_RANGE_SEMITONES: f64 = 120.0;

/// Same range as CLAP_NOTE_EXPRESSION_VOLUME. See also [Event::PolyVolume]'s documented range.
const VOLUME_RANGE_GAIN: f64 = 4.0;

/// A description of a standard VST3 note-expression, along with the conversions between its
/// normalized value and the string the host displays.
pub(super) struct NoteExpressionDescriptor {
    pub type_id: NoteExpressionTypeID,
    pub title: &'static str,
    pub short_title: &'static str,
    pub units: &'static str,
    pub default_value: NoteExpressionValue,
    pub is_enabled: fn(&NoteExpressions) -> bool,
    // Formats a normalized [0, 1] value for display.
    pub format: fn(NoteExpressionValue) -> String,
    // Parses a displayed value back into normalized [0, 1].
    pub parse: fn(&str) -> Option<NoteExpressionValue>,
}

// Standard VST3 note-expression types. Only the enabled subset (per `NoteExpressions` config)
// is exposed to the host.
//
// Pressure is excluded from the descriptors. It is delivered as `kPolyPressureEvent` in VST3.
//
// This set matches CLAPs default set of note expressions, as we currently don't allow registering
// VST3 note expressions dynamically and want to simplify cross plugin format compatibility.
static NOTE_EXPRESSION_DESCRIPTORS: [NoteExpressionDescriptor; 6] = [
    NoteExpressionDescriptor {
        type_id: kVolumeTypeID,
        title: "Volume",
        short_title: "Volume",
        units: "dB",
        default_value: 0.25, // 0 dB
        is_enabled: NoteExpressions::volume,
        format: |value| format!("{:.2} dB", NoteExpressionDescriptor::normalized_to_db(value)),
        parse: |string| NoteExpressionDescriptor::parse_number(string).map(NoteExpressionDescriptor::db_to_normalized),
    },
    NoteExpressionDescriptor {
        type_id: kPanTypeID,
        title: "Panning",
        short_title: "Panning",
        units: "%",
        default_value: 0.5, // center
        is_enabled: NoteExpressions::pan,
        format: |value| format!("{:.1} %", NoteExpressionDescriptor::normalized_to_pan(value) * 100.0),
        parse: |string| NoteExpressionDescriptor::parse_number(string).map(|percent| NoteExpressionDescriptor::pan_to_normalized(percent / 100.0)),
    },
    NoteExpressionDescriptor {
        type_id: kTuningTypeID,
        title: "Tuning",
        short_title: "Tuning",
        units: "st",
        default_value: 0.5, // center
        is_enabled: NoteExpressions::tuning,
        format: |value| format!("{:+.2} st", NoteExpressionDescriptor::normalized_to_semitones(value)),
        parse: |string| NoteExpressionDescriptor::parse_number(string).map(NoteExpressionDescriptor::semitones_to_normalized),
    },
    NoteExpressionDescriptor {
        type_id: kVibratoTypeID,
        title: "Vibrato",
        short_title: "Vibrato",
        units: "",
        default_value: 0.0,
        is_enabled: NoteExpressions::vibrato,
        format: NoteExpressionDescriptor::format_normalized,
        parse: NoteExpressionDescriptor::parse_normalized,
    },
    NoteExpressionDescriptor {
        type_id: kExpressionTypeID,
        title: "Expression",
        short_title: "Expression",
        units: "",
        default_value: 0.0,
        is_enabled: NoteExpressions::expression,
        format: NoteExpressionDescriptor::format_normalized,
        parse: NoteExpressionDescriptor::parse_normalized,
    },
    NoteExpressionDescriptor {
        type_id: kBrightnessTypeID,
        title: "Brightness",
        short_title: "Brightness",
        units: "",
        default_value: 0.0,
        is_enabled: NoteExpressions::brightness,
        format: NoteExpressionDescriptor::format_normalized,
        parse: NoteExpressionDescriptor::parse_normalized,
    },
];

impl NoteExpressionDescriptor {
    /// Lookup a descriptor for the given `type_id`. The plugin needs that expression enabled.
    pub fn find(
        note_expressions: NoteExpressions,
        type_id: NoteExpressionTypeID,
    ) -> Option<&'static Self> {
        Self::enabled_expressions(note_expressions).find(|descriptor| descriptor.type_id == type_id)
    }

    /// The enabled expressions, in the order the host sees them.
    pub fn enabled_expressions(
        note_expressions: NoteExpressions,
    ) -> impl Iterator<Item = &'static Self> {
        NOTE_EXPRESSION_DESCRIPTORS
            .iter()
            .filter(move |descriptor| (descriptor.is_enabled)(&note_expressions))
    }

    /// Maps a normalized [0, 1] volume value to a linear gain in [0, 4], where 1 is 0 dB.
    pub fn normalized_to_gain(value: NoteExpressionValue) -> f64 {
        value * VOLUME_RANGE_GAIN
    }

    /// Maps a normalized [0, 1] volume value to dB. Returns -inf for silence.
    pub fn normalized_to_db(value: NoteExpressionValue) -> f64 {
        20.0 * Self::normalized_to_gain(value).log10()
    }

    /// Maps dB back to a normalized [0, 1] volume value.
    pub fn db_to_normalized(db: f64) -> NoteExpressionValue {
        (10f64.powf(db / 20.0) / VOLUME_RANGE_GAIN).clamp(0.0, 1.0)
    }

    /// Maps a normalized [0, 1] tuning value to a +- semitones value.
    pub fn normalized_to_semitones(value: NoteExpressionValue) -> f64 {
        value * (2.0 * TUNING_RANGE_SEMITONES) - TUNING_RANGE_SEMITONES
    }

    /// Maps semitones back to a normalized [0, 1] tuning value.
    pub fn semitones_to_normalized(semitones: f64) -> NoteExpressionValue {
        ((semitones + TUNING_RANGE_SEMITONES) / (2.0 * TUNING_RANGE_SEMITONES)).clamp(0.0, 1.0)
    }

    /// Maps a normalized [0, 1] panning value to [-1, 1], where 0 is center.
    pub fn normalized_to_pan(value: NoteExpressionValue) -> f64 {
        value * 2.0 - 1.0
    }

    /// Maps a [-1, 1] panning value back to a normalized [0, 1] value.
    pub fn pan_to_normalized(pan: f64) -> NoteExpressionValue {
        ((pan + 1.0) / 2.0).clamp(0.0, 1.0)
    }

    /// Fills the host's note expression type info from this descriptor.
    fn fill_type_info(&self, info: &mut NoteExpressionTypeInfo) {
        info.typeId = self.type_id;
        info.unitId = -1;
        info.associatedParameterId = u32::MAX;
        info.flags = 0;
        info.valueDesc.minimum = 0.0;
        info.valueDesc.maximum = 1.0;
        info.valueDesc.stepCount = 0;
        info.valueDesc.defaultValue = self.default_value;
        copy_str_to_char16(self.title, &mut info.title);
        copy_str_to_char16(self.short_title, &mut info.shortTitle);
        copy_str_to_char16(self.units, &mut info.units);
    }

    /// Parses a string as raw number value, stripping all non numeric characters.
    fn parse_number(string: &str) -> Option<f64> {
        let digits: String = string.chars().filter(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+')).collect();
        digits.parse::<f64>().ok()
    }

    /// Formats a plain normalized [0, 1] value.
    fn format_normalized(value: NoteExpressionValue) -> String {
        format!("{:.2}", value)
    }

    /// Parses a plain normalized value, clamped to [0, 1].
    fn parse_normalized(string: &str) -> Option<NoteExpressionValue> {
        Self::parse_number(string).map(|value| value.clamp(0.0, 1.0))
    }
}

impl<P: Vst3Plugin + 'static> INoteExpressionControllerTrait for PluginComponent<P> {
    unsafe fn getNoteExpressionCount(&self, bus_index: int32, channel: int16) -> int32 {
        tracing::trace!("INoteExpressionController::getNoteExpressionCount");

        if !P::HAS_NOTE_INPUT || bus_index != 0 || !(0..MIDI_CHANNEL_COUNT as i16).contains(&channel) {
            return 0;
        }

        NoteExpressionDescriptor::enabled_expressions(P::NOTE_EXPRESSIONS).count() as i32
    }

    unsafe fn getNoteExpressionInfo(&self, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> tresult {
        tracing::trace!("INoteExpressionController::getNoteExpressionInfo");

        if !P::HAS_NOTE_INPUT || bus_index != 0 || !(0..MIDI_CHANNEL_COUNT as i16).contains(&channel) || info.is_null() {
            return kInvalidArgument;
        }

        let Ok(index) = usize::try_from(note_expression_index) else {
            return kInvalidArgument;
        };

        let Some(descriptor) = NoteExpressionDescriptor::enabled_expressions(P::NOTE_EXPRESSIONS).nth(index) else {
            return kInvalidArgument;
        };

        descriptor.fill_type_info(unsafe { &mut *info });
        kResultOk
    }

    unsafe fn getNoteExpressionStringByValue(&self, bus_index: int32, channel: int16, id: NoteExpressionTypeID, value_normalized: NoteExpressionValue, string: *mut String128) -> tresult {
        tracing::trace!("INoteExpressionController::getNoteExpressionStringByValue");

        if !P::HAS_NOTE_INPUT || bus_index != 0 || !(0..MIDI_CHANNEL_COUNT as i16).contains(&channel) || string.is_null() {
            return kInvalidArgument;
        }

        // Only expressions that we advertised to the host should resolve here
        let Some(descriptor) = NoteExpressionDescriptor::find(P::NOTE_EXPRESSIONS, id) else {
            return kInvalidArgument;
        };

        copy_str_to_char16(&(descriptor.format)(value_normalized), unsafe { &mut *string });
        kResultOk
    }

    unsafe fn getNoteExpressionValueByString(&self, bus_index: int32, channel: int16, id: NoteExpressionTypeID, string: *const TChar, value_normalized: *mut NoteExpressionValue) -> tresult {
        tracing::trace!("INoteExpressionController::getNoteExpressionValueByString");

        if !P::HAS_NOTE_INPUT || bus_index != 0 || !(0..MIDI_CHANNEL_COUNT as i16).contains(&channel) || string.is_null() || value_normalized.is_null() {
            return kInvalidArgument;
        }

        // Only expressions that we advertised to the host should resolve here
        let Some(descriptor) = NoteExpressionDescriptor::find(P::NOTE_EXPRESSIONS, id) else {
            return kInvalidArgument;
        };

        let string = unsafe { U16CStr::from_ptr_str(string as _) };
        let Ok(string) = string.to_string() else {
            return kInvalidArgument;
        };

        let Some(value) = (descriptor.parse)(&string) else {
            return kResultFalse;
        };

        unsafe { *value_normalized = value };
        kResultOk
    }
}

impl<P: Vst3Plugin> INoteExpressionPhysicalUIMappingTrait for PluginComponent<P> {
    unsafe fn getPhysicalUIMapping(&self, bus_index: int32, channel: int16, list: *mut PhysicalUIMapList) -> tresult {
        tracing::trace!("INoteExpressionPhysicalUIMapping::getPhysicalUIMapping");

        if !P::HAS_NOTE_INPUT || bus_index != 0 || !(0..MIDI_CHANNEL_COUNT as i16).contains(&channel) || list.is_null() {
            return kInvalidArgument;
        }

        let list = unsafe { &mut *list };

        for i in 0..list.count as usize {
            let entry = unsafe { &mut *list.map.add(i) };
            #[allow(non_upper_case_globals)]
            let ne_type = if entry.physicalUITypeID as PhysicalUITypeIDs == kPUIXMovement {
                // Horizontal (slide left/right) -> per-note pitch / tuning
                if P::NOTE_EXPRESSIONS.tuning() {
                    kTuningTypeID
                } else {
                    kInvalidTypeID
                }
            } else if entry.physicalUITypeID as PhysicalUITypeIDs == kPUIYMovement {
                // Vertical (slide up/down) -> brightness / timbre
                if P::NOTE_EXPRESSIONS.brightness() {
                    kBrightnessTypeID
                } else {
                    kInvalidTypeID
                }
            } else if entry.physicalUITypeID as PhysicalUITypeIDs == kPUIPressure {
                // Pressure (Z-axis) -> delivered as kPolyPressureEvent, not note expression
                kInvalidTypeID
            } else {
                kInvalidTypeID
            };
            entry.noteExpressionTypeID = ne_type;
        }

        kResultOk
    }
}
