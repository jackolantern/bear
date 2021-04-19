use std::mem::transmute_copy;
// use std::convert::TryInto;
use std::convert::TryFrom;

use crate::cell;
use crate::device::{Device, DMARequest};
pub use crate::cell::Cell;

// TODO: Traps and Trap Handlers.

/**
 * Runtime errors.
 */
#[derive(Debug)]
pub struct Error {
    ip: Option<usize>,
    message: String
}

impl Error {
    fn data_underflow() -> Error {
        Error{ message: String::from("Data stack underflow."), ip: None }
    }

    fn address_underflow() -> Error {
        Error{ message: String::from("Address stack underflow."), ip: None }
    }

    fn ip_oob(ip: usize) -> Error {
        Error{ message: String::from("IP went out of bounds."), ip: Some(ip) }
    }

    fn invalid_instruction(byte: u8) -> Error {
        Error{ message: format!("Invalid opcode: 0x{:x}", byte), ip: None }
    }

    fn with_ip(mut self, ip: usize) -> Self {
        self.ip = Some(ip);
        return self;
    }

    fn with_ip_from_state(mut self, state: &ExecutionState) -> Self {
        self.ip = Some(state.loaded_word_index * 4 + state.instruction_index);
        return self;
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Error{ message: format!("Arithmetic error: '{}'.", e), ip: None }
    }
}

// WARN: If this enum changes, make sure to update `impl TryFrom<u8> for OpCode`.
/**
 * The opCodes recognized by the VM.
 */
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum OpCode {
    /// Do nothing.
    Nop,

    /// Push the next cell in memory onto the data stack.
    Lit,

    /// Duplicate the top of the data stack.
    Dup,
    /// Drop the value on the top of the data stack.
    Drop,
    /// Swap the values on the top of the data stack.
    Swap,
    /// Remove the value on the top of the data stack and push it onto the top of the address
    /// stack.
    MoveDataToAddr,
    /// Remove the value on the top of the address stack and push it onto the top of the data
    /// stack.
    MoveAddrToData,

    /// Perform the bitwise NOT operation on the value on top of the data stack.
    Not,
    /// Perform the logical AND operation on the top two values of the data stack.
    And,
    /// Perform the logical OR operation on the top two values of the data stack.
    Or,
    /// Perform the logical XOR operation on the top two values of the data stack.
    Xor,
    /// If the top two values of the data stack are equal then replace them with a `1`, otherwise a
    /// replace them with a `0`.
    Equal,
    /// If the top of the data stack is less than the second value on the data stack, replace them
    /// with a `1` otherwise replace them with a `0`.
    LessThan,
    /// If the top of the data stack is greater than the second value on the data stack, replace them
    /// with a `1` otherwise replace them with a `0`.
    GreaterThan,

    /// Replace the top two values on the data stack with their sum.
    Add,
    /// Replace the top two values on the the data stack with their difference (tos - nos).
    Sub,
    /// Replace the top two values on the the data stack with their product.
    Mul,
    /// Replace the top two values on the the data stack with their quotient (tos / nos).
    Div,
    /// Replace the top two values on the the data stack with their "modulus" (tos % nos).
    Mod,
    // TODO: Signed Shift?
    /// Shift the second value on the data stack by the value top of the data stack (nos << tos).
    Shift,
    /// Sign extend the 8 bit value on the top of the data stack to a 32 bit signed value.
    Sext8,
    /// Sign extend the 16 bit value on the top of the data stack to a 32 bit signed value.
    Sext16,

    /// Pop the value on top of the data stack,
    /// push the current address to the address stack and set `ip` to the value poped off of the data stack.
    Call,
    /// Pop the value on top of the data stack,
    /// and set `ip` to the value poped off of the data stack.
    Jump,
    /// Pop a value off of the address stack and set `ip` to the value.
    Return,

    /// Conditional `Call`.  Do a call only if the value on top of the data stack is `0`.
    CallIfZ,
    /// Conditional `Jump`.  Do a jump only if the value on top of the data stack is `0`.
    JumpIfZ,
    /// Conditional `Return`.  Do a return only if the value on top of the data stack is `0`.
    ReturnIfZ,

    Load,
    Loads,
    Store,
    Stores,
    Load8,
    Store8,
    Loads8,
    Stores8,

    Io,

    /// Halt execution.  If the value on top of the data stack is `-1` then perform a core dump.
    Halt = 0b_0111_1111
}

