use clap::Parser;
use hound::*;
use std::{fs::File, io::BufReader, path::Path};

const LIM_AMPLITUDE: i32 = 100;
const DEFAULT_DURATION: usize = 1000;

#[derive(Parser)]
#[command(about = "Cuts silences in wav files.")]
struct Args {
    /// The limit value of amplitude under which you want to cut the samples
    #[arg(short, long, default_value_t = LIM_AMPLITUDE)]
    limit_amplitude: i32,
    /// The minimum duration (in samples) to consider a silence to cut
    #[arg(short, long, default_value_t = DEFAULT_DURATION)]
    duration: usize,
    /// The file to work on
    input_file: String,
    /// The destination file
    output_file: String,
}

fn cutblanc(
    reader: &mut WavReader<BufReader<File>>,
    limit_amplitude: i32,
    duration: usize,
) -> Result<Vec<i32>> {
    // Values of each sample
    let mut amplitudes = Vec::with_capacity(reader.len() as usize);
    // Counts how many samples have amplitude < limit_amplitude in a row
    let mut count = 0;

    for n in reader.samples::<i32>() {
        let amplitude = n.unwrap_or(0);
        if amplitude.abs() <= limit_amplitude {
            count += 1;
            if count < duration {
                amplitudes.push(amplitude);
            }
        } else {
            amplitudes.push(amplitude);
            count = 0;
        }
    }

    Ok(amplitudes)
}

fn write_to_path(data: &[i32], spec: WavSpec, path: &Path) -> Result<()> {
    let mut writer = WavWriter::create(path, spec)?;
    data.iter()
        .try_for_each(|sample| writer.write_sample(*sample))?;
    writer.finalize()
}

fn main() {
    let args = Args::parse();
    match WavReader::open(args.input_file) {
        Err(err) => eprintln!("{err}"),
        Ok(mut reader) => match cutblanc(&mut reader, args.limit_amplitude, args.duration) {
            Ok(data) => {
                println!(
                    "Audio duration decreased from {}s to {}s",
                    reader.duration() as f32 / reader.spec().sample_rate as f32,
                    data.len() as f32
                        / reader.spec().channels as f32
                        / reader.spec().sample_rate as f32
                );
                match write_to_path(&data, reader.spec(), args.output_file.as_ref()) {
                    Ok(()) => {
                        println!(
                            "New audio written to {}",
                            Path::new(&args.output_file).to_string_lossy()
                        );
                    }
                    Err(err) => eprintln!("{err}"),
                }
            }
            Err(err) => eprintln!("{err}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_loop() {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create("test.wav", spec).unwrap();
        for t in 0..44100 {
            if t > 250 {
                writer.write_sample(10000 as i16).unwrap();
            } else {
                writer.write_sample(0 as i16).unwrap();
            }
        }

        writer.finalize().unwrap();

        let mut reader = hound::WavReader::open("test.wav").unwrap();
        let cut = cutblanc(&mut reader, 300, 100).unwrap();
        assert_eq!(cut.len(), 44100 - 152)
    }
}
