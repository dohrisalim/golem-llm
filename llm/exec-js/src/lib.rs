use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use tempfile::NamedTempFile;
use thiserror::Error;
use base64::Engine;

mod bindings;

use bindings::exports::golem::exec::{
    executor::{Guest as ExecutorGuest},
    session::{Guest as SessionGuest},
};

use bindings::golem::exec::types::{
    Language, LanguageKind, File, Limits, StageResult, ExecResult, Error, Encoding,
};

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    #[error("Timeout")]
    Timeout,
    #[error("Resource exceeded")]
    ResourceExceeded,
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<ExecError> for Error {
    fn from(value: ExecError) -> Self {
        match value {
            ExecError::UnsupportedLanguage(_) => Error::UnsupportedLanguage,
            ExecError::CompilationFailed(msg) => Error::CompilationFailed(StageResult {
                stdout: String::new(),
                stderr: msg,
                exit_code: Some(1),
                signal: None,
            }),
            ExecError::RuntimeError(msg) => Error::RuntimeFailed(StageResult {
                stdout: String::new(),
                stderr: msg,
                exit_code: Some(1),
                signal: None,
            }),
            ExecError::Timeout => Error::Timeout,
            ExecError::ResourceExceeded => Error::ResourceExceeded,
            ExecError::Internal(msg) => Error::Internal(msg),
            ExecError::IoError(e) => Error::Internal(format!("IO error: {}", e)),
        }
    }
}

fn decode_content(file: &File) -> Result<Vec<u8>, ExecError> {
    let content = match file.encoding {
        Some(Encoding::Utf8) | None => file.content.clone(),
        Some(Encoding::Base64) => {
            let content_str = String::from_utf8(file.content.clone())
                .map_err(|e| ExecError::Internal(format!("Invalid UTF-8 in base64 content: {}", e)))?;
            base64::engine::general_purpose::STANDARD.decode(&content_str)
                .map_err(|e| ExecError::Internal(format!("Base64 decode error: {}", e)))?
        },
        Some(Encoding::Hex) => {
            let content_str = String::from_utf8(file.content.clone())
                .map_err(|e| ExecError::Internal(format!("Invalid UTF-8 in hex content: {}", e)))?;
            hex::decode(&content_str)
                .map_err(|e| ExecError::Internal(format!("Hex decode error: {}", e)))?
        },
    };
    Ok(content)
}

fn execute_javascript(
    code: &str,
    args: Vec<String>,
    env_vars: Vec<(String, String)>,
    stdin: Option<String>,
    constraints: Option<Limits>,
) -> Result<ExecResult, ExecError> {
    let start_time = Instant::now();

    // Create a temporary file with the JavaScript code
    let mut temp_file = NamedTempFile::new()?;
    writeln!(temp_file, "{}", code)?;

    // Build the command - try node first, then nodejs
    let mut cmd = Command::new("node");
    cmd.arg(temp_file.path());
    
    // Add arguments to the JavaScript process
    for arg in &args {
        cmd.arg(arg);
    }

    // Set up environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Set up stdin if provided
    let mut child = if stdin.is_some() {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .or_else(|_| {
                // Fallback to nodejs if node command doesn't exist
                let mut fallback_cmd = Command::new("nodejs");
                fallback_cmd.arg(temp_file.path());
                for arg in &args {
                    fallback_cmd.arg(arg);
                }
                fallback_cmd.stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
            })?
    } else {
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .or_else(|_| {
                // Fallback to nodejs if node command doesn't exist
                let mut fallback_cmd = Command::new("nodejs");
                fallback_cmd.arg(temp_file.path());
                for arg in &args {
                    fallback_cmd.arg(arg);
                }
                fallback_cmd.stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
            })?
    };

    if let Some(stdin_content) = stdin {
        if let Some(mut stdin_handle) = child.stdin.take() {
            stdin_handle.write_all(stdin_content.as_bytes())?;
        }
    }

    // Execute the command with timeout
    let output = if let Some(constraints) = constraints {
        if let Some(time_ms) = constraints.time_ms {
            // For WASM builds without wait_timeout, use a simple wait
            #[cfg(target_arch = "wasm32")]
            {
                let _ = time_ms; // Suppress unused warning for WASM
                child.wait_with_output()?
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                use wait_timeout::ChildExt;
                match child.wait_timeout(Duration::from_millis(time_ms))? {
                    Some(_status) => {
                        child.wait_with_output()?
                    }
                    None => {
                        child.kill().map_err(|e| ExecError::Internal(format!("Failed to kill process: {}", e)))?;
                        return Err(ExecError::Timeout);
                    }
                }
            }
        } else {
            child.wait_with_output()?
        }
    } else {
        child.wait_with_output()?
    };

    let elapsed_time = start_time.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok(ExecResult {
        compile: None,
        run: StageResult {
            stdout,
            stderr,
            exit_code: Some(output.status.code().unwrap_or(-1)),
            signal: None,
        },
        time_ms: Some(elapsed_time.as_millis() as u64),
        memory_bytes: None,
    })
}

struct ExecutionSession {
    language: Language,
    files: HashMap<String, Vec<u8>>,
    working_dir: String,
}

impl ExecutionSession {
    fn new(language: Language) -> Result<Self, Error> {
        if language.kind != LanguageKind::Javascript {
            return Err(Error::UnsupportedLanguage);
        }

        Ok(ExecutionSession {
            language,
            files: HashMap::new(),
            working_dir: "/".to_string(),
        })
    }

