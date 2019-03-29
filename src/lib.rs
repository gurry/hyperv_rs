use powershell_rs::{PsCommand, Stdio, PsProcess, Output};
use failure::Fail;
use serde_derive::Deserialize;
use uuid::Uuid;
use std::fmt;
use std::path::Path;
use std::io::{BufReader, BufRead};

pub struct Hyperv;

pub type Result<T> = std::result::Result<T, HypervError>;

impl Hyperv {
    pub fn get_vms() -> Result<Vec<Vm>> {
        let process = Self::spawn("get-vm|select-object -property Id,Name |convertto-json")?;
        let stdout = process.stdout().ok_or_else(|| HypervError::new("Could not access stdout of powershell process"))?;

        let vms: Vec<Vm> = serde_json::from_reader(stdout)
            .map_err(|e| HypervError::new(format!("Failed to parse powershell output: {}", e)))?;

        Ok(vms)
    }

    pub fn import_vm<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = Self::validate_file_path(path.as_ref())?;
        Self::spawn_and_wait(&format!("import-vm -Path \"{}\"", path))?;
        Ok(())
    }

    pub fn compare_vm<P: AsRef<Path>>(path: P) -> Result<Vec<VmIncompatibility>> {
        let path = Self::validate_file_path(path.as_ref())?;
        let command = format!(
            "$report = compare-vm -Path \"{}\";
            $report.Incompatibilities | Format-Table -Property MessageId, Message -HideTableHeaders"
            , path);
        let process = Self::spawn(&command)?;

        Self::map_lines(process, |line: &str| {
            let line = line.trim();
            if line.is_empty() {
                return Ok(None)
            }
            let mut parts = line.splitn(2, ' ');
            let msg_id = parts.next().ok_or_else(|| HypervError { msg: "Failed to parse to VmIncomatibility. No MessageId in string".to_owned() })?;
            let msg = parts.next().ok_or_else(|| HypervError { msg: "Failed to parse to VmIncomatibility. No Message in string".to_owned() })?;
            let msg_id = msg_id.parse::<i64>().map_err(|e| HypervError { msg: format!("Failed to parse to VmIncomatibility. Cannot parse MessageId to i64: {}", e) })?;
            Ok(Some(VmIncompatibility::from(msg_id, msg.to_owned())))
        })
    }

    fn validate_file_path(path: &Path) -> Result<&str> {
        if !path.is_file() {
            Err(HypervError::new("Path does not point to a valid file"))
        } else {
            let path = path.to_str().ok_or_else(|| HypervError { msg: "Bad path".to_owned() })?;
            Ok(path)
        }
    }

    fn map_lines<T, F: Fn(&str) -> Result<Option<T>>>(process: PsProcess, f: F) -> Result<Vec<T>> {
        let stdout = process.stdout().ok_or_else(|| HypervError::new("Could not access stdout of powershell process"))?;
        
        let mut vec = Vec::new();
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    if let Some(t) = f(&line)? {
                        vec.push(t)
                    }
                }
                Err(e) => Err(HypervError::new(format!("Failed to process powershell output. Could not split stdout into lines: {}", e)))?,
            }
        }

        Ok(vec)
    }

    fn spawn(command: &str) -> Result<PsProcess> {
        PsCommand::new(command)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| HypervError::new(format!("Failed to spawn PowerShell process: {}", e)))
    }

    fn spawn_and_wait(command: &str) -> Result<Output> {
        let output = Self::spawn(command)?
            .wait_with_output()
            .map_err(|e| HypervError::new(format!("Failed to spawn PowerShell process: {}", e)))?;

        if !output.status.success() {
            let exit_code_str = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "<none>".to_owned());
            let stdout = to_string_truncated(&output.stdout, 1000);
            let stderr = to_string_truncated(&output.stderr, 1000);
            fn handle_blank(s: String) -> String { if !s.is_empty() { s } else { "<empty>".to_owned() } }
            return Err(HypervError { msg: format!("Powershell returned failure exit code: {}.\nStdout: {} \nStderr: {}", exit_code_str, handle_blank(stdout), handle_blank(stderr)) });
        }

        Ok(output)
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

#[derive(Debug)]
pub enum VmIncompatibility {
    CannotCreateExternalConfigStore(String),
    TooManyCores(String),
    CannotChangeCheckpointLocation(String),
    CannotChangeSmartPagingStore(String),
    CannotRestoreSavedState(String),
    MissingSwitch(String),
    Other(String, i64),
}

impl VmIncompatibility {
    fn from(msg_id: i64, msg: String) -> Self {
        match msg_id {
            13000 => VmIncompatibility::CannotCreateExternalConfigStore(msg),
            14420 => VmIncompatibility::TooManyCores(msg),
            16350 => VmIncompatibility::CannotChangeCheckpointLocation(msg),
            16352 => VmIncompatibility::CannotChangeSmartPagingStore(msg),
            25014 => VmIncompatibility::CannotRestoreSavedState(msg),
            33012 => VmIncompatibility::MissingSwitch(msg),
            msg_id => VmIncompatibility::Other(msg, msg_id)
        }
    }

    pub fn message_id(&self) -> i64 {
        match self {
            VmIncompatibility::CannotCreateExternalConfigStore(_) => 13000,
            VmIncompatibility::TooManyCores(_) => 14420,
            VmIncompatibility::CannotChangeCheckpointLocation(_) => 16350,
            VmIncompatibility::CannotChangeSmartPagingStore(_) => 16352,
            VmIncompatibility::CannotRestoreSavedState(_) => 25014,
            VmIncompatibility::MissingSwitch(_) => 33012,
            VmIncompatibility::Other(_, i) => *i,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            VmIncompatibility::CannotCreateExternalConfigStore(s) => &s,
            VmIncompatibility::TooManyCores(s) => &s,
            VmIncompatibility::CannotChangeCheckpointLocation(s) => &s,
            VmIncompatibility::CannotChangeSmartPagingStore(s) => &s,
            VmIncompatibility::CannotRestoreSavedState(s) => &s,
            VmIncompatibility::MissingSwitch(s) => &s,
            VmIncompatibility::Other(s, _) => &s,
        }
    }
}

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

fn to_string_truncated(bytes: &[u8], take: usize) -> String {
    let len = std::cmp::min(bytes.len(), take);
    String::from_utf8_lossy(&bytes[..len]).to_string()
}