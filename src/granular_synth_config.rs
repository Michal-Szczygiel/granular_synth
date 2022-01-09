pub mod tools {
    use std::collections::HashMap;
    use std::fs::read_to_string;
    use std::path::Path;

    use serde::Deserialize;
    use serde_json::{from_str, from_value, Value};

    use crate::audio::tools::AudioBuffer;
    use crate::sampler::core::Sampler;
    use crate::sequencer::core::Sequencer;

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Konfiguracja syntezatora.
    #[derive(Debug, Deserialize)]
    pub struct SynthConfiguration {
        pub beat_length_ms: f64,
        pub engine_sampling_rate: u32,
        pub output_directory: String,
        pub output_sampling_rate: u32,
        pub output_bit_depth: u16,
    }

    impl SynthConfiguration {
        // Buduje konfigurację na podstawie obiektu Value i sprawdza poprawność wczytanych danych.
        pub fn load(_json_file_value: &Value) -> Result<Self, Vec<String>> {
            let synth_config_result: Result<SynthConfiguration, _> =
                from_value(_json_file_value["SynthConfiguration"].clone());

            let mut errors: Vec<String> = Vec::new();

            let synth_config: SynthConfiguration = match synth_config_result {
                Ok(_synth_config) => _synth_config,
                Err(_serde_error) => {
                    return Err(vec![format!("Serializer Error ->\n\t{} :/", _serde_error)])
                }
            };

            if synth_config.beat_length_ms < 100.0 || synth_config.beat_length_ms > 30_000.0 {
                errors.push(
                    "Błąd danych - \'SynthConfiguration\' ->\n\tnieprawidłowa wartość zmiennej: \'beat_length_ms\' (100.0 ... 30 000.0) :/"
                    .to_string()
                );
            }
            if synth_config.engine_sampling_rate < 48_000
                || synth_config.engine_sampling_rate > 384_000
            {
                errors.push(
                    "Błąd danych - \'SynthConfiguration\' ->\n\tnieprawidłowa wartość zmiennej: \'engine_sampling_rate\' (48 000 ... 384 000) :/"
                    .to_string());
            }
            if synth_config.output_sampling_rate < 48_000
                || synth_config.output_sampling_rate > 384_000
            {
                errors.push(
                    "Błąd danych - \'SynthConfiguration\' ->\n\tnieprawidłowa wartość zmiennej: \'output_sampling_rate\' (48 000 ... 384 000) :/"
                    .to_string());
            }
            if matches!(synth_config.output_bit_depth, 8 | 16 | 24 | 32) == false {
                errors.push(
                    "Błąd danych - \'SynthConfiguration\' ->\n\tnieprawidłowa wartość zmiennej: \'output_bit_depth\' (8, 16, 24, 32) :/"
                    .to_string());
            }

            if errors.is_empty() == true {
                return Ok(synth_config);
            } else {
                return Err(errors);
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Właściwoci ścieżki.
    #[derive(Debug, Deserialize)]
    pub struct TrackProperties {
        pub track_name: String,
        pub track_normalization_level: f64,
        pub track_panorama: f64,
    }

    impl TrackProperties {
        // Sprawdza poprawność wczytanych danych konfiguracyjnych ścieżki.
        fn validate(&self, _track_number: usize) -> Result<(), Vec<String>> {
            let mut errors: Vec<String> = Vec::new();

            if self.track_name.is_empty() == true {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - \'track_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'track_name\' (\'track_name\' nie może być pusty) :/",
                        _track_number
                    )
                )
            }
            if self.track_normalization_level < 0.0 || self.track_normalization_level > 1.0 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - \'track_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'track_normalization_level\' (0.0 < \'track_normalization_level\' < 1.0) :/",
                        _track_number
                    )
                )
            }
            if self.track_panorama < -1.0 || self.track_panorama > 1.0 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - \'track_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'track_panorama\' (-1.0 < \'track_panorama\' < 1.0) :/",
                        _track_number
                    )
                )
            }

            if errors.is_empty() == true {
                return Ok(());
            } else {
                return Err(errors);
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Typ enumeracyjny długości granulki.
    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    pub enum GrainsLength {
        Fixed { equal: f64 },
        Range { from: f64, to: f64 },
    }

    impl GrainsLength {
        // Sprawdza poprawność wczytanych danych typu enumeracyjnego długości granulki.
        fn validate(
            &self,
            _synth_configuration: &SynthConfiguration,
            _track_number: usize,
        ) -> Result<(), String> {
            match self {
                GrainsLength::Fixed { equal } => {
                    if *equal < 10.0 || *equal > _synth_configuration.beat_length_ms - 0.1 {
                        return Err(
                            format!(
                                "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'grains_length\' (10.0 < \'equal\' < \'beat_length_ms\') :/",
                                _track_number
                            )
                        );
                    }
                }
                GrainsLength::Range { from, to } => {
                    if *to <= *from {
                        return Err(
                            format!(
                                "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'grains_length\' (\'from\' < \'to\') :/",
                                _track_number
                            )
                        );
                    } else if *from < 10.0 || *to > _synth_configuration.beat_length_ms - 0.1 {
                        return Err(
                            format!(
                                "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'grains_length\' (\'from\' > 10.0, \'to\' < \'beat_length_ms\') :/",
                                _track_number
                            )
                        );
                    }
                }
            }

            return Ok(());
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Typ enumeracyjny typu okna czasowego granulki.
    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    pub enum GWFunction {
        SmoothstepRegular {
            slope: f64,
        },
        SmoothstepUnregular {
            slope_attack: f64,
            slope_release: f64,
        },
    }

    impl GWFunction {
        // Sprawdzenie poprawnośći wczytanych danych typu enumeracyjnego okna czasowego granulki.
        fn validate(
            &self,
            _grains_length: &GrainsLength,
            _track_number: usize,
        ) -> Result<(), String> {
            match self {
                GWFunction::SmoothstepRegular { slope } => {
                    if *slope < 0.1 {
                        return Err(
                        format!(
                            "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'window_function\' (\'slope\' > 0.1) :/",
                            _track_number
                        )
                    );
                    } else {
                        match _grains_length {
                            GrainsLength::Fixed { equal } => {
                                if 2.0 * *slope + 0.1 > *equal {
                                    return Err(
                                format!(
                                    "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'window_function\' (2 x \'slope\' < \'grains_length_ms\': \'equal\') :/",
                                    _track_number
                                )
                            );
                                }
                            }
                            GrainsLength::Range { from, to: _ } => {
                                if 2.0 * *slope + 0.1 > *from {
                                    return Err(
                                format!(
                                    "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'window_function\' (2 x \'slope\' < \'grains_length_ms\': \'from\') :/",
                                    _track_number
                                )
                            );
                                }
                            }
                        };
                    }
                }
                GWFunction::SmoothstepUnregular {
                    slope_attack,
                    slope_release,
                } => {
                    if (*slope_attack < 0.1) || (*slope_release < 0.1) {
                        return Err(
                        format!(
                            "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'window_function\' (\'slope_attack\' > 0.1, \'slope_release\' > 0.1) :/",
                            _track_number
                        )
                    );
                    } else {
                        match _grains_length {
                            GrainsLength::Fixed { equal } => {
                                if *slope_attack + *slope_release + 0.1 > *equal {
                                    return Err(
                                format!(
                                    "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'window_function\' (\'slope_attack\' + \'slope_release\' < \'grains_length_ms\': \'equal\') :/",
                                    _track_number
                                )
                            );
                                }
                            }
                            GrainsLength::Range { from, to: _ } => {
                                if *slope_attack + *slope_release + 0.1 > *from {
                                    return Err(
                                format!(
                                    "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'window_function\' (\'slope_attack\' + \'slope_release\' < \'grains_length_ms\': \'from\') :/",
                                    _track_number
                                )
                            );
                                }
                            }
                        };
                    }
                }
            }

            return Ok(());
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Typ enumeracyjny wysokości dźwięku granulki.
    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    pub enum GrainsPitch {
        Fixed,
        Steps { steps: Vec<(f64, f64)> },
    }

    impl GrainsPitch {
        // Sprawdzenie poprawności wczytanych danych typu enumeracyjnego wysokości dźwieku granulki.
        fn validate(&self, _track_number: usize) -> Result<(), Vec<String>> {
            let mut errors: Vec<String> = Vec::new();

            match self {
                GrainsPitch::Fixed => {}
                GrainsPitch::Steps { steps } => {
                    let mut fraction_accumulated: f64 = 0.0;
                    let mut fraction_correctness: bool = true;

                    for (_step_index, _step) in steps.iter().enumerate() {
                        if _step.0 < 0.25 || _step.0 > 5.0 {
                            errors.push(
                                format!(
                                    "Błąd danych - track: [{}] - step: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'grains_pitch\' (0.25 < \'pitch\' < 5.0) :/",
                                    _track_number, _step_index + 1
                                )
                            );
                        }
                        if _step.1 < 0.0 || _step.1 > 100.0 {
                            errors.push(
                                format!(
                                    "Błąd danych - track: [{}] - step: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'grains_pitch\' (0% < \'fraction\' < 100%) :/",
                                    _track_number, _step_index + 1
                                )
                            );

                            fraction_correctness = false;
                        }

                        fraction_accumulated += _step.1;
                    }

                    if (fraction_correctness == true)
                        && (fraction_accumulated < 99.9999 || fraction_accumulated > 100.0001)
                    {
                        errors.push(
                            format!(
                                "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'grains_pitch\' (\'fraction_total\' = 100%) :/",
                                _track_number
                            )
                        );
                    }
                }
            }

            if errors.is_empty() == true {
                return Ok(());
            } else {
                return Err(errors);
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Właściwości granulek.
    #[derive(Debug, Deserialize)]
    pub struct GrainsProperties {
        pub sample_file_path: String,
        pub grains_count: usize,
        pub grains_length_ms: GrainsLength,
        pub window_function: GWFunction,
        pub grains_laudness_normalization: bool,
        pub grains_pitch: GrainsPitch,
    }

    impl GrainsProperties {
        // Sprawdzenie poprawności wczytanych danych własności granulek.
        fn validate(
            &self,
            _synth_configuration: &SynthConfiguration,
            _track_number: usize,
        ) -> Result<(), Vec<String>> {
            let mut errors: Vec<String> = Vec::new();

            if self.sample_file_path.is_empty() == true {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'sample_file_path\' (\'sample_file_path\' nie może być pusty) :/",
                        _track_number
                    )
                )
            }

            if self.grains_count < 4 || self.grains_count > 1_000_000 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - \'grains_properties\' ->\n\tnieprawidłowa wartość zmiennej: \'grains_count\' (4 < \'grains_count\' < 1 000 000) :/",
                        _track_number
                    )
                );
            }

            match self
                .grains_length_ms
                .validate(_synth_configuration, _track_number)
            {
                Err(_error) => {
                    errors.push(_error);
                }
                _ => {
                    match self
                        .window_function
                        .validate(&self.grains_length_ms, _track_number)
                    {
                        Err(_error) => {
                            errors.push(_error);
                        }
                        _ => {}
                    }
                }
            }

            match &mut self.grains_pitch.validate(_track_number) {
                Err(_errors) => {
                    errors.append(_errors);
                }
                _ => {}
            }

            if errors.is_empty() == true {
                return Ok(());
            } else {
                return Err(errors);
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Konfiguracja beatu.
    #[derive(Debug, Deserialize)]
    pub struct BeatConfiguration {
        pub subdivisions: usize,
        pub coverage_percentage: f64,
        pub humanization_percents: f64,
        pub volume_deviation_percents: f64,
        pub panorama_deviation_percents: f64,
    }

    impl BeatConfiguration {
        // Sprawdzenie poprawności wczytanych danych konfiguracji beatu.
        fn validate(
            &self,
            _synth_configuration: &SynthConfiguration,
            _track_number: usize,
            _beat_number: usize,
        ) -> Result<(), Vec<String>> {
            let mut errors: Vec<String> = Vec::new();

            if (_synth_configuration.beat_length_ms / self.subdivisions as f64) < 10.0 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - beat: [{}] - \'beat_sequence\' ->\n\tnieprawidłowa wartość zmiennej: \'subdivisions\' (\'SynthConfiguration\': \'beat_length_ms\' / \'subdivisions\' > 10) :/",
                        _track_number, _beat_number
                    )
                )
            }
            if self.coverage_percentage < 0.0 || self.coverage_percentage > 100.0 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - beat: [{}] - \'beat_sequence\' ->\n\tnieprawidłowa wartość zmiennej: \'coverage_percentage\' (0% < \'coverage_percentage\' < 100%) :/",
                        _track_number, _beat_number
                    )
                )
            }
            if self.humanization_percents < 0.0 || self.humanization_percents > 50.0 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - beat: [{}] - \'beat_sequence\' ->\n\tnieprawidłowa wartość zmiennej: \'humanization_percents\' (0% < \'humanization_percents\' < 50%) :/",
                        _track_number, _beat_number
                    )
                )
            }
            if self.volume_deviation_percents < 0.0 || self.volume_deviation_percents > 100.0 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - beat: [{}] - \'beat_sequence\' ->\n\tnieprawidłowa wartość zmiennej: \'volume_deviation_percents\' (0% < \'volume_deviation_percents\' < 100%) :/",
                        _track_number, _beat_number
                    )
                )
            }
            if self.panorama_deviation_percents < 0.0 || self.panorama_deviation_percents > 100.0 {
                errors.push(
                    format!(
                        "Błąd danych - track: [{}] - beat: [{}] - \'beat_sequence\' ->\n\tnieprawidłowa wartość zmiennej: \'panorama_deviation_percents\' (0% < \'panorama_deviation_percents\' < 100%) :/",
                        _track_number, _beat_number
                    )
                )
            }

            if errors.is_empty() == true {
                return Ok(());
            } else {
                return Err(errors);
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Struktura reprezentująca całą ścieżkę.
    #[derive(Debug, Deserialize)]
    pub struct Track {
        pub track_properties: TrackProperties,
        pub grains_properties: GrainsProperties,
        pub beat_sequence: Vec<BeatConfiguration>,

        #[serde(default = "AudioBuffer::default")]
        pub canva: AudioBuffer,

        #[serde(default = "Sampler::default")]
        pub sampler: Sampler,

        #[serde(default = "Sequencer::default")]
        pub sequencer: Sequencer,
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Wczytuje plik .json z konfiguracją i buduje na jego podstawie obiekt Value.
    pub fn load_json_file(_json_file_path: &String) -> Result<Value, String> {
        let json_file_content_result = read_to_string(Path::new(_json_file_path));

        let json_file_content = match json_file_content_result {
            Ok(_json_file_content) => _json_file_content,
            Err(_system_error) => {
                return Err(format!(
                    "Błąd odczytu ->\n\tplik: \'{}\' nie został znaleziony.\n\tSystem error: {} :/",
                    _json_file_path, _system_error
                ))
            }
        };

        let json_file_value_result: Result<Value, _> = from_str(json_file_content.as_str());

        let json_file_value = match json_file_value_result {
            Ok(_json_file_value) => _json_file_value,
            Err(_serde_error) => {
                return Err(format!(
                    "Błąd parsowania ->\n\twystąpił problem z parsowaniem pliku: \'{}\'.\n\tSerializer error: {} :/",
                    _json_file_path, _serde_error
                ))
            }
        };

        return Ok(json_file_value);
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Wczytuje konfiguracje ścieżek na podstawie obiektu Value.
    pub fn load_tracks_configurations(
        _json_file_value: &Value,
        _synth_configuration: &SynthConfiguration,
    ) -> Result<Vec<Track>, Vec<String>> {
        let tracks_result: Result<Vec<Track>, _> = from_value(_json_file_value["Tracks"].clone());

        let tracks = match tracks_result {
            Ok(_tracks) => _tracks,
            Err(_serde_error) => {
                return Err(vec![format!("Serializer Error ->\n\t{} :/", _serde_error)]);
            }
        };

        let mut errors: Vec<String> = Vec::new();
        let mut tracks_names: HashMap<String, u16> = HashMap::new();

        for (_track_number, _track) in tracks.iter().enumerate() {
            match &mut _track.track_properties.validate(_track_number + 1) {
                Err(_errors) => {
                    errors.append(_errors);
                }
                _ => {
                    *tracks_names
                        .entry(_track.track_properties.track_name.clone())
                        .or_insert(0) += 1;
                }
            }

            match &mut _track
                .grains_properties
                .validate(_synth_configuration, _track_number + 1)
            {
                Err(_errors) => {
                    errors.append(_errors);
                }
                _ => {}
            }

            for (_beat_number, _beat) in _track.beat_sequence.iter().enumerate() {
                match &mut _beat.validate(_synth_configuration, _track_number + 1, _beat_number + 1)
                {
                    Err(_errors) => {
                        errors.append(_errors);
                    }
                    _ => {}
                }
            }
        }

        for (_track_name, _count) in tracks_names.iter() {
            if *_count > 1 {
                errors.push(
                    format!("Błąd danych ->\n\t\'track_name\': \'{}\' występuje \'{}\' razy (nazwy ścieżek muszą być unikatowe) :/", _track_name, _count)
                )
            }
        }

        if errors.is_empty() == true {
            return Ok(tracks);
        } else {
            return Err(errors);
        }
    }
}