impl TryFrom<u8> for OpCode {
    type Error = Error;

    fn try_from(byte: u8) -> Result<OpCode, Self::Error> {
        if (OpCode::Io as u8) < byte && byte != OpCode::Halt as u8 {
            Err(Error::invalid_instruction(byte))
        } else {
            Ok(unsafe { ::std::mem::transmute(byte) })
        }
    }
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpCode::Halt => write!(f, "halt"),
            OpCode::Lit => write!(f, "lit"),
            OpCode::Sext8 => write!(f, "sext.8"),
            OpCode::Sext16 => write!(f, "sext.16"),

            OpCode::Dup => write!(f, "dup"),
            OpCode::Drop => write!(f, "drop"),
            OpCode::Swap => write!(f, "swap"),
            OpCode::MoveDataToAddr => write!(f, "push"),
            OpCode::MoveAddrToData => write!(f, "pop"),

            OpCode::Call => write!(f, "call"),
            OpCode::Jump => write!(f, "jump"),
            OpCode::Return => write!(f, "ret"),
            OpCode::CallIfZ => write!(f, "ifz:call"),
            OpCode::JumpIfZ => write!(f, "ifz:jump"),
            OpCode::ReturnIfZ => write!(f, "ifz:ret"),

            OpCode::Load => write!(f, "load"),
            OpCode::Store => write!(f, "store"),
            OpCode::Load8 => write!(f, "load.8"),
            OpCode::Store8 => write!(f, "store.8"),
            OpCode::Loads => write!(f, "loads"),
            OpCode::Stores => write!(f, "stores"),
            OpCode::Loads8 => write!(f, "loads.8"),
            OpCode::Stores8 => write!(f, "stores.8"),
            OpCode::Io => write!(f, "io"),

            OpCode::And => write!(f, "and"),
            OpCode::Or => write!(f, "or"),
            OpCode::Xor => write!(f, "xor"),
            OpCode::Not => write!(f, "not"),
            OpCode::Equal => write!(f, "eq"),
            OpCode::LessThan => write!(f, "lt"),
            OpCode::GreaterThan => write!(f, "gt"),

            OpCode::Add => write!(f, "add"),
            OpCode::Sub => write!(f, "sub"),
            OpCode::Mul => write!(f, "mul"),
            OpCode::Div => write!(f, "div"),
            OpCode::Mod => write!(f, "mod"),
            OpCode::Shift => write!(f, "shift"),

            OpCode::Nop => write!(f, "nop"),
        }
    }
}

impl OpCode {
    pub fn into_u8(self) -> u8 {
        unsafe { ::std::mem::transmute(self) }
    }
}

/// Allows for very simple debugging.
pub struct CallbackDebugger {
    pub ip: fn(usize),
    pub data_pop: fn(),
    pub data_push: fn(Cell),
    pub address_pop: fn(),
    pub address_push: fn(Cell),
    pub store_8: fn(Cell, Cell),
    pub store_16: fn(Cell, Cell),
}

/// The runtime state of the VM.
#[derive(Default)]
pub struct ExecutionState {
    /// The index in the current cell of the address being executed.
    pub instruction_index: usize,
    /// The index in the binary image of the currently loaded cell.
    pub loaded_word_index: usize,
    /// The index in the binary image of the next word to be loaded.
    pub current_word_index: usize,
    /// The loaded cell as an array of bytes.
    pub word: [u8; 4],
    /// Indicates if the VM is running or halted.
    pub running: bool,
    /// The VM that this is the execution state of.
    pub vm: BearVM,
}

