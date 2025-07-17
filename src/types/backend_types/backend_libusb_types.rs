#[allow(dead_code)]
pub enum LibUsbInterfaceSupportedClass {
	HID,
	UNSUPPORTED
}

#[allow(dead_code)]
pub enum LibUsbEndpointDirection {
	IN,
	OUT
}

#[allow(dead_code)]
pub struct LibUsbEndpointDetails {
	pub address: u8,
	pub direction: LibUsbEndpointDirection
}

#[allow(dead_code)]
pub struct LibUsbInterfaceDetails {
	pub number: u8,
	pub device_class: LibUsbInterfaceSupportedClass,
	pub endpoints: Vec<LibUsbEndpointDetails>
}

#[allow(dead_code)]
pub struct LibUsbDeviceDetails {
	pub vendor_id: u16,
	pub product_id: u16,
	pub interfaces: Vec<LibUsbInterfaceDetails>
}

impl LibUsbDeviceDetails {
	#[allow(dead_code)]
	pub fn new() -> LibUsbDeviceDetails {
		LibUsbDeviceDetails {
			vendor_id: 0,
			product_id: 0,
			interfaces: vec![ ]
		}
	}
}

#[allow(dead_code)]
pub struct BackendLibUsb {
	
}

impl BackendLibUsb {
	#[allow(dead_code)]
	pub fn new() -> BackendLibUsb {
		BackendLibUsb {
			
		}
	}
}
