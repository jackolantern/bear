pub type ErrorCode = u32;
pub type RegisterIndex = u8;
pub type RegisterValue = u16;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandTag {
    Reset = 0,
    Get = 1,
    Set = 2,
    Exec = 3
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamCommand {
    Read = 0,
    Write = 1,
    Seek = 2
}

/**
 * The `GenricDevice` interface is an optional interface that a device can implement.
 */

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericDeviceState {
    Error(ErrorCode),
    ReadyForCommand,
    Busy
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericDeviceCommand {
    Reset,
    Execute{ command: u8, argument: u8 },
    GetRegister(RegisterIndex),
    SetRegister(RegisterIndex, RegisterValue),
}

impl GenericDeviceCommand {
    pub fn get(index: u8) -> GenericDeviceCommand {
        GenericDeviceCommand::GetRegister(index)
    }

    pub fn set(index: u8, value: u16) -> GenericDeviceCommand {
        GenericDeviceCommand::SetRegister(index, value)
    }

    pub fn reset() -> GenericDeviceCommand {
        GenericDeviceCommand::Reset
    }

    pub fn decode(value: u32) -> Option<GenericDeviceCommand> {
        let command = ((value & 0xFF000000) >> 24) as u8;
        if command == CommandTag::Get as u8 {
            if (value & 0x0000FFFF) != 0 {
                return None;
            }
            let index = ((value & 0x00FF0000) >> 16) as u8;
            return Some(GenericDeviceCommand::GetRegister(index));
        } else if command == CommandTag::Set as u8 {
            let index = ((value & 0x00FF0000) >> 16) as u8;
            let value = (value & 0x0000FFFF) as u16;
            return Some(GenericDeviceCommand::SetRegister(index, value));
        } else if command == CommandTag::Exec as u8 {
            let command = ((value & 0x0000FF00) >> 8) as u8;
            let argument = (value & 0x000000FF) as u8;
            return Some(GenericDeviceCommand::Execute{ command, argument });
        } else if value == 0 {
            return Some(GenericDeviceCommand::Reset);
        } else {
            return None;
        }
    }

    pub fn encode(self) -> u32 {
        match self {
            GenericDeviceCommand::Reset => 0,
            GenericDeviceCommand::GetRegister(index) => (1 << 24) | ((index as u32) << 16),
            GenericDeviceCommand::SetRegister(index, value) => (2 << 24) | ((index as u32) << 16) | (value as u32),
            GenericDeviceCommand::Execute{ command, argument } => (3 << 24) | ((command as u32) << 8) | (argument as u32)
        }
    }
}

pub trait Device {
    fn ioctl(&mut self, message: u32) -> u32;
    fn dma_poll(&mut self) -> Option<DMARequest>;
    fn dma_write_response(&mut self, address: usize);
    fn dma_read_response(&mut self, address: usize, value: u32);
}

/**
 * DMA requests are only partially implemented.
 */
pub enum DMARequest {
    Read(usize),
    Write(usize, u32)
}

