use nih_plug::prelude::*;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Mooxide {
    params: Arc<MooxideParams>,
    sample_rate: f32,
    phases: [f32; 3],
    midi_note_id: u8,
    midi_note_frequency: f32,
    midi_note_velocity: Smoother<f32>,
    filter_biquad: [f32; 4],
    note_time: u32,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum Waveform {
    #[name = "Triangle"]
    Triangle,
    #[name = "Triangle-Sawtooth"]
    TriangleSawtooth,
    #[name = "Sawtooth"]
    Sawtooth,
    #[name = "ReverseSawtooth"]
    ReverseSawtooth,
    #[name = "Square"]
    Square,
    #[name = "Wide-Pulse"]
    WidePulse,
    #[name = "Narrow-Pulse"]
    NarrowPulse,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum Range {
    #[name = "2"]
    Two,
    #[name = "4"]
    Four,
    #[name = "8"]
    Eight,
    #[name = "16"]
    Sixteen,
    #[name = "32"]
    ThirtyTwo,
    #[name = "64"]
    SixtyFour,
}
#[derive(Enum, PartialEq, Clone, Copy)]
pub enum NoiseKind {
    #[name = "White"]
    White,
    #[name = "Pink"]
    Pink,
}

#[derive(Params)]
struct MooxideParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "tune"]
    pub tune: FloatParam,
    #[id = "occ1_range"]
    pub osc1_range: EnumParam<Range>,
    #[id = "osc1_wave"]
    pub osc1_wave: EnumParam<Waveform>,
    #[id = "osc1_mix"]
    pub osc1_mix: FloatParam,
    #[id = "osc2_range"]
    pub osc2_range: EnumParam<Range>,
    #[id = "osc2_detune"]
    pub osc2_detune: FloatParam,
    #[id = "osc2_wave"]
    pub osc2_wave: EnumParam<Waveform>,
    #[id = "osc2_mix"]
    pub osc2_mix: FloatParam,
    #[id = "osc3_range"]
    pub osc3_range: EnumParam<Range>,
    #[id = "osc3_detune"]
    pub osc3_detune: FloatParam,
    #[id = "osc3_wave"]
    pub osc3_wave: EnumParam<Waveform>,
    #[id = "osc3_mix"]
    pub osc3_mix: FloatParam,

    #[id = "noise"]
    pub noise: EnumParam<NoiseKind>,
    #[id = "noise_mix"]
    pub noise_mix: FloatParam,

    #[id = "filter_cutoff"]
    pub filter_cutoff: FloatParam,
    #[id = "filter_emphasis"]
    pub filter_emphasis: FloatParam,
    #[id = "filter_contour"]
    pub filter_contour: FloatParam,
    #[id = "filter_attack"]
    pub filter_attack: FloatParam,
    #[id = "filter_decay"]
    pub filter_decay: FloatParam,
    #[id = "filter_sustain"]
    pub filter_sustain: FloatParam,
    #[id = "contour_attack"]
    pub contour_attack: FloatParam,
    #[id = "contour_decay"]
    pub contour_decay: FloatParam,
    #[id = "contour_sustain"]
    pub contour_sustain: FloatParam,

    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for Mooxide {
    fn default() -> Self {
        Self {
            params: Arc::new(MooxideParams::default()),
            sample_rate: 1.0,
            phases: [0.0; 3],
            midi_note_id: 0,
            midi_note_frequency: 1.0,
            midi_note_velocity: Smoother::new(SmoothingStyle::Linear(5.0)),
            filter_biquad: [0.0; 4],
            note_time: 0,
        }
    }
}

impl Default for MooxideParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("Gain", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            tune: FloatParam::new(
                "Tune",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            osc1_range: EnumParam::new("Range", Range::Sixteen),
            osc1_wave: EnumParam::new("Waveform", Waveform::Triangle),
            osc1_mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            osc2_range: EnumParam::new("Range", Range::ThirtyTwo),
            osc2_detune: FloatParam::new(
                "Detune",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            osc2_wave: EnumParam::new("Waveform", Waveform::Triangle),
            osc2_mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            osc3_range: EnumParam::new("Range", Range::ThirtyTwo),
            osc3_detune: FloatParam::new(
                "Detune",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            osc3_wave: EnumParam::new("Waveform", Waveform::Triangle),
            osc3_mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),

            noise: EnumParam::new("Noise", NoiseKind::White),
            noise_mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),

            filter_cutoff: FloatParam::new(
                "Filter Cutoff Frequency",
                0.0,
                FloatRange::Linear {
                    min: -5.0,
                    max: 5.0,
                },
            ),
            filter_emphasis: FloatParam::new(
                "Filter Emphasis",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            filter_contour: FloatParam::new(
                "Filter Contour",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            filter_attack: FloatParam::new(
                "Filter Attack Time",
                0.6,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-0.9),
                },
            ),
            filter_decay: FloatParam::new(
                "Filter Decay Time",
                0.6,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-0.9),
                },
            ),
            filter_sustain: FloatParam::new(
                "Filter Sustain Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            contour_attack: FloatParam::new(
                "Contour Attack Time",
                0.6,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-0.9),
                },
            ),
            contour_decay: FloatParam::new(
                "Contour Decay Time",
                0.6,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-0.9),
                },
            ),
            contour_sustain: FloatParam::new(
                "Contour Sustain Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
        }
    }
}

