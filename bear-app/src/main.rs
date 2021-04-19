use std::path::Path;

use clap::{Arg, App};

mod devices;
use devices::{StdinDevice, StdoutDevice};

fn make_vm_from_path(path: &Path, devices: Vec<Box<dyn bear_vm::device::Device>>) -> bear_vm::vm::BearVM {
    let image_path = path.with_extension("bin");
    let image = std::fs::read(image_path.clone()).expect(&format!("No image: {:?}", image_path));
    let mut vm = bear_vm::vm::BearVM::new(bear_vm::util::convert_slice8_to_vec32(&image));
    for device in devices.into_iter() {
        vm = vm.with_device(device);
    }
    return vm;
}

fn main() {
    let args = App::new("BearVM")
        .version("0.1.0")
        .author("John Connor <john.theman.connor@gmail.com>")
        .arg(Arg::with_name("binary")
                 .takes_value(true))
        .arg(Arg::with_name("debug")
                 .long("debug")
                 .short("d")
                 .takes_value(false))
        .arg(Arg::with_name("stdin")
                 .long("stdin")
                 .takes_value(true))
        .arg(Arg::with_name("stdout")
                 .long("stdout")
                 .takes_value(true))
        .get_matches();
    let stdin: Box<dyn bear_vm::device::Device> = if args.is_present("stdin") {
        Box::new(StdinDevice::new(std::fs::File::open(args.value_of("stdin").unwrap()).unwrap()))
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
    let vm = make_vm_from_path(path, vec![stdout, stdin]);
    let mut state = vm.start().expect("Could not start vm.");
    match state.run() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("IP: {}", state.ip());
            eprintln!("Error: {:?}", e);
        }
    }
}

