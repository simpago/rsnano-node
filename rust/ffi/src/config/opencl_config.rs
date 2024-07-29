use rsnano_node::config::OpenclConfigToml;

#[repr(C)]
pub struct OpenclConfigDto {
    pub opencl_enable: bool,
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}

pub fn fill_opencl_config_dto(dto: &mut OpenclConfigDto, config: &OpenclConfigToml) {
    dto.opencl_enable = config.enable.unwrap();
    dto.platform = config.platform.unwrap();
    dto.device = config.device.unwrap();
    dto.threads = config.threads.unwrap();
}

impl From<&OpenclConfigDto> for OpenclConfigToml {
    fn from(dto: &OpenclConfigDto) -> Self {
        Self {
            enable: Some(dto.opencl_enable),
            platform: Some(dto.platform),
            device: Some(dto.device),
            threads: Some(dto.threads),
        }
    }
}
