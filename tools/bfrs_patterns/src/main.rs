use bfrs_common::parser;
use bfrs_input::bytes::BufferedBytes;
use bfrs_patterns::r#match::MatchSM;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "bfrs_patterns",
    about = "detect patterns within brainfuck code"
)]
struct Opt {
    /// the pattern to search for
    #[structopt(short, long)]
    pattern: String,

    /// the file to search in
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

fn main() {
    if let Err(ref e) = run() {
        eprintln!("Error: {}", e);
        ::std::process::exit(1)
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let scope = bfrs_patterns::pattern::parse_pattern(&opt.pattern)?;
    let src = {
        use std::fs::File;
        let input = File::open(opt.file)?;
        BufferedBytes::new(input)
    };

    let instructions: Vec<_> = parser::parse(src).collect::<Result<_, _>>()?;

    for res in MatchSM::find_all(&instructions, &scope) {
        let str: String = res.commands.iter().map(|&i| i as u8 as char).collect();
        println!("result: `{}`", str);
        for (key, offsets) in res.relative_offsets {
            println!(
                "offsets for `{}`",
                scope.bindings.get_by_left(&key).unwrap()
            );
            for (other, offt) in offsets.iter().filter(|(k, _)| **k != key) {
                println!(
                    "\t`{}` -> {}",
                    scope.bindings.get_by_left(other).unwrap(),
                    offt
                );
            }
        }
    }

    Ok(())
}
