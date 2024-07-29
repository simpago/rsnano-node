use rsnano_node::config::OpenclConfig;

#[repr(C)]
pub struct OpenclConfigDto {
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}

pub fn fill_opencl_config_dto(dto: &mut OpenclConfigDto, config: &OpenclConfig) {
    dto.platform = config.platform;
    dto.device = config.device;
    dto.threads = config.threads;
}

impl From<&OpenclConfigDto> for OpenclConfig {
    fn from(dto: &OpenclConfigDto) -> Self {
        Self {
            platform: dto.platform,
            device: dto.device,
            threads: dto.threads,
        }
    }
}
