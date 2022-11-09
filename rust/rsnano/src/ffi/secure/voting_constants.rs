use crate::VotingConstants;

#[repr(C)]
pub struct VotingConstantsDto {
    pub max_cache: usize,
    pub delay_s: i64,
}

pub fn fill_voting_constants_dto(dto: &mut VotingConstantsDto, voting: &VotingConstants) {
    dto.max_cache = voting.max_cache;
    dto.delay_s = voting.delay_s;
}

impl From<&VotingConstantsDto> for VotingConstants {
    fn from(dto: &VotingConstantsDto) -> Self {
        Self {
            max_cache: dto.max_cache,
            delay_s: dto.delay_s,
        }
    }
}
