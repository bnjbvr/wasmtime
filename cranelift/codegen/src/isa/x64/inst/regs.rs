//! Registers, the Universe thereof, and printing.
//!
//! These are ordered by sequence number, as required in the Universe.  The strange ordering is
//! intended to make callee-save registers available before caller-saved ones.  This is a net win
//! provided that each function makes at least one onward call.  It'll be a net loss for leaf
//! functions, and we should change the ordering in that case, so as to make caller-save regs
//! available first.
//!
//! TODO Maybe have two different universes, one for leaf functions and one for non-leaf functions?
//! Also, they will have to be ABI dependent.  Need to find a way to avoid constructing a universe
//! for each function we compile.

use crate::{machinst::pretty_print::ShowWithRRU, settings};
use alloc::vec::Vec;
use regalloc::{RealReg, RealRegUniverse, Reg, RegClass, RegClassInfo, NUM_REG_CLASSES};
use std::string::String;

// Hardware encodings for a few registers.

const ENC_RAX: u8 = 0;
const ENC_RCX: u8 = 1;
const ENC_RDX: u8 = 2;
pub const ENC_RBX: u8 = 3;
pub const ENC_RSP: u8 = 4;
pub const ENC_RBP: u8 = 5;
const ENC_RSI: u8 = 6;
const ENC_RDI: u8 = 7;
const ENC_R8: u8 = 8;
const ENC_R9: u8 = 9;
const ENC_R10: u8 = 10;
const ENC_R11: u8 = 11;
pub const ENC_R12: u8 = 12;
pub const ENC_R13: u8 = 13;
pub const ENC_R14: u8 = 14;
pub const ENC_R15: u8 = 15;

fn gpr(enc: u8, index: u8) -> Reg {
    Reg::new_real(RegClass::I64, enc, index)
}

fn fpr(enc: u8, index: u8) -> Reg {
    Reg::new_real(RegClass::V128, enc, index)
}

pub(crate) struct X86Universe {
    gpr_to_reg_index: [usize; 16],
    universe: RealRegUniverse,
}

impl X86Universe {
    fn new(use_pinned_reg: bool) -> Self {
        let mut regs = Vec::<(RealReg, String)>::with_capacity(32);
        let mut gpr_to_reg_index = [0; 16];
        let mut allocable_by_class = [None; NUM_REG_CLASSES];

        // First push the 16 FPR.
        for enc in 0..16 {
            regs.push((fpr(enc, enc).to_real_reg(), format!("%xmm{}", enc)));
        }

        // All the FPR can be register-allocated. Use xmm15 as the scratch register, if useful.
        // Nothing must be done in particular with respect to ordering, since all the FP registers
        // are callee-preserved.
        allocable_by_class[RegClass::V128.rc_to_usize()] = Some(RegClassInfo {
            first: 0,
            last: 15,
            suggested_scratch: Some(15), // xmm15
        });

        // Then push GPR.s
        let first_gpr = regs.len();

        let mut add_gpr = |enc| {
            let index = regs.len();
            gpr_to_reg_index[enc as usize] = index;
            regs.push((
                gpr(enc, index as u8).to_real_reg(),
                GPR_NAMES[enc as usize].into(),
            ));
        };

        // Allocatable GPR: first all the callee-saved, then all the caller-saved.
        if use_pinned_reg {
            for &enc in &[
                // Callee-saved:
                ENC_R12, ENC_R13, ENC_R14, ENC_RBX, // Then the caller-saved:
                ENC_RSI, ENC_RDI, ENC_RAX, ENC_RCX, ENC_RDX, ENC_R8, ENC_R9, ENC_R10, ENC_R11,
            ] {
                add_gpr(enc);
            }
        } else {
            for &enc in &[
                // Callee-saved:
                ENC_R12, ENC_R13, ENC_R14, ENC_R15, ENC_RBX, // Then the caller-saved:
                ENC_RSI, ENC_RDI, ENC_RAX, ENC_RCX, ENC_RDX, ENC_R8, ENC_R9, ENC_R10, ENC_R11,
            ] {
                add_gpr(enc);
            }
        }

        allocable_by_class[RegClass::I64.rc_to_usize()] = Some(RegClassInfo {
            first: first_gpr,
            last: regs.len() - 1,
            suggested_scratch: Some(gpr_to_reg_index[ENC_R12 as usize]),
        });

        let allocable = regs.len();

        // Non-allocatable registers:
        if use_pinned_reg {
            regs.push((
                gpr(ENC_R15, regs.len() as u8).to_real_reg(),
                "r15/pinned".into(),
            ));
        }

        for &enc in &[ENC_RSP, ENC_RBP] {
            let index = regs.len();
            gpr_to_reg_index[enc as usize] = index;
            regs.push((
                gpr(enc, index as u8).to_real_reg(),
                GPR_NAMES[enc as usize].into(),
            ));
        }

        let universe = RealRegUniverse {
            regs,
            allocable,
            allocable_by_class,
        };

        Self {
            gpr_to_reg_index,
            universe,
        }
    }

