//! Compilation backend pipeline: optimized IR to VCode / binemit.

use crate::ir::Function;
use crate::machinst::*;
use crate::timing;

use log::debug;
use regalloc::{allocate_registers, RegAllocAlgorithm};
use std::env;

/// Compile the given function down to VCode with allocated registers, ready
/// for binary emission.
pub fn compile<B: LowerBackend>(
    f: &mut Function,
    b: &B,
    abi: Box<dyn ABIBody<B::MInst>>,
) -> VCode<B::MInst>
where
    B::MInst: ShowWithRRU,
{
    // This lowers the CL IR.
    let mut vcode = Lower::new(f, abi).lower(b);

    let universe = &B::MInst::reg_universe();

    debug!("vcode from lowering: \n{}", vcode.show_rru(Some(universe)));

    // Perform register allocation.
    let algorithm = match env::var("REGALLOC") {
        Ok(str) => match str.as_str() {
            "lsrac" => RegAllocAlgorithm::LinearScanChecked,
            "lsra" => RegAllocAlgorithm::LinearScan,
            _ => RegAllocAlgorithm::Backtracking,
        },
        Err(_) => RegAllocAlgorithm::Backtracking,
    };

    let result = {
        let _tt = timing::regalloc();
        allocate_registers(&mut vcode, algorithm, universe)
            .map_err(|err| {
                debug!(
                    "Register allocation error for vcode\n{}\nError: {:?}",
                    vcode.show_rru(Some(universe)),
                    err
                );
                err
            })
            .expect("register allocation")
    };

    // Reorder vcode into final order and copy out final instruction sequence
    // all at once. This also inserts prologues/epilogues.
    vcode.replace_insns_from_regalloc(result);

    debug!("vcode after regalloc:\n{}", vcode.show_rru(Some(universe)));

    vcode.remove_redundant_branches();

    debug!(
        "vcode after removing redundant branches:\n{}",
        vcode.show_rru(Some(universe))
    );

    // Do final passes over code to finalize branches.
    vcode.finalize_branches();

    debug!("final VCode:\n{}", vcode.show_rru(Some(universe)));

    //println!("{}\n", vcode.show_rru(Some(&B::MInst::reg_universe())));

    vcode
}
