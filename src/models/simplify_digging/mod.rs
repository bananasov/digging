use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum SimplifyPacket {
    Init {
        // Fuck you, chummy
        program_arguments: serde_json::Value,
        turtle_id: u16,
    },

    Keepalive,

    State {
        state: States,
    },

    Status {
        pos: Position,
        facing: Facing,
        fuel: Fuel,
    },

    Completion {
        completion_percent: f32,
    },

    Complete,

    Panic {
        reason: String,
        pos: Position,
        facing: Facing,
        fuel: Fuel,
    },

    Error {
        message: String,
        pos: Position,
        facing: Facing,
        fuel: Fuel,
    },
}

/// The current state of the turtle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum States {
    /// The turtle is initializing (recovering or just starting up).
    #[default]
    Init,
    /// The turtle is idle, waiting for work (or in between states).
    Idle,
    /// The turtle is actively digging.
    Digging,
    /// The turtle is returning home to refuel/dump items.
    ReturnHome,
    /// The turtle is returning to the mine after going home to refuel/dump items.
    ReturnMine,
    /// The turtle is stuck and cannot continue without intervention.
    Stuck,
    /// The turtle has encountered an error and cannot continue.
    Error,
    /// Dig is complete.
    Done,
    /// Easter egg state
    Teapot,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Facing {
    #[default]
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl Serialize for Facing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for Facing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = u8::deserialize(deserializer)?;
        Self::try_from(n).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<u8> for Facing {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::North),
            1 => Ok(Self::East),
            2 => Ok(Self::South),
            3 => Ok(Self::West),
            _ => Err("Facing must be 0..=3"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fuel {
    Limited(i32),
    Unlimited,
}

impl Default for Fuel {
    fn default() -> Self {
        Self::Limited(0)
    }
}

impl Serialize for Fuel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Limited(amount) => serializer.serialize_i32(*amount),
            Self::Unlimited => serializer.serialize_str("unlimited"),
        }
    }
}

impl<'de> Deserialize<'de> for Fuel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum FuelHelper {
            Int(i32),
            String(String),
        }

        match FuelHelper::deserialize(deserializer)? {
            FuelHelper::Int(amount) => Ok(Self::Limited(amount)),
            FuelHelper::String(s) if s == "unlimited" => Ok(Self::Unlimited),
            FuelHelper::String(s) => {
                Err(serde::de::Error::custom(format!("Invalid fuel value: {s}")))
            }
        }
    }
}
