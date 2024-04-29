use std::collections::HashMap;
use std::path::Path;

use clap::{App, Arg};

mod devices;
use bear_vm::vm::CallbackDebugger;
use devices::{StdinDevice, StdoutDevice};

use colored::*;


struct DebugInfo {
    line: usize,
    labels: Vec<String>,
}

struct BasicDebugger {
    info: HashMap<usize, DebugInfo>,
}

fn make_debug_info(raw: bear_ass::parser::ast::Debug) -> HashMap<usize, DebugInfo> {
    let mut hm = HashMap::new();
    for e in raw.entries.iter() {
        hm.insert(
            e.address,
            DebugInfo {
                line: e.line,
                labels: e.names.clone(),
            },
        );
    }
    hm
}

impl CallbackDebugger for BasicDebugger {
    fn ip(&self, state: &bear_vm::vm::ExecutionState, op: bear_vm::vm::OpCode) {
        let ip = state.ip();
        let ii = state.instruction_index;
        let lw = state.loaded_word_index;
        let cw = state.current_word_index;

        match self.info.get(&ip) {
            None => {}
            Some(e) => {
                eprintln!("line #: {} -- {:?}", e.line, e.labels);
            }
        }
        eprint!("{}", "ii: ".bold());
        eprint!("{}", ii.to_string().truecolor(0x35, 0xBA, 0xF6));
        eprint!(", ");
        eprint!("{}", "cw: ".bold());
        eprint!("{}", cw.to_string().truecolor(0x35, 0xBA, 0xF6));
        eprint!(", ");
        eprint!("{}", "lw: ".bold());
        eprint!("{}", lw.to_string().truecolor(0x35, 0xBA, 0xF6));
        eprint!(", ");
        eprint!("{}", "op: ".bold());
        eprintln!("{}", op.to_string().yellow());

        eprint!("{}", "data: ".bold());
        for e in state.vm.data.iter().rev() {
            eprint!("{}", "| ".bold());
            eprint!("{} ", e.0.to_string().truecolor(0x35, 0xBA, 0xF6));
        }
        eprintln!();

        eprint!("{}", "addr: ".bold());
        for e in state.vm.address.iter().rev() {
            eprint!("{}", "| ".bold());
            eprint!("{} ", e.0.to_string().truecolor(0x35, 0xBA, 0xF6));
        }
        eprintln!("\n");
    }

    fn data_pop(&self, _vm: &bear_vm::vm::BearVM) {}
    fn data_push(&self, _vm: &bear_vm::vm::BearVM, _cell: bear_vm::vm::Cell) {}
    fn address_pop(&self, _vm: &bear_vm::vm::BearVM) {}
    fn address_push(&self, _vm: &bear_vm::vm::BearVM, _cell: bear_vm::vm::Cell) {}
}

fn make_vm_from_path(
    path: &Path,
    devices: Vec<Box<dyn bear_vm::device::Device>>,
    debug: bool,
) -> bear_vm::vm::BearVM {
    let image_path = path.with_extension("bin");
    let image = std::fs::read(image_path.clone()).unwrap_or_else(|_| panic!("No image: {:?}", image_path));
    let mut vm = bear_vm::vm::BearVM::new(bear_vm::util::convert_slice8_to_vec32(&image));
    for device in devices.into_iter() {
        vm = vm.with_device(device);
    }
    if debug {
        let dbg_path = path.with_extension("debug");
        let dbg_raw =
            std::fs::read_to_string(dbg_path.clone()).unwrap_or_else(|_| panic!("No image: {:?}", dbg_path));
        let dbg_info: bear_ass::parser::ast::Debug =
            serde_json::from_str(&dbg_raw).expect("Could not load debug info.");
        return vm.with_callback_debugger(Box::new(BasicDebugger {
            info: make_debug_info(dbg_info),
        }));
    }
    vm
}

fn main() {
    let args = App::new("BearVM")
        .version("0.1.0")
        .author("John Connor <john.theman.connor@gmail.com>")
        .arg(Arg::with_name("binary").takes_value(true))
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .short("d")
                .takes_value(false),
        )
        .arg(Arg::with_name("stdin").long("stdin").takes_value(true))
        .arg(Arg::with_name("stdout").long("stdout").takes_value(true))
        .get_matches();
    let stdin: Box<dyn bear_vm::device::Device> = if args.is_present("stdin") {
        Box::new(StdinDevice::new(
            std::fs::File::open(args.value_of("stdin").unwrap()).unwrap(),
        ))
    } else {
        Box::new(StdinDevice::new(std::io::stdin()))
    };
    let stdout: Box<dyn bear_vm::device::Device> = if args.is_present("stdout") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(args.value_of("stdout").unwrap())
            .unwrap();
        Box::new(StdoutDevice::new(file))
    } else {
        Box::new(StdoutDevice::new(std::io::stdout()))
    };
    let path = Path::new(args.value_of("binary").unwrap());
    let vm = make_vm_from_path(path, vec![stdin, stdout], args.is_present("debug"));
    let mut state = vm.start().expect("Could not start vm.");
    match state.run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("IP: {}", state.ip());
            eprintln!("Error: {:?}", e);
        }
    }
}
