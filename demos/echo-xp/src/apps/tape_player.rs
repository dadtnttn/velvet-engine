pub struct TranscriptLine {
    pub time: f32,
    pub speaker: &'static str,
    pub text: &'static str,
}

pub struct TapePlayerApp {
    pub playing: bool,
    pub position: f32,
    pub duration: f32,
    pub tape_name: &'static str,
    pub transcript: Vec<TranscriptLine>,
}

impl TapePlayerApp {
    pub fn new() -> Self {
        let transcript = vec![
            TranscriptLine {
                time: 0.0,
                speaker: "VOICE 1",
                text: "How many children lived in the house?",
            },
            TranscriptLine {
                time: 3.0,
                speaker: "VOICE 2",
                text: "Two.",
            },
            TranscriptLine {
                time: 5.0,
                speaker: "VOICE 1",
                text: "The registry lists one.",
            },
            TranscriptLine {
                time: 7.5,
                speaker: "VOICE 2",
                text: "It listed two yesterday.",
            },
            TranscriptLine {
                time: 9.5,
                speaker: "NOISE",
                text: "[static interference]",
            },
            TranscriptLine {
                time: 10.5,
                speaker: "UNKNOWN",
                text: "Mara is the one who stayed.",
            },
        ];

        Self {
            playing: false,
            position: 0.0,
            duration: 12.0,
            tape_name: "C17_RECOVERED_02.wav",
            transcript,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if self.playing {
            self.position += dt;
            if self.position >= self.duration {
                self.position = self.duration;
                self.playing = false;
            }
        }
    }

    pub fn play(&mut self) {
        if self.position >= self.duration {
            self.position = 0.0;
        }
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.position = 0.0;
    }

    pub fn current_transcript(&self) -> Option<&TranscriptLine> {
        self.transcript
            .iter()
            .rev()
            .find(|line| self.position >= line.time)
    }
}
