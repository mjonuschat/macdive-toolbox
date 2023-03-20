use crate::arguments::MtpOptions;

#[derive(Debug)]
pub enum DeviceSelector {
    First,
    ManufacturerName(String),
    ModelName(String),
    SerialNumber(String),
}

impl From<MtpOptions> for DeviceSelector {
    fn from(params: MtpOptions) -> Self {
        if let Some(serial) = params.serial {
            DeviceSelector::SerialNumber(serial)
        } else if let Some(model) = params.model {
            DeviceSelector::ModelName(model)
        } else if let Some(manufacturer) = params.manufacturer {
            DeviceSelector::ManufacturerName(manufacturer)
        } else {
            DeviceSelector::First
        }
    }
}
