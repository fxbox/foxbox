extern crate foxbox_taxonomy;
extern crate libc;
extern crate transformable_channels;
#[macro_use]
extern crate assert_matches;

use foxbox_taxonomy::adapters::adapter::Signature;
use foxbox_taxonomy::adapters::manager::*;
use foxbox_taxonomy::adapters::fake_adapter::*;
use foxbox_taxonomy::api::native::{ API, Targetted, User };
use foxbox_taxonomy::api::error::*;
use foxbox_taxonomy::api::selector::*;
use foxbox_taxonomy::api::services::*;
use foxbox_taxonomy::io::parse::*;
use foxbox_taxonomy::io::range::*;
use foxbox_taxonomy::io::serialize::*;
use foxbox_taxonomy::io::types::*;
use foxbox_taxonomy::misc::util::Description;

use transformable_channels::mpsc::*;

use std::collections::{ HashMap, HashSet };
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

#[derive(PartialEq, PartialOrd, Debug)]
struct IsOn(bool);
impl Into<Value> for IsOn {
    fn into(self) -> Value {
        Value {
            data: Arc::new(self)
        }
    }
}
impl Format for IsOn {
    fn description(&self) -> String {
        "bool".to_owned()
    }
    fn serialize(&self, value: &Value, _: &SerializeSupport) -> Result<JSON, SerializeError> {
        match value.cast::<Self>() {
            None => Err(SerializeError::ExpectedType(self.description())),
            Some(value) => {
                if value.0 {
                    Ok(JSON::Bool(true))
                } else {
                    Ok(JSON::Bool(false))
                }
            }
        }
    }
    fn deserialize(&self, path: Path, data: &JSON, _: &DeserializeSupport) -> Result<Value, ParseError> {
        match *data {
            JSON::Bool(ref b) => Ok(IsOn(*b).into()),
            _ => Err(ParseError::TypeError { expected: self.description(), at: path.to_string(), name: "IsOn".to_owned()})
        }
    }
}

pub fn get_db_environment() -> PathBuf {
    use libc::getpid;
    use std::thread;
    let tid = format!("{:?}", thread::current()).replace("(", "+").replace(")", "+");
    let s = format!("./tagstore_db_test-{}-{}.sqlite", unsafe { getpid() }, tid.replace("/", "42"));
    PathBuf::from(s)
}

pub fn remove_test_db() {
    use std::fs;

    let dbfile = get_db_environment();
    match fs::remove_file(dbfile.clone()) {
        Err(e) => panic!("Error {} cleaning up {}", e, dbfile.display()),
        _ => assert!(true)
    }
}

#[test]
#[allow(unused_variables)]
fn test_tags_in_db() {
    // Simple RAII style struct to delete the test db.
    struct AutoDeleteDb { };
    impl Drop for AutoDeleteDb {
        fn drop(&mut self) {
            remove_test_db();
        }
    }
    let auto_db = AutoDeleteDb { };

    let id_1 = Id::<AdapterId>::new("adapter id 1");
    let service_id_1 = Id::<ServiceId>::new("service id 1");
    let feature_id_1 = Id::<FeatureId>::new("feature id 1");

    let tag_id_1 = Id::<TagId>::new("tag id 1");
    let tag_id_2 = Id::<TagId>::new("tag id 2");
    let tag_id_3 = Id::<TagId>::new("tag id 3");
    let tag_id_4 = Id::<TagId>::new("tag id 4");

    let service_1 = Service {
        id: service_id_1.clone(),
        adapter: id_1.clone(),
        tags: vec![],
        properties: vec![],
    };

    let feature_1 : Feature = Feature {
        implements: vec!["light-on", "x-light-on"],
        fetch: None,
        send: None,
        ..Feature::empty(&feature_id_1, &service_id_1)
    };

    // Fist "session", starting from an empty state.
    {
        println!("* Create an adapter manager, a service, a feature, add tags. They should be persisted to the database.");
        let manager = AdapterManager::new(Some(get_db_environment()));
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        manager.add_feature(feature_1.clone()).unwrap();

        manager.add_service_tags(vec![ServiceSelector::new().with_id(service_id_1.clone())],
                                 vec![tag_id_1.clone(), tag_id_2.clone()]);

        manager.add_feature_tags(vec![FeatureSelector::new().with_id(feature_id_1.clone())],
                                 vec![tag_id_2.clone(), tag_id_3.clone()]);

        println!("* Remove the service and the feature. The data should still be in the database.");

        manager.remove_feature(&feature_id_1).unwrap();
        manager.remove_service(&service_id_1).unwrap();
        assert_eq!(manager.get_services(vec![]).len(), 0);

        println!("* Re-add the service. The data should be restored from the database.");

        // Re-add the same service and feature to check if we persisted the tags.
        manager.add_service(service_1.clone()).unwrap();

        let services = manager.get_services(vec![]);
        assert_eq!(services.len(), 1);

        let ref service = services[0];
        assert_eq!(service.tags.len(), 2);
        assert_eq!(service.tags.contains(&tag_id_1), true);
        assert_eq!(service.tags.contains(&tag_id_2), true);

        println!("* Re-add the feature. The data should be restored from the database.");

        manager.add_feature(feature_1.clone()).unwrap();

        let features = manager.get_features(vec![FeatureSelector::new()]);
        assert_eq!(features.len(), 1);
        let ref feature = features[0];
        assert_eq!(feature.tags.len(), 2);
        assert_eq!(feature.tags.contains(&tag_id_2), true);
        assert_eq!(feature.tags.contains(&tag_id_3), true);

        println!("* Terminate the manager. The data should still exist on the disk.");
        manager.remove_adapter(&id_1).unwrap();
        manager.stop();
    }

    // Second "session", starting with content added in session 1.
    {
        println!("* Start a new manager, reconfigure it as previously.");
        let manager = AdapterManager::new(Some(get_db_environment()));
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        manager.add_feature(feature_1.clone()).unwrap();

        let services = manager.get_services(vec![]);
        assert_eq!(services.len(), 1);

        println!("* The service should have the same tags.");
        let ref service = services[0];
        assert_eq!(service.tags.len(), 2);
        assert_eq!(service.tags.contains(&tag_id_1), true);
        assert_eq!(service.tags.contains(&tag_id_2), true);

        println!("* The features should have the same tags.");
        let features = manager.get_features(vec![FeatureSelector::new()]);
        assert_eq!(features.len(), 1);
        let ref feature = features[0];
        assert_eq!(feature.tags.len(), 2);
        assert_eq!(feature.tags.contains(&tag_id_2), true);
        assert_eq!(feature.tags.contains(&tag_id_3), true);

        println!("* Clear the tags, this should be remembered.");

        // Remove all the tags, to check in session 3 if we start empty again.
        manager.remove_service_tags(vec![ServiceSelector::new().with_id(service_id_1.clone())],
                                    vec![tag_id_1.clone(), tag_id_2.clone()]);
        let services = manager.get_services(vec![]);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].tags.len(), 0);

        manager.remove_feature_tags(vec![FeatureSelector::new().with_id(feature_id_1.clone())],
                                vec![tag_id_2.clone(), tag_id_3.clone()]);
        let features = manager.get_features(vec![FeatureSelector::new()]);
        assert_eq!(features.len(), 1);
        let ref feature = features[0];
        assert_eq!(feature.tags.len(), 0);

        manager.remove_adapter(&id_1).unwrap();
        manager.stop();
    }

    // Third "session", checking that we have no tags anymore.
    {
        println!("* Start a new manager, reconfigure it as previously.");

        let manager = AdapterManager::new(Some(get_db_environment()));
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        manager.add_feature(feature_1.clone()).unwrap();

        println!("* The service should have no tags.");
        let services = manager.get_services(vec![]);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].tags.len(), 0);

        println!("* The feature should have no tags.");
        let features = manager.get_features(vec![FeatureSelector::new()]);
        assert_eq!(features.len(), 1);
        let ref feature = features[0];
        assert_eq!(feature.tags.len(), 0);

        manager.remove_adapter(&id_1).unwrap();
        manager.stop();
    }
}

