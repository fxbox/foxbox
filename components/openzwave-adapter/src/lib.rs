extern crate openzwave;
extern crate foxbox_adapters as adapters;
extern crate foxbox_taxonomy as taxonomy;
extern crate transformable_channels;

use taxonomy::util::Id as TaxId;
use taxonomy::services::{ AdapterId, Getter, Setter };
use taxonomy::values::{ Value, Range };
use taxonomy::api::{ ResultMap, Error as TaxError };
use adapters::adapter::{ AdapterWatchGuard, WatchEvent };
use transformable_channels::mpsc::ExtSender;

struct OpenzwaveAdapter {
    version: [u32; 4]
}

impl adapters::adapter::Adapter for OpenzwaveAdapter {
    fn id(&self) -> TaxId<AdapterId> {
        unimplemented!()
    }

    fn name(&self) -> &str {
        unimplemented!()
    }

    fn vendor(&self) -> &str {
        unimplemented!()
    }

    fn version(&self) -> &[u32; 4] {
        &self.version
    }

    fn fetch_values(&self, set: Vec<TaxId<Getter>>) -> ResultMap<TaxId<Getter>, Option<Value>, TaxError> {
        unimplemented!()
    }

    fn send_values(&self, values: Vec<(TaxId<Setter>, Value)>) -> ResultMap<TaxId<Setter>, (), TaxError> {
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
