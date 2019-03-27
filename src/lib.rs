use powershell_rs::{PsCommand, Stdio, PsProcess, ExitStatus};
use failure::Fail;
use serde_derive::Deserialize;
use uuid::Uuid;
use std::fmt;

pub struct Hyperv;

pub type Result<T> = std::result::Result<T, HypervError>;

impl Hyperv {
    pub fn get_vms() -> Result<Vec<Vm>> {
        let process = Self::spawn("get-vm|select-object -property Id,Name |convertto-json")?;
        let stdout = process.stdout().ok_or(HypervError::new("Could not access stdout of powershell process"))?;

        let vms: Vec<Vm> = serde_json::from_reader(stdout)
            .map_err(|e| HypervError::new(format!("Failed to parse powershell output: {}", e)))?;

        Ok(vms)
    }

    fn spawn(command: &str) -> Result<PsProcess> {
        PsCommand::new(command)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| HypervError::new(format!("Failed to spawn PowerShell process: {}", e)))
    }

    fn spawn_and_wait(command: &str) -> Result<ExitStatus> {
        Self::spawn(command)?
            .wait()
            .map_err(|e| HypervError::new(format!("Failed to spawn PowerShell process: {}", e)))
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

impl HypervError {
    fn new<T: Into<String>>(msg: T) -> Self {
        Self { msg: msg.into() }
    }
}

impl fmt::Display for HypervError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}