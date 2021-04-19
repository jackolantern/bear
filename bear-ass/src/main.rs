extern crate bear_vm;
extern crate bear_ass;

mod cli;

use bear_ass::Error;

const USAGE: &str= "bear-asm v1.0\n\
\n\
USAGE: bear-asm in out\n";

fn main() {
    // pretty_env_logger::init();
    match cli::go() {
        Ok(()) => { std::process::exit(0); },
        Err(Error::Usage) => eprintln!("{}", USAGE),
        Err(error) => eprintln!("{:?}", error),
    };
    std::process::exit(-1)
}

#[cfg(test)]
mod test {
    use bear_vm::vm::{BearVM, ExecutionState};
    use bear_ass::{Error, parser, processor, assembler};

    fn run(program: &str) -> Result<ExecutionState, Error> {
        // let mut image = Vec::new();
        // let mut program = program.as_bytes();
        let program = parser::Parser{}.parse(&program).map_err(|e| Error::ParserError(e))?;
        let processor = processor::Processor::process(program).expect("Processor error.");
        let image = assembler::Assembler::assemble(processor).expect("Assembler error.");
        eprintln!("image: {:?}", image);
        let vm = BearVM::new(bear_vm::util::convert_slice8_to_vec32(&image));
        let mut state = vm.start().map_err(|e| Error::Unknown(format!("{:?}", e)))?;
        state.run().map_err(|e| Error::Unknown(format!("{:?}", e)))?;
        return Ok(state);
    }

    #[test]
    fn test_sext8_neg() -> Result<(), Error> {
        let state = run("
            lit sext.8 halt
            ===
            d8 -3
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 2);
        assert!(state.vm.data == vec![(-3).into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_sext8_pos() -> Result<(), Error> {
        let state = run("
            lit sext.8 halt
            ===
            d32 1
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 2);
        assert!(state.vm.data == vec![1.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_sext16_neg() -> Result<(), Error> {
        let state = run("
            lit sext.16 halt
            ===
            d32 (2^16)-2
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 2);
        assert!(state.vm.data == vec![(-2).into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_sext16_pos() -> Result<(), Error> {
        let state = run("
            lit sext.16 halt
            ===
            d32 257
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 2);
        assert!(state.vm.data == vec![257.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_add_pos_pos() -> Result<(), Error> {
        let state = run("
            lit lit add halt
            d32 7
            d32 2
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![9.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_add_pos_neg() -> Result<(), Error> {
        let state = run("
            lit lit sext.8 add
            d32 7
            d8 -2
            ===
            halt
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 12);
        assert!(state.vm.data == vec![5.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_add_neg_pos() -> Result<(), Error> {
        let state = run("
            lit lit add halt
            d32 -7
            d32 2
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![(-5).into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_add_neg_neg() -> Result<(), Error> {
        let state = run("
            lit lit add halt
            d32 -7
            d32 -2
            ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![(-9).into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_sub_pos_pos_pos() -> Result<(), Error> {
        let state = run("
            lit lit sub halt
            d32 2
            d32 7
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![5.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_sub_pos_pos_neg() -> Result<(), Error> {
        let state = run("
            lit lit sub halt
            d32 7
            d32 2
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![(-5).into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_mul_pos_pos() -> Result<(), Error> {
        let state = run("
            lit lit mul halt
            d32 7
            d32 2
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![14.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_mul_pos_neg() -> Result<(), Error> {
        let state = run("
            lit lit mul halt
            d32 7
            d32 -2
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![(-14).into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_mul_neg_pos() -> Result<(), Error> {
        let state = run("
            lit sext.8 lit mul
            d8 -7
            ===
            d32 2
            halt
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 12);
        assert!(state.vm.data == vec![(-14).into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_mul_neg_neg() -> Result<(), Error> {
        let state = run("
            lit lit mul halt
            d32 -2
            d32 -7
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![14.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_store() -> Result<(), Error> {
        let state = run("
            lit lit store halt
            d32 $>
            d32 1000
            $ d32 0
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![16.into()]);
        assert!(state.vm.address.len() == 0);
        assert!(state.vm.image[state.vm.image.len() - 1] == 1000);
        return Ok(());
    }

    #[test]
    fn test_store_8() -> Result<(), Error> {
        let state = run("
            lit lit store.8 halt
            d32 $>
            d32 7
            $ d32 0
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data == vec![13.into()]);
        assert!(state.vm.address.len() == 0);
        assert!(state.vm.image[state.vm.image.len() - 1] == 7);
        return Ok(());
    }

    #[test]
    fn test_store_8_consec() -> Result<(), Error> {
        let state = run("
            lit lit store.8 lit
            d32 $>
            d32 2
            d32 3
            store.8 lit store.8 lit
            d32 4
            d32 5
            store.8 halt
            ===
            $ d32 0
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        eprintln!("image: {:?}", state.vm.image);
        assert!(state.ip() == 29);
        assert!(state.vm.data == vec![36.into()]);
        assert!(state.vm.address.len() == 0);
        assert!(state.vm.image[state.vm.image.len() - 1] == (5 << 24) + (4 << 16) + (3 << 8) + 2);
        return Ok(());
    }

    #[test]
    fn test_jump() -> Result<(), Error> {
        let state = run("
            lit jump halt nop
            d32 $>
            halt halt halt halt
            $
            lit halt nop nop
            d32 7
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 13);
        assert!(state.vm.data == vec![7.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_jump_ifz_true() -> Result<(), Error> {
        let state = run("
            lit lit ifz:jump halt
            d32 0
            d32 $>
            halt halt halt halt
            $
            lit halt nop nop
            d32 7
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 17);
        assert!(state.vm.data == vec![7.into()]);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_jump_ifz_false() -> Result<(), Error> {
        let state = run("
            lit lit ifz:jump halt
            d32 $>
            d32 1
            halt halt halt halt
            $
            lit halt nop nop
            d32 7
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert!(state.ip() == 3);
        assert!(state.vm.data.len() == 0);
        assert!(state.vm.address.len() == 0);
        return Ok(());
    }

    #[test]
    fn test_rot() -> Result<(), Error> {
        let state = run("
            #include \"../roms/std.bear\";
            lit lit lit nop
            d32 1
            d32 2
            d32 3
            !rot
            halt
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert_eq!(state.ip(), 4 * (state.vm.image.len() - 1));
        assert_eq!(state.vm.data, vec![2.into(), 3.into(), 1.into()]);
        assert_eq!(state.vm.address.len(), 0);
        return Ok(());
    }

    #[test]
    fn test_rotrot() -> Result<(), Error> {
        let state = run("
            #include \"../roms/std.bear\";
            lit lit lit nop
            d32 1
            d32 2
            d32 3
            !rotrot
            halt
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert_eq!(state.ip(), 4 * (state.vm.image.len() - 1));
        assert_eq!(state.vm.data, vec![3.into(), 1.into(), 2.into()]);
        assert_eq!(state.vm.address.len(), 0);
        return Ok(());
    }

    #[test]
    fn test_over() -> Result<(), Error> {
        let state = run("
            #include \"../roms/std.bear\";
            lit lit lit nop
            d32 3
            d32 2
            d32 1
            !over
            halt
        ")?;
        eprintln!("ip: {:?}, data: {:?}", state.ip(), state.vm.data);
        assert_eq!(state.ip(), 4 * (state.vm.image.len() - 1));
        assert_eq!(state.vm.data, vec![3.into(), 1.into(), 2.into(), 1.into()]);
        assert_eq!(state.vm.address.len(), 0);
        return Ok(());
    }
}