impl Mooxide {
    fn osc(&mut self, index: usize, freq: f32, wave: Waveform) -> f32 {
        let phase = &mut self.phases[index];
        fn triangle(phase: f32) -> f32 {
            1.0 - 2.0 * (phase + 0.75).fract().abs()
        }
        fn sawtooth(phase: f32) -> f32 {
            2.0 * phase - 1.0
        }
        fn square(phase: f32, width: f32) -> f32 {
            (phase < width) as i32 as f32 * 2.0 - 1.0
        }
        let out = match wave {
            Waveform::Triangle => triangle(*phase),
            Waveform::TriangleSawtooth => 0.5 * triangle(*phase) + 0.5 * sawtooth(*phase),
            Waveform::Sawtooth => sawtooth(*phase),
            Waveform::ReverseSawtooth => 1.0 - *phase * 2.0,
            Waveform::Square => square(*phase, 0.5),
            Waveform::WidePulse => square(*phase, 0.25),
            Waveform::NarrowPulse => square(*phase, 0.125),
        };

        // update phase
        *phase += freq / self.sample_rate;
        if *phase >= 1.0 {
            *phase -= 1.0;
        }
        out
    }
    fn get_range_mult(&self, range: Range) -> f32 {
        match range {
            Range::Two => 0.25,
            Range::Four => 0.5,
            Range::Eight => 1.0,
            Range::Sixteen => 2.0,
            Range::ThirtyTwo => 4.0,
            Range::SixtyFour => 8.0,
        }
    }

    fn noise(&self, kind: NoiseKind) -> f32 {
        match kind {
            NoiseKind::White => rand::random::<f32>() * 2.0 - 1.0,
            NoiseKind::Pink => {
                let mut sum = 0.0;
                for _ in 0..10 {
                    sum += rand::random::<f32>() * 2.0 - 1.0;
                }
                sum / 10.0
            }
        }
    }

    fn envelope(&self) -> f32 {
        let time = self.note_time as f32 / self.sample_rate;
        let attack_phase = (time / self.params.contour_attack.value()).clamp(0.0, 1.0);
        let decay_phase = ((time - self.params.contour_attack.value())
            / self.params.contour_decay.value())
        .clamp(0.0, 1.0);
        attack_phase * (1.0 - decay_phase) + (self.params.contour_sustain.value() * decay_phase)
    }

    fn filter_envelope(&self) -> f32 {
        let time = self.note_time as f32 / self.sample_rate;
        let attack_phase = (time / self.params.filter_attack.value()).clamp(0.0, 1.0);
        let decay_phase = ((time - self.params.filter_attack.value())
            / self.params.filter_decay.value())
        .clamp(0.0, 1.0);
        attack_phase * (1.0 - decay_phase) + (self.params.filter_sustain.value() * decay_phase)
    }

