use crate::types::foreign_instruments_types::{
	ForeignInstrumentDetails,
	AccessorDetails,
	AccessorWatchdogPolicy,
	AccessorFallbackPolicy::{TerminateOnFailure},
	BackendAccessorDeviceDetails
};
use log::info;

#[allow(dead_code)]
pub fn details() -> ForeignInstrumentDetails {
    info!("Instantiating Komplete Kontrol S25 details");
    ForeignInstrumentDetails {
        name: "Komplete Kontrol S25".to_string(),
        accessor_details_list: vec![
            AccessorDetails {
                watchdog_policy: AccessorWatchdogPolicy {
                    retry_attempts: 1,
                    fallback_policy: TerminateOnFailure()
                },
                backend_device_details: BackendAccessorDeviceDetails::BackendDummyDeviceDetails()
            },
        ]
    }
}