// TODO: Make everything private and expose through interface.
// TODO: The original design forced the image to be present at construction.
// Because of web-assembly shenanigans, this was changed.
// It would be nice to go back.
#[derive(Default)]
pub struct BearVM {
    /// The binary image being executed.
    pub image: Vec<u32>,
    /// The data stack.
    pub data: Vec<Cell>,
    /// The address stack.
    pub address: Vec<Cell>,
    /// The external devices.
    pub devices: Vec<Box<dyn Device>>,
    /// Optional logger.
    pub debug_logger: Option<fn(&str)>,
    /// Optional debuger.
    pub callback_debugger: Option<CallbackDebugger>
}

/// Wraps calls to push and pop the stacks with calls to the debugger and error handling code.
impl BearVM {
    pub fn data_pop(&mut self) -> Result<Cell, Error> {
        self.callback_debugger.as_ref().map(|d| (d.data_pop)());
        self.data.pop().ok_or(Error::data_underflow())
    }

    fn data_peek(&mut self) -> Result<Cell, Error> {
        self.data.last().cloned().ok_or(Error::data_underflow())
    }

    pub fn data_push(&mut self, cell: Cell) {
        self.callback_debugger.as_ref().map(|d| (d.data_push)(cell));
        self.data.push(cell);
    }

    fn address_pop(&mut self) -> Result<Cell, Error> {
        self.callback_debugger.as_ref().map(|d| (d.address_pop)());
        let value = self.address.pop().ok_or(Error::address_underflow())?;
        return Ok(value as Cell);
    }

    fn address_push(&mut self, cell: Cell) {
        self.callback_debugger.as_ref().map(|d| (d.address_push)(cell));
        self.address.push(unsafe { ::std::mem::transmute(cell) });
    }
}

impl ExecutionState {
    pub fn ip(&self) -> usize { self.current_word_index * cell::SIZE + self.instruction_index }

    pub fn ip_set(&mut self, loaded_word_index: usize, current_word_index: usize, instruction_index: usize) -> Result<(), Error> {
        // self.callback_debugger.as_ref().map(|d| (d.ip)(value));
        if self.vm.image.len() < loaded_word_index {
            Err(Error::ip_oob(loaded_word_index * cell::SIZE + instruction_index))
        } else {
            self.loaded_word_index = loaded_word_index;
            self.current_word_index = current_word_index;
            self.instruction_index = instruction_index;
            let word = self.vm.image[self.loaded_word_index];
            self.word = word.to_le_bytes();
            Ok(())
        }
    }

    pub fn ip_get_encoded(&self) -> u32 {
        assert!(self.instruction_index < 4);
        assert!(self.loaded_word_index & 0x7FFF == self.loaded_word_index);
        assert!(self.current_word_index & 0x7FFF == self.current_word_index);
        let encoded = ((self.loaded_word_index << 17) | (self.current_word_index << 2) | self.instruction_index) as u32;
        return encoded;
    }

    pub fn ip_set_encoded(&mut self, ip: u32) -> Result<(), Error> {
        let ii = ip & 3;
        let lw = ip >> 17;
        let cw = (ip >> 2) & 0x7FFF;
        return self.ip_set(lw as usize, cw as usize, ii as usize);
    }

    pub fn ip_inc(&mut self) -> Result<(), Error> {
        if self.instruction_index == (cell::SIZE - 1) {
            self.current_word_index += 1;
            self.loaded_word_index = self.current_word_index;
            self.instruction_index = 0;
            if self.vm.image.len() < self.loaded_word_index {
                return Err(Error::ip_oob(self.ip()));
            }
            let word = self.vm.image[self.loaded_word_index];
            self.word = word.to_le_bytes();
        } else {
            self.instruction_index += 1;
        }
        return Ok(());
    }

