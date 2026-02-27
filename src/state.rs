/// Register-based scratchpad utilize state to track the currently assigned window to that scratchpad register.
use serde::{Deserialize, Serialize};
use std::{hash::Hash, io::Result};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Register {
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub window_id: u64,
    pub number: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub registers: Vec<Register>,
}

pub enum AddResult {
    Added,
    AlreadyExists(Register),
}

pub enum RegisterUpdate {
    Add(Register),
    Update(Register),
    Delete(i32),
}

impl State {
    pub fn new() -> Self {
        State { registers: vec![] }
    }

    pub fn add_register(
        &mut self,
        register_number: i32,
        id: u64,
        title: Option<String>,
        app_id: Option<String>,
    ) -> Result<()> {
        self.registers.push(Register {
            window_id: id,
            number: register_number,
            app_id,
            title,
        });
        Ok(())
    }

    pub fn delete_register(&mut self, register_number: i32) {
        self.registers
            .retain(|register| register.number != register_number);
    }

    pub fn get_register_by_number(&self, register_number: i32) -> Option<Register> {
        self.registers
            .iter()
            .find(|register| register.number == register_number)
            .cloned()
    }

    pub fn get_register_ref_by_number(&self, register_number: i32) -> Option<&Register> {
        self.registers
            .iter()
            .find(|register| register.number == register_number)
    }

    pub fn get_tracked_registers(&self) -> Vec<&Register> {
        self.registers.iter().collect()
    }

    pub fn syncronize_registers(&mut self, register_updates: Vec<RegisterUpdate>) -> Result<()> {
        for register_update in register_updates {
            match register_update {
                RegisterUpdate::Add(register) => self.registers.push(register),
                RegisterUpdate::Update(register) => {
                    if let Some(stored_register) = self
                        .registers
                        .iter_mut()
                        .find(|found_register| found_register.number == register.number)
                    {
                        *stored_register = register;
                    }
                }
                RegisterUpdate::Delete(register_number) => self
                    .registers
                    .retain(|stored_register| stored_register.number != register_number),
            };
        }
        Ok(())
    }

    pub fn update_register(&mut self, register_update: Register) {
        let Some(register) = self
            .registers
            .iter_mut()
            .find(|register| register.number == register_update.number)
        else {
            return ();
        };
        *register = register_update;
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