#[test]
fn test_add_remove_adapter() {
    for clear in vec![false, true] {
        println!("\n # Starting test with clear: {}\n", clear);

        let manager = AdapterManager::new(None);
        let id_1 = Id::new("id 1");
        let id_2 = Id::new("id 2");

        println!("* Adding two distinct test adapters should work.");
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))).unwrap();
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_2))).unwrap();

        println!("* Attempting to add yet another test adapter with id_1 or id_2 should fail.");
        match manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))) {
            Err(Error::InternalError(InternalError::DuplicateAdapter(ref id))) if *id == id_1 => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match manager.add_adapter(Arc::new(FakeAdapter::new(&id_2))) {
            Err(Error::InternalError(InternalError::DuplicateAdapter(ref id))) if *id == id_2 => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Removing id_1 should succeed. At this stage, we still shouldn't be able to add id_2, \
                  but we should be able to re-add id_1");
        manager.remove_adapter(&id_1).unwrap();
        match manager.add_adapter(Arc::new(FakeAdapter::new(&id_2))) {
            Err(Error::InternalError(InternalError::DuplicateAdapter(ref id))) if *id == id_2 => {},
            other => panic!("Unexpected result {:?}", other)
        }
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))).unwrap();

        println!("* Removing id_1 twice should fail the second time.");
        manager.remove_adapter(&id_1).unwrap();
        match manager.remove_adapter(&id_1) {
            Err(Error::InternalError(InternalError::NoSuchAdapter(ref id))) if *id == id_1 => {},
            other => panic!("Unexpected result {:?}", other)
        }

        if clear {
            println!("* Clearing does not need break the manager.");
            manager.stop();
        } else {
            println!("* Not clearing does not need break the manager.");
        }
    }
}

#[test]
fn test_add_remove_services() {
    for clear in vec![false, true] {
        println!("\n # Starting test with clear: {}\n", clear);

        let manager = AdapterManager::new(None);
        let api = API::new(&manager);
        let id_1 = Id::<AdapterId>::new("adapter id 1");
        let id_2 = Id::<AdapterId>::new("adapter id 2");

        let feature_id_1 = Id::<FeatureId>::new("feature id 1");
        let feature_id_2 = Id::<FeatureId>::new("feature id 2");
        let feature_id_3 = Id::<FeatureId>::new("feature id 3");

        let service_id_1 = Id::<ServiceId>::new("service id 1");
        let service_id_2 = Id::<ServiceId>::new("service id 2");
        let service_id_3 = Id::<ServiceId>::new("service id 3");

        let is_on : Arc<Format> = Arc::new(IsOn(false));
        let returns_is_on = Some(Signature {
            returns: Expects::Requires(is_on.clone()),
            ..Signature::default()
        });
        let expects_is_on = Some(Signature {
            accepts: Expects::Requires(is_on.clone()),
            ..Signature::default()
        });

        let feature_1 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            fetch: returns_is_on.clone(),
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_1, &service_id_1)
        };

        let service_1 = Service {
            id: service_id_1.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let service_2 = Service {
            id: service_id_2.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let feature_2 : Feature = Feature {
            implements: vec!["light-color"],
            fetch: returns_is_on.clone(),
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_2, &service_id_2)
        };

        let feature_3_no_such_service : Feature = Feature {
            implements: vec!["light-color"],
            fetch: returns_is_on.clone(),
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_3, &service_id_3)
        };

        println!("* Adding a service should fail if there is no adapter.");
        match manager.add_service(service_1.clone()) {
            Err(Error::InternalError(InternalError::NoSuchAdapter(ref err))) if *err == id_1 => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Adding a service should fail if the adapter doesn't exist.");
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_2))).unwrap();
        match manager.add_service(service_1.clone()) {
            Err(Error::InternalError(InternalError::NoSuchAdapter(ref err))) if *err == id_1 => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Make sure that none of the services has been added.");
        assert_eq!(api.get_services(vec![ServiceSelector::new()]).len(), 0);

        println!("* Adding a service can succeed.");
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        assert_eq!(api.get_services(vec![ServiceSelector::new()]).len(), 1);

        println!("* Make sure that we are finding the right service.");
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_id(service_id_1.clone())]).len(), 1);
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_id(service_id_2.clone())]).len(), 0);

        println!("* Adding a second service with the same id should fail.");
        match manager.add_service(service_1.clone()) {
            Err(Error::InternalError(InternalError::DuplicateService(ref err))) if *err == service_id_1 => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Adding features should fail if the service doesn't exist.");
        match manager.add_feature(feature_3_no_such_service.clone()) {
            Err(Error::InternalError(InternalError::NoSuchService(ref err))) if *err == service_id_3 => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* The attempt shouldn't let any channel lying around.");
        assert_eq!(api.get_features(vec![FeatureSelector::new()]).len(), 0);

        println!("* Adding features can succeed.");
        manager.add_feature(feature_1.clone()).unwrap();

        println!("* We can find the features that have been added.");
        assert_eq!(api.get_features(vec![FeatureSelector::new()]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_1.clone())]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_2.clone())]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_1.clone())
        ])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_3.clone())
        ])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-on"))]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-off"))]).len(), 0);

        println!("* Removing a non existent-feature fails.");
        match manager.remove_feature(&feature_id_3) {
            Err(Error::InternalError(InternalError::NoSuchFeature(ref err))) if *err == feature_id_3 => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Removing a non existent-feature has no effect.");
        assert_eq!(api.get_features(vec![FeatureSelector::new()]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_1.clone())]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_2.clone())]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_1.clone())
        ])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_3.clone())
        ])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-on"))]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-off"))]).len(), 0);

        println!("* Removing an installed feature can succeed.");
        manager.remove_feature(&feature_id_1).unwrap();

        println!("* We can observe taht the features have been removed.");
        assert_eq!(api.get_features(vec![FeatureSelector::new()]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_1.clone())]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_2.clone())]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_1.clone())
        ])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_3.clone())
        ])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-on"))]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-off"))]).len(), 0);

        println!("* We can remove a service without features.");
        manager.remove_service(&service_id_1).unwrap();

        println!("* We can add several services, then several features.");
        manager.add_service(service_1.clone()).unwrap();
        manager.add_service(service_2.clone()).unwrap();
        manager.add_feature(feature_1.clone()).unwrap();
        manager.add_feature(feature_2.clone()).unwrap();


        println!("* These features are detected.");
        assert_eq!(api.get_features(vec![FeatureSelector::new()]).len(), 2);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_1.clone())]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_2.clone())]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_1.clone())
        ])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_2.clone())
        ])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_3.clone())
        ])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-on"))]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-color"))]).len(), 1);
        assert_eq!(api.get_features(vec![
            FeatureSelector::new().with_implements(Id::new("light-on")),
            FeatureSelector::new().with_implements(Id::new("light-color"))
        ]).len(), 2);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-off"))]).len(), 0);

        println!("* A feature that matches several selectors is only returned once.");
        assert_eq!(api.get_features(vec![
            FeatureSelector::new().with_implements(Id::new("light-on")),
            FeatureSelector::new().with_implements(Id::new("x-light-on")),
            FeatureSelector::new().with_id(feature_id_1.clone()),
            FeatureSelector::new().with_service(vec![
                ServiceSelector::new().with_id(service_id_1.clone())
            ])
        ]).len(), 1);

        println!("* We can remove a service with features.");
        manager.remove_service(&service_id_1).unwrap();
        assert_eq!(api.get_services(vec![ServiceSelector::new()]).len(), 1);
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_id(service_id_1.clone())]).len(), 0);
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_id(service_id_2.clone())]).len(), 1);
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_id(service_id_3.clone())]).len(), 0);


        println!("* Removing a service with features also removes its features.");
        assert_eq!(api.get_features(vec![FeatureSelector::new()]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_1.clone())]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_1.clone())
        ])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-on"))]).len(), 0);

        println!("* Removing a service with features also removes other features.");
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_id(feature_id_2.clone())]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_2.clone())
        ])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_id(service_id_3.clone())
        ])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-color"))]).len(), 1);
        assert_eq!(api.get_features(vec![
            FeatureSelector::new().with_implements(Id::new("light-on")),
            FeatureSelector::new().with_implements(Id::new("light-color"))
        ]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_implements(Id::new("light-off"))]).len(), 0);


        if clear {
            println!("* Clearing does not need break the manager.");
            manager.stop();
        } else {
            println!("* Not clearing does not need break the manager.");
        }
    }
}

