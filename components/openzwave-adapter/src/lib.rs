extern crate openzwave;
extern crate foxbox_adapters as adapters;
extern crate foxbox_taxonomy as taxonomy;
extern crate transformable_channels;

use taxonomy::util::Id as TaxId;
use taxonomy::services::{ AdapterId, Getter, Setter };
use taxonomy::values::{ Value, Range };
use taxonomy::api::{ ResultMap, Error as TaxError };
use adapters::adapter::{ AdapterManagerHandle, AdapterWatchGuard, WatchEvent };
use transformable_channels::mpsc::ExtSender;

struct OpenzwaveAdapter {
    id: TaxId<AdapterId>,
    name: String,
    vendor: String,
    version: [u32; 4],
    manager: Box<AdapterManagerHandle + Send>
}

impl OpenzwaveAdapter {
    fn init<T: AdapterManagerHandle + Clone + Send + 'static> (manager: &T) -> Result<(), TaxError> {
        let name = String::from("OpenZwave Adapter");
        let adapter = Box::new(OpenzwaveAdapter {
            id: TaxId::new(name.clone()), // replace with &name once we update to latest taxonomy
            name: name,
            vendor: String::from("Mozilla"),
            version: [1, 0, 0, 0],
            manager: Box::new(manager.clone())
        });

        try!(manager.add_adapter(adapter));

        Ok(())
    }
}

impl adapters::adapter::Adapter for OpenzwaveAdapter {
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