    pub fn ip_get_next(&self) -> usize {
        if self.instruction_index == (cell::SIZE - 1) {
            cell::SIZE * (self.loaded_word_index + 1)
        } else {
            self.ip() + 1
        }
    }

    pub fn instruction(&self) -> Result<OpCode, Error> {
        let byte = self.word[self.instruction_index];
        return OpCode::try_from(byte).map_err(|e| e.with_ip(self.ip()));
    }
}

impl ExecutionState {
    fn data_pop(&mut self) -> Result<Cell, Error> {
        self.vm.data_pop().map_err(|e| e.with_ip_from_state(self))
    }

    fn data_peek(&mut self) -> Result<Cell, Error> {
        self.vm.data_peek().map_err(|e| e.with_ip_from_state(self))
    }
}

impl ExecutionState {
    fn inst_lit_next_word(&mut self) -> Result<(), Error> {
        // assert!(self.instruction_index == 3);

        self.current_word_index += 1;
        let value = self.vm.image[self.current_word_index];
        self.vm.data_push(value.into());
        // self.instruction_index = 0;
        // self.word = self.vm.image[self.word_index].to_le_bytes();
        return Ok(());
    }

    fn inst_sext_8(&mut self) -> Result<(), Error> {
        let value: u32 = self.data_pop()?.into();
        if value <= u8::MAX as u32 {
            let value = value as u8;
            let value: i8 = unsafe { transmute_copy(&value) };
            let value: i32 = value as i32;
            let value: u32 = unsafe { transmute_copy(&value) };
            self.vm.data_push(value.into());
        } else {
            self.vm.data_push(value.into());
        }
        return Ok(());
    }

    fn inst_sext_16(&mut self) -> Result<(), Error> {
        let value: u32 = self.data_pop()?.into();
        if value <= u16::MAX as u32 {
            let value = value as u16;
            let value: i16 = unsafe { transmute_copy(&value) };
            let value: i32 = value as i32;
            let value: u32 = unsafe { transmute_copy(&value) };
            self.vm.data_push(value.into());
        } else {
            self.vm.data_push(value.into());
        }
        return Ok(());
    }
}

impl ExecutionState {
    pub fn dump(&self) -> Result<(), std::io::Error> {
        let v = crate::util::convert_slice32_to_vec8(&self.vm.image);
        std::fs::write("core.bin", v)
    }

    /**
     * Halt.
     */
    fn inst_halt(&mut self) -> Result<(), Error> {
        match self.vm.data.last() {
            Some(Cell(u32::MAX)) => {
                self.dump().ok();
                Ok(())
            },
            _ => Ok(())
        }
    }

    /**
     * Do nothing.
     */
    fn inst_nop(&self) -> Result<(), Error> {
        return Ok(());
    }

    /**
     * Duplicate the top value on the data stack.
     */
    fn inst_dup(&mut self) -> Result<(), Error> {
        let tos = self.data_peek()?;
        self.vm.data_push(tos);
        return Ok(());
    }

    fn inst_drop(&mut self) -> Result<(), Error> {
        self.vm.data_pop()?;
        return Ok(());
    }

    /**
     * Swap the top two values on the data stack.
     */
    fn inst_swap(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        self.vm.data_push(tos);
        self.vm.data_push(nos);
        return Ok(());
    }

    /**
     * Pop the top value on the data stack and push it onto the return stack.
     */
    fn inst_move_data_to_address(&mut self) -> Result<(), Error> {
        let data = self.data_pop()?;
        self.vm.address_push(data);
        return Ok(());
    }

    /**
     * Pop the top value on the return stack and push it onto the data stack.
     */
    fn inst_move_address_to_data(&mut self) -> Result<(), Error> {
        let addr = self.vm.address_pop()?;
        self.vm.data_push(addr);
        return Ok(());
    }
}