#[test]
fn test_add_remove_tags() {
    println!("");
    for clear in vec![false, true] {
        println!("\n # Starting test with clear: {}\n", clear);

        let manager = AdapterManager::new(None);
        let api = API::new(&manager);
        let id_1 = Id::<AdapterId>::new("adapter id 1");
        let id_2 = Id::<AdapterId>::new("adapter id 2");

        let feature_id_1 = Id::<FeatureId>::new("feature id 1");
        let feature_id_2 = Id::<FeatureId>::new("feature id 2");

        let service_id_1 = Id::<ServiceId>::new("service id 1");
        let service_id_2 = Id::<ServiceId>::new("service id 2");

        let is_on : Arc<Format> = Arc::new(IsOn(false));
        let returns_is_on = Some(Signature {
            returns: Expects::Requires(is_on.clone()),
            ..Signature::default()
        });
        let expects_is_on = Some(Signature {
            accepts: Expects::Requires(is_on.clone()),
            ..Signature::default()
        });

        let feature_1 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            fetch: returns_is_on.clone(),
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_1, &service_id_1)
        };

        let service_1 = Service {
            id: service_id_1.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let service_2 = Service {
            id: service_id_2.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let feature_2 : Feature = Feature {
            implements: vec!["light-color"],
            fetch: returns_is_on.clone(),
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_2, &service_id_2)
        };

        let tag_1 = Id::<TagId>::new("tag_1");
        let tag_2 = Id::<TagId>::new("tag_2");
        let tag_3_service = Id::<TagId>::new("tag_3 (for the service)");
        let tag_4_feature = Id::<TagId>::new("tag_4 (for the feature)");

        println!("* Initially, there are no tags.");
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);

        println!("* After adding an adapter, service, feature, still no tags.");
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_1))).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        manager.add_feature(feature_1.clone()).unwrap();
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);

        println!("* Removing tags from non-existent services and features doesn't hurt and returns 0.");
        assert_eq!(manager
            .remove_service_tags(
                vec![ServiceSelector::new().with_id(service_id_2.clone())], vec![tag_2.clone(), tag_3_service.clone()]
            ),
            0);
        assert_eq!(manager
            .remove_feature_tags(
                vec![FeatureSelector::new().with_id(feature_id_2.clone())], vec![tag_2.clone(), tag_4_feature.clone()]
            ),
            0);

        println!("* Adding tags to non-existent services and channels doesn't hurt and returns 0.");
        assert_eq!(manager
            .add_service_tags(
                vec![ServiceSelector::new().with_id(service_id_2.clone())], vec![tag_2.clone(), tag_3_service.clone()]
            ),
            0);
        assert_eq!(manager
            .add_feature_tags(
                vec![FeatureSelector::new().with_id(feature_id_2.clone())], vec![tag_2.clone(), tag_4_feature.clone()]
            ),
            0);

        println!("* There are still no tags.");
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);

        println!("* Removing non-added tags from existent services and features doesn't hurt and returns 1.");
        manager.add_adapter(Arc::new(FakeAdapter::new(&id_2))).unwrap();
        manager.add_service(service_2.clone()).unwrap();
        manager.add_feature(feature_2.clone()).unwrap();
        assert_eq!(manager
            .remove_service_tags(
                vec![ServiceSelector::new().with_id(service_id_2.clone())], vec![tag_2.clone(), tag_3_service.clone()]
            ),
            1);
        assert_eq!(manager
            .remove_feature_tags(
                vec![FeatureSelector::new().with_id(feature_id_2.clone())], vec![tag_2.clone(), tag_4_feature.clone()]
            ),
            1);

        println!("* We can add tags tags to services and features, this returns 1.");
        assert_eq!(manager
            .add_service_tags(
                vec![ServiceSelector::new().with_id(service_id_2.clone())], vec![tag_2.clone(), tag_3_service.clone()]
            ),
            1);
        assert_eq!(manager
            .add_feature_tags(
                vec![FeatureSelector::new().with_id(feature_id_2.clone())], vec![tag_2.clone(), tag_4_feature.clone()]
            ),
            1);

        println!("* We can select using these tags.");
        assert_eq!(api.get_services(vec![ServiceSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_tags(&vec![tag_1.clone()])]).len(), 0);

        assert_eq!(api.get_services(vec![ServiceSelector::new().with_tags(&vec![tag_2.clone()])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_tags(&vec![tag_2.clone()])]).len(), 1);

        assert_eq!(api.get_services(vec![ServiceSelector::new().with_tags(&vec![tag_3_service.clone()])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_tags(&vec![tag_3_service.clone()])]).len(), 0);

        assert_eq!(api.get_services(vec![ServiceSelector::new().with_tags(&vec![tag_4_feature.clone()])]).len(), 0);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_tags(&vec![tag_4_feature.clone()])]).len(), 1);

        println!("* We can select a feature using service tags.");
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_tags(&vec![tag_3_service.clone()])
        ])]).len(), 1);
        assert_eq!(api.get_features(vec![FeatureSelector::new().with_service(vec![
            ServiceSelector::new().with_tags(&vec![tag_4_feature.clone()])
        ])]).len(), 0);

        println!("* The tags are only applied to the right services/features.");
        assert_eq!(api.get_services(vec![
            ServiceSelector::new()
                .with_tags(&vec![tag_2.clone()])
                .with_id(service_id_1.clone())
            ]).len(), 0
        );
        assert_eq!(api.get_features(vec![
            FeatureSelector::new()
                .with_tags(&vec![tag_2.clone()])
                .with_id(feature_id_1.clone())
            ]).len(), 0
        );

        println!("* The tags are applied to the expected services/getters.");
        let selection = api.get_services(vec![
            ServiceSelector::new()
                .with_tags(&vec![tag_2.clone()])
                .with_id(service_id_2.clone())
            ]);
        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].id, service_id_2);
        assert_eq!(selection[0].tags.len(), 2);
        assert!(selection[0].tags.contains(&tag_2));
        assert!(selection[0].tags.contains(&tag_3_service));

        let selection = api.get_features(vec![
            FeatureSelector::new()
                .with_tags(&vec![tag_2.clone()])
                .with_id(feature_id_2.clone())
        ]);
        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].id, feature_id_2);
        assert_eq!(selection[0].tags.len(), 2);
        assert!(selection[0].tags.contains(&tag_2));
        assert!(selection[0].tags.contains(&tag_4_feature));

        let selection = api.get_services(vec![
            ServiceSelector::new()
                .with_tags(&vec![tag_3_service.clone()])
                .with_id(service_id_2.clone())
            ]);
        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].id, service_id_2);
        assert_eq!(selection[0].tags.len(), 2);
        assert!(selection[0].tags.contains(&tag_2));
        assert!(selection[0].tags.contains(&tag_3_service));

        let selection = api.get_features(vec![
            FeatureSelector::new()
                .with_tags(&vec![tag_4_feature.clone()])
                .with_id(feature_id_2.clone())
        ]);
        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].id, feature_id_2);
        assert_eq!(selection[0].tags.len(), 2);
        assert!(selection[0].tags.contains(&tag_2));
        assert!(selection[0].tags.contains(&tag_4_feature));

        println!("* We can remove tags, both existent and non-existent.");
        assert_eq!(manager
            .remove_service_tags(
                vec![ServiceSelector::new().with_id(service_id_2.clone())], vec![tag_1.clone(), tag_3_service.clone()]
            ),
            1);
        assert_eq!(manager
            .remove_feature_tags(
                vec![FeatureSelector::new().with_id(feature_id_2.clone())], vec![tag_1.clone(), tag_4_feature.clone()]
            ),
            1);

        println!("* Looking by tags has been updated.");
        let selection = api.get_services(vec![
            ServiceSelector::new()
                .with_tags(&vec![tag_2.clone()])
                .with_id(service_id_2.clone())
            ]);
        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].id, service_id_2);
        assert_eq!(selection[0].tags.len(), 1);
        assert!(selection[0].tags.contains(&tag_2));

        let selection = api.get_features(vec![
            FeatureSelector::new()
                .with_tags(&vec![tag_2.clone()])
                .with_id(feature_id_2.clone())
        ]);
        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].id, feature_id_2);
        assert_eq!(selection[0].tags.len(), 1);
        assert!(selection[0].tags.contains(&tag_2));

        let selection = api.get_services(vec![
            ServiceSelector::new()
                .with_tags(&vec![tag_3_service.clone()])
                .with_id(service_id_2.clone())
            ]);
        assert_eq!(selection.len(), 0);

        let selection = api.get_features(vec![
            FeatureSelector::new()
                .with_tags(&vec![tag_4_feature.clone()])
                .with_id(feature_id_2.clone())
        ]);
        assert_eq!(selection.len(), 0);

        if clear {
            println!("* Clearing does not need break the manager.");
            manager.stop();
        } else {
            println!("* Not clearing does not need break the manager.");
        }
    }

    println!("");
}

