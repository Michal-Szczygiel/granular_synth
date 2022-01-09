pub mod tools {
    use std::fs::File;
    use std::path::Path;

    use std::i16;
    use std::i32;

    use serde::Deserialize;
    use wav::{BitDepth, Header, WAV_FORMAT_IEEE_FLOAT, WAV_FORMAT_PCM};

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Bufor audio, występuje w dwóch rodzajach: mono i stereo.
    #[derive(Debug, Clone, Deserialize)]
    pub enum AudioBuffer {
        Mono([Vec<f64>; 1]),
        Stereo([Vec<f64>; 2]),
    }

    impl AudioBuffer {
        // Domyślny konstruktor dla serde.
        pub fn default() -> Self {
            return AudioBuffer::Stereo([vec![], vec![]]);
        }

        // Zwraca długość bufora niezależnie od typu mono/stereo.
        pub fn len(&self) -> usize {
            match self {
                AudioBuffer::Mono(_buffer) => {
                    return _buffer[0].len();
                }
                AudioBuffer::Stereo(_buffer) => {
                    return _buffer[0].len();
                }
            }
        }

        // Alokuje bufor o zadanym rozmiarze i wypełnia go zerami.
        pub fn blank(&mut self, _size: usize) {
            match self {
                AudioBuffer::Mono(_buffer) => {
                    _buffer[0] = vec![0.0; _size];
                }
                AudioBuffer::Stereo(_buffer) => {
                    _buffer[0] = vec![0.0; _size];
                    _buffer[1] = vec![0.0; _size];
                }
            }
        }

        // Zwraca lewy kanał bufora stereo.
        pub fn left(&mut self) -> &mut Vec<f64> {
            match self {
                AudioBuffer::Stereo(_buffer) => {
                    return &mut _buffer[0];
                }
                _ => panic!("Wywołanie \'left\' na buforze mono."),
            }
        }

        // Zwraca prawy kanał bufora stereo.
        pub fn rigth(&mut self) -> &mut Vec<f64> {
            match self {
                AudioBuffer::Stereo(_buffer) => {
                    return &mut _buffer[1];
                }
                _ => panic!("Wywołanie \'rigth\' na buforze mono."),
            }
        }

        // Zwraca cały bufor stereo jako tablicę.
        pub fn stereo(&self) -> &[Vec<f64>; 2] {
            match self {
                AudioBuffer::Stereo(_buffer) => {
                    return &_buffer;
                }
                _ => panic!(),
            }
        }

        // Normalizuje bufor do zadanego poziomu z przedziału: 0.0 - 1.0.
        pub fn normalize(&mut self, _level: f64) {
            match self {
                // --- Mono --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- ---
                AudioBuffer::Mono(_buffer) => {
                    let max_level_abs: f64 = _buffer[0]
                        .iter()
                        .max_by(|a, b| a.partial_cmp(&b).unwrap())
                        .unwrap()
                        .abs();
                    let min_level_abs: f64 = _buffer[0]
                        .iter()
                        .min_by(|a, b| a.partial_cmp(&b).unwrap())
                        .unwrap()
                        .abs();

                    let max_deviation: f64 = if max_level_abs > min_level_abs {
                        max_level_abs
                    } else {
                        min_level_abs
                    };

                    _buffer[0].iter_mut().for_each(|_sample_value| {
                        *_sample_value = (_level * *_sample_value) / max_deviation
                    });
                }
                // --- Stereo --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- ---
                AudioBuffer::Stereo(_buffer) => {
                    let max_level_abs_left: f64 = _buffer[0]
                        .iter()
                        .max_by(|a, b| a.partial_cmp(&b).unwrap())
                        .unwrap()
                        .abs();
                    let min_level_abs_left: f64 = _buffer[0]
                        .iter()
                        .min_by(|a, b| a.partial_cmp(&b).unwrap())
                        .unwrap()
                        .abs();

                    let max_deviation_left: f64 = if max_level_abs_left > min_level_abs_left {
                        max_level_abs_left
                    } else {
                        min_level_abs_left
                    };

                    let max_level_abs_right: f64 = _buffer[1]
                        .iter()
                        .max_by(|a, b| a.partial_cmp(&b).unwrap())
                        .unwrap()
                        .abs();
                    let min_level_abs_right: f64 = _buffer[1]
                        .iter()
                        .min_by(|a, b| a.partial_cmp(&b).unwrap())
                        .unwrap()
                        .abs();

                    let max_deviation_right: f64 = if max_level_abs_right > min_level_abs_right {
                        max_level_abs_right
                    } else {
                        min_level_abs_right
                    };

                    let max_deviation: f64 = if max_deviation_left > max_deviation_right {
                        max_deviation_left
                    } else {
                        max_deviation_right
                    };

                    _buffer[0].iter_mut().for_each(|_sample_value| {
                        *_sample_value = (_level * *_sample_value) / max_deviation
                    });
                    _buffer[1].iter_mut().for_each(|_sample_value| {
                        *_sample_value = (_level * *_sample_value) / max_deviation
                    });
                }
            }
        }

        // Zapisuje zawartość bufora do pliku o zadanej nazwie i z zadaną częstotliwością próbkowania i głębią bitową.
        pub fn save_audio(
            &self,
            _output_file_path: &String,
            _sampling_rate: u32,
            _byte_rate: u16,
        ) -> Result<(), String> {
            let output_file_result = File::create(Path::new(_output_file_path));

            let mut output_file = match output_file_result {
                Ok(_output_file) => _output_file,
                Err(_system_error) => {
                    return Err(format!(
                        "Błąd ->\n\tnie można utworzyć pliku \'{}\'.\n\tSystem error: {} :/",
                        _output_file_path, _system_error
                    ))
                }
            };

            match (self, _byte_rate) {
                // --- Mono --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- ---
                (AudioBuffer::Mono(_samples), 16) => {
                    let header = Header::new(WAV_FORMAT_PCM, 1, _sampling_rate, 16);
                    let bytes: Vec<i16> = _samples[0]
                        .iter()
                        .map(|_sample_value| (*_sample_value * i16::MAX as f64).round() as i16)
                        .collect();

                    wav::write(header, &BitDepth::Sixteen(bytes), &mut output_file).unwrap();
                }
                (AudioBuffer::Mono(_samples), 24) => {
                    let header = Header::new(WAV_FORMAT_PCM, 1, _sampling_rate, 24);
                    let bytes: Vec<i32> = _samples[0]
                        .iter()
                        .map(|_sample_value| (*_sample_value * i32::MAX as f64).round() as i32)
                        .collect();

                    wav::write(header, &BitDepth::TwentyFour(bytes), &mut output_file).unwrap();
                }
                (AudioBuffer::Mono(_samples), 32) => {
                    let header = Header::new(WAV_FORMAT_IEEE_FLOAT, 1, _sampling_rate, 32);
                    let bytes: Vec<f32> = _samples[0]
                        .iter()
                        .map(|_sample_value| *_sample_value as f32)
                        .collect();

                    wav::write(header, &BitDepth::ThirtyTwoFloat(bytes), &mut output_file).unwrap();
                }

                // --- Stereo --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- ---
                (AudioBuffer::Stereo(_samples), 16) => {
                    let header = Header::new(WAV_FORMAT_PCM, 2, _sampling_rate, 16);
                    let mut bytes: Vec<i16> = Vec::with_capacity(_samples[0].len() * 2);

                    _samples[0].iter().zip(&_samples[1]).for_each(
                        |(_sample_value_left, _sample_value_right)| {
                            bytes.push((*_sample_value_left * i16::MAX as f64).round() as i16);
                            bytes.push((*_sample_value_right * i16::MAX as f64).round() as i16);
                        },
                    );

                    wav::write(header, &BitDepth::Sixteen(bytes), &mut output_file).unwrap();
                }
                (AudioBuffer::Stereo(_samples), 24) => {
                    let header = Header::new(WAV_FORMAT_PCM, 2, _sampling_rate, 24);
                    let mut bytes: Vec<i32> = Vec::with_capacity(_samples[0].len() * 2);

                    _samples[0].iter().zip(&_samples[1]).for_each(
                        |(_sample_value_left, _sample_value_right)| {
                            bytes.push((*_sample_value_left * i32::MAX as f64).round() as i32);
                            bytes.push((*_sample_value_right * i32::MAX as f64).round() as i32);
                        },
                    );

                    wav::write(header, &BitDepth::TwentyFour(bytes), &mut output_file).unwrap();
                }
                (AudioBuffer::Stereo(_samples), 32) => {
                    let header = Header::new(WAV_FORMAT_IEEE_FLOAT, 2, _sampling_rate, 32);
                    let mut bytes: Vec<f32> = Vec::with_capacity(_samples[0].len() * 2);

                    _samples[0].iter().zip(&_samples[1]).for_each(
                        |(_sample_value_left, _sample_value_right)| {
                            bytes.push(*_sample_value_left as f32);
                            bytes.push(*_sample_value_right as f32);
                        },
                    );

                    wav::write(header, &BitDepth::ThirtyTwoFloat(bytes), &mut output_file).unwrap();
                }
                _ => {}
            }

            return Ok(());
        }

        // Wczytuje zawartość pliku .wav do bufora odpowiedniego typu.
        pub fn load_audio(_sample_file_path: &String) -> Result<(AudioBuffer, u32), String> {
            let sample_file_result = File::open(Path::new(_sample_file_path));
    
            let mut sample_file = match sample_file_result {
                Ok(_sample_file) => _sample_file,
                Err(_system_error) => {
                    return Err(format!(
                        "Błąd odczytu ->\n\tplik: \'{}\' nie został znaleziony.\n\tSystem error: {} :/",
                        _sample_file_path, _system_error
                    ))
                }
            };
    
            let file_content_result = wav::read(&mut sample_file);
    
            let (header, buffer) = match file_content_result {
                Ok((_header, _buffer)) => (_header, _buffer),
                Err(_wav_error) => {
                    return Err(format!(
                        "Błąd odczytu ->\n\tnie można odczytać danych dźwiękowych z pliku: \'{}\'.\n\tWAV error: {} :/",
                        _sample_file_path, _wav_error
                    ))
                }
            };
    
            let normalized_audio_buffer: AudioBuffer = match (buffer, header.channel_count) {
                // --- Mono --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- ---
                (BitDepth::Sixteen(_samples), 1) => AudioBuffer::Mono([_samples
                    .iter()
                    .map(|_sample_value| *_sample_value as f64 / i16::MAX as f64)
                    .collect()]),
                (BitDepth::TwentyFour(_samples), 1) => AudioBuffer::Mono([_samples
                    .iter()
                    .map(|_sample_value| *_sample_value as f64 / i32::MAX as f64)
                    .collect()]),
                (BitDepth::ThirtyTwoFloat(_samples), 1) => AudioBuffer::Mono([_samples
                    .iter()
                    .map(|_sample_value| *_sample_value as f64)
                    .collect()]),
    
                // --- Stereo --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- ---
                (BitDepth::Sixteen(_samples), 2) => AudioBuffer::Stereo([
                    _samples
                        .iter()
                        .step_by(2)
                        .map(|_sample_value| *_sample_value as f64 / i16::MAX as f64)
                        .collect(),
                    _samples
                        .iter()
                        .skip(1)
                        .step_by(2)
                        .map(|_sample_value| *_sample_value as f64 / i16::MAX as f64)
                        .collect(),
                ]),
                (BitDepth::TwentyFour(_samples), 2) => AudioBuffer::Stereo([
                    _samples
                        .iter()
                        .step_by(2)
                        .map(|_sample_value| *_sample_value as f64 / i32::MAX as f64)
                        .collect(),
                    _samples
                        .iter()
                        .skip(1)
                        .step_by(2)
                        .map(|_sample_value| *_sample_value as f64 / i32::MAX as f64)
                        .collect(),
                ]),
                (BitDepth::ThirtyTwoFloat(_samples), 2) => AudioBuffer::Stereo([
                    _samples
                        .iter()
                        .step_by(2)
                        .map(|_sample_value| *_sample_value as f64)
                        .collect(),
                    _samples
                        .iter()
                        .skip(1)
                        .step_by(2)
                        .map(|_sample_value| *_sample_value as f64)
                        .collect(),
                ]),
    
                // --- Error --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- --- ---
                _ => {
                    return Err(format!(
                        "Błąd danych ->\n\tplik: \'{}\' zawiera nieobsługiwany format audio :/",
                        _sample_file_path
                    ))
                }
            };
    
            return Ok((normalized_audio_buffer, header.sampling_rate));
        }
    }
}
