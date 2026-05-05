use crate::engine::command::{Command, Cue, Duration, Value};
use crate::engine::target::{Target, ChannelList};
use crate::engine::effects::{Effect, Easing, MathMethod};

pub fn parse_command_line(line: &str) -> Option<Cue> {
    if let Some(cmd) = parse_command(line) {
        let mut cue = Cue::new("CLI", vec![cmd]);
        cue.raw_commands = vec![line.to_string()];
        Some(cue)
    } else {
        None
    }
}

pub fn parse_command(line: &str) -> Option<Command> {
    let line = line.trim();
    if line.is_empty() { return None; }
    
    let parts: Vec<&str> = line.splitn(2, '@').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let target_str = parts[0].trim();
    let action_str = parts[1].trim();
    
    let target = if target_str.starts_with('g') || target_str.starts_with('G') {
        Target::Groups(vec![target_str[1..].to_string()])
    } else if target_str.starts_with('f') || target_str.starts_with('F') {
        Target::Fixtures(vec![target_str.to_uppercase()])
    } else if target_str.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',' || c.is_whitespace()) {
        let mut channels = Vec::new();
        for part in target_str.split(',') {
            let part = part.trim();
            if part.contains("..") {
                let range: Vec<&str> = part.split("..").collect();
                if range.len() == 2 {
                    if let (Ok(start), Ok(end)) = (range[0].parse::<u16>(), range[1].parse::<u16>()) {
                        for ch in start..=end {
                            channels.push(ch);
                        }
                    } else { return None; }
                } else { return None; }
            } else if let Ok(ch) = part.parse::<u16>() {
                channels.push(ch);
            }
        }
        channels.sort_unstable();
        channels.dedup();
        if channels.is_empty() { return None; }
        Target::Channels(ChannelList(channels))
    } else {
        Target::Parameters(vec![target_str.to_string()])
    };

    let command = if action_str.starts_with("cycle(") && action_str.ends_with(')') {
        let inner = &action_str[6..action_str.len()-1];
        let args: Vec<&str> = inner.split(',').collect();
        if args.len() == 3 {
            let min_val = args[0].trim().parse::<f32>().unwrap_or(0.0);
            let max_val = args[1].trim().parse::<f32>().unwrap_or(255.0);
            let duration = parse_duration(args[2].trim()).unwrap_or(Duration::Time(1.0));
            Command::StartEffect {
                target,
                effect: Effect::Cycle {
                    min_val,
                    max_val,
                    duration,
                    easing: Easing::Sine,
                    method: MathMethod::Absolute,
                }
            }
        } else { return None; }
    } else if action_str.contains('/') {
        let parts: Vec<&str> = action_str.splitn(2, '/').collect();
        let val_str = parts[0].trim();
        let dur_str = parts[1].trim();
        
        let target_val = val_str.parse::<f32>().unwrap_or(0.0);
        let duration = parse_duration(dur_str).unwrap_or(Duration::Time(0.0));
        
        Command::StartEffect {
            target,
            effect: Effect::Fade { target_val, duration }
        }
    } else {
        let val_str = if action_str.ends_with('!') { &action_str[..action_str.len()-1] } else { action_str };
        if let Ok(num) = val_str.parse::<f32>() {
            Command::SetLevel { target, value: Value::Numeric(num) }
        } else {
            let lower = val_str.to_lowercase();
            if ["red", "green", "blue", "purple", "white"].contains(&lower.as_str()) {
                Command::SetColor { target, color: Value::Semantic(lower) }
            } else if ["center", "up", "down"].contains(&lower.as_str()) {
                Command::SetPosition { target, pos: Value::Semantic(lower) }
            } else {
                Command::SetLevel { target, value: Value::Semantic(lower) }
            }
        }
    };

    Some(command)
}

fn parse_duration(s: &str) -> Option<Duration> {
    if s.ends_with('b') {
        let num_str = &s[..s.len()-1];
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
        let num_str = &s[..s.len()-1];
        if let Ok(val) = num_str.parse::<f32>() {
            return Some(Duration::Time(val));
        }
    } else {
        if let Ok(val) = s.parse::<f32>() {
            return Some(Duration::Time(val));
        }
    }
    None
}
