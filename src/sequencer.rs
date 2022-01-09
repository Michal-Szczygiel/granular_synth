pub mod core {
    use rand::distributions::{Distribution, Uniform};
    use rand::prelude::{SliceRandom, ThreadRng};
    use rand::thread_rng;
    use serde::Deserialize;

    use crate::granular_synth_config::tools::{BeatConfiguration, SynthConfiguration};

    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Reprezentacja pojedynczego zdarzenia w sekwencji.
    #[derive(Debug)]
    pub struct Event {
        pub start_index: usize,
        pub panorama: f64,
        pub volume: f64,
    }

    // Struktura reprezentująca sekwencer.
    #[derive(Debug, Deserialize)]
    pub struct Sequencer {
        #[serde(skip_deserializing)]
        pub sequence: Vec<Event>,

        #[serde(skip_deserializing)]
        randomness_source: ThreadRng,
    }

    impl Sequencer {
        // Domyślny konstruktor obiektu Sequencer dla serde.
        pub fn default() -> Self {
            return Sequencer {
                sequence: Vec::with_capacity(2048),
                randomness_source: thread_rng(),
            };
        }

        // Generuje sekwencję wystąpień granulek w funkcji czasu.
        pub fn generate_sequence(
            &mut self,
            _beat_sequence: &Vec<BeatConfiguration>,
            _synth_configuration: &SynthConfiguration,
        ) {
            let beat_length: usize = (_synth_configuration.engine_sampling_rate as f64
                * (_synth_configuration.beat_length_ms / 1000.0))
                .round() as usize;

            // Pętla po każdym beacie w sekwencji.
            for (_beat_number, _beat) in _beat_sequence.iter().enumerate() {
                let sub_beat_length: f64 = beat_length as f64 / _beat.subdivisions as f64;
                let mut sub_coordinates: Vec<usize> = Vec::with_capacity(_beat.subdivisions);

                // Określenie położeń sub-beatów z i bez humanizacji.
                if _beat.humanization_percents != 0.0 {
                    let max_humanization: f64 =
                        sub_beat_length * (_beat.humanization_percents / 100.0);
                    let humanization_distr =
                        Uniform::<f64>::new(-max_humanization, max_humanization);

                    for _index in 0.._beat.subdivisions {
                        let offset = humanization_distr.sample(&mut self.randomness_source);
                        let index: usize = (sub_beat_length * _index as f64 + offset).round()
                            as usize
                            + (beat_length * (_beat_number + 1));

                        sub_coordinates.push(index);
                    }
                } else {
                    for _index in 0.._beat.subdivisions {
                        let index: usize = (sub_beat_length * _index as f64).round() as usize
                            + (beat_length * (_beat_number + 1));

                        sub_coordinates.push(index);
                    }
                }

                // Wybranie wskazanej frakcji sub-beatów ('coverage_percentage').
                let chosen_sub_beats_number: usize = (_beat.subdivisions as f64
                    * (_beat.coverage_percentage / 100.0))
                    .round() as usize;
                let chosen_sub_beats: Vec<usize> = sub_coordinates
                    .choose_multiple(&mut self.randomness_source, chosen_sub_beats_number)
                    .cloned()
                    .collect();

                // Pętla po każdym wybranym sub-beacie, wyznaczneie panoramy i wzmocnienia.
                for _sub_beat in chosen_sub_beats.iter() {
                    let panorama_value: f64 = if _beat.panorama_deviation_percents != 0.0 {
                        let panorama_max_dev = _beat.panorama_deviation_percents / 100.0;
                        let panormama_distr =
                            Uniform::<f64>::new(-panorama_max_dev, panorama_max_dev);

                        panormama_distr.sample(&mut self.randomness_source)
                    } else {
                        0.0
                    };

                    let volume_value: f64 = if _beat.volume_deviation_percents != 0.0 {
                        let volume_max_dev = _beat.volume_deviation_percents / 100.0;
                        let volume_distr =
                            Uniform::<f64>::new(1.0 - volume_max_dev, 1.0 + volume_max_dev);

                        volume_distr.sample(&mut self.randomness_source)
                    } else {
                        1.0
                    };

                    self.sequence.push(Event {
                        start_index: *_sub_beat,
                        panorama: panorama_value,
                        volume: volume_value,
                    })
                }
            }

            // Sortowanie zdarzeń zgodnie z roznącym indeksem.
            self.sequence
                .sort_by(|a, b| a.start_index.cmp(&b.start_index));
        }
    }
}
