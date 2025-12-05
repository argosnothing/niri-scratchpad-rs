use serde::{Deserialize, Serialize};
use std::{env::var, io::{Result}, path::PathBuf, hash::{Hash}, fs, io};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Scratchpad {
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub id: u64,
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

pub enum ScratchpadUpdate {
    Add(Scratchpad),
    Update(Scratchpad),
    Delete(i32),
}

impl State {
    pub fn new() -> Result<Self> {
        let state_path = Self::get_state_path()?;

        if state_path.exists() {
            let contents = fs::read_to_string(&state_path)?;
            let state: State = serde_json::from_str(&contents)?;
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
        app_id: Option<String>,
    ) -> Result<()> {
        self.scratchpads.push(Scratchpad {
            id,
            scratchpad_number,
            app_id,
            title,
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

    pub fn get_tracked_scratchpads(&self) -> Vec<&Scratchpad> {
        self.scratchpads.iter().collect()
    }

    pub fn syncronize_scratchpads(
        &mut self,
        scratchpad_updates: Vec<ScratchpadUpdate>,
    ) -> Result<()> {
        for scratchpad_update in scratchpad_updates {
            match scratchpad_update {
                ScratchpadUpdate::Add(scratchpad) => self.scratchpads.push(scratchpad),
                ScratchpadUpdate::Update(scratchpad) => {
                    if let Some(stored_scratchpad) =
                        self.scratchpads.iter_mut().find(|found_scratchpad| {
                            found_scratchpad.scratchpad_number == scratchpad.scratchpad_number
                        })
                    {
                        *stored_scratchpad = scratchpad;
                    }
                }
                ScratchpadUpdate::Delete(scratchpad_number) => {
                    self.scratchpads.retain(|stored_scratchpad| {
                        stored_scratchpad.scratchpad_number != scratchpad_number
                    })
                }
            };
        }
        Ok(())
    }

    pub fn update_scratchpad(&mut self, scratchpad_update: Scratchpad) -> Result<()> {
        let Some(scratchpad) = self
            .scratchpads
            .iter_mut()
            .find(|scratchpad| scratchpad.scratchpad_number == scratchpad_update.scratchpad_number)
        else {
            return Ok(());
        };
        *scratchpad = scratchpad_update;
        self.update()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| State {
            scratchpads: vec![],
        })
    }
}
