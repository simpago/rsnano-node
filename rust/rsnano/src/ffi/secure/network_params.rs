use std::convert::{TryFrom, TryInto};

use super::{
    bootstrap_constants::{fill_bootstrap_constants_dto, BootstrapConstantsDto},
    node_constants::{fill_node_constants_dto, NodeConstantsDto},
    portmapping_constants::{fill_portmapping_constants_dto, PortmappingConstantsDto},
    voting_constants::{fill_voting_constants_dto, VotingConstantsDto},
};
use crate::{
    ffi::{
        fill_network_constants_dto,
        ledger::{fill_ledger_constants_dto, LedgerConstantsDto},
        work::{fill_work_thresholds_dto, WorkThresholdsDto},
        NetworkConstantsDto,
    },
    NetworkParams,
};
use num::FromPrimitive;

#[repr(C)]
pub struct NetworkParamsDto {
    pub kdf_work: u32,
    pub work: WorkThresholdsDto,
    pub network: NetworkConstantsDto,
    pub ledger: LedgerConstantsDto,
    pub voting: VotingConstantsDto,
    pub node: NodeConstantsDto,
    pub portmapping: PortmappingConstantsDto,
    pub bootstrap: BootstrapConstantsDto,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_params_create(
    dto: *mut NetworkParamsDto,
    network: u16,
) -> i32 {
    let network = match FromPrimitive::from_u16(network) {
        Some(n) => n,
        None => return -1,
    };
    let params = match NetworkParams::new(network) {
        Ok(p) => p,
        Err(_) => return -1,
    };
    let dto = &mut (*dto);
    fill_network_params_dto(dto, &params);
    0
}

pub fn fill_network_params_dto(dto: &mut NetworkParamsDto, params: &NetworkParams) {
    dto.kdf_work = params.kdf_work;
    fill_work_thresholds_dto(&mut dto.work, &params.work);
    fill_network_constants_dto(&mut dto.network, &params.network);
    fill_ledger_constants_dto(&mut dto.ledger, &params.ledger);
    fill_voting_constants_dto(&mut dto.voting, &params.voting);
    fill_node_constants_dto(&mut dto.node, &params.node);
    fill_portmapping_constants_dto(&mut dto.portmapping, &params.portmapping);
    fill_bootstrap_constants_dto(&mut dto.bootstrap, &params.bootstrap);
}

impl TryFrom<&NetworkParamsDto> for NetworkParams {
    type Error = anyhow::Error;

    fn try_from(value: &NetworkParamsDto) -> Result<Self, Self::Error> {
        let network_params = NetworkParams {
            kdf_work: value.kdf_work,
            work: (&value.work).into(),
            network: (&value.network).try_into()?,
            ledger: (&value.ledger).try_into()?,
            voting: (&value.voting).into(),
            node: (&value.node).into(),
            portmapping: (&value.portmapping).into(),
            bootstrap: (&value.bootstrap).into(),
        };

        Ok(network_params)
    }
}