    pub(crate) fn xmm(&self, index: usize) -> Reg {
        fpr(index as u8, index as u8)
    }
    pub(crate) fn gpr(&self, enc: usize) -> Reg {
        self.universe.regs[self.gpr_to_reg_index[enc]].0.to_reg()
    }
    pub(crate) fn reg_universe(&self) -> &RealRegUniverse {
        &self.universe
    }
}

/// Mapping from general-purpose register encoding to their name.
static GPR_NAMES: &[&'static str; 16] = &[
    "%rax", "%rcx", "%rdx", "%rbx", "%rsp", "%rbp", "%rsi", "%rdi", "%r8", "%r9", "%r10", "%r11",
    "%r12", "%r13", "%r14", "%r15",
];

pub(crate) fn r12() -> Reg {
    gpr(ENC_R12, 16)
}
pub(crate) fn r13() -> Reg {
    gpr(ENC_R13, 17)
}
pub(crate) fn r14() -> Reg {
    gpr(ENC_R14, 18)
}
pub(crate) fn rbx() -> Reg {
    gpr(ENC_RBX, 19)
}
pub(crate) fn rsi() -> Reg {
    gpr(ENC_RSI, 20)
}
pub(crate) fn rdi() -> Reg {
    gpr(ENC_RDI, 21)
}
pub(crate) fn rax() -> Reg {
    gpr(ENC_RAX, 22)
}
pub(crate) fn rcx() -> Reg {
    gpr(ENC_RCX, 23)
}
pub(crate) fn rdx() -> Reg {
    gpr(ENC_RDX, 24)
}
pub(crate) fn r8() -> Reg {
    gpr(ENC_R8, 25)
}
pub(crate) fn r9() -> Reg {
    gpr(ENC_R9, 26)
}
pub(crate) fn r10() -> Reg {
    gpr(ENC_R10, 27)
}
pub(crate) fn r11() -> Reg {
    gpr(ENC_R11, 28)
}

pub(crate) fn r15() -> Reg {
    // r15 is put aside since this is the pinned register.
    gpr(ENC_R15, 29)
}

pub(crate) fn pinned_reg() -> Reg {
    r15()
}