    fn filter(&mut self, input: f32) -> f32 {
        // https://www.utsbox.com/?page_id=523
        let openture = self.midi_note_frequency
            * (2.0 + self.filter_envelope() + (self.params.filter_cutoff.value() / 5.0));
        let omega = 2.0 * std::f32::consts::PI * openture / self.sample_rate;
        let alpha = omega.sin() / 2.0 / self.params.filter_emphasis.value();
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * omega.cos();
        let a2 = 1.0 - alpha;
        let b0 = (1.0 - omega.cos()) / 2.0;
        let b1 = 1.0 - omega.cos();
        let b2 = (1.0 - omega.cos()) / 2.0;
        let y = b0 / a0 * input + b1 / a0 * self.filter_biquad[0] + b2 / a0 * self.filter_biquad[1]
            - a1 / a0 * self.filter_biquad[2]
            - a2 / a0 * self.filter_biquad[3];
        self.filter_biquad[1] = self.filter_biquad[0];
        self.filter_biquad[0] = input;
        self.filter_biquad[3] = self.filter_biquad[2];
        self.filter_biquad[2] = y;
        y
    }
}

impl Plugin for Mooxide {
    const NAME: &'static str = "Mooxide";
    const VENDOR: &'static str = "minerva-jupiter";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "ryouturn@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
        self.phases = [0.0; 3];
        self.midi_note_id = 0;
        self.midi_note_frequency = 1.0;
        self.midi_note_velocity.reset(0.0);
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            // This plugin can be either triggered by MIDI or controleld by a parameter
            let displacement = {
                // Act on the next MIDI event
                while let Some(event) = next_event {
                    if event.timing() > sample_id as u32 {
                        break;
                    }

                    match event {
                        NoteEvent::NoteOn { note, velocity, .. } => {
                            self.midi_note_id = note;
                            self.midi_note_frequency = util::midi_note_to_freq(note);
                            self.midi_note_velocity
                                .set_target(self.sample_rate, velocity);
                            self.filter_biquad = [0.0; 4];
                            self.note_time = 0;
                        }
                        NoteEvent::NoteOff { note, .. } if note == self.midi_note_id => {
                            self.midi_note_velocity.set_target(self.sample_rate, 0.0);
                        }
                        NoteEvent::PolyPressure { note, pressure, .. }
                            if note == self.midi_note_id =>
                        {
                            self.midi_note_velocity
                                .set_target(self.sample_rate, pressure);
                        }
                        _ => (),
                    }

                    next_event = context.next_event();
                }
                let osc1 = self.osc(
                    0,
                    self.midi_note_frequency * self.get_range_mult(self.params.osc1_range.value()),
                    self.params.osc1_wave.value(),
                );
                let osc2 = self.osc(
                    1,
                    self.midi_note_frequency
                        * self.get_range_mult(self.params.osc2_range.value())
                        * (1.0 + self.params.osc2_detune.value() * 0.01),
                    self.params.osc2_wave.value(),
                );
                let osc3 = self.osc(
                    2,
                    self.midi_note_frequency
                        * self.get_range_mult(self.params.osc3_range.value())
                        * (1.0 + self.params.osc3_detune.value() * 0.01),
                    self.params.osc3_wave.value(),
                );

                let noise = self.noise(self.params.noise.value());

                self.filter(
                    (osc1 * self.params.osc1_mix.value()
                        + osc2 * self.params.osc2_mix.value()
                        + osc3 * self.params.osc3_mix.value()
                        + noise * self.params.noise_mix.value())
                        * self.midi_note_velocity.next(),
                ) * self.envelope()
            };
            for sample in channel_samples {
                *sample = displacement * util::db_to_gain_fast(gain);
            }
            self.note_time += 1;
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Mooxide {
    const CLAP_ID: &'static str = "net.minervajuppiter.mooxide";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("simple synth");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

nih_export_clap!(Mooxide);
