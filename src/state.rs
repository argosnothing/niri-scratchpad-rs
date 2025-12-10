use serde::{Deserialize, Serialize};
use std::{hash::Hash, io::Result};

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
    pub fn new() -> Self {
        State {
            scratchpads: vec![],
        }
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

    pub fn delete_scratchpad(&mut self, scratchpad_number: i32) {
        self.scratchpads
            .retain(|scratchpad| scratchpad.scratchpad_number != scratchpad_number);
    }

    pub fn get_scratchpad_by_number(&self, scratchpad_number: i32) -> Option<Scratchpad> {
        self.scratchpads
            .iter()
            .find(|scratchpad| scratchpad.scratchpad_number == scratchpad_number)
            .cloned()
    }

    pub fn get_scratchpad_ref_by_number(&self, scratchpad_number: i32) -> Option<&Scratchpad> {
        self.scratchpads
            .iter()
            .find(|scratchpad| scratchpad.scratchpad_number == scratchpad_number)
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

    pub fn update_scratchpad(&mut self, scratchpad_update: Scratchpad) {
        let Some(scratchpad) = self
            .scratchpads
            .iter_mut()
            .find(|scratchpad| scratchpad.scratchpad_number == scratchpad_update.scratchpad_number)
        else {
            return ();
        };
        *scratchpad = scratchpad_update;
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
