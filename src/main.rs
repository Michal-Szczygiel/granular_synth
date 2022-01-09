mod audio;
mod granular_synth;
mod granular_synth_config;
mod sampler;
mod sequencer;

use granular_synth::synth::GranularSynth;

fn synthesize_tracks() -> Result<(), Vec<String>> {
    let json_file_path: String = match std::env::args().nth(1) {
        Some(_json_file_path) => _json_file_path,
        None => {
            return Err(vec![
                "Błąd danych ->\n\tnie podano ścieżki do pliku konfiguracyjnego (wymagana ścieżka do odpowiedniego pliku \'.json\') :/".to_string()
            ])
        }
    };

    let mut granular_synth = match GranularSynth::configure(&json_file_path) {
        Ok(_granular_synth) => _granular_synth,
        Err(_errors) => {
            return Err(_errors);
        }
    };

    match granular_synth.run() {
        Err(_error) => {
            return Err(vec![_error]);
        }
        _ => {}
    }

    match granular_synth.save_tracks() {
        Err(_error) => {
            return Err(vec![_error]);
        }
        _ => {}
    }

    return Ok(());
}

fn main() {
    match synthesize_tracks() {
        Err(_errors) => {
            for error in _errors.iter() {
                println!("{}", error);
            }
        },
        _ => {}
    }
}
