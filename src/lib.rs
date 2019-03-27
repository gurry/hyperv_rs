use powershell_rs::{PsCommand, Stdio};
use failure::Fail;
use serde_derive::Deserialize;
use uuid::Uuid;
use std::fmt;

pub struct Hyperv;

pub type Result<T> = std::result::Result<T, HypervError>;

impl Hyperv {
    pub fn get_vms() -> Result<Vec<Vm>> {
        let process = PsCommand::new("get-vm|select-object -property Id,Name |convertto-json")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| HypervError { msg: format!("Failed to get VMs. {}", e) })?;
        
        let stdout = process.stdout().ok_or(HypervError { msg: "Could not access stdout of powershell process".to_owned()})?;

        let vms: Vec<Vm> = serde_json::from_reader(stdout)
            .map_err(|e| HypervError { msg: format!("Failed to get VMs. Failed to parse powershell output: {}", e) })?;

        Ok(vms)
    }
}

#[derive(Debug, Deserialize)]
pub struct Vm {
    #[serde(rename = "Id")]
    pub id: VmId,
    #[serde(rename = "Name")]
    pub name: String,
}

// TODO: should this be a newtype?
pub type VmId = Uuid;

// TODO: We need to do proper design of error types. Just this one type is not enough
#[derive(Debug, Fail)]
pub struct HypervError  {
    pub msg: String,
}

impl fmt::Display for HypervError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}