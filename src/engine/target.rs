use serde::{Deserialize, Serialize, Serializer, Deserializer, de};

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelList(pub Vec<u16>);

impl Serialize for ChannelList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut result = String::new();
        if self.0.is_empty() {
            return serializer.serialize_str("");
        }

        let mut sorted = self.0.clone();
        sorted.sort_unstable();
        
        let mut start = sorted[0];
        let mut end = start;

        for &ch in sorted.iter().skip(1) {
            if ch == end + 1 {
                end = ch;
            } else {
                if start == end {
                    result.push_str(&format!("{},", start));
                } else {
                    result.push_str(&format!("{}..{},", start, end));
                }
                start = ch;
                end = ch;
            }
        }
        
        if start == end {
            result.push_str(&format!("{}", start));
        } else {
            result.push_str(&format!("{}..{}", start, end));
        }

        serializer.serialize_str(&result)
    }
}

impl<'de> Deserialize<'de> for ChannelList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut channels = Vec::new();
        if s.trim().is_empty() {
            return Ok(ChannelList(channels));
        }

        for part in s.split(',') {
            let part = part.trim();
            if part.contains("..") {
                let range: Vec<&str> = part.split("..").collect();
                if range.len() == 2 {
                    let start = range[0].parse::<u16>().map_err(de::Error::custom)?;
                    let end = range[1].parse::<u16>().map_err(de::Error::custom)?;
                    for ch in start..=end {
                        channels.push(ch);
                    }
                } else {
                    return Err(de::Error::custom("Invalid range format"));
                }
            } else {
                let ch = part.parse::<u16>().map_err(de::Error::custom)?;
                channels.push(ch);
            }
        }
        
        channels.sort_unstable();
        channels.dedup();

        Ok(ChannelList(channels))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Target {
    Channels(ChannelList),
    Fixtures(Vec<String>),
    Groups(Vec<String>),
    Parameters(Vec<String>),
    Mixed(Vec<Target>),
}
