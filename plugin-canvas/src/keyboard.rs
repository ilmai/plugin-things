use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct KeyboardModifiers: u32 {
        const Alt       = 0b0001;
        const Control   = 0b0010;
        const Meta      = 0b0100;
        const Shift     = 0b1000;
    }
}
