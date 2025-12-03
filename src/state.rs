use serde::{Deserialize, Serialize};
use std::env::var;
use std::io::Result;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scratchpad {
    pub title: Option<String>,
    pub id: u64,
    pub command: Option<Vec<String>>,
    pub scratchpad_number: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub scratchpads: Vec<Scratchpad>,
}

pub enum AddResult {
    Added,
    AlreadyExists(Scratchpad),
}

impl State {
    pub fn new() -> Result<Self> {
        let state_path = Self::get_state_path()?;

        if state_path.exists() {
            let contents = fs::read_to_string(&state_path)?;
            let state: State = serde_json::from_str(&contents).unwrap_or_else(|_| State {
                scratchpads: vec![],
            });
            Ok(state)
        } else {
            let state = State {
                scratchpads: vec![],
            };
            let json = serde_json::to_string_pretty(&state)?;
            if let Some(parent) = state_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&state_path, json)?;
            Ok(state)
        }
    }

    pub fn update(&self) -> Result<()> {
        let state_path = Self::get_state_path()?;
        if state_path.exists() {
            let json = serde_json::to_string_pretty(&self)?;
            if let Some(parent) = state_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&state_path, json)?;
            return Ok(());
        }
        Ok(())
    }

    fn get_state_path() -> Result<PathBuf> {
        let runtime_dir = var("XDG_RUNTIME_DIR")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "XDG_RUNTIME_DIR not set"))?;
        Ok(PathBuf::from(runtime_dir).join("niri-scratchpad.json"))
    }

    pub fn add_scratchpad(
        &mut self,
        scratchpad_number: i32,
        id: u64,
        title: Option<String>,
        command_str: Option<String>,
    ) -> Result<()> {
        self.scratchpads.push(Scratchpad {
            id,
            command: command_str.map(|command| {
                command
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            }),
            scratchpad_number,
            title
        });
        Ok(())
    }

    pub fn delete_scratchpad(&mut self, scratchpad_number: i32) -> Result<()> {
        self.scratchpads
            .retain(|scratchpad| scratchpad.scratchpad_number != scratchpad_number);
        self.update()?;
        Ok(())
    }

    pub fn get_scratchpad_by_number(&self, scratchpad_number: i32) -> Option<Scratchpad> {
        self.scratchpads
            .iter()
            .find(|scratchpad| scratchpad.scratchpad_number == scratchpad_number)
            .cloned()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| State {
            scratchpads: vec![],
        })
    }
}
