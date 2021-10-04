use std::env;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use serde_json;

use bear_ass::assembler::Assembler;
use bear_ass::parser;
use bear_ass::processor::Processor;
use bear_ass::Error;

pub fn go() -> Result<(), Error> {
    let mut args: Vec<String> = env::args().collect();

    args.reverse();
    args.pop();

    if args.len() != 2 && args.len() != 3 {
        return Err(Error::Usage);
    }

    let arg1 = args.pop().ok_or(Error::Usage)?;
    let arg2 = args.pop().ok_or(Error::Usage)?;
    // let arg3 = args.pop();
    let in_path = Path::new(&arg1);
    let out_bin_path = Path::new(&arg2);
    let out_debug_path = out_bin_path
        .with_file_name(out_bin_path.file_stem().expect("No output filename."))
        .with_extension("debug");
    let output_debug_symbols = true; // !arg3.is_none() && (arg3 == Some("-d".to_string()) || arg3 == Some("--debug".to_string()));
    let out_bin = std::fs::File::create(out_bin_path)
        .expect(&format!("Unable to create file: {:?}", out_bin_path));
    let mut outbin_buf = std::io::BufWriter::new(out_bin);
    let in_file = std::fs::File::open(in_path).expect("Can't open file.");
    let mut reader = std::io::BufReader::new(in_file);

    let program = parse(&mut reader)?;
    let processor = match Processor::process(program) {
        Err(e) => {
            panic!("Processor error: {:?}", e)
        }
        Ok(p) => p,
    };
    if output_debug_symbols {
        let out_debug = std::fs::File::create(&out_debug_path)
            .expect(&format!("Unable to create file: {:?}", out_debug_path));
        let mut outdebug_buf = std::io::BufWriter::new(out_debug);
        write_debug(&processor, &mut outdebug_buf)?;
    }
    let bits = Assembler::assemble(processor).expect("Assembler error");
    outbin_buf.write_all(&bits).map_err(|e| Error::IOError(e))?;
    return Ok(());
}

pub fn parse(reader: &mut dyn Read) -> Result<parser::ast::Program, Error> {
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    let program = parser::Parser {}
        .parse(&contents)
        .map_err(|e| Error::ParserError(e))?;
    return Ok(program);
}

pub fn write_debug(p: &Processor, buf: &mut dyn Write) -> Result<(), Error> {
    let entries = p.make_debug().expect("Debug error.");
    serde_json::to_writer_pretty(buf, &entries).map_err(|e| Error::SerdeError(e))?;
    return Ok(());
}
