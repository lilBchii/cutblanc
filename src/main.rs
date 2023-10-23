use hound::*;
use minimp3::{Decoder, Frame};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use colored::Colorize;

const LIM_VAL: i32 = 100;

fn convert_mp3_to_wav(input_file: &String, output_file: &String) -> () {
    // Read the MP3 file
    let mut mp3_data = Vec::new();
    let mut file = File::open(&input_file).expect("Failed to open input file");
    file.read_to_end(&mut mp3_data)
        .expect("Failed to read input file");

    // Create the MP3 decoder
    let mut decoder = Decoder::new(mp3_data.as_slice());

    let loop_duration = 10.0; // Duration in seconds to play output sound
    let mut samples_written = 0;

    let mut decoded_data = Vec::new();
    let mut sample_rate = 0;
    let mut channels = 0;

    // Decode MP3 frames
    loop {
        match decoder.next_frame() {
            Ok(Frame {
                data,
                sample_rate: frame_sample_rate,
                channels: frame_channels,
                ..
            }) => {
                if sample_rate == 0 {
                    sample_rate = frame_sample_rate;
                    channels = frame_channels;
                }
                decoded_data.extend_from_slice(&data);
            }
            Err(minimp3::Error::Eof) => break,
            Err(e) => {
                eprintln!("Error decoding MP3: {:?}", e);
                std::process::exit(1);
            }
        }
    }

    // Apply fade-in and fade-out effects
    let fade_duration = 1.0; // Fade duration in seconds
    apply_fade_in_fade_out(&mut decoded_data, channels, fade_duration, sample_rate);

    // Calculate target samples count for looped output
    let target_samples = (loop_duration * sample_rate as f64 * channels as f64).ceil() as usize;

    // Initialize WAV writer
    let spec = WavSpec {
        channels: channels as _,
        sample_rate: sample_rate as _,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let path = Path::new(output_file);
    let mut wav_writer = WavWriter::create(path, spec).expect("Failed to create WAV file");

    // Write samples to the WAV file, looping until the desired length is reached
    while samples_written < target_samples {
        for sample in &decoded_data {
            wav_writer
                .write_sample(*sample)
                .expect("Failed to write to WAV file");
            samples_written += 1;
            if samples_written >= target_samples {
                break;
            }
        }
    }

    println!("Conversion completed.");
}

fn apply_fade_in_fade_out(
    data: &mut Vec<i16>,
    channels: usize,
    fade_duration: f64,
    sample_rate: i32,
) {
    let fade_samples = (fade_duration * sample_rate as f64).ceil() as usize;

    for i in 0..fade_samples {
        let factor = i as f64 / fade_samples as f64;
        for channel in 0..channels {
            let idx = i * channels + channel;
            data[idx] = (data[idx] as f64 * factor).round() as i16;
        }
    }

    let total_samples = data.len() / channels;
    for i in (total_samples - fade_samples..total_samples).rev() {
        let factor = (total_samples - i) as f64 / fade_samples as f64;
        for channel in 0..channels {
            let idx = i * channels + channel;
            data[idx] = (data[idx] as f64 * factor).round() as i16;
        }
    }
}

fn cutblanc(input_file: &String, output_file: &String) -> () {
    println!("Opening file...");
    let mut reader = WavReader::open(input_file).unwrap();
    let spec = reader.spec();

    //Values of each sample
    let mut ampl = Vec::new();

    for n in reader.samples::<i32>() {
        if let Ok(num) = n {
            ampl.push(num);
        } else {
            ampl.push(0);
            //panic!("failed to read sample value!");
        }
    }

    println!("Ok!");
    println!("Cutting silences...");

    // Remove silences
    let l = ampl.len();
    let mut res = Vec::new();

    let mut ind = 0;
    let mut count = 0;
    while ind < l {
        let val = ampl[ind];
        if val.abs() <= LIM_VAL {
            count += 1;
            ind += 1;
        } else {
            if count < 1000 {
                for i in ind - count..=ind {
                    res.push(ampl[i]);
                }
            } else {
                res.push(val);
            }
            count = 0;
            ind += 1;
        }
    }

    println!("Ok!");
    println!(
        "from {}s to {}s",
        l as f32 / spec.sample_rate as f32,
        res.len() as f32 / spec.sample_rate as f32
    );

    println!("Writting file...");

    // Write new file from cleaned samples
    let path: &Path = output_file.as_ref();

    let mut writer = match path.is_file() {
        true => {
            println!("Appends to {}", output_file);
            WavWriter::append(path).unwrap()
        }
        false => WavWriter::create(path, reader.spec()).unwrap(),
    };

    assert_eq!(reader.spec(), writer.spec());

    //Write new audio
    for t in 0..res.len() {
        writer.write_sample(res[t] as i32).unwrap();
    }

    //writer.finalize().unwrap();
    println!("Done")
}

fn main() {
    //env::set_var("RUST_BACKTRACE", "1");
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        println!("{}:", "USAGE".bold());
        println!(
            "    {} [ACTION] [input_file] [output_file]",
            "cutblanc".bold()
        );
        println!("{}:", "ACTIONS".bold());
        println!("    {}      to cut silences", "- cut".bold());
        println!("    {}  to convert a mp3 to wav", "- convert".bold());
        std::process::exit(1);

        eprintln!("Usage: {} <action> <input_file> <output_file>", &args[0]);
        std::process::exit(1);
    } else {
        for arg in args.iter() {
            if arg == "cut" {
                cutblanc(&args[2], &args[3]);
            } else if arg == "convert" {
                convert_mp3_to_wav(&args[2], &args[3]);
            }
        }
    }
}
