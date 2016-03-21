use std::collections::HashMap;
use std::path::{ Path, PathBuf };

use ast::Script;
use compile::ExecutableDevEnv;
use parse::ParseError;
use run::{ Execution, ExecutionEvent, Error as RunError, StartStopError };

use foxbox_taxonomy::util::{ Id };
use foxbox_taxonomy::api::ResultMap;
use rusqlite;
use transformable_channels::mpsc::{ channel, ExtSender, TransformableSender };

/// A ScriptManager error.
#[derive(Serialize, Debug)]
pub enum Error {
    /// The script you requested (by ID) does not exist.
    NoSuchScriptError,

    /// There was an error executing some SQL.
    SQLError(String),

    /// There was an error attempting to run a script. (See `run.rs`.)
    RunError(RunError),

    /// There was an error parsing the script's JSON.
    ParseError(String),
}

/// A type for ensuring type-safety (Id<ScriptId>).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct ScriptId;

/// ScriptManager stores a persistent database of scripts and executes them.
/// Each script can be individually enabled or disabled.
/// When a script is enabled, it is always running (unless an error occured during launch).
/// Script sources are stored as JSON strings in a SQLite database.
pub struct ScriptManager<Env, T> where Env: ExecutableDevEnv + 'static {
    env: Env,

    /// The path to the SQLite file to store, e.g. "./database.sqlite"
    path: PathBuf,

    /// A map to track currently-executing scripts.
    runners: HashMap<Id<ScriptId>, Execution<Env>>,

    /// The tx end of the channel passed to ScriptManager::new()
    tx: Box<T>,
}

