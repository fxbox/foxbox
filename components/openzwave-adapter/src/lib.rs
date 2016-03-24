extern crate openzwave_stateful as openzwave;
extern crate foxbox_taxonomy as taxonomy;
extern crate transformable_channels;

use taxonomy::util::Id as TaxId;
use taxonomy::services::{ AdapterId, Getter, Setter };
use taxonomy::values::{ Value, Range };
use taxonomy::api::{ ResultMap, Error as TaxError };
use taxonomy::adapter::{ AdapterManagerHandle, AdapterWatchGuard, WatchEvent };
use transformable_channels::mpsc::ExtSender;

use openzwave::InitOptions;
use openzwave::{ ValueGenre, ValueID };

use std::error;
use std::fmt;
use std::collections::HashMap;

#[derive(Debug)]
pub enum OpenzwaveError {
    RegisteringError(TaxError),
    UnknownError
}

impl From<TaxError> for OpenzwaveError {
    fn from(err: TaxError) -> Self {
        OpenzwaveError::RegisteringError(err)
    }
}

impl From<()> for OpenzwaveError {
    fn from(_: ()) -> Self {
        OpenzwaveError::UnknownError
    }
}

impl fmt::Display for OpenzwaveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OpenzwaveError::RegisteringError(ref err) => write!(f, "IO error: {}", err),
            OpenzwaveError::UnknownError => write!(f, "Unknown error"),
        }
    }
}

impl error::Error for OpenzwaveError {
    fn description(&self) -> &str {
        match *self {
            OpenzwaveError::RegisteringError(ref err) => err.description(),
            OpenzwaveError::UnknownError => "Unknown error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            OpenzwaveError::RegisteringError(ref err) => Some(err),
            OpenzwaveError::UnknownError => None,
        }
    }
}

pub struct OpenzwaveAdapter {
    id: TaxId<AdapterId>,
    name: String,
    vendor: String,
    version: [u32; 4],
    manager: Box<AdapterManagerHandle + Send>
}

impl OpenzwaveAdapter {
    pub fn init<T: AdapterManagerHandle + Clone + Send + 'static> (manager: &T) -> Result<(), OpenzwaveError> {
        let name = String::from("OpenZwave Adapter");
        let adapter = Box::new(OpenzwaveAdapter {
            id: TaxId::new(&name),
            name: name,
            vendor: String::from("Mozilla"),
            version: [1, 0, 0, 0],
            manager: Box::new(manager.clone())
        });

        try!(manager.add_adapter(adapter));

        let options = InitOptions {
            device: None // TODO we should expose this as a Value
        };

        let ozw = try!(openzwave::init(&options));

        Ok(())
    }
}

impl taxonomy::adapter::Adapter for OpenzwaveAdapter {
    fn id(&self) -> TaxId<AdapterId> {
        self.id.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn vendor(&self) -> &str {
        &self.vendor
    }

    fn version(&self) -> &[u32; 4] {
        &self.version
    }

    fn fetch_values(&self, set: Vec<TaxId<Getter>>) -> ResultMap<TaxId<Getter>, Option<Value>, TaxError> {
        unimplemented!()
    }

    fn send_values(&self, values: HashMap<TaxId<Setter>, Value>) -> ResultMap<TaxId<Setter>, (), TaxError> {
        unimplemented!()
    }

    fn register_watch(&self, values: Vec<(TaxId<Getter>, Option<Range>)>, cb: Box<ExtSender<WatchEvent>>) -> ResultMap<TaxId<Getter>, Box<AdapterWatchGuard>, TaxError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
