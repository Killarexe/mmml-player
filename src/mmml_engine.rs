/// MMML Synthesizer in Rust
/// 
/// This module handles sequencing from .mmmldata files.
/// It generates 1-bit (stored as 8-bit) mono audio samples.
// Note table (plus an initial 'wasted' entry for rests)
const NOTES: [u16; 13] = [
    // the rest command is technically note 0 and thus requires a frequency
    255,
    // one octave of notes, equal temperament
    1644, 1551, 1464, 1382, 1305, 1231, 1162, 1097, 1035, 977, 922, 871,
];

// Location of individual samples in sample array
const SAMPLE_INDICIES: [u8; 6] = [0, 19, 34, 74, 118, 126];

// Raw PWM sample data
const SAMPLES: [u8; SAMPLE_LENGTH] = [
    // bwoop (0)
    0b10101010, 0b10110110, 0b10000111, 0b11111000,
    0b10000100, 0b00110111, 0b11101000, 0b11000001,
    0b00000111, 0b00111101, 0b11111000, 0b11100000,
    0b10010001, 0b10000111, 0b00000111, 0b00001111,
    0b00001111, 0b00011011, 0b00011110,
    // beep (19)
    0b10101010, 0b00101010, 0b00110011, 0b00110011,
    0b00110011, 0b00110011, 0b00110011, 0b11001101,
    0b11001100, 0b11001100, 0b11001100, 0b10101100,
    0b10011001, 0b00110001, 0b00110011,
    // kick (34)
    0b10010101, 0b10110010, 0b00000000, 0b11100011,
    0b11110000, 0b00000000, 0b11111111, 0b00000000,
    0b11111110, 0b00000000, 0b00000000, 0b00000000,
    0b11111111, 0b11111111, 0b11111111, 0b00100101,
    0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b11111111, 0b11110111, 0b11111111, 0b11111111,
    0b11111111, 0b10111111, 0b00010010, 0b00000000,
    0b10000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b11101110, 0b11111111, 0b11111111,
    0b11111111, 0b11110111, 0b11111111, 0b11111110,
    // snare (74)
    0b10011010, 0b10011010, 0b10101010, 0b10010110,
    0b01110100, 0b10010101, 0b10001010, 0b11011110,
    0b01110100, 0b10100000, 0b11110111, 0b00100101,
    0b01110100, 0b01101000, 0b11111111, 0b01011011,
    0b01000001, 0b10000000, 0b11010100, 0b11111101,
    0b11011110, 0b00010010, 0b00000100, 0b00100100,
    0b11101101, 0b11111011, 0b01011011, 0b00100101,
    0b00000100, 0b10010001, 0b01101010, 0b11011111,
    0b01110111, 0b00010101, 0b00000010, 0b00100010,
    0b11010101, 0b01111010, 0b11101111, 0b10110110,
    0b00100100, 0b10000100, 0b10100100, 0b11011010,
    // hi-hat (118)
    0b10011010, 0b01110100, 0b11010100, 0b00110011,
    0b00110011, 0b11101000, 0b11101000, 0b01010101,
    0b01010101,
    // end (126)
];

const SAMPLE_SPEED: u8 = 3;      // the sampler playback rate
const SAMPLE_LENGTH: usize = 127; // the length of the sample array
const MAXLOOPS: usize = 5;        // the maximum number of nested loops
const TOTAL_VOICES: usize = 4;    // total number of 1-bit voices to synthesize
const AMPLITUDE: u8 = 127;        // waveform high position (maximum from DC zero is 127)
const DC_OFFSET: u8 = 0;        // waveform low position (127 is DC zero)

const LOOP_START: u8 = 0x00;
const LOOP_END: u8 = 0x01;
const MACRO: u8 = 0x02;
const TEMPO: u8 = 0x03;
const CHANNEL_END: u8 = 0x0F;
const OCTAVE: u8 = 0x0D;
const VOLUME: u8 = 0x0E;

/// Stores the state for a single voice channel
pub struct VoiceChannel {
    output: u8,
    octave: u8,
    volume: u8,
    length: u8,
    loops_active: u8,
    frequency: u16,
    data_pointer: u16,
    waveform: u16,
    pitch_counter: u16,
    loop_duration: [u16; MAXLOOPS],
    loop_point: [u16; MAXLOOPS],
    pointer_location: u16,
    pub is_muted: bool
}

impl VoiceChannel {
    fn new() -> Self {
        VoiceChannel {
            output: 0,
            octave: 3,    // default octave: o3
            volume: 1,    // default volume: 50% pulse wave
            length: 0,
            loops_active: 0,
            frequency: 255, // random frequency (won't ever be sounded)
            data_pointer: 0,
            waveform: 0,
            pitch_counter: 0,
            loop_duration: [0; MAXLOOPS],
            loop_point: [0; MAXLOOPS],
            pointer_location: 0,
            is_muted: false
        }
    }
}

