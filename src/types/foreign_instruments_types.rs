use std::mem::{Discriminant,discriminant};

use crate::types::backend_types::backend_libusb_types::{
	LibUsbDeviceDetails,
	BackendLibUsb
};

#[allow(dead_code)]
pub enum AccessorFallbackPolicy {
	AllowFallback(),
	TerminateOnFailure()
}

#[allow(dead_code)]
pub struct AccessorWatchdogPolicy {
	pub retry_attempts: u8,
	pub fallback_policy: AccessorFallbackPolicy
}

#[allow(dead_code)]
pub enum BackendAccessorDeviceDetails {
	BackendLibUsbDeviceDetails(LibUsbDeviceDetails),
	BackendDummyDeviceDetails()
}

#[allow(dead_code)]
pub struct AccessorDetails {
	pub watchdog_policy: AccessorWatchdogPolicy,
	pub backend_device_details: BackendAccessorDeviceDetails
}

#[allow(dead_code)]
pub struct ForeignInstrumentDetails {
	pub name: String,
	pub accessor_details_list: Vec<AccessorDetails>
}

#[allow(dead_code)]
pub enum BackendAccessor {
	AccessorLibUsb(BackendLibUsb),
	AccessorDummy()
}

impl BackendAccessor {
	#[allow(dead_code)]
	pub fn new(device_details: &BackendAccessorDeviceDetails) -> BackendAccessor {
		match device_details {
			BackendAccessorDeviceDetails::BackendLibUsbDeviceDetails(_d) => {
				BackendAccessor::AccessorLibUsb(BackendLibUsb::new())
			},
			BackendAccessorDeviceDetails::BackendDummyDeviceDetails() => {
				BackendAccessor::AccessorDummy()
			}
		}
	}
	#[allow(dead_code)]
	pub fn get_discriminant(&self) -> Discriminant<BackendAccessor> {
		discriminant(&self)
	}
	//pub initialize(&self) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    Active,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ManagedDevice {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub state: DeviceState,
}
