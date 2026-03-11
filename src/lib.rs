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
            osc2_range: EnumParam::new("Range", Range::Sixteen),
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
            osc3_range: EnumParam::new("Range", Range::Sixteen),
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
            Range::Two => 0.5,
            Range::Four => 1.0,
            Range::Eight => 2.0,
            Range::Sixteen => 4.0,
            Range::ThirtyTwo => 8.0,
        }
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

                (osc1 * self.params.osc1_mix.value()
                    + osc2 * self.params.osc2_mix.value()
                    + osc3 * self.params.osc3_mix.value())
                    * self.midi_note_velocity.next()
            };
            for sample in channel_samples {
                *sample = displacement * util::db_to_gain_fast(gain);
            }
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