/// Sampler state for percussion samples
struct Sampler {
    current_byte: u8,
    current_bit: u8,
    sample_counter: u8,
    current_sample: u8,
}

impl Sampler {
    fn new() -> Self {
        Sampler {
            current_byte: 0,
            current_bit: 0,
            sample_counter: 0,
            current_sample: 0,
        }
    }
}

/// Main synthesizer state
pub struct MMMLSynthesizer {
    pub channels: [VoiceChannel; TOTAL_VOICES],
    sampler: Sampler,
    tick_counter: u16,
    tick_speed: u16,
    header_size: u16,
}

impl MMMLSynthesizer {
    pub fn new() -> Self {
        Self {
            channels: [
                VoiceChannel::new(),
                VoiceChannel::new(),
                VoiceChannel::new(),
                VoiceChannel::new()
            ],
            sampler: Sampler::new(),
            tick_counter: 0,
            tick_speed: 0,
            header_size: 0,
        }
    }

    /// Initialize the synthesizer with MMML data
    fn initialize(&mut self, mmml_source: &[u8]) {
        for i in 0..TOTAL_VOICES {
            self.channels[i].data_pointer = ((mmml_source[i * 2] as u16) << 8) | (mmml_source[i * 2 + 1] as u16);
        }
        self.header_size = self.channels[0].data_pointer;
    }

