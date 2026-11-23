//! How the simulator answers one received line: `$` commands and the G-code words the job
//! writer uses (motion, spindle, air).

use super::{Block, GrblSim, Spindle};

fn words(line: &str) -> Vec<(char, f64)> {
    let mut out = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let letter = chars[i].to_ascii_uppercase();
        i += 1;
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_digit() || matches!(chars[i], '.' | '-' | '+'))
        {
            i += 1;
        }
        let number: String = chars[start..i].iter().collect();
        if let Ok(value) = number.parse() {
            out.push((letter, value));
        }
    }
    out
}

impl GrblSim {
    fn system(&mut self, upper: &str) {
        match upper {
            "$I" => {
                let version = self.config.version;
                self.print(&format!("[VER:{version}.20190825:]"));
                self.print("[OPT:V,15,128]");
            }
            "$$" => {
                let lines: Vec<String> = self
                    .settings
                    .iter()
                    .map(|(n, v)| format!("${n}={v}"))
                    .collect();
                for line in lines {
                    self.print(&line);
                }
            }
            "$X" => {
                if self.alarm {
                    self.alarm = false;
                    self.print("[MSG:Caution: Unlocked]");
                }
            }
            "$H" => {
                if self.moving() || self.in_hold() {
                    // GRBL's "not idle": `$H` only in Idle and Alarm (`specs/0118` M7).
                    self.print("error:8");
                } else if !self.start_homing() {
                    self.print("error:5");
                }
                return; // `ok` when homing ends
            }
            "$#" | "$G" | "$C" => {}
            other => {
                let set = other
                    .strip_prefix('$')
                    .and_then(|s| s.split_once('='))
                    .and_then(|(n, v)| Some((n.parse::<u16>().ok()?, v.to_owned())));
                match set {
                    Some((n, v)) => self.set_setting(n, &v),
                    None => {
                        self.print("error:3");
                        return;
                    }
                }
            }
        }
        self.print("ok");
    }

    /// A `$J=` line (relative, millimetres): refused while locked, while homing or when soft
    /// limits would be crossed; else a jog block that takes the time its feed asks for.
    fn jog_line(&mut self, text: &str) {
        let mut delta = [0.0; 3];
        let mut feed = None;
        for (letter, value) in words(text) {
            match letter {
                'X' | 'Y' | 'Z' => delta[usize::from(letter as u8 - b'X')] = value,
                'F' => feed = Some(value),
                _ => {}
            }
        }
        let Some(feed) = feed.filter(|f| *f > 0.0) else {
            self.print("error:22");
            return;
        };
        if self.alarm {
            self.print("error:9");
            return;
        }
        let from = self.planned;
        let target = [from[0] + delta[0], from[1] + delta[1], from[2] + delta[2]];
        let t = self.config.travel;
        let outside = target[0] < -t[0] - 1e-9
            || target[0] > 1e-9
            || target[1] < -t[1] - 1e-9
            || target[1] > 1e-9;
        if self.config.soft_limits && outside {
            self.print("error:15");
            return;
        }
        let length = delta.iter().map(|d| d * d).sum::<f64>().sqrt();
        // One simulated step is 10 ms.
        let total = ((length * 60.0 / feed) / 0.01).ceil().max(1.0);
        // invariant: a positive finite step count far below u32::MAX in these tests.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total = total as u32;
        self.planned = target;
        self.push_block(Block::Jog {
            from: self.pos,
            target,
            total,
            left: total,
        });
        self.print("ok");
    }

    fn gcode(&mut self, line: &str) {
        let mut target = self.planned;
        let mut motion = false;
        let mut spindle = None;
        let mut power = None;
        for (letter, value) in words(line) {
            match letter {
                'G' if value == 90.0 => self.relative = false,
                'G' if value == 91.0 => self.relative = true,
                'G' if [0.0, 1.0, 2.0, 3.0].contains(&value) => motion = true,
                'X' | 'Y' | 'Z' => {
                    let axis = usize::from(letter as u8 - b'X');
                    target[axis] = if self.relative {
                        target[axis] + value
                    } else {
                        value
                    };
                    motion = true;
                }
                'M' if value == 3.0 => spindle = Some(Spindle::Cw),
                'M' if value == 4.0 => spindle = Some(Spindle::Ccw),
                'M' if value == 5.0 => spindle = Some(Spindle::Off),
                'M' if value == 7.0 || value == 8.0 => self.push_block(Block::Air(true)),
                'M' if value == 9.0 => self.push_block(Block::Air(false)),
                'S' => power = Some(value),
                _ => {}
            }
        }
        if spindle.is_some() || power.is_some() {
            let s = spindle.unwrap_or(self.spindle);
            let p = if s == Spindle::Off {
                0.0
            } else {
                power.unwrap_or(self.power)
            };
            self.push_block(Block::Spindle(s, p));
        }
        if motion && target != self.planned {
            let steps = self.config.steps_per_motion;
            self.planned = target;
            self.push_block(Block::Move { target, steps });
        }
        self.print("ok");
    }

    /// Answers received line `n`.
    pub(crate) fn execute(&mut self, n: u32, line: &str) {
        if let Some((_, code)) = self.config.error_on_line.filter(|(at, _)| *at == n) {
            self.print(&format!("error:{code}"));
            return;
        }
        let upper = line.to_ascii_uppercase();
        if let Some(jog) = upper.strip_prefix("$J=") {
            self.jog_line(jog);
        } else if upper.starts_with('$') {
            self.system(&upper);
        } else if self.alarm {
            self.print("error:9");
        } else {
            self.gcode(&upper);
        }
    }
}
