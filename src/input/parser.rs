use crate::engine::command::{Command, Cue, Duration, Value};
use crate::engine::effects::{Easing, Effect, MathMethod};
use crate::engine::target::{ChannelList, Target};
use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Default)]
pub struct ParserState {
    pub last_target: Option<Target>,
    pub pending_commands: Vec<Command>,
    pub saved_effects: HashMap<String, Effect>,
}

pub static PARSER_STATE: LazyLock<Mutex<ParserState>> =
    LazyLock::new(|| Mutex::new(ParserState::default()));

pub fn parse_command_line(line: &str) -> Result<Option<Cue>, String> {
    let mut state = PARSER_STATE.lock().unwrap();
    let line = line.trim();
    if line.to_lowercase().starts_with("go ") {
        let list = line[3..].trim().to_string();
        let mut cue = Cue::new("CLI", vec![Command::StartCueList { list }]);
        cue.raw_commands = vec![line.to_string()];
        return Ok(Some(cue));
    }
    
    if line.to_lowercase().starts_with("stop ") {
        let list = line[5..].trim().to_string();
        let mut cue = Cue::new("CLI", vec![Command::StopCueList { list }]);
        cue.raw_commands = vec![line.to_string()];
        return Ok(Some(cue));
    }

    if line.is_empty() || line.eq_ignore_ascii_case("go") {
        if state.pending_commands.is_empty() {
            return Err("No pending commands".to_string());
        }
        let commands = std::mem::take(&mut state.pending_commands);
        let mut cue = Cue::new("Delayed", commands);
        cue.raw_commands = vec![line.to_string()];
        return Ok(Some(cue));
    }

    let is_delayed = line.ends_with('?');
    let line_to_parse = if is_delayed {
        &line[..line.len() - 1]
    } else {
        line
    };

    if let Some(mut cmds) = parse_command(line_to_parse, &mut state) {
        if is_delayed {
            state.pending_commands.append(&mut cmds);
            Ok(None)
        } else {
            state.pending_commands.clear();
            let mut cue = Cue::new("CLI", cmds);
            cue.raw_commands = vec![line.to_string()];
            Ok(Some(cue))
        }
    } else {
        Err("Unrecognized command format.".to_string())
    }
}

pub fn parse_command(line: &str, state: &mut ParserState) -> Option<Vec<Command>> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.splitn(2, '@').collect();
    let target_str = parts[0].trim();

    let target = if target_str.is_empty() {
        state.last_target.clone()?
    } else {
        let t = parse_target(target_str)?;
        state.last_target = Some(t.clone());
        t
    };

    if parts.len() < 2 {
        return Some(vec![]);
    }

    let action_str = parts[1].trim();
    if action_str.is_empty() {
        return Some(vec![]);
    }

    let command = if action_str.starts_with("fx(") && action_str.ends_with(')') {
        let inner = &action_str[3..action_str.len() - 1];
        let args: Vec<&str> = inner.split(',').collect();
        if args.is_empty() {
            return None;
        }

        let effect_name = args[0].trim().to_lowercase();
        
        let min_str = if args.len() > 1 && !args[1].trim().is_empty() { args[1].trim() } else { "0" };
        let max_str = if args.len() > 2 && !args[2].trim().is_empty() { args[2].trim() } else { "100" };
        let min_val = min_str.replace("%", "").parse::<f32>().unwrap_or(0.0);
        let max_val = max_str.replace("%", "").parse::<f32>().unwrap_or(100.0);
        let arg_offset = 3;

        let period_str = if args.len() > arg_offset && !args[arg_offset].trim().is_empty() {
            args[arg_offset].trim()
        } else {
            "1b"
        };
        let duration = parse_duration(period_str).unwrap_or(Duration::Tempo(1.0));

        let wave_str = if args.len() > arg_offset + 1 && !args[arg_offset + 1].trim().is_empty() {
            args[arg_offset + 1].trim().to_lowercase()
        } else {
            "linear".to_string()
        };
        let easing = match wave_str.as_str() {
            "sine" => Easing::Sine,
            "step" => Easing::Step,
            _ => Easing::Linear,
        };

        let method_str = if args.len() > arg_offset + 2 && !args[arg_offset + 2].trim().is_empty() {
            args[arg_offset + 2].trim().to_lowercase()
        } else {
            "absolute".to_string()
        };
        let method = match method_str.as_str() {
            "add" => MathMethod::Add,
            "subtract" => MathMethod::Subtract,
            "multiply" => MathMethod::Multiply,
            _ => MathMethod::Absolute,
        };

        let effect = if effect_name == "rnd" {
            Effect::Random {
                min_val,
                max_val,
                duration,
                easing,
                method,
            }
        } else if effect_name == "rnd0" {
            Effect::Random0 {
                min_val,
                max_val,
                duration,
                easing,
                method,
            }
        } else {
            Effect::Cycle {
                min_val,
                max_val,
                duration,
                easing,
                method,
            }
        };

        Command::StartEffect {
            target,
            effect,
        }
    } else if let Some(saved_effect) = state.saved_effects.get(action_str) {
        Command::StartEffect {
            target,
            effect: saved_effect.clone(),
        }
    } else if action_str.contains('/') {
        let parts: Vec<&str> = action_str.splitn(2, '/').collect();
        let val_str = parts[0].trim();
        let dur_str = parts[1].trim();

        let target_val = if val_str.eq_ignore_ascii_case("full") {
            255.0
        } else {
            let num = val_str.replace("%", "").parse::<f32>().unwrap_or(0.0);
            if val_str.contains("%") { num * 2.55 } else { num }
        };
        let duration = parse_duration(dur_str).unwrap_or(Duration::Time(0.0));

        Command::StartEffect {
            target,
            effect: Effect::Fade {
                target_val,
                duration,
            },
        }
    } else {
        let mut val_str = if action_str.ends_with('!') {
            &action_str[..action_str.len() - 1]
        } else {
            action_str
        };
        
        let mut multiplier = 1.0;
        if val_str.ends_with('%') {
            val_str = &val_str[..val_str.len() - 1];
            multiplier = 2.55;
        }

        if val_str.eq_ignore_ascii_case("full") {
            Command::SetLevel {
                target,
                value: Value::Numeric(255.0),
            }
        } else if let Ok(num) = val_str.parse::<f32>() {
            Command::SetLevel {
                target,
                value: Value::Numeric(num * multiplier),
            }
        } else {
            let lower = val_str.to_lowercase();
            if ["red", "green", "blue", "purple", "white"].contains(&lower.as_str()) {
                Command::SetColor {
                    target,
                    color: Value::Semantic(lower),
                }
            } else if ["center", "up", "down"].contains(&lower.as_str()) {
                Command::SetPosition {
                    target,
                    pos: Value::Semantic(lower),
                }
            } else {
                Command::SetLevel {
                    target,
                    value: Value::Semantic(lower),
                }
            }
        }
    };

    Some(vec![command])
}

