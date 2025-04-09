use std::{fs::File, io::{BufWriter, Error, ErrorKind}};

use args::MMMLPlayerArgs;
use clap::Parser;
use hound::{SampleFormat, WavSpec, WavWriter};
use mmml_compiler::{compiler::Compiler, lexer::Lexer};
use mmml_engine::MMMLSynthesizer;

mod args;
mod mmml_engine;

fn main() {
    let args: MMMLPlayerArgs = MMMLPlayerArgs::parse();
    
    match std::fs::read(args.input_file.clone()) {
        Ok(data) => play_mmml(data, args),
        Err(err) => {
            println!("Failed to read file:\n\t{}", err);
        }
    }
}

fn get_mmml_data(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    if data[data.len() - 1] == 0xFF {
        let mut d: Vec<u8> = data.clone();
        d.push(0x00);
        return Ok(d);
    }
    if let Ok(source_code) = String::from_utf8(data) {
        println!("Compiling µMML file...");
        let mut lexer: Lexer = Lexer::new(source_code);
        let mut compiler: Compiler = Compiler::new(lexer.tokenize()?);
        let mut mmml_data: Vec<u8> = compiler.compile()?;
        mmml_data.push(0x00);
        println!("Compiling complete!");
        return Ok(mmml_data);
    }
    Err(Error::new(ErrorKind::InvalidData, "Invaild µMML file."))
}

fn play_mmml(data: Vec<u8>, args: MMMLPlayerArgs) {
    match get_mmml_data(data) {
        Ok(mmml_data) => {
            let mut mmml: MMMLSynthesizer = MMMLSynthesizer::new();
            mmml.channels[0].is_muted = args.ch1_muted;
            mmml.channels[1].is_muted = args.ch2_muted;
            mmml.channels[2].is_muted = args.ch3_muted;
            mmml.channels[3].is_muted = args.ch4_muted;
            println!("Generating samples...");
            let samples: Vec<u8> = mmml.generate_mmml(&mmml_data);
            println!("Samples generated!");

            println!("Creating WAV file...");
            let specs: WavSpec = WavSpec {
                channels: 1,
                sample_rate: 352800, // 1-bit music demands higer rates
                bits_per_sample: 8,
                sample_format: SampleFormat::Int
            };
            let mut writer: WavWriter<BufWriter<File>> = WavWriter::create(args.get_output_path(), specs).unwrap();
            for sample in samples {
                writer.write_sample(sample as i8).unwrap();
            }
            writer.finalize().unwrap();
            println!("µMML Music sythetized successfuly!");
        },
        Err(err) => {
            println!("Failed to get µMML data:\n\t{}", err);
        }
    }
}
