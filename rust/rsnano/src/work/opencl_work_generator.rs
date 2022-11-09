use super::{CpuWorkGenerator, WorkGenerator, WorkThresholds, WorkTicket};
use crate::core::{Root, WorkVersion};
use std::time::Duration;

pub type OpenClWorkFunc = dyn Fn(WorkVersion, Root, u64, &WorkTicket) -> Option<u64> + Send + Sync;

pub(crate) struct OpenClWorkGenerator {
    opencl: Box<OpenClWorkFunc>,
    cpu_gen: CpuWorkGenerator,
}

// Tries to create PoW with OpenCL. If no OpenCL callback is given, then PoW will be generated on the CPUs
impl OpenClWorkGenerator {
    pub fn new(rate_limiter: Duration, opencl: Box<OpenClWorkFunc>) -> Self {
        Self {
            opencl,
            cpu_gen: CpuWorkGenerator::new(rate_limiter),
        }
    }

    fn create_opencl_work(
        &self,
        version: WorkVersion,
        item: &Root,
        min_difficulty: u64,
        work_ticket: &WorkTicket,
    ) -> Option<(u64, u64)> {
        let work = (self.opencl)(version, *item, min_difficulty, &work_ticket);
        work.map(|work| (work, WorkThresholds::difficulty_v1(item, work)))
    }
}

impl WorkGenerator for OpenClWorkGenerator {
    fn create(
        &mut self,
        version: WorkVersion,
        item: &Root,
        min_difficulty: u64,
        work_ticket: &WorkTicket,
    ) -> Option<(u64, u64)> {
        let result = self.create_opencl_work(version, &item, min_difficulty, &work_ticket);

        if result.is_some() {
            result
        } else {
            self.cpu_gen
                .create(version, &item, min_difficulty, &work_ticket)
        }
    }
}