pub(crate) fn xmm0() -> Reg {
    fpr(0, 0)
}
pub(crate) fn xmm1() -> Reg {
    fpr(1, 1)
}
pub(crate) fn xmm2() -> Reg {
    fpr(2, 2)
}
pub(crate) fn xmm3() -> Reg {
    fpr(3, 3)
}
pub(crate) fn xmm4() -> Reg {
    fpr(4, 4)
}
pub(crate) fn xmm5() -> Reg {
    fpr(5, 5)
}
pub(crate) fn xmm6() -> Reg {
    fpr(6, 6)
}
pub(crate) fn xmm7() -> Reg {
    fpr(7, 7)
}
pub(crate) fn xmm8() -> Reg {
    fpr(8, 8)
}
pub(crate) fn xmm9() -> Reg {
    fpr(9, 9)
}
pub(crate) fn xmm10() -> Reg {
    fpr(10, 10)
}
pub(crate) fn xmm11() -> Reg {
    fpr(11, 11)
}
pub(crate) fn xmm12() -> Reg {
    fpr(12, 12)
}
pub(crate) fn xmm13() -> Reg {
    fpr(13, 13)
}
pub(crate) fn xmm14() -> Reg {
    fpr(14, 14)
}
pub(crate) fn xmm15() -> Reg {
    fpr(15, 15)
}

pub(crate) fn rsp() -> Reg {
    gpr(ENC_RSP, 30)
}
pub(crate) fn rbp() -> Reg {
    gpr(ENC_RBP, 31)
}

/// Create the register universe for X64.
///
/// The ordering of registers matters, as commented in the file doc comment: assumes the
/// calling-convention is SystemV, at the moment.
pub(crate) fn create_reg_universe(flags: &settings::Flags) -> X86Universe {
    let systemv = X86Universe::new(flags.enable_pinned_reg());

    // Sanity-check: the index passed to the Reg ctor must match the order in the register list.
    assert_eq!(systemv.universe.regs.len(), 32);
    for (i, reg) in systemv.universe.regs.iter().enumerate() {
        assert_eq!(i, reg.0.get_index());
    }

    systemv
}

/// If `ireg` denotes an I64-classed reg, make a best-effort attempt to show its name at some
/// smaller size (4, 2 or 1 bytes).
pub(crate) fn show_ireg_sized(reg: Reg, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
    let mut s = reg.show_rru(mb_rru);

    if reg.get_class() != RegClass::I64 || size == 8 {
        // We can't do any better.
        return s;
    }

    if reg.is_real() {
        // Change (eg) "rax" into "eax", "ax" or "al" as appropriate.  This is something one could
        // describe diplomatically as "a kludge", but it's only debug code.
        let remapper = match s.as_str() {
            "%rax" => Some(["%eax", "%ax", "%al"]),
            "%rbx" => Some(["%ebx", "%bx", "%bl"]),
            "%rcx" => Some(["%ecx", "%cx", "%cl"]),
            "%rdx" => Some(["%edx", "%dx", "%dl"]),
            "%rsi" => Some(["%esi", "%si", "%sil"]),
            "%rdi" => Some(["%edi", "%di", "%dil"]),
            "%rbp" => Some(["%ebp", "%bp", "%bpl"]),
            "%rsp" => Some(["%esp", "%sp", "%spl"]),
            "%r8" => Some(["%r8d", "%r8w", "%r8b"]),
            "%r9" => Some(["%r9d", "%r9w", "%r9b"]),
            "%r10" => Some(["%r10d", "%r10w", "%r10b"]),
            "%r11" => Some(["%r11d", "%r11w", "%r11b"]),
            "%r12" => Some(["%r12d", "%r12w", "%r12b"]),
            "%r13" => Some(["%r13d", "%r13w", "%r13b"]),
            "%r14" => Some(["%r14d", "%r14w", "%r14b"]),
            "%r15" => Some(["%r15d", "%r15w", "%r15b"]),
            _ => None,
        };
        if let Some(smaller_names) = remapper {
            match size {
                4 => s = smaller_names[0].into(),
                2 => s = smaller_names[1].into(),
                1 => s = smaller_names[2].into(),
                _ => panic!("show_ireg_sized: real"),
            }
        }
    } else {
        // Add a "l", "w" or "b" suffix to RegClass::I64 vregs used at narrower widths.
        let suffix = match size {
            4 => "l",
            2 => "w",
            1 => "b",
            _ => panic!("show_ireg_sized: virtual"),
        };
        s = s + suffix;
    }

    s
}
