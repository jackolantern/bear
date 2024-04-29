use std::io::{Read, Write};

use bear_vm::device;

#[derive(Debug, Clone)]
pub struct Register {
    value: Option<u32>,
    can_read: bool,
    can_write: bool,
}

#[derive(Debug, Clone)]
pub struct StdinDevice<T: Read> {
    state: device::GenericDeviceState,
    registers: [Register; 0],
    handle: T,
}

#[derive(Debug, Clone)]
pub struct StdoutDevice<T: Write> {
    state: device::GenericDeviceState,
    registers: [Register; 0],
    handle: T,
}

impl<T: Read> StdinDevice<T> {
    pub fn new(handle: T) -> StdinDevice<T> {
        StdinDevice {
            handle,
            state: device::GenericDeviceState::ReadyForCommand,
            registers: [],
        }
    }

    pub fn reset(&mut self) {
        self.state = device::GenericDeviceState::ReadyForCommand;
        for reg in &mut self.registers {
            reg.value = None;
        }
    }
}

impl<T: Read> device::Device for StdinDevice<T> {
    fn ioctl(&mut self, command: u32) -> u32 {
        let command = device::GenericDeviceCommand::decode(command);
        match self.state {
            device::GenericDeviceState::ReadyForCommand => match command {
                Some(device::GenericDeviceCommand::Reset) => {
                    self.reset();
                    0
                }
                Some(device::GenericDeviceCommand::GetRegister(index)) => {
                    if (index as usize) < self.registers.len() {
                        let reg = &self.registers[index as usize];
                        if reg.can_read {
                            return reg.value.unwrap_or(u32::MAX);
                        }
                    }
                    u32::MAX
                }
                Some(device::GenericDeviceCommand::SetRegister(index, value)) => {
                    if (index as usize) < self.registers.len() {
                        let reg = &mut self.registers[index as usize];
                        if reg.can_write {
                            reg.value = Some(value as u32);
                            return 0;
                        }
                    }
                    u32::MAX
                }
                Some(device::GenericDeviceCommand::Execute {
                    command,
                    argument: _,
                }) => {
                    if command == device::StreamCommand::Seek as u8 {
                        u32::MAX
                    } else if command == device::StreamCommand::Write as u8 {
                        u32::MAX
                    } else if command == device::StreamCommand::Read as u8 {
                        let mut buffer = vec![0u8];
                        match self.handle.read(&mut buffer) {
                            Ok(n) => {
                                if n == 0 {
                                    u32::MAX
                                } else {
                                    assert_eq!(n, 1);
                                    buffer[0] as u32
                                }
                            }
                            Err(_) => u32::MAX,
                        }
                    } else {
                        u32::MAX
                    }
                }
                None => u32::MAX,
            },
            device::GenericDeviceState::Error(_code) => u32::MAX,
            device::GenericDeviceState::Busy => u32::MAX,
        }
    }

    fn dma_poll(&mut self) -> Option<device::DMARequest> {
        None
    }

    fn dma_read_response(&mut self, _address: usize, _value: u32) {}

    fn dma_write_response(&mut self, _address: usize) {}
}

impl<T: Write> StdoutDevice<T> {
    pub fn new(handle: T) -> StdoutDevice<T> {
        StdoutDevice {
            handle,
            state: device::GenericDeviceState::ReadyForCommand,
            registers: [],
        }
    }

    pub fn reset(&mut self) {
        self.state = device::GenericDeviceState::ReadyForCommand;
        for reg in &mut self.registers {
            reg.value = None;
        }
    }
}

impl<T: Write> device::Device for StdoutDevice<T> {
    fn ioctl(&mut self, command: u32) -> u32 {
        let command = device::GenericDeviceCommand::decode(command);
        match self.state {
            device::GenericDeviceState::ReadyForCommand => match command {
                Some(device::GenericDeviceCommand::Reset) => {
                    self.reset();
                    0
                }
                Some(device::GenericDeviceCommand::GetRegister(index)) => {
                    if (index as usize) < self.registers.len() {
                        let reg = &self.registers[index as usize];
                        if reg.can_read {
                            return reg.value.unwrap_or(u32::MAX);
                        }
                    }
                    u32::MAX
                }
                Some(device::GenericDeviceCommand::SetRegister(index, value)) => {
                    if (index as usize) < self.registers.len() {
                        let reg = &mut self.registers[index as usize];
                        if reg.can_write {
                            reg.value = Some(value as u32);
                            return 0;
                        }
                    }
                    u32::MAX
                }
                Some(device::GenericDeviceCommand::Execute { command, argument }) => {
                    if command == device::StreamCommand::Seek as u8 {
                        u32::MAX
                    } else if command == device::StreamCommand::Read as u8 {
                        u32::MAX
                    } else if command == device::StreamCommand::Write as u8 {
                        let buffer = vec![argument];
                        match self.handle.write(&buffer) {
                            Ok(_) => 0_u32,
                            Err(_) => u32::MAX,
                        }
                    } else {
                        u32::MAX
                    }
                }
                None => u32::MAX,
            },
            device::GenericDeviceState::Error(_code) => u32::MAX,
            device::GenericDeviceState::Busy => u32::MAX,
        }
    }

    fn dma_poll(&mut self) -> Option<device::DMARequest> {
        None
    }

    fn dma_write_response(&mut self, _address: usize) {}

    fn dma_read_response(&mut self, _address: usize, _value: u32) {}
}