impl ExecutionState {
    fn inst_or(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let value = tos | nos;
        self.vm.data_push(value);
        return Ok(());
    }

    fn inst_and(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let value = tos & nos;
        self.vm.data_push(value);
        return Ok(());
    }

    fn inst_xor(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let value = tos ^ nos;
        self.vm.data_push(value);
        return Ok(());
    }

    fn inst_not(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let value = !tos;
        self.vm.data_push(value);
        return Ok(());
    }

    fn inst_equal(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        self.vm.data_push(if tos == nos { (-1).into() } else { 0.into() });
        return Ok(());
    }

    fn inst_less_than(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        self.vm.data_push(if tos < nos { (-1).into() } else { 0.into() });
        return Ok(());
    }

    fn inst_greater_than(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        self.vm.data_push(if tos > nos { (-1).into() } else { 0.into() });
        return Ok(());
    }

    fn inst_add(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let value = tos + nos;
        self.vm.data_push(value);
        return Ok(());
    }

    fn inst_sub(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let value = tos - nos;
        self.vm.data_push(value);
        return Ok(());
    }

    fn inst_mul(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let value = tos * nos;
        self.vm.data_push(value);
        return Ok(());
    }

    fn inst_div(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let q = tos / nos;
        self.vm.data_push(q);
        return Ok(());
    }

    fn inst_rem(&mut self) -> Result<(), Error> {
        let tos = self.data_pop()?;
        let nos = self.data_pop()?;
        let r = tos.rem(nos);
        self.vm.data_push(r);
        return Ok(());
    }

    fn inst_shift(&mut self) -> Result<(), Error> {
        let tos: i32 = self.data_pop()?.into();
        let nos: u32 = self.data_pop()?.into();
        // TODO: trap if oob?
        let value = if tos < 0 {
            nos >> tos.abs()
        } else {
            nos << tos.abs()
        };
        self.vm.data_push(value.into());
        return Ok(());
    }
}

impl ExecutionState {
    fn inst_jump(&mut self, ifz: bool) -> Result<(), Error> {
        let ip = self.data_pop()?.0 as usize;
        if ifz {
            if self.data_pop()?.0 != 0 {
                return Ok(());
            }
        }
        let (w, i) = if ip != 0 && ip % 4 == 0 { ((ip / 4) - 1, 3)} else { ((ip / 4), (ip % 4) - 1)};
        self.ip_set(w, w, i)?;
        return Ok(());
    }

    fn inst_call(&mut self, ifz: bool) -> Result<(), Error> {
        let ip = self.data_pop()?.0 as usize;
        if ifz {
            if self.data_pop()?.0 != 0 {
                return Ok(());
            }
        }
        let current = self.ip_get_encoded();
        self.vm.address_push(Cell::try_from(current).map_err(|_| Error::ip_oob(current as usize))?);
        let (w, i) = if ip != 0 && ip % 4 == 0 { ((ip / 4) - 1, 3)} else { ((ip / 4), (ip % 4) - 1)};
        self.ip_set(w, w, i)?;
        return Ok(());
    }

    // This instruction will leave the value on the stack if it is not zero.
    fn inst_return(&mut self, ifz: bool) -> Result<(), Error> {
        if ifz {
            if self.data_peek()?.0 != 0 {
                return Ok(());
            }
            self.vm.data_pop()?;
        }
        let ip = self.vm.address_pop()?;
        self.ip_set_encoded(ip.0)?;
        return Ok(());
    }
}

impl ExecutionState {
    fn inst_io(&mut self) -> Result<(), Error> {
        let command = self.data_pop()?;
        let device_id = self.data_pop()?;
        let device = &mut self.vm.devices[device_id.0 as usize];
        let result = device.ioctl(command.0);
        self.vm.data_push(device_id.into());
        self.vm.data_push(result.into());
        return Ok(());
    }