impl<Env, T> ScriptManager<Env, T>
    where Env: ExecutableDevEnv + 'static,
          T: ExtSender<(Id<ScriptId>, ExecutionEvent)> + TransformableSender<(Id<ScriptId>, ExecutionEvent)> {

    /// Create a ScriptManager using a SQLite database file with the given path, i.e. filename.
    /// If the database file does not exist, it will be created.
    ///
    /// NOTE: You MUST consume the contents of `tx` to prevent memory leaks.
    ///
    /// The database stores the raw script source, but only after the source has been parsed
    /// to ensure validity.
    pub fn new(env: Env, path: &Path, tx: Box<T>) -> Result<Self, Error> {

        let connection = try!(rusqlite::Connection::open(&path));
        try!(connection.execute("CREATE TABLE IF NOT EXISTS scripts (
            id          TEXT NOT NULL PRIMARY KEY,
            source      TEXT NOT NULL,
            is_enabled  BOOL NOT NULL DEFAULT 1
        )", &[]));

        Ok(ScriptManager {
            path: path.to_owned(),
            env: env,
            runners: HashMap::new(),
            tx: tx
        })
    }

    /// Load and launch all existing scripts from the database.
    pub fn load(&mut self) -> Result<ResultMap<Id<ScriptId>, (), Error>, Error> {
        let connection = try!(rusqlite::Connection::open(&self.path));
        let mut result_map = HashMap::new();
        let mut stmt = try!(connection.prepare("SELECT id, source, is_enabled FROM scripts"));
        let rows = try!(stmt.query(&[]));

        for result_row in rows {
            let row = try!(result_row);
            let id_string: String = try!(row.get_checked(0));
            let id: Id<ScriptId> = Id::new(&id_string);
            let source: String = try!(row.get_checked(1));
            let is_enabled: bool = try!(row.get_checked(2));

            if is_enabled {
                result_map.insert(
                    id.clone(),
                    self.start_script(&id, &source).map(|_| ()));
            }
        }
        Ok(result_map)
    }

    /// Attempt to add a new script. The script will be executed and persisted to disk.
    /// The ID is chosen by the consumer and must be unique.
    pub fn put(&mut self, id: &Id<ScriptId>, source: &String) -> Result<(), Error> {
        try!(self.start_script(&id, &source));

        let connection = try!(rusqlite::Connection::open(&self.path));
        connection.execute("INSERT OR REPLACE INTO scripts (id, source, is_enabled)
                VALUES ($1, $2, $3)", &[&id.to_string(), source, &1])
            .map(|_| ()).map_err(From::from)
    }

    /// Enable or disable a script, starting or stopping the script if necessary.
    pub fn set_enabled(&mut self, id: &Id<ScriptId>, enabled: bool) -> Result<(), Error> {
        let source = try!(self.get_source(id));
        let is_running = self.runners.contains_key(&id);
        match (enabled, is_running) {
            (false, true) => {
                if let Some(mut runner) = self.runners.remove(&id) {
                    let (tx, rx) = channel();
                    runner.stop(move |result| {
                        let _ = tx.send(result);
                    });
                    if let Err(_) = rx.recv() {
                        return Err(Error::RunError(RunError::StartStopError(StartStopError::ThreadError)));
                    }
                }

                let connection = try!(rusqlite::Connection::open(&self.path));
                try!(connection.execute("UPDATE scripts SET is_enabled = 0 WHERE id = $1",
                                        &[&id.to_string()]));
            },
            (true, false) => {
                try!(self.start_script(id, &source));
                let connection = try!(rusqlite::Connection::open(&self.path));
                try!(connection.execute("UPDATE scripts SET is_enabled = 1 WHERE id = $1",
                                        &[&id.to_string()]));
            },
            _ => {
                // Nothing to do.
            }
        }

        Ok(())
    }

    /// Remove a script entirely, stopping it if necessary.
    /// If the script cannot be stopped (due to an error), it will not be removed.
    pub fn remove(&mut self, id: &Id<ScriptId>) -> Result<(), Error> {
        try!(self.set_enabled(id, false));
        let connection = try!(rusqlite::Connection::open(&self.path));
        connection.execute("DELETE FROM scripts WHERE id = $1", &[&id.to_string()])
            .map(|_| ())
            .map_err(From::from)
    }

    /// Remove all scripts, stopping any running scripts.
    /// (If any scripts fail to stop, we store and return those errors in a Vec<Error> so that
    /// we ensure that the database always gets wiped.)
    pub fn remove_all(&mut self) -> Result<Vec<Error>, Error> {
        // Get a copy of the keys (so that we don't borow `self` twice).
        let keys: Vec<Id<ScriptId>> = self.runners.keys().cloned().collect();
        let mut errors = Vec::new();
        // Remove any running scripts, storing errors for later return so that we
        // for sure end up nuking the database at the end.
        for id in keys {
            if let Err(err) = self.remove(&id) {
                errors.push(err);
            }
        }
        // Nuke the scripts database.
        let connection = try!(rusqlite::Connection::open(&self.path));
        try!(connection.execute("DELETE FROM scripts", &[])
                .map(|_| ()));
        Ok(errors)
    }

    /// Get the number of currently-running scripts.
    pub fn get_running_count(&self) -> usize {
        self.runners.len()
    }

    /// Get the source for a script with the given id.
    fn get_source(&self, id: &Id<ScriptId>) -> Result<String, Error> {
        let connection = try!(rusqlite::Connection::open(&self.path));
        let mut stmt = try!(connection.prepare("SELECT source FROM scripts WHERE id = $1"));
        let mut rows = try!(stmt.query(&[&id.to_string()]));
        let first_row = try!(try!(rows.nth(0).ok_or(Error::NoSuchScriptError)));
        Ok(try!(first_row.get_checked(0)))
    }

    /// Execute a script. Returns an error if the script is already running,
    /// or it won't parse, or it won't compile.
    fn start_script(&mut self, id: &Id<ScriptId>, source: &String) -> Result<(), Error> {
        if self.runners.contains_key(id) {
            return Err(Error::RunError(RunError::StartStopError(StartStopError::AlreadyRunning)));
        }
        let mut runner = Execution::<Env>::new();
        let tx_id = id.clone();
        let tx = self.tx.map(move |event| {
            (tx_id.clone(), event)
        });
        let parsed_source = try!(Script::parse(source));
        try!(runner.start(self.env.clone(), parsed_source, tx));
        self.runners.insert(id.clone(), runner);
        Ok(())
    }
}


impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::SQLError(format!("{:?}", err))
    }
}

impl From<RunError> for Error {
    fn from(err: RunError) -> Error {
        Error::RunError(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::ParseError(format!("{:?}", err))
    }
}
