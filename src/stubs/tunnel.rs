use std::sync::{ Arc, Mutex };
use tunnel_controller::TunnelConfigTrait;
use std::process::{ Child, Command };
use std::io::Result;

#[derive(Clone)]
pub struct TunnelConfigStub {
    pub spawn_called_count: Arc<Mutex<u16>>
}

impl TunnelConfigTrait for TunnelConfigStub {
    fn new(_: String, _: String, _: u16, _: u16, _: String) -> Self {
        TunnelConfigStub {
            spawn_called_count: Arc::new(Mutex::new(0))
        }
    }
    fn spawn(&self) -> Result<Child> {
        let mut spawn_called_count = self.spawn_called_count.lock().unwrap();
        *spawn_called_count += 1;
        Command::new("sleep").arg("10000").spawn()
    }
}

impl TunnelConfigStub {
    pub fn stub() -> Self {
        TunnelConfigStub::new("a".to_owned(), "b".to_owned(), 3, 4, "e".to_owned())
    }
}