    /**
     * [&x] -> [(&x)+4, x]
     */
    fn inst_load(&mut self, stream: bool) -> Result<(), Error> {
        let address: usize = self.data_pop()?.into();
        let r = address % 4;
        let value = if r == 0 {
            self.vm.image[address / 4]
        } else {
            let shift = 2 * r;
            let mask = 0xFFFFFFFF >> shift;
            let high = (self.vm.image[address / 4] & mask) << shift;
            let low = (self.vm.image[address / 4 + 1] & !mask) >> (8 - shift);
            high | low
        };
        self.vm.data_push(Cell::from(value));
        if stream {
            self.vm.data_push(Cell::from(address as u32 + 4));
        }
        return Ok(());
    }

    fn inst_load_8(&mut self, stream: bool) -> Result<(), Error> {
        let address: usize = self.data_pop()?.into();
        let word = self.vm.image[address / 4];
        let byte = word.to_le_bytes()[address % 4];
        self.vm.data_push(Cell::from(byte));
        if stream {
            self.vm.data_push(Cell::from(address as u32 + 1));
        }
        return Ok(());
    }

    /**
     * [Y, &x] -> [(&x) + 4]; &x = Y
     */
    fn inst_store(&mut self, stream: bool) -> Result<(), Error> {
        let value = self.data_pop()?;
        let address  = self.data_pop()?;
        self.vm.callback_debugger.as_ref().map(|d| (d.store_8)(address, value));
        let value: u32 = value.into();
        let address: usize = address.into();
        let r = address % 4;
        if r == 0 {
            self.vm.image[address as usize / 4] = value;
        } else {
            let shift = 2 * r;
            let mask = 0xFFFFFFFF >> shift;
            let low = value & !mask;
            let high = value & mask;
            self.vm.image[address / 4] = (self.vm.image[address / 4] & mask) | low;
            self.vm.image[address / 4 + 1] = (self.vm.image[address / 4 + 1] & mask) | high;
        }
        if stream {
            self.vm.data_push(Cell::from((address + 4) as u32));
        }
        return Ok(());
    }

    fn inst_store_8(&mut self, stream: bool) -> Result<(), Error> {
        let value = self.data_pop()?;
        let address  = self.data_pop()?;
        self.vm.callback_debugger.as_ref().map(|d| (d.store_8)(address, value));
        // TODO: interupt if too big.
        let value: u32 = value.into();
        let address: usize = address.into();
        let word = self.vm.image[address as usize / 4];
        let mask = 0xFF << (address % 4) * 8;
        let value = value << (address % 4) * 8;
        self.vm.image[address as usize / 4] = (word & !mask) | value;
        if stream {
            self.vm.data_push(Cell::from((address + 1) as u32));
        }
        return Ok(());
    }
}

impl ExecutionState {
    pub fn run(&mut self) -> Result<(), Error> {
        self.instruction_index = 0;
        self.loaded_word_index = 0;
        self.current_word_index = 0;
        let word = self.vm.image[self.loaded_word_index];
        self.word = word.to_le_bytes();
        self.running = true;

        loop {
            self.step()?;
            if !self.running {
                break;
            }
            self.sync();
        }

        return Ok(());
    }