    fn upload_file(&mut self, file: File) -> Result<(), Error> {
        let content = decode_content(&file)?;
        self.files.insert(file.name, content);
        Ok(())
    }

    fn execute_code(&self, entrypoint: &str, args: Vec<String>, env: Vec<(String, String)>, stdin: Option<String>, constraints: Option<Limits>) -> Result<ExecResult, Error> {
        let code_content = self.files.get(entrypoint)
            .ok_or_else(|| Error::Internal(format!("Entrypoint file '{}' not found", entrypoint)))?;

        let code = String::from_utf8(code_content.clone())
            .map_err(|e| Error::Internal(format!("Invalid UTF-8 in JavaScript code: {}", e)))?;

        execute_javascript(&code, args, env, stdin, constraints)
            .map_err(Into::into)
    }
}

// Global session storage (simple approach for now)
static mut SESSIONS: Option<HashMap<u32, ExecutionSession>> = None;
static mut SESSION_COUNTER: u32 = 1;

fn get_sessions() -> &'static mut HashMap<u32, ExecutionSession> {
    unsafe {
        if SESSIONS.is_none() {
            SESSIONS = Some(HashMap::new());
        }
        SESSIONS.as_mut().unwrap()
    }
}

fn next_session_id() -> u32 {
    unsafe {
        let id = SESSION_COUNTER;
        SESSION_COUNTER += 1;
        id
    }
}

struct Executor;

impl ExecutorGuest for Executor {
    fn run(
        lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        if lang.kind != LanguageKind::Javascript {
            return Err(Error::UnsupportedLanguage);
        }

        // Find the main file to execute
        let main_file = files.iter()
            .find(|f| f.name == "main.js" || f.name == "index.js")
            .or_else(|| files.first())
            .ok_or_else(|| Error::Internal("No JavaScript files provided".to_string()))?;

        let code_content = decode_content(main_file)?;
        let code = String::from_utf8(code_content)
            .map_err(|e| Error::Internal(format!("Invalid UTF-8 in JavaScript code: {}", e)))?;

        execute_javascript(&code, args, env, stdin, constraints)
            .map_err(Into::into)
    }
}

struct Session;

impl SessionGuest for Session {
    fn create(lang: Language) -> Result<u32, Error> {
        let handle = next_session_id();
        let session = ExecutionSession::new(lang)?;
        get_sessions().insert(handle, session);
        Ok(handle)
    }

    fn upload(session: u32, file: File) -> Result<(), Error> {
        let sessions = get_sessions();
        let session = sessions.get_mut(&session)
            .ok_or_else(|| Error::Internal("Session not found".to_string()))?;
        session.upload_file(file)
    }

    fn run(
        session: u32,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        let sessions = get_sessions();
        let session = sessions.get(&session)
            .ok_or_else(|| Error::Internal("Session not found".to_string()))?;

        session.execute_code(&entrypoint, args, env, stdin, constraints)
    }

    fn download(session: u32, path: String) -> Result<Vec<u8>, Error> {
        let sessions = get_sessions();
        let session = sessions.get(&session)
            .ok_or_else(|| Error::Internal("Session not found".to_string()))?;

        session.files.get(&path)
            .cloned()
            .ok_or_else(|| Error::Internal(format!("File '{}' not found", path)))
    }

    fn list_files(session: u32, _dir: String) -> Result<Vec<String>, Error> {
        let sessions = get_sessions();
        let session = sessions.get(&session)
            .ok_or_else(|| Error::Internal("Session not found".to_string()))?;

        Ok(session.files.keys().cloned().collect())
    }

    fn set_working_dir(session: u32, path: String) -> Result<(), Error> {
        let sessions = get_sessions();
        let session = sessions.get_mut(&session)
            .ok_or_else(|| Error::Internal("Session not found".to_string()))?;

        session.working_dir = path;
        Ok(())
    }

    fn close(session: u32) {
        get_sessions().remove(&session);
    }
}

// Export the component implementations
struct Component;

impl bindings::exports::golem::exec::executor::Guest for Component {
    fn run(
        lang: Language,
        files: Vec<File>,
        stdin: Option<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        Executor::run(lang, files, stdin, args, env, constraints)
    }
}

impl bindings::exports::golem::exec::session::Guest for Component {
    fn create(lang: Language) -> Result<u32, Error> {
        Session::create(lang)
    }

    fn upload(session: u32, file: File) -> Result<(), Error> {
        Session::upload(session, file)
    }

    fn run(
        session: u32,
        entrypoint: String,
        args: Vec<String>,
        stdin: Option<String>,
        env: Vec<(String, String)>,
        constraints: Option<Limits>,
    ) -> Result<ExecResult, Error> {
        Session::run(session, entrypoint, args, stdin, env, constraints)
    }

    fn download(session: u32, path: String) -> Result<Vec<u8>, Error> {
        Session::download(session, path)
    }

    fn list_files(session: u32, dir: String) -> Result<Vec<String>, Error> {
        Session::list_files(session, dir)
    }

    fn set_working_dir(session: u32, path: String) -> Result<(), Error> {
        Session::set_working_dir(session, path)
    }

    fn close(session: u32) {
        Session::close(session)
    }
}

bindings::export!(Component with_types_in bindings);