    /// Generate audio samples from MMML data
    pub fn generate_mmml(&mut self, mmml_source: &[u8]) -> Vec<u8> {

        self.initialize(mmml_source);

        let mut result: Vec<u8> = Vec::new();

        loop {
            /**********************
             *  Synthesizer Code  *
             **********************/

            // Sampler (channel D) code
            if self.sampler.sample_counter == 0 {
                if self.sampler.current_byte < self.sampler.current_sample - 1 && (self.sampler.current_byte as usize) < SAMPLE_LENGTH {
                    // Read individual bits from the sample array
                    self.channels[TOTAL_VOICES - 1].output = 
                        ((SAMPLES[self.sampler.current_byte as usize] >> self.sampler.current_bit) & 1) as u8;
                    self.sampler.current_bit += 1;
                } else {
                    // Silence the channel when the sample is over
                    self.channels[TOTAL_VOICES - 1].output = 0;
                }

                // Move to the next byte on bit pointer overflow
                if self.sampler.current_bit > 7 {
                    self.sampler.current_byte += 1;
                    self.sampler.current_bit = 0;
                }
                self.sampler.sample_counter = SAMPLE_SPEED;
            } else {
                self.sampler.sample_counter -= 1;
            }

            // Calculate pulse values
            for v in 0..TOTAL_VOICES - 1 {
                self.channels[v].pitch_counter += self.channels[v].octave as u16;
                if self.channels[v].pitch_counter >= self.channels[v].frequency {
                    self.channels[v].pitch_counter = self.channels[v].pitch_counter - self.channels[v].frequency;
                }
                if self.channels[v].pitch_counter <= self.channels[v].waveform {
                    self.channels[v].output = 1;
                }
                if self.channels[v].pitch_counter >= self.channels[v].waveform {
                    self.channels[v].output = 0;
                }
            }

            // Output and interleave samples using PIM
            for v in 0..TOTAL_VOICES {
                if !self.channels[v].is_muted {
                    result.push((self.channels[v].output * AMPLITUDE) + DC_OFFSET);
                } else {
                    result.push(DC_OFFSET);
                }
                if result.len() >= 1073741824 {
                    println!("Error: Buffer over 1GB! Abort!");
                    return result;
                }
            }



            /**************************
             *  Data Processing Code  *
             **************************/

            if self.tick_counter == 0 {
                // Variable tempo, sets the fastest / smallest possible clock event.
                self.tick_counter = self.tick_speed;
                let mut has_ended: [bool; TOTAL_VOICES] = [false, false, false, false];

                for v in 0..TOTAL_VOICES {
                    // If the note ended, start processing the next byte of data.
                    if self.channels[v].length == 0 {
                        'voice_processing: loop {
                            // Temporary storage of data for quick processing.
                            let data_ptr = self.channels[v].data_pointer as usize;
                            // First nibble of data
                            let buffer1 = (mmml_source[data_ptr] >> 4) & 0x0F;
                            // Second nibble of data
                            let buffer2 = mmml_source[data_ptr] & 0x0F;

                            // Function command
                            if buffer1 == 15 {
                                // Another buffer for commands that require an additional byte.
                                let buffer3 = mmml_source[data_ptr + 1];

                                // Process function commands using match
                                match buffer2 {
                                    LOOP_START => {
                                        self.channels[v].loops_active += 1;
                                        let active_loop = (self.channels[v].loops_active - 1) as usize;
                                        self.channels[v].loop_point[active_loop] = self.channels[v].data_pointer + 2;
                                        self.channels[v].loop_duration[active_loop] = buffer3 as u16 - 1;
                                        self.channels[v].data_pointer += 2;
                                    },
                                    LOOP_END => {
                                        let active_loop = (self.channels[v].loops_active - 1) as usize;
                                        if self.channels[v].loop_duration[active_loop] > 0 {
                                            self.channels[v].data_pointer = self.channels[v].loop_point[active_loop];
                                            self.channels[v].loop_duration[active_loop] -= 1;
                                        } else {
                                            self.channels[v].loops_active -= 1;
                                            self.channels[v].data_pointer += 1;
                                        }
                                    },
                                    MACRO => {
                                        self.channels[v].pointer_location = self.channels[v].data_pointer + 2;
                                        let macro_ptr = ((buffer3 as usize) + TOTAL_VOICES) * 2;
                                        self.channels[v].data_pointer = ((mmml_source[macro_ptr] as u16) << 8) | 
                                                                       (mmml_source[macro_ptr + 1] as u16);
                                    },
                                    TEMPO => {
                                        self.tick_speed = (buffer3 as u16) << 3;
                                        self.channels[v].data_pointer += 2;
                                    },
                                    4 => {
                                        println!("Warning: Transpose command found! This command isn't supported by the player! Skipping...");
                                        self.channels[v].data_pointer += 2;
                                    },
                                    5 => {
                                        println!("Warning: Instrumeent command found! This command isn't supported by the player! Skipping...");
                                        self.channels[v].data_pointer += 2;
                                    }
                                    6 => {
                                        println!("Warning: Tie command found! This command isn't supported by the player! Skipping...");
                                        self.channels[v].data_pointer += 1;
                                    }
                                    7 => {
                                        println!("Warning: Panning command found! This command isn't supported by the player! Skipping...");
                                        self.channels[v].data_pointer += 2;
                                    }
                                    // Debug pointer flag
                                    14 => {
                                        println!("Flag location: {}", self.channels[v].data_pointer);
                                        self.channels[v].data_pointer += 1;
                                    },
                                    CHANNEL_END => {
                                        if self.channels[v].pointer_location != 0 {
                                            self.channels[v].data_pointer = self.channels[v].pointer_location;
                                            self.channels[v].pointer_location = 0;
                                        } else {
                                            // Goes to loop again
                                            self.channels[v].data_pointer = ((mmml_source[v * 2] as u16) << 8) | 
                                                                           (mmml_source[v * 2 + 1] as u16);
                                            has_ended[v] = true;
                                        }
                                    },
                                    _ => {
                                        println!("Warning: Unknown command found: {:02X}. Skipping...", mmml_source[data_ptr]);
                                        self.channels[v].data_pointer += 1;
                                    }
                                }
                                let mut all_end: bool = true;
                                for v in 0..TOTAL_VOICES {
                                    if !has_ended[v] {
                                        all_end = false;
                                        break;
                                    }
                                }
                                if all_end {
                                    return result;
                                }

                                continue 'voice_processing;
                            }
                            
                            match buffer1 {
                                OCTAVE => {
                                    self.channels[v].octave = 2 << buffer2;
                                    self.channels[v].data_pointer += 1;
                                    continue 'voice_processing;
                                },
                                VOLUME => {
                                    self.channels[v].volume = buffer2;
                                    self.channels[v].data_pointer += 1;
                                    continue 'voice_processing;
                                },
                                _ => {}
                            }

                            // Note value processing
                            if buffer1 != 0 && buffer1 < 14 {
                                if v < TOTAL_VOICES - 1 {
                                    let buffer4 = NOTES[buffer1 as usize];
                                    self.channels[v].frequency = buffer4;

                                    /* Calculate the waveform duty cycle by dividing the frequency by
                                     * powers of two. */
                                    self.channels[v].waveform = buffer4 >> self.channels[v].volume;
                                } else {
                                    // Reset the sampler
                                    self.sampler.current_bit = 0;
                                    self.sampler.current_byte = SAMPLE_INDICIES[(buffer1 - 1) as usize];
                                    self.sampler.current_sample = SAMPLE_INDICIES[buffer1 as usize];
                                }
                            } else {
                                // Rest
                                self.channels[v].waveform = 0;
                            }

                            // Note duration value
                            self.channels[v].length = match buffer2 {
                                0..=7 => 0x7F >> buffer2,         // Standard duration
                                _ => 95 >> (buffer2 & 7),        // Dotted (1 + 1/2) duration
                            };

                            // Next element in data
                            self.channels[v].data_pointer += 1;
                            break 'voice_processing;
                        }
                    } else {
                        // Keep waiting until the note is over...
                        self.channels[v].length -= 1;
                    }
                }
            } else {
                self.tick_counter -= 1;
            }

        }
    }
}