    pub fn step(&mut self) -> Result<(), Error> {
        let instruction = self.instruction()?;
        match instruction {
            OpCode::Nop => self.inst_nop(),

            OpCode::Or => self.inst_or(),
            OpCode::And => self.inst_and(),
            OpCode::Xor => self.inst_xor(),
            OpCode::Not => self.inst_not(),
            OpCode::Equal => self.inst_equal(),
            OpCode::LessThan => self.inst_less_than(),
            OpCode::GreaterThan => self.inst_greater_than(),

            OpCode::Add => self.inst_add(),
            OpCode::Sub => self.inst_sub(),
            OpCode::Mul => self.inst_mul(),
            OpCode::Div => self.inst_div(),
            OpCode::Mod => self.inst_rem(),
            OpCode::Shift => self.inst_shift(),

            OpCode::Dup  => self.inst_dup(),
            OpCode::Drop  => self.inst_drop(),
            OpCode::Swap => self.inst_swap(),
            OpCode::MoveDataToAddr => self.inst_move_data_to_address(),
            OpCode::MoveAddrToData => self.inst_move_address_to_data(),

            OpCode::Call => self.inst_call(false),
            OpCode::Jump => self.inst_jump(false),
            OpCode::Return => self.inst_return(false),
            OpCode::CallIfZ => self.inst_call(true),
            OpCode::JumpIfZ => self.inst_jump(true),
            OpCode::ReturnIfZ => self.inst_return(true),

            OpCode::Load => self.inst_load(false),
            OpCode::Store => self.inst_store(false),
            OpCode::Load8 => self.inst_load_8(false),
            OpCode::Store8 => self.inst_store_8(false),
            OpCode::Loads => self.inst_load(true),
            OpCode::Stores => self.inst_store(true),
            OpCode::Loads8 => self.inst_load_8(true),
            OpCode::Stores8 => self.inst_store_8(true),
            OpCode::Io => self.inst_io(),

            OpCode::Lit => self.inst_lit_next_word(),
            OpCode::Sext8 => self.inst_sext_8(),
            OpCode::Sext16 => self.inst_sext_16(),

            OpCode::Halt => {
                self.inst_halt()?;
                self.running = false;
                return Ok(());
            }
        }?;

        self.ip_inc()?;
        return Ok(());
    }

    pub fn sync(&mut self) {
        for i in 0..self.vm.devices.len() {
            let device = &mut self.vm.devices[i];
            loop {
                match device.dma_poll() {
                    None => break,
                    Some(DMARequest::Read(address)) => {
                        assert!(address % 4 == 0);
                        let word = self.vm.image[address / 4];
                        device.dma_read_response(address, word);
                    },
                    Some(DMARequest::Write(address, value)) => {
                        assert!(address % 4 == 0);
                        self.vm.image[address / 4] = value;
                        device.dma_write_response(address);
                    }
                }
            }
        }
    }
}

impl BearVM {
    fn log(&self, _message: &str) {
        #[cfg(debug)]{
            self.debug_logger.map(|f| {
                f(_message);
                return f;
            });
        }
    }
}

impl BearVM {
    pub fn empty() -> BearVM {
        BearVM{ image: vec![], data: Vec::new(), address: Vec::new(), debug_logger: None, devices: Vec::new(), callback_debugger: None }
    }

    pub fn new(image: Vec<u32>) -> BearVM {
        BearVM{ image, data: Vec::new(), address: Vec::new(), debug_logger: None, devices: Vec::new(), callback_debugger: None }
    }

    pub fn with_logger(mut self, logger: fn(&str)) -> BearVM {
        self.debug_logger = Some(logger);
        return self;
    }

    pub fn with_callback_debugger(mut self, debugger: CallbackDebugger) -> BearVM {
        self.callback_debugger = Some(debugger);
        return self;
    }

    pub fn with_device(mut self, device: Box<dyn Device>) -> BearVM {
        self.devices.push(device);
        return self;
    }

    pub fn start(self) -> Result<ExecutionState, Error> {
        self.log(&format!("stated."));

        let state = ExecutionState{
            loaded_word_index: 0,
            current_word_index: 0,
            instruction_index: 0,
            word: self.image[0].to_le_bytes(),
            running: true,
            vm: self
        };
        return Ok(state);
    }

    pub fn load_image(&mut self, image: Vec<u8>) -> Result<(), Error> {
        self.image = crate::util::convert_slice8_to_vec32(&image);
        self.data.clear();
        self.address.clear();
        return Ok(());
    }
}

