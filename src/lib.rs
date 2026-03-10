use nih_plug::prelude::*;
use std::f32::consts;
use std::str::SplitAsciiWhitespace;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Mooxide {
    params: Arc<MooxideParams>,
    sample_rate: f32,
    phase: f32,
    midi_note_id: u8,
    midi_note_frequency: f32,
    midi_note_velocity: Smoother<f32>,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum Waveform {
    #[name = "Sine"]
    Sine,
    #[name = "Triangle"]
    Triangle,
    #[name = "Sawtooth"]
    Sawtooth,
}

#[derive(Params)]
struct MooxideParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "osc_wave"]
    pub wave: EnumParam<Waveform>,
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for Mooxide {
    fn default() -> Self {
        Self {
            params: Arc::new(MooxideParams::default()),
            sample_rate: 1.0,
            phase: 0.0,
            midi_note_id: 0,
            midi_note_frequency: 1.0,
            midi_note_velocity: Smoother::new(SmoothingStyle::Linear(5.0)),
        }
    }
}

impl Default for MooxideParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                -10.0,
                FloatRange::Linear {
                    min: -30.0,
                    max: 0.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0))
            .with_step_size(0.01)
            .with_unit(" dB"),
            wave: EnumParam::new("Waveform", Waveform::Sine),
        }
    }
}

impl Mooxide {
    fn sin(&mut self, frequency: f32) -> f32 {
        let sine = (self.phase * consts::TAU).sin();
        self.update_phase(frequency);
        sine
    }
    fn triangle(&mut self, frequency: f32) -> f32 {
        let triangle = 2.0 * (1.0 - self.phase).abs() - 1.0;
        self.update_phase(frequency);
        triangle
    }
    fn sawtooth(&mut self, frequency: f32) -> f32 {
        let sawtooth = self.phase * 2.0 - 1.0;
        self.update_phase(frequency);
        sawtooth
    }

    fn update_phase(&mut self, frequency: f32) {
        self.phase += frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
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
        self.phase = 0.0;
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
            let sine = {
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

                // This gain envelope prevents clicks with new notes and with released notes
                let current_waveform = self.params.wave.value();
                match current_waveform {
                    Waveform::Sine => {
                        self.sin(self.midi_note_frequency) * self.midi_note_velocity.next()
                    }
                    Waveform::Triangle => {
                        self.triangle(self.midi_note_frequency) * self.midi_note_velocity.next()
                    }
                    Waveform::Sawtooth => {
                        self.sawtooth(self.midi_note_frequency) * self.midi_note_velocity.next()
                    }
                    _ => self.sin(self.midi_note_frequency) * self.midi_note_velocity.next(),
                }
            };
            for sample in channel_samples {
                *sample = sine * util::db_to_gain_fast(gain);
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