#[test]
fn test_fetch() {
    println!("");

    for clear in vec![false, true] {
        println!("\n # Starting test with clear: {}\n", clear);

        let manager = AdapterManager::new(None);
        let api = API::new(&manager);
        let id_1 = Id::<AdapterId>::new("adapter id 1");
        let id_2 = Id::<AdapterId>::new("adapter id 2");

        let feature_id_1_1 = Id::<FeatureId>::new("feature id 1.1");
        let feature_id_1_2 = Id::<FeatureId>::new("feature id 1.2");
        let feature_id_1_3 = Id::<FeatureId>::new("feature id 1.3");
        let feature_id_1_4 = Id::<FeatureId>::new("feature id 1.4");
        let feature_id_2 = Id::<FeatureId>::new("feature id 2");

        let service_id_1 = Id::<ServiceId>::new("service id 1");
        let service_id_2 = Id::<ServiceId>::new("service id 2");

        let is_on : Arc<Format> = Arc::new(IsOn(false));
        let returns_is_on = Some(Signature {
            returns: Expects::Requires(is_on.clone()),
            ..Signature::default()
        });

        let feature_1_1 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            fetch: returns_is_on.clone(),
            ..Feature::empty(&feature_id_1_1, &service_id_1)
        };

        let feature_1_2 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            fetch: returns_is_on.clone(),
            ..Feature::empty(&feature_id_1_2, &service_id_1)
        };

        let feature_1_3 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            fetch: returns_is_on.clone(),
            ..Feature::empty(&feature_id_1_3, &service_id_1)
        };

        let feature_1_4 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            fetch: None,
            ..Feature::empty(&feature_id_1_4, &service_id_1)
        };

        let feature_2 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            fetch: returns_is_on.clone(),
            ..Feature::empty(&feature_id_2, &service_id_1)
        };

        let service_1 = Service {
            id: service_id_1.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let service_2 = Service {
            id: service_id_2.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let tag_1 = Id::<TagId>::new("tag_1");
        let tag_2 = Id::<TagId>::new("tag_2");

        let adapter_1 = FakeAdapter::new(&id_1);
        let adapter_2 = FakeAdapter::new(&id_2);
        let tweak_1 = adapter_1.get_tweak();
        println!("* Without adapters, fetching values from a selector that has no channels returns an empty vector.");
        assert_eq!(api.place_method_call(MethodCall::Fetch, vec![Targetted::new(vec![FeatureSelector::new()], None)], User::None).len(), 0);

        println!("* With adapters, fetching values from a selector that has no features returns an empty vector.");
        manager.add_adapter(Arc::new(adapter_1)).unwrap();
        manager.add_adapter(Arc::new(adapter_2)).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        manager.add_service(service_2.clone()).unwrap();
        assert_eq!(api.place_method_call(MethodCall::Fetch, vec![Targetted::new(vec![FeatureSelector::new()], None)], User::None).len(), 0);

        println!("* Fetching empty values from a selector that has channels returns a type error if the adapter cannot provide the values.");
        manager.add_feature(feature_1_1.clone()).unwrap();
        manager.add_feature(feature_1_2.clone()).unwrap();
        manager.add_feature(feature_1_3.clone()).unwrap();
        manager.add_feature(feature_1_4.clone()).unwrap();
        manager.add_feature(feature_2.clone()).unwrap();
        let mut data = api.place_method_call(MethodCall::Fetch, vec![Targetted::new(vec![FeatureSelector::new()], None)], User::None);
        assert_eq!(data.len(), 4);

        for result in data.drain(..) {
            if let (_, Err(Error::TypeError(_))) = result {
                // We're good.
            } else {
                panic!("Unexpected result {:?}", result)
            }
        }

        println!("* Fetching values from the universal selector returns the right values.");
        tweak_1(Tweak::InjectGetterValue(feature_id_1_1.clone(), Ok(Some(Value::new(true)))));
        tweak_1(Tweak::InjectGetterValue(feature_id_1_2.clone(), Ok(Some(Value::new(false)))));
        let mut data = api.place_method_call(MethodCall::Fetch, vec![Targetted::new(vec![FeatureSelector::new()], None)], User::None);
        let data : HashMap<_, _> = data.drain(..).collect();
        assert_eq!(data.len(), 4);
        match data.get(&feature_id_1_1) {
            Some(&Ok(Some(ref value))) if *value.cast::<bool>().unwrap() => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_1_2) {
            Some(&Ok(Some(ref value))) if !*value.cast::<bool>().unwrap() => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_1_3) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_2) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Fetching values from feature tag returns the right values.");
        assert_eq!(api.add_feature_tags(vec![FeatureSelector::new().with_id(feature_id_1_1.clone())],
            vec![tag_1.clone()]), 1);
        assert_eq!(api.add_feature_tags(vec![
            FeatureSelector::new().with_id(feature_id_1_2.clone()),
            FeatureSelector::new().with_id(feature_id_1_3.clone()),
            FeatureSelector::new().with_id(feature_id_2.clone()),
        ], vec![tag_2.clone()]), 3);
        let mut data = api.place_method_call(MethodCall::Fetch, vec![
            Targetted::new(vec![FeatureSelector::new().with_tags(&vec![tag_1.clone()])], None),
            Targetted::new(vec![FeatureSelector::new().with_tags(&vec![tag_2.clone()])], None),
        ], User::None);
        let data : HashMap<_, _> = data.drain(..).collect();
        assert_eq!(data.len(), 4);
        match data.get(&feature_id_1_1) {
            Some(&Ok(Some(ref value))) if *value.cast::<bool>().unwrap() => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_1_2) {
            Some(&Ok(Some(ref value))) if !*value.cast::<bool>().unwrap() => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_1_3) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_2) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Fetching values from id returns the right errors.");
        tweak_1(Tweak::InjectGetterValue(feature_id_1_1.clone(), Err(Error::InternalError(InternalError::NoSuchFeature(feature_id_1_1.clone())))));
        let mut data = api.place_method_call(MethodCall::Fetch, vec![Targetted::new(vec![FeatureSelector::new()], None)], User::None);
        let data : HashMap<_, _> = data.drain(..).collect();

        assert_eq!(data.len(), 4);
        match data.get(&feature_id_1_1) {
            Some(&Err(Error::InternalError(InternalError::NoSuchFeature(ref id)))) if *id == feature_id_1_1 => {},
            other => panic!("Unexpected result, {:?}", other)
        }
        match data.get(&feature_id_1_2) {
            Some(&Ok(Some(ref value))) if !*value.cast::<bool>().unwrap() => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_1_3) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_2) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }

        println!("* Fetching values from tags returns the right errors.");
        let mut data = api.place_method_call(MethodCall::Fetch, vec![
            Targetted::new(vec![FeatureSelector::new().with_tags(&vec![tag_1.clone()])], None),
            Targetted::new(vec![FeatureSelector::new().with_tags(&vec![tag_2.clone()])], None),
        ], User::None);
        let data : HashMap<_, _> = data .drain(..).collect();
        assert_eq!(data.len(), 4);

        match data.get(&feature_id_1_1) {
            Some(&Err(Error::InternalError(InternalError::NoSuchFeature(ref id)))) if *id == feature_id_1_1 => {},
            other => panic!("Unexpected result, {:?}", other)
        }
        match data.get(&feature_id_1_2) {
            Some(&Ok(Some(ref value))) if !*value.cast::<bool>().unwrap() => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_1_3) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }
        match data.get(&feature_id_2) {
            Some(&Err(Error::TypeError(_))) => {},
            other => panic!("Unexpected result {:?}", other)
        }

        if clear {
            println!("* Clearing does not need break the manager.");
            manager.stop();
        } else {
            println!("* Not clearing does not need break the manager.");
        }
    }

    println!("");
}

