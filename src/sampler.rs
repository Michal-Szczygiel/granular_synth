pub mod core {
    use std::collections::VecDeque;

    use rand::distributions::{Distribution, Uniform};
    use rand::prelude::{SliceRandom, ThreadRng};
    use rand::thread_rng;
    use serde::Deserialize;

    use crate::audio::tools::AudioBuffer;
    use crate::granular_synth_config::tools::{
        GWFunction, GrainsLength, GrainsPitch, GrainsProperties, SynthConfiguration,
    };

    use rubato::{
        interpolator_avx::AvxInterpolator, InterpolationType, Resampler, SincFixedIn,
        WindowFunction,
    };

    // ------------------------------------------------------------------------------------------------------------------------------------------

    const SINC_LEN: usize = 256;
    const OVERSAMPLING_FACTOR: usize = 256;

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Struktura reprezentująca sampler.
    #[derive(Debug, Deserialize)]
    pub struct Sampler {
        #[serde(skip_deserializing)]
        pub grains_buffer: VecDeque<AudioBuffer>,

        #[serde(skip_deserializing)]
        randomness_source: ThreadRng,
    }

    impl Sampler {
        // Konstruktor domyślnego obiektu klasy Sampler dla serde.
        pub fn default() -> Self {
            return Sampler {
                grains_buffer: VecDeque::new(),
                randomness_source: thread_rng(),
            };
        }

        // Wczytanie sampla i utworzenie granulek.
        pub fn prepare(
            &mut self,
            _synth_configuration: &SynthConfiguration,
            _grains_properties: &GrainsProperties,
        ) -> Result<(), String> {
            let sample_result = AudioBuffer::load_audio(&_grains_properties.sample_file_path);

            // Wczytanie sampla, obsługa błędów długości.
            let (sample_audio_buffer, sample_sampling_rate) = match sample_result {
                Ok((_audio_buffer, _sampling_rate)) => {
                    let sample_length_ms =
                        (_audio_buffer.len() as f64 / _sampling_rate as f64) * 1000.0;

                    match _grains_properties.grains_length_ms {
                        GrainsLength::Fixed { equal } => {
                            if equal * 1.5 > sample_length_ms {
                                return Err(
                                format!(
                                    "Błąd danych ->\n\tsampel: \'{}\' jest za krótki (sampel musi mieć długość co najmniej: 1.5 x \'grains_length_ms\': \'equal\') :/",
                                    _grains_properties.sample_file_path
                                )
                            );
                            }
                        }
                        GrainsLength::Range { from: _, to } => {
                            if to * 1.5 > sample_length_ms {
                                return Err(
                                format!(
                                    "Błąd danych ->\n\tsampel: \'{}\' jest za krótki (sampel musi mieć długość co najmniej: 1.5 x \'grains_length_ms\': \'to\') :/",
                                    _grains_properties.sample_file_path
                                )
                            );
                            }
                        }
                    };

                    (_audio_buffer, _sampling_rate)
                }
                Err(_error) => {
                    return Err(_error);
                }
            };

            // Utworzenie granulek dla każdego modelu modyfikacji wysokości dźwięku.
            self.grains_buffer.reserve(_grains_properties.grains_count);

            match &_grains_properties.grains_pitch {
                GrainsPitch::Fixed => {
                    let resampled_audio_buffer: AudioBuffer = self.resample_audio(
                        &sample_audio_buffer,
                        sample_sampling_rate,
                        1.0,
                        _synth_configuration,
                    );

                    for _ in 0.._grains_properties.grains_count {
                        let grain = self.get_random_grain(
                            &resampled_audio_buffer,
                            _synth_configuration,
                            _grains_properties,
                        );
                        self.grains_buffer.push_back(grain);
                    }
                }
                GrainsPitch::Steps { steps } => {
                    for (_pitch, _fraction) in steps.iter() {
                        let resampled_buffer: AudioBuffer = self.resample_audio(
                            &sample_audio_buffer,
                            sample_sampling_rate,
                            *_pitch,
                            _synth_configuration,
                        );
                        let grains_count: usize = (_grains_properties.grains_count as f64
                            * (*_fraction / 100.0))
                            .round() as usize;

                        for _ in 0..grains_count {
                            let grain = self.get_random_grain(
                                &resampled_buffer,
                                _synth_configuration,
                                _grains_properties,
                            );

                            self.grains_buffer.push_back(grain);
                        }
                    }

                    self.grains_buffer
                        .make_contiguous()
                        .shuffle(&mut self.randomness_source);
                }
            }

            return Ok(());
        }

        // Zwraca losowo wybraną granulkę z bufora.
        pub fn sample(&mut self) -> &AudioBuffer {
            let half_buffer: usize = self.grains_buffer.len() / 2;
            let pick_distr = Uniform::new(0, half_buffer);

            let grain = self
                .grains_buffer
                .remove(pick_distr.sample(&mut self.randomness_source))
                .unwrap();
            self.grains_buffer.push_back(grain);

            return self.grains_buffer.back().unwrap();
        }

        // Zmiana częstotliwości samplowania i/lub wysokości dźwięku.
        fn resample_audio(
            &self,
            _audio_buffer: &AudioBuffer,
            _audio_sampling_rate: u32,
            _pitch: f64,
            _synth_configuration: &SynthConfiguration,
        ) -> AudioBuffer {
            let interpolator = AvxInterpolator::<f64>::new(
                SINC_LEN,
                OVERSAMPLING_FACTOR,
                0.95,
                WindowFunction::BlackmanHarris2,
            )
            .unwrap();

            let resampled_audio_buffer: AudioBuffer = match _audio_buffer {
                AudioBuffer::Mono(_buffer) => {
                    let mut resampler = SincFixedIn::<f64>::new_with_interpolator(
                        _synth_configuration.engine_sampling_rate as f64
                            / (_audio_sampling_rate as f64 * _pitch),
                        InterpolationType::Cubic,
                        Box::new(interpolator),
                        _audio_buffer.len(),
                        1,
                    );

                    let resampled_audio_buffer = resampler.process(_buffer).unwrap().pop().unwrap();
                    AudioBuffer::Mono([resampled_audio_buffer])
                }
                AudioBuffer::Stereo(_buffer) => {
                    let mut resampler = SincFixedIn::<f64>::new_with_interpolator(
                        _synth_configuration.engine_sampling_rate as f64
                            / (_audio_sampling_rate as f64 * _pitch),
                        InterpolationType::Cubic,
                        Box::new(interpolator),
                        _audio_buffer.len(),
                        2,
                    );

                    let mut resampled_audio_buffer = resampler.process(_buffer).unwrap();
                    let resampled_audio_buffer_rigth = resampled_audio_buffer.pop().unwrap();
                    let resampled_audio_buffer_left = resampled_audio_buffer.pop().unwrap();

                    AudioBuffer::Stereo([resampled_audio_buffer_left, resampled_audio_buffer_rigth])
                }
            };

            return resampled_audio_buffer;
        }

        // Wycięcie granulki o zadanej długości z losowego miejsca sampla.
        fn get_random_grain(
            &mut self,
            _sample: &AudioBuffer,
            _synth_configuration: &SynthConfiguration,
            _grains_properties: &GrainsProperties,
        ) -> AudioBuffer {
            match _grains_properties.grains_length_ms {
                GrainsLength::Fixed { equal } => {
                    let grain_length: usize = ((equal / 1000.0)
                        * _synth_configuration.engine_sampling_rate as f64)
                        .round() as usize;

                    match _sample {
                        AudioBuffer::Mono(_buffer) => {
                            let mut windows = _buffer[0].windows(grain_length);

                            let windows_number = windows.len();
                            let nth_distr = Uniform::new(0, windows_number);
                            let window_index = nth_distr.sample(&mut self.randomness_source);

                            let mut grain = windows.nth(window_index).unwrap().to_vec();

                            match _grains_properties.window_function {
                                GWFunction::SmoothstepRegular { slope } => {
                                    self.smoothsteep_regular(
                                        &mut grain,
                                        slope,
                                        _synth_configuration,
                                    );
                                }
                                GWFunction::SmoothstepUnregular {
                                    slope_attack,
                                    slope_release,
                                } => {
                                    self.smoothsteep_unregular(
                                        &mut grain,
                                        slope_attack,
                                        slope_release,
                                        _synth_configuration,
                                    );
                                }
                            }

                            let mut grain_output = AudioBuffer::Mono([grain]);

                            if _grains_properties.grains_laudness_normalization == true {
                                grain_output.normalize(1.0);
                            }

                            return grain_output;
                        }
                        AudioBuffer::Stereo(_buffer) => {
                            let mut windows_left = _buffer[0].windows(grain_length);
                            let mut windows_rigth = _buffer[1].windows(grain_length);

                            let windows_number = windows_left.len();
                            let nth_distr = Uniform::new(0, windows_number);
                            let window_index = nth_distr.sample(&mut self.randomness_source);

                            let mut grain_left = windows_left.nth(window_index).unwrap().to_vec();
                            let mut grain_rigth = windows_rigth.nth(window_index).unwrap().to_vec();

                            match _grains_properties.window_function {
                                GWFunction::SmoothstepRegular { slope } => {
                                    self.smoothsteep_regular(
                                        &mut grain_left,
                                        slope,
                                        _synth_configuration,
                                    );
                                    self.smoothsteep_regular(
                                        &mut grain_rigth,
                                        slope,
                                        _synth_configuration,
                                    );
                                }
                                GWFunction::SmoothstepUnregular {
                                    slope_attack,
                                    slope_release,
                                } => {
                                    self.smoothsteep_unregular(
                                        &mut grain_left,
                                        slope_attack,
                                        slope_release,
                                        _synth_configuration,
                                    );
                                    self.smoothsteep_unregular(
                                        &mut grain_rigth,
                                        slope_attack,
                                        slope_release,
                                        _synth_configuration,
                                    );
                                }
                            };

                            let mut grain_output = AudioBuffer::Stereo([grain_left, grain_rigth]);

                            if _grains_properties.grains_laudness_normalization == true {
                                grain_output.normalize(1.0);
                            }

                            return grain_output;
                        }
                    }
                }
                GrainsLength::Range { from, to } => {
                    let grain_length_short: usize = ((from / 1000.0)
                        * _synth_configuration.engine_sampling_rate as f64)
                        .round() as usize;
                    let grain_length_long: usize = ((to / 1000.0)
                        * _synth_configuration.engine_sampling_rate as f64)
                        .round() as usize;

                    let length_distr = Uniform::new(grain_length_short, grain_length_long);
                    let grain_length = length_distr.sample(&mut self.randomness_source);

                    match _sample {
                        AudioBuffer::Mono(_buffer) => {
                            let mut windows = _buffer[0].windows(grain_length);

                            let windows_number = windows.len();
                            let nth_distr = Uniform::new(0, windows_number);
                            let window_index = nth_distr.sample(&mut self.randomness_source);

                            let mut grain = windows.nth(window_index).unwrap().to_vec();

                            match _grains_properties.window_function {
                                GWFunction::SmoothstepRegular { slope } => {
                                    self.smoothsteep_regular(
                                        &mut grain,
                                        slope,
                                        _synth_configuration,
                                    );
                                }
                                GWFunction::SmoothstepUnregular {
                                    slope_attack,
                                    slope_release,
                                } => {
                                    self.smoothsteep_unregular(
                                        &mut grain,
                                        slope_attack,
                                        slope_release,
                                        _synth_configuration,
                                    );
                                }
                            }

                            let mut grain_output = AudioBuffer::Mono([grain]);

                            if _grains_properties.grains_laudness_normalization == true {
                                grain_output.normalize(1.0);
                            }

                            return grain_output;
                        }
                        AudioBuffer::Stereo(_buffer) => {
                            let mut windows_left = _buffer[0].windows(grain_length);
                            let mut windows_rigth = _buffer[1].windows(grain_length);

                            let windows_number = windows_left.len();
                            let nth_distr = Uniform::new(0, windows_number);
                            let window_index = nth_distr.sample(&mut self.randomness_source);

                            let mut grain_left = windows_left.nth(window_index).unwrap().to_vec();
                            let mut grain_rigth = windows_rigth.nth(window_index).unwrap().to_vec();

                            match _grains_properties.window_function {
                                GWFunction::SmoothstepRegular { slope } => {
                                    self.smoothsteep_regular(
                                        &mut grain_left,
                                        slope,
                                        _synth_configuration,
                                    );
                                    self.smoothsteep_regular(
                                        &mut grain_rigth,
                                        slope,
                                        _synth_configuration,
                                    );
                                }
                                GWFunction::SmoothstepUnregular {
                                    slope_attack,
                                    slope_release,
                                } => {
                                    self.smoothsteep_unregular(
                                        &mut grain_left,
                                        slope_attack,
                                        slope_release,
                                        _synth_configuration,
                                    );
                                    self.smoothsteep_unregular(
                                        &mut grain_rigth,
                                        slope_attack,
                                        slope_release,
                                        _synth_configuration,
                                    );
                                }
                            };

                            let mut grain_output = AudioBuffer::Stereo([grain_left, grain_rigth]);

                            if _grains_properties.grains_laudness_normalization == true {
                                grain_output.normalize(1.0);
                            }

                            return grain_output;
                        }
                    }
                }
            }
        }

        // ------------------------------------------------------------------------------------------------------------------------------------------

        // Funkcja okna czasowego smoothstep w wersji regularnej.
        fn smoothsteep_regular(
            &self,
            _buffer: &mut Vec<f64>,
            _slope: f64,
            _synth_configuration: &SynthConfiguration,
        ) {
            let slope_length: f64 =
                (_slope / 1000.0) * _synth_configuration.engine_sampling_rate as f64;
            let curvature: f64 = 1.0 / slope_length;

            for (_index, _sample_value) in _buffer.iter_mut().enumerate() {
                if _index > slope_length as usize {
                    break;
                }

                *_sample_value = *_sample_value
                    * (6.0 * (curvature * _index as f64).powi(5)
                        - 15.0 * (curvature * _index as f64).powi(4)
                        + 10.0 * (curvature * _index as f64).powi(3));
            }

            for (_index, _sample_value) in _buffer.iter_mut().rev().enumerate() {
                if _index > slope_length as usize {
                    break;
                }

                *_sample_value = *_sample_value
                    * (6.0 * (curvature * _index as f64).powi(5)
                        - 15.0 * (curvature * _index as f64).powi(4)
                        + 10.0 * (curvature * _index as f64).powi(3));
            }
        }

        // Funkcja okna czasowego smoothstep w wersji nieregularnej.
        fn smoothsteep_unregular(
            &self,
            _buffer: &mut Vec<f64>,
            _slope_attack: f64,
            _slope_release: f64,
            _synth_configuration: &SynthConfiguration,
        ) {
            let slope_attack_length: f64 =
                (_slope_attack / 1000.0) * _synth_configuration.engine_sampling_rate as f64;
            let curvature_attack: f64 = 1.0 / slope_attack_length;

            let slope_release_length: f64 =
                (_slope_release / 1000.0) * _synth_configuration.engine_sampling_rate as f64;
            let curvature_release: f64 = 1.0 / slope_release_length;

            for (_index, _sample_value) in _buffer.iter_mut().enumerate() {
                if _index > slope_attack_length as usize {
                    break;
                }

                *_sample_value = *_sample_value
                    * (6.0 * (curvature_attack * _index as f64).powi(5)
                        - 15.0 * (curvature_attack * _index as f64).powi(4)
                        + 10.0 * (curvature_attack * _index as f64).powi(3));
            }

            for (_index, _sample_value) in _buffer.iter_mut().rev().enumerate() {
                if _index > slope_release_length as usize {
                    break;
                }

                *_sample_value = *_sample_value
                    * (6.0 * (curvature_release * _index as f64).powi(5)
                        - 15.0 * (curvature_release * _index as f64).powi(4)
                        + 10.0 * (curvature_release * _index as f64).powi(3));
            }
        }
    }
}
