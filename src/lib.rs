mod types;
mod hid_devices;
pub mod rusb_hid_manager;
pub mod event_driven_main;
pub mod event_driven_main_rusb;

use futures_core::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use log::{info, warn, error};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Initialization failed")] 
    InitializationFailed,
    #[error("Accessor not found")] 
    AccessorNotFound,
    #[error("Unknown error: {0}")]
    Unknown(String),
}

mod devices;

//pub fn get_distinct_backends() -> HashMap<Discriminant<BackendAccessor>,BackendAccessor> {
	//let mut backends = HashMap::new();
	//for instrument_details in FOREIGN_INSTRUMENT_DETAILS.iter() {
		//eprintln!("Finding backends for: {}", instrument_details.name);
		//for accessor_details in instrument_details.accessor_details_list.iter() {
			//let backend = BackendAccessor::new(&accessor_details.backend_device_details);
			//let backend_discriminant = backend.get_discriminant();
			//if ! backends.contains_key(&backend_discriminant) {
				//eprintln!("Adding new backend: {:#?}", backend_discriminant);
				//backends.insert(backend_discriminant, backend);
			//}
		//}
	//}
	////eprintln!("All backends: {:#?}", backends);
	//backends
//}

pub trait Detector: Stream<Item = u8> + Unpin + Send {
    fn get_name(&self) -> String;
}

pub trait Instrument: Send {
    fn get_name(&self) -> String;
    fn get_accessor(&self) -> Result<Box<dyn Accessor>, DeviceError>;
}

pub trait Accessor: Send {
    fn initialize(&self) -> Result<bool, DeviceError>;
}

pub struct DummyDetector {
    name: String,
}
impl DummyDetector {
    fn new() -> DummyDetector {
        DummyDetector {
            name: "Dummy Detectorzzz".to_string(),
        }
    }
}
impl Detector for DummyDetector {
    fn get_name(&self) -> String {
        self.name.to_string()
    }
}
impl Stream for DummyDetector {
    type Item = u8;
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(7))
    }
}

pub struct DummyInstrument {
    name: String,
}
impl DummyInstrument {
    #[allow(dead_code)]
    fn new() -> DummyInstrument {
        DummyInstrument {
            name: "Dummy Instrument".to_string(),
        }
    }
}
impl Instrument for DummyInstrument {
    fn get_name(&self) -> String {
        self.name.to_string()
    }
    fn get_accessor(&self) -> Result<Box<dyn Accessor>, DeviceError> {
        info!("Getting accessor for instrument: {}", self.name);
        Ok(Box::new(DummyAccessor {}))
    }
}

pub struct DummyAccessor;
impl Accessor for DummyAccessor {
    fn initialize(&self) -> Result<bool, DeviceError> {
        info!("Initializing DummyAccessor");
        Ok(true)
    }
}

//pub type DetectorList = Vec<fn() -> Box<Detector<Item = u8, Error = ()> + Send>>;
pub type InstrumentBoxed = Box<dyn Instrument>;
pub type DetectorBoxed = Box<dyn Detector>;
pub type DetectorCreatorPair = (&'static str, fn() -> DetectorBoxed);
pub type InstrumentList = Vec<Box<dyn Instrument>>;

fn dummy_detector_boxed_creator() -> DetectorBoxed {
    Box::new(DummyDetector::new())
}

pub fn get_detector_creator_pairs() -> Vec<DetectorCreatorPair> {
    vec![ ("Dummy Detector", dummy_detector_boxed_creator) ]
}


//pub fn is_supported_vid_pid_pair(v: u16, p: u16) -> bool {
	////println!("Is valid? ID {:04x}:{:04x}", v, p);
	////FOREIGN_INSTRUMENTS.contains_key(&(v,p))
	//false
//}