fn parse_target(target_str: &str) -> Option<Target> {
    let mut channels = Vec::new();
    let mut fixtures = Vec::new();
    let mut groups = Vec::new();

    for part in target_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let lower = part.to_lowercase();
        if lower.starts_with('g') {
            groups.push(part[1..].to_string());
        } else if lower.starts_with('f') {
            let f_part = &lower[1..];
            if f_part.contains("..") {
                let range: Vec<&str> = f_part.split("..").collect();
                if range.len() == 2 {
                    if let (Ok(start), Ok(end)) = (range[0].parse::<u16>(), range[1].parse::<u16>())
                    {
                        for i in start..=end {
                            fixtures.push(format!("F{}", i));
                        }
                    }
                }
            } else if let Ok(num) = f_part.parse::<u16>() {
                fixtures.push(format!("F{}", num));
            } else {
                fixtures.push(part[1..].to_string()); // use original case or lower?
            }
        } else if lower.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            if lower.contains("..") {
                let range_and_step: Vec<&str> = lower.split('*').collect();
                let range: Vec<&str> = range_and_step[0].split("..").collect();
                let step = if range_and_step.len() == 2 {
                    range_and_step[1].parse::<usize>().unwrap_or(1)
                } else {
                    1
                };

                if range.len() == 2 {
                    if let (Ok(start), Ok(end)) = (range[0].parse::<u16>(), range[1].parse::<u16>())
                    {
                        for ch in (start..=end).step_by(step) {
                            channels.push(ch);
                        }
                    }
                }
            } else if let Ok(ch) = lower.parse::<u16>() {
                channels.push(ch);
            }
        }
    }

    let mut targets = Vec::new();
    if !channels.is_empty() {
        channels.sort_unstable();
        channels.dedup();
        targets.push(Target::Channels(ChannelList(channels)));
    }
    if !fixtures.is_empty() {
        targets.push(Target::Fixtures(fixtures));
    }
    if !groups.is_empty() {
        targets.push(Target::Groups(groups));
    }

    match targets.len() {
        0 => None,
        1 => Some(targets.into_iter().next().unwrap()),
        _ => Some(Target::Mixed(targets)),
    }
}

fn parse_duration(s: &str) -> Option<Duration> {
    if s.ends_with('b') {
        let num_str = &s[..s.len() - 1];
        if num_str.contains('/') {
            let parts: Vec<&str> = num_str.splitn(2, '/').collect();
            if let (Ok(num), Ok(den)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                if den != 0.0 {
                    return Some(Duration::Tempo(num / den));
                }
            }
        } else if let Ok(val) = num_str.parse::<f32>() {
            return Some(Duration::Tempo(val));
        }
    } else if s.ends_with('s') {
        let num_str = &s[..s.len() - 1];
        if let Ok(val) = num_str.parse::<f32>() {
            return Some(Duration::Time(val));
        }
    } else if let Ok(val) = s.parse::<f32>() {
        return Some(Duration::Time(val));
    }
    None
}