#[test]
fn test_send() {
    println!("");

    for clear in vec![false, true] {
        println!("\n # Starting test with clear: {}\n", clear);
        let manager = AdapterManager::new(None);
        let api = API::new(&manager);
        let id_1 = Id::<AdapterId>::new("adapter id 1");
        let id_2 = Id::<AdapterId>::new("adapter id 2");

        let feature_id_1_1 = Id::<FeatureId>::new("feature id 1.1");
        let feature_id_1_2 = Id::<FeatureId>::new("feature id 1.2");
        let feature_id_1_3 = Id::<FeatureId>::new("feature id 1.3");
        let feature_id_1_4 = Id::<FeatureId>::new("feature id 1.4");
        let feature_id_2 = Id::<FeatureId>::new("feature id 2");

        let service_id_1 = Id::<ServiceId>::new("service id 1");
        let service_id_2 = Id::<ServiceId>::new("service id 2");

        let service_1 = Service {
            id: service_id_1.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let service_2 = Service {
            id: service_id_2.clone(),
            adapter: id_2.clone(),
            tags: vec![],
            properties: vec![],
        };

        let is_on : Arc<Format> = Arc::new(IsOn(false));
        let expects_is_on = Some(Signature {
            accepts: Expects::Requires(is_on.clone()),
            ..Signature::default()
        });

        let feature_1_1 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_1_1, &service_id_1)
        };

        let feature_1_2 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_1_2, &service_id_1)
        };

        let feature_1_3 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_1_3, &service_id_1)
        };

        let feature_1_4_no_send : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            send: None,
            ..Feature::empty(&feature_id_1_4, &service_id_1)
        };

        let feature_2 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            send: expects_is_on.clone(),
            ..Feature::empty(&feature_id_2, &service_id_2)
        };

        let adapter_1 = FakeAdapter::new(&id_1);
        let adapter_2 = FakeAdapter::new(&id_2);
        let tweak_1 = adapter_1.get_tweak();
        let rx_adapter_1 = adapter_1.take_rx();
        let rx_adapter_2 = adapter_2.take_rx();

        println!("* Without adapters, sending values to a selector that has no channels returns an empty vector.");
        let data = api.place_method_call(MethodCall::Send, vec![Targetted::new(vec![FeatureSelector::new()], Some(Value::new(true)))], User::None);
        assert_eq!(data.len(), 0);

        println!("* With adapters, sending values to a selector that has no channels returns an empty vector.");
        manager.add_adapter(Arc::new(adapter_1)).unwrap();
        manager.add_adapter(Arc::new(adapter_2)).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        manager.add_service(service_2.clone()).unwrap();
        let data = api.place_method_call(MethodCall::Send, vec![Targetted::new(vec![FeatureSelector::new()], Some(Value::new(true)))], User::None);
        assert_eq!(data.len(), 0);

        println!("* Sending well-typed values to channels succeeds if the adapter succeeds.");
        manager.add_feature(feature_1_1.clone()).unwrap();
        manager.add_feature(feature_1_2.clone()).unwrap();
        manager.add_feature(feature_1_3.clone()).unwrap();
        manager.add_feature(feature_1_4_no_send.clone()).unwrap();
        manager.add_feature(feature_2.clone()).unwrap();

        let mut data = api.place_method_call(MethodCall::Send, vec![Targetted::new(vec![FeatureSelector::new()], Some(Value::new(true)))], User::None);
        let data : HashMap<_, _> = data.drain(..).collect();
        assert_eq!(data.len(), 4);
        for result in data.values() {
            if let Ok(_) = *result {
                // We're good.
            } else {
                panic!("Unexpected result {:?}", result)
            }
        }

        println!("* All the values should have been received.");
        let mut data = HashMap::new();
        for _ in 0..3 {
            let Effect::ValueSent(id, value) = rx_adapter_1.try_recv().unwrap();
            data.insert(id, value);
        }
        assert_eq!(data.len(), 3);

        let value = rx_adapter_2.recv().unwrap();
        let Effect::ValueSent(id, value) = value;
        assert_eq!(*value.unwrap().cast::<bool>().unwrap(), true);
        assert_eq!(id, feature_id_2);

        println!("* No further value should have been received.");
        assert!(rx_adapter_1.try_recv().is_err());
        assert!(rx_adapter_2.try_recv().is_err());

        println!("* Sending values that cause channel errors will propagate the errors.");
        tweak_1(Tweak::InjectSetterError(feature_id_1_1.clone(), Some(Error::InternalError(InternalError::InvalidInitialService))));

        let mut data = api
            .place_method_call(MethodCall::Send, vec![Targetted::new(vec![FeatureSelector::new()], Some(Value::new(true)))], User::None);
        let data : HashMap<_, _> = data.drain(..).collect();
        assert_eq!(data.len(), 4);
        for id in vec![&feature_id_2, &feature_id_1_2, &feature_id_2] {
            match data.get(id) {
                Some(&Ok(_)) => {},
                other => panic!("Unexpected result for {:?}: {:?}", id, other)
            }
        }

        for id in vec![&feature_id_1_1] {
            match data.get(id) {
                Some(&Err(Error::InternalError(InternalError::InvalidInitialService))) => {},
                other => panic!("Unexpected result for {:?}: {:?}", id, other)
            }
        }

        println!("* All the non-errored values should have been received.");
        for _ in 0..2 {
            match rx_adapter_1.try_recv().unwrap() {
                Effect::ValueSent(ref id, ref value)
                    if *id != feature_id_1_1 && *value.clone().unwrap().cast::<bool>().unwrap() => {},
                effect => panic!("Unexpected effect {:?}", effect)
            }
        }
        match rx_adapter_2.try_recv().unwrap() {
            Effect::ValueSent(ref id, ref value)
                if *id == feature_id_2 && *value.clone().unwrap().cast::<bool>().unwrap() => {},
            effect => panic!("Unexpected effect {:?}", effect)
        }

        println!("* No further value should have been received.");
        assert!(rx_adapter_1.try_recv().is_err());
        assert!(rx_adapter_2.try_recv().is_err());
        tweak_1(Tweak::InjectSetterError(feature_id_1_1.clone(), None));

        if clear {
            println!("* Clearing does not need break the manager.");
            manager.stop();
        } else {
            println!("* Not clearing does not need break the manager.");
        }
    }

    println!("");
}

