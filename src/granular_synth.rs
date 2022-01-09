pub mod synth {
    use rubato::{
        interpolator_avx::AvxInterpolator, InterpolationType, Resampler, SincFixedIn,
        WindowFunction,
    };

    use crate::audio::tools::AudioBuffer;
    use crate::granular_synth_config::tools::{
        load_json_file, load_tracks_configurations, SynthConfiguration, Track,
    };

    // ------------------------------------------------------------------------------------------------------------------------------------------

    const SINC_LEN: usize = 256;
    const OVERSAMPLING_FACTOR: usize = 256;

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Syntezator granularny.
    #[derive(Debug)]
    pub struct GranularSynth {
        pub synth_configuration: SynthConfiguration,
        pub tracks: Vec<Track>,
    }

    impl GranularSynth {
        // Wczytuje plik konfiguracyjny .json i sprawdza poprawność danych.
        pub fn configure(_json_file_path: &String) -> Result<GranularSynth, Vec<String>> {
            let json_file_value_result = load_json_file(_json_file_path);

            let json_file_value = match json_file_value_result {
                Ok(_json_file_value) => _json_file_value,
                Err(_error) => {
                    return Err(vec![_error]);
                }
            };

            let synth_configuration_result = SynthConfiguration::load(&json_file_value);

            let _synth_configuration = match synth_configuration_result {
                Ok(_synth_configuration) => _synth_configuration,
                Err(_errors) => {
                    return Err(_errors);
                }
            };

            let tracks_result = load_tracks_configurations(&json_file_value, &_synth_configuration);

            let _tracks = match tracks_result {
                Ok(_tracks_configurations) => _tracks_configurations,
                Err(_errors) => {
                    return Err(_errors);
                }
            };

            // Wypisanie podsumowania wczytanego pliku konfiguracyjnego.
            println!(
                "Konfiguracja syntezatora:\n\t# długość beatu: {} ms\n\t# częstotliwość próbkowania silnika: {} Hz\n\t# katalog wyjściowy: \'{}\'\n\t# wyjściowa częstotliwość próbkowania: {} Hz\n\t# wyjściowa głębia bitowa: {} bit",
                _synth_configuration.beat_length_ms, _synth_configuration.engine_sampling_rate, _synth_configuration.output_directory, _synth_configuration.output_sampling_rate, _synth_configuration.output_bit_depth
            );

            println!("\nŚcieżki:");

            for (_track_number, _track) in _tracks.iter().enumerate() {
                println!(
                    "\t# ścieżka [{}] -> nazwa: \'{}\', sampel: \'{}\'",
                    _track_number + 1, _track.track_properties.track_name, _track.grains_properties.sample_file_path
                );
            }

            return Ok(GranularSynth {
                synth_configuration: _synth_configuration,
                tracks: _tracks,
            });
        }

        // Wczytuje sample, dzieli je na granulki i syntetyzuje ścieżki dźwiękowe.
        pub fn run(&mut self) -> Result<(), String> {
            for track in self.tracks.iter_mut() {
                match track
                    .sampler
                    .prepare(&self.synth_configuration, &track.grains_properties)
                {
                    Err(_error) => {
                        return Err(_error);
                    }
                    _ => {}
                }

                track
                    .sequencer
                    .generate_sequence(&track.beat_sequence, &self.synth_configuration);

                let beat_size = ((self.synth_configuration.beat_length_ms as f64 / 1000.0)
                    * self.synth_configuration.engine_sampling_rate as f64)
                    .round() as usize;
                let canva_size = beat_size * (track.beat_sequence.len() + 2);

                track.canva.blank(canva_size);

                let left_volume_track: f64;
                let rigth_volume_track: f64;

                if track.track_properties.track_panorama < 0.0 {
                    left_volume_track = 1.0;
                    rigth_volume_track = 1.0 + track.track_properties.track_panorama;
                } else {
                    left_volume_track = 1.0 - track.track_properties.track_panorama;
                    rigth_volume_track = 1.0;
                }

                for event in track.sequencer.sequence.iter() {
                    let grain = track.sampler.sample();

                    let left_volume: f64;
                    let rigth_volume: f64;

                    if event.panorama < 0.0 {
                        left_volume = 1.0;
                        rigth_volume = 1.0 + event.panorama;
                    } else {
                        left_volume = 1.0 - event.panorama;
                        rigth_volume = 1.0;
                    }

                    match grain {
                        AudioBuffer::Mono(_buffer) => {
                            track
                                .canva
                                .left()
                                .iter_mut()
                                .skip(event.start_index)
                                .zip(_buffer[0].iter())
                                .for_each(|(_canva_sample, _grain_sample)| {
                                    *_canva_sample += *_grain_sample * event.volume * left_volume * left_volume_track
                                });
                            track
                                .canva
                                .rigth()
                                .iter_mut()
                                .skip(event.start_index)
                                .zip(_buffer[0].iter())
                                .for_each(|(_canva_sample, _grain_sample)| {
                                    *_canva_sample += *_grain_sample * event.volume * rigth_volume * rigth_volume_track
                                });
                        }
                        AudioBuffer::Stereo(_buffer) => {
                            track
                                .canva
                                .left()
                                .iter_mut()
                                .skip(event.start_index)
                                .zip(_buffer[0].iter())
                                .for_each(|(_canva_sample, _grain_sample)| {
                                    *_canva_sample += *_grain_sample * event.volume * left_volume * left_volume_track
                                });
                            track
                                .canva
                                .rigth()
                                .iter_mut()
                                .skip(event.start_index)
                                .zip(_buffer[1].iter())
                                .for_each(|(_canva_sample, _grain_sample)| {
                                    *_canva_sample += *_grain_sample * event.volume * rigth_volume * rigth_volume_track
                                });
                        }
                    }
                }

                track.canva.normalize(track.track_properties.track_normalization_level);
            }

            return Ok(());
        }

        // Zapisuję zsyntetyzowane ścieżki do plików dźwiękowych.
        pub fn save_tracks(&self) -> Result<(), String> {
            println!("\nWyjście:");

            for track in self.tracks.iter() {
                if self.synth_configuration.engine_sampling_rate
                    == self.synth_configuration.output_sampling_rate
                {
                    match track.canva.save_audio(
                        &format!("{}/{}.wav", self.synth_configuration.output_directory, track.track_properties.track_name),
                        self.synth_configuration.output_sampling_rate,
                        self.synth_configuration.output_bit_depth,
                    ) {
                        Err(_error) => {
                            return Err(_error);
                        }
                        _ => {}
                    }
                } else {
                    let interpolator = AvxInterpolator::<f64>::new(
                        SINC_LEN,
                        OVERSAMPLING_FACTOR,
                        0.95,
                        WindowFunction::BlackmanHarris2,
                    )
                    .unwrap();

                    let mut resampler = SincFixedIn::<f64>::new_with_interpolator(
                        self.synth_configuration.output_sampling_rate as f64 / self.synth_configuration.engine_sampling_rate as f64,
                        InterpolationType::Cubic,
                        Box::new(interpolator),
                        track.canva.len(),
                        2,
                    );

                    let mut resampled_audio = resampler.process(track.canva.stereo()).unwrap();
                    let resampled_audio_rigth = resampled_audio.pop().unwrap();
                    let resampled_audio_left = resampled_audio.pop().unwrap();

                    let audio_buffer_output =
                        AudioBuffer::Stereo([resampled_audio_left, resampled_audio_rigth]);

                    match audio_buffer_output.save_audio(
                        &format!("{}/{}.wav", self.synth_configuration.output_directory, track.track_properties.track_name),
                        self.synth_configuration.output_sampling_rate,
                        self.synth_configuration.output_bit_depth,
                    ) {
                        Err(_error) => {
                            return Err(_error);
                        }
                        _ => {}
                    }
                }

                println!(
                    "\tzapisano ścieżkę: \'{0}\' jako: \'{0}.wav\' w katalogu \'{1}\'",
                    track.track_properties.track_name, self.synth_configuration.output_directory
                );
            }

            return Ok(());
        }
    }
}
