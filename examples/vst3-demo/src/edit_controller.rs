use vst3::Steinberg::{
    IPluginBaseTrait,
    Vst::{IEditController2, IEditController2Trait, IEditControllerTrait, ParameterInfo, ParamID, ParamValue, String128, TChar, IComponentHandler, KnobMode}, IBStream, tresult, int32, FIDString, IPlugView, TBool, FUnknown,
};

pub struct EditController;

impl vst3::Class for EditController {
    type Interfaces = (IEditController2,);
}

#[allow(non_snake_case)]
impl IEditControllerTrait for EditController {
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        todo!()
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        todo!()
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        todo!()
    }

    unsafe fn getParameterCount(&self) -> int32 {
        todo!()
    }

    unsafe fn getParameterInfo(&self, paramIndex: int32, info: *mut ParameterInfo) -> tresult {
        todo!()
    }

    unsafe fn getParamStringByValue(
        &self,
        id: ParamID,
        valueNormalized: ParamValue,
        string: *mut String128,
    ) -> tresult {
        todo!()
    }

    unsafe fn getParamValueByString(
        &self,
        id: ParamID,
        string: *mut TChar,
        valueNormalized: *mut ParamValue,
    ) -> tresult {
        todo!()
    }

    unsafe fn normalizedParamToPlain(
        &self,
        id: ParamID,
        valueNormalized: ParamValue,
    ) -> ParamValue {
        todo!()
    }

    unsafe fn plainParamToNormalized(&self, id: ParamID, plainValue: ParamValue) -> ParamValue {
        todo!()
    }

    unsafe fn getParamNormalized(&self, id: ParamID) -> ParamValue {
        todo!()
    }

    unsafe fn setParamNormalized(&self, id: ParamID, value: ParamValue) -> tresult {
        todo!()
    }

    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        todo!()
    }

    unsafe fn createView(&self, name: FIDString) -> *mut IPlugView {
        todo!()
    }
}

#[allow(non_snake_case)]
impl IEditController2Trait for EditController {
    unsafe fn setKnobMode(&self, mode: KnobMode) -> tresult {
        todo!()
    }

    unsafe fn openHelp(&self, onlyCheck: TBool) -> tresult {
        todo!()
    }

    unsafe fn openAboutBox(&self, onlyCheck: TBool) -> tresult {
        todo!()
    }
}

impl IPluginBaseTrait for EditController {
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        todo!()
    }

    unsafe fn terminate(&self) -> tresult {
        todo!()
    }
}