#[test]
fn test_watch() {
    println!("");

    for clear in vec![false, true] {
        println!("\n # Starting test with clear: {}\n", clear);

        let manager = AdapterManager::new(None);
        let api = API::new(&manager);
        let id_1 = Id::<AdapterId>::new("adapter id 1");
        let id_2 = Id::<AdapterId>::new("adapter id 2");

        let feature_id_1_1 = Id::<FeatureId>::new("feature id 1.1");
        let feature_id_1_2 = Id::<FeatureId>::new("feature id 1.2");
        let feature_id_1_3 = Id::<FeatureId>::new("feature id 1.3");
        let feature_id_1_4_no_watch = Id::<FeatureId>::new("feature id 1.4 (does not support watch)");
        let feature_id_1_5 = Id::<FeatureId>::new("feature id 1.5");
        let feature_id_2 = Id::<FeatureId>::new("feature id 2");

        let service_id_1 = Id::<ServiceId>::new("service id 1");
        let service_id_2 = Id::<ServiceId>::new("service id 2");

        let is_on : Arc<Format> = Arc::new(IsOn(false));
        let watch_is_on = Some(Signature {
            accepts: Expects::Requires(is_on.clone()),
            returns: Expects::Requires(is_on.clone()),
        });

        let feature_1_1 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            watch: watch_is_on.clone(),
            ..Feature::empty(&feature_id_1_1, &service_id_1)
        };

        let feature_1_2 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            watch: watch_is_on.clone(),
            ..Feature::empty(&feature_id_1_2, &service_id_1)
        };

        let feature_1_3 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            watch: watch_is_on.clone(),
            ..Feature::empty(&feature_id_1_3, &service_id_1)
        };

        let feature_1_4_no_watch : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            watch: None,
            ..Feature::empty(&feature_id_1_4_no_watch, &service_id_1)
        };

        let feature_1_5 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            watch: watch_is_on.clone(),
            ..Feature::empty(&feature_id_1_5, &service_id_1)
        };

        let feature_2 : Feature = Feature {
            implements: vec!["light-on", "x-light-on"],
            watch: watch_is_on.clone(),
            ..Feature::empty(&feature_id_2, &service_id_2)
        };

        let service_1 = Service {
            id: service_id_1.clone(),
            adapter: id_1.clone(),
            tags: vec![],
            properties: vec![],
        };

        let service_2 = Service {
            id: service_id_2.clone(),
            adapter: id_2.clone(),
            tags: vec![],
            properties: vec![],
        };

        let tag_1 = Id::<TagId>::new("tag 1");
        let tag_2_service = Id::<TagId>::new("tag 2");

        let adapter_1 = FakeAdapter::new(&id_1);
        let adapter_2 = FakeAdapter::new(&id_2);
        let tweak_1 = adapter_1.get_tweak();
        let tweak_2 = adapter_2.get_tweak();

        let mut guards = vec![];

        println!("* Without adapters, watching values from a selector that has no channels does nothing.");
        let (tx_watch_1, rx_watch_1) = channel();
        thread::spawn(move || {
            for msg in rx_watch_1 {
                panic!("We should not have received any message {:?}", msg);
            }
        });
        guards.push(api.register_watch(vec![Targetted::new(
            vec![FeatureSelector::new().with_id(Id::new("No such getter"))],
            Exactly::Always
        )], Box::new(tx_watch_1)));

        println!("* With adapters, watching values from a selector that has no channels does nothing.");
        manager.add_adapter(Arc::new(adapter_1)).unwrap();
        manager.add_adapter(Arc::new(adapter_2)).unwrap();
        manager.add_service(service_1.clone()).unwrap();
        manager.add_service(service_2.clone()).unwrap();
        api.add_service_tags(vec![
            ServiceSelector::new().with_id(service_id_1.clone())
        ], vec![tag_2_service.clone()]);
        let (tx_watch, rx_watch) = channel();
        thread::spawn(move || {
            for msg in rx_watch {
                panic!("We should not have received any message {:?}", msg);
            }
        });
        guards.push(api.register_watch(vec![Targetted::new(
            vec![FeatureSelector::new().with_id(Id::new("No such feature"))],
            Exactly::Always
        )], Box::new(tx_watch)));

        println!("* We can observe features being added, regardless of supports_watch.");
        let (tx_watch, rx_watch) = channel();
        let guard = api.register_watch(vec![Targetted::new(
            vec![FeatureSelector::new()],
            Exactly::Always
        )], Box::new(tx_watch)); // We keep `guard` out of `guards` to drop it manually later.

        manager.add_feature(feature_1_1.clone()).unwrap();
        manager.add_feature(feature_1_2.clone()).unwrap();
        manager.add_feature(feature_1_3.clone()).unwrap();
        manager.add_feature(feature_1_4_no_watch.clone()).unwrap();
        manager.add_feature(feature_2.clone()).unwrap();

        let events : HashSet<_> = (0..5).map(|_| {
            match rx_watch.recv().unwrap() {
                GenericWatchEvent::FeatureAdded {id, connection: true} => id,
                other => panic!("Unexpected event {:?}", other)
            }
        }).collect();

        assert_eq!(events.len(), 5);

        assert!(rx_watch.try_recv().is_err());

        println!("* We can observe features being removed, regardless of supports_watch.");

        manager.remove_feature(&feature_id_1_2).unwrap();
        match rx_watch.recv().unwrap() {
            GenericWatchEvent::FeatureRemoved{ ref id, connection: true } if *id == feature_id_1_2 => {}
            other => panic!("Unexpected event {:?}", other)
        }
        assert!(rx_watch.try_recv().is_err());

        println!("* We can observe value changes.");
        tweak_1(Tweak::InjectGetterValue(feature_id_1_1.clone(), Ok(Some(Value::new(true)))));
        tweak_1(Tweak::InjectGetterValue(feature_id_1_2.clone(), Ok(Some(Value::new(true)))));
        tweak_1(Tweak::InjectGetterValue(feature_id_1_3.clone(), Ok(Some(Value::new(false)))));
        tweak_1(Tweak::InjectGetterValue(feature_id_1_4_no_watch.clone(), Ok(Some(Value::new(false)))));

        let events : HashMap<_, _> = (0..2).map(|_| {
            match rx_watch.recv().unwrap() {
                GenericWatchEvent::EnterRange {
                    id,
                    value
                } => (id, value),
                other => panic!("Unexpected event {:?}", other)
            }
        }).collect();
        assert_eq!(events.get(&feature_id_1_1).unwrap().cast::<bool>().unwrap(), &true);
        assert_eq!(events.get(&feature_id_1_3).unwrap().cast::<bool>().unwrap(), &false);

        println!("* We do not receive Enter/Exit events for channels that have been removed or do not support watch.");
        assert!(rx_watch.try_recv().is_err());

        println!("* We can have several watchers at once.");
        assert_eq!(api.add_feature_tags(vec![
            FeatureSelector::new().with_id(feature_id_1_3.clone()),
            FeatureSelector::new().with_id(feature_id_2.clone()),
        ], vec![tag_1.clone()]), 2);

        let (tx_watch_2, rx_watch_2) = channel();
        guards.push(api.register_watch(vec![Targetted::new(
            vec![
                FeatureSelector::new()
                    .with_tags(&vec![tag_1.clone()])
            ],
            Exactly::Exactly(Value::new(Range::Eq(Value::new(true))))
        )], Box::new(tx_watch_2)));

        let (tx_watch_3, rx_watch_3) = channel();
        guards.push(api.register_watch(vec![Targetted::new(
            vec![
                FeatureSelector::new()
                    .with_service(vec![
                        ServiceSelector::new()
                            .with_tags(&vec![
                                tag_2_service.clone()
                            ])
                    ]),
            ], Exactly::Always
        )], Box::new(tx_watch_3)));

        println!("* Triggering value changes that should be observed on all watchers.");
        tweak_1(Tweak::InjectGetterValue(feature_id_1_1.clone(), Ok(Some(Value::new(false)))));
        tweak_1(Tweak::InjectGetterValue(feature_id_1_2.clone(), Ok(Some(Value::new(false))))); // This feature has been removed.
        tweak_1(Tweak::InjectGetterValue(feature_id_1_3.clone(), Ok(Some(Value::new(false)))));
        tweak_2(Tweak::InjectGetterValue(feature_id_2.clone(), Ok(Some(Value::new(false)))));
        tweak_2(Tweak::InjectGetterValue(feature_id_2.clone(), Ok(Some(Value::new(true)))));

        println!("* Value changes are observed by the universal observer.");
        let mut events : HashMap<_, _> = (0..4).map(|_| {
            match rx_watch.recv().unwrap() {
                GenericWatchEvent::EnterRange { id, value } => {
                    (id, *value.cast::<bool>().unwrap())
                }
                other => panic!("Unexpected event {:?}", other)
            }
        }).collect();

        println!("* Value changes are observed by the feature tags observer.");
        match rx_watch_2.recv().unwrap() {
            GenericWatchEvent::EnterRange { id, value } => {
                events.insert(id, *value.cast::<bool>().unwrap());
            }
            other => panic!("Unexpected event {:?}", other)
        }

        println!("* Value changes are observed by the service tags observer.");
        for _ in 0..2 {
            match rx_watch_3.recv().unwrap() {
                GenericWatchEvent::EnterRange { id, value } => {
                    events.insert(id, *value.cast::<bool>().unwrap());
                }
                other => panic!("Unexpected event {:?}", other)
            }
        }

        assert_eq!(events.len(), 3);

        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());

        println!("* Watchers with ranges emit both EnterRange and ExitRange");

        tweak_2(Tweak::InjectGetterValue(feature_id_2.clone(), Ok(Some(Value::new(false)))));
        match rx_watch_2.recv().unwrap() {
            GenericWatchEvent::ExitRange { ref id, .. } if *id == feature_id_2 => { }
            other => panic!("Unexpected event {:?}", other)
        }
        match rx_watch.recv().unwrap() {
            GenericWatchEvent::EnterRange { ref id, .. } if *id == feature_id_2 => { }
            other => panic!("Unexpected event {:?}", other)
        }
        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());

        println!("* We stop receiving value change notifications once we have dropped the guard.");
        drop(guard);
        assert!(rx_watch.try_recv().is_err());

        tweak_1(Tweak::InjectGetterValue(feature_id_1_1.clone(), Ok(Some(Value::new(true)))));
        tweak_1(Tweak::InjectGetterValue(feature_id_1_2.clone(), Ok(Some(Value::new(true)))));
        tweak_1(Tweak::InjectGetterValue(feature_id_1_3.clone(), Ok(Some(Value::new(true)))));
        tweak_2(Tweak::InjectGetterValue(feature_id_2.clone(), Ok(Some(Value::new(true)))));

        let events : HashSet<_> = (0..2).map(|_| {
                match rx_watch_2.recv().unwrap() {
                    GenericWatchEvent::EnterRange { id, .. } => id,
                    other => panic!("Unexpected event {:?}", other)
                }
        }).collect();
        assert_eq!(events.len(), 2);
        assert!(events.contains(&feature_id_1_3));
        assert!(events.contains(&feature_id_2));

        let events : HashSet<_> = (0..2).map(|_| {
                match rx_watch_3.recv().unwrap() {
                    GenericWatchEvent::EnterRange { id, .. } => id,
                    other => panic!("Unexpected event {:?}", other)
                }
        }).collect();
        assert_eq!(events.len(), 2);
        assert!(events.contains(&feature_id_1_1));
        assert!(events.contains(&feature_id_1_3));

        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());

        println!("* We stop receiving connection notifications once we have dropped the guard.");
        manager.add_feature(feature_1_5.clone()).unwrap();
        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        match rx_watch_3.recv().unwrap() {
            GenericWatchEvent::FeatureAdded { id, connection: true } => assert_eq!(id, feature_id_1_5),
            other => panic!("Unexpected event {:?}", other)
        }
        assert!(rx_watch_3.try_recv().is_err());

        println!("* We stop receiving disconnection notifications once we have dropped the guard.");
        manager.remove_feature(&feature_id_1_5).unwrap();
        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        match rx_watch_3.recv().unwrap() {
            GenericWatchEvent::FeatureRemoved { id, connection: true } => assert_eq!(id, feature_id_1_5),
            other => panic!("Unexpected event {:?}", other)
        }
        assert!(rx_watch_3.try_recv().is_err());

        println!("* We are notified when a feature is removed from a watch by changing a service tag.");
        api.remove_service_tags(vec![
            ServiceSelector::new().with_id(service_id_1.clone())
        ], vec![tag_2_service.clone()]);
        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        let events : HashSet<_> = (0..3).map(|_| {
            match rx_watch_3.recv().unwrap() {
                GenericWatchEvent::FeatureRemoved { id, connection: false } => id,
                other => panic!("Unexpected event {:?}", other)
            }
        }).collect();
        assert_eq!(events.len(), 3);
        assert!(events.contains(&feature_id_1_1));
        assert!(events.contains(&feature_id_1_3));
        assert!(events.contains(&feature_id_1_4_no_watch));

        assert!(rx_watch_3.try_recv().is_err());

        println!("* We stop receiving connection notifications from a watch-by-service-tag once the service has lost the tag.");
        manager.add_feature(feature_1_5.clone()).unwrap();
        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());

        println!("* We stop receiving disconnection notifications from a watch-by-service-tag once the service has lost the tag.");
        manager.remove_feature(&feature_id_1_5).unwrap();
        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());

        println!("* We are notified when a feature is added to a watch by changing a tag.");

        assert_eq!(api.add_feature_tags(vec![
            FeatureSelector::new().with_id(feature_id_1_1.clone()),
            FeatureSelector::new().with_id(feature_id_2.clone()),
        ], vec![tag_1.clone()]), 2);
        match rx_watch_2.recv().unwrap() {
            GenericWatchEvent::FeatureAdded { ref id, connection: false } if *id == feature_id_1_1 => { }
            other => panic!("Unexpected event {:?}", other)
        }
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());


        println!("* We are notified when a feature is removed from a watch by changing a tag.");

        assert_eq!(api.remove_feature_tags(vec![
            FeatureSelector::new().with_id(feature_id_1_1.clone()),
        ], vec![tag_1.clone()]), 1);
        match rx_watch_2.recv().unwrap() {
            GenericWatchEvent::FeatureRemoved { ref id, connection: false } if *id == feature_id_1_1 => { }
            other => panic!("Unexpected event {:?}", other)
        }
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());

        println!("* Make sure that we haven't forgotten to eat a message.");
        thread::sleep(std::time::Duration::new(1, 0));
        assert!(rx_watch.try_recv().is_err());
        assert!(rx_watch_2.try_recv().is_err());
        assert!(rx_watch_3.try_recv().is_err());

        if clear {
            println!("* Clearing does not need break the manager.");
            manager.stop();
        } else {
            println!("* Not clearing does not need break the manager.");
        }
    }

    println!("");
}
