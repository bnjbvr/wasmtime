//! Registers, the Universe thereof, and printing.
//!
//! These are ordered by sequence number, as required in the Universe.
//!
//! The caller-saved registers are placed first in order to prefer not to clobber (requiring
//! saves/restores in prologue/epilogue code) when possible. Note that there is no other heuristic
//! in the backend that will apply such pressure; the register allocator's cost heuristics are not
//! aware of the cost of clobber-save/restore code.
//!
//! One might worry that this pessimizes code with many callsites, where using caller-saves causes
//! us to have to save them (as we are the caller) frequently. However, the register allocator
//! *should be* aware of *this* cost, because it sees that the call instruction modifies all of the
//! caller-saved (i.e., callee-clobbered) registers.
//!
//! Hence, this ordering encodes pressure in one direction (prefer not to clobber registers that we
//! ourselves have to save) and this is balanaced against the RA's pressure in the other direction
//! at callsites.

use crate::settings;
use alloc::vec::Vec;
use regalloc::{
    PrettyPrint, RealReg, RealRegUniverse, Reg, RegClass, RegClassInfo, NUM_REG_CLASSES,
};
use std::string::String;

/// Creates a general-purpose register whose encoding is `enc`, and whose index in the
/// RealRegUniverse is `index`.
#[inline]
fn gpr(enc: u8, index: u8) -> Reg {
    Reg::new_real(RegClass::I64, enc, index)
}

/// Creates a floating-point register whose encoding is `enc`, and whose index in the
/// RealRegUniverse is `index`.
#[inline]
fn fpr(enc: u8, index: u8) -> Reg {
    Reg::new_real(RegClass::V128, enc, index)
}

// Hardware encodings for a few registers.

pub const ENC_RBX: u8 = 3;
pub const ENC_RSP: u8 = 4;
pub const ENC_RBP: u8 = 5;
pub const ENC_R12: u8 = 12;
pub const ENC_R13: u8 = 13;
pub const ENC_R14: u8 = 14;
pub const ENC_R15: u8 = 15;

#[derive(Clone)]
pub struct RegDefs {
    pub xmm0: Reg,
    pub xmm1: Reg,
    pub xmm2: Reg,
    pub xmm3: Reg,
    pub xmm4: Reg,
    pub xmm5: Reg,
    pub xmm6: Reg,
    pub xmm7: Reg,
    pub xmm8: Reg,
    pub xmm9: Reg,
    pub xmm10: Reg,
    pub xmm11: Reg,
    pub xmm12: Reg,
    pub xmm13: Reg,
    pub xmm14: Reg,
    pub xmm15: Reg,
    pub rsi: Reg,
    pub rdi: Reg,
    pub rax: Reg,
    pub rcx: Reg,
    pub rdx: Reg,
    pub r8: Reg,
    pub r9: Reg,
    pub r10: Reg,
    pub r11: Reg,
    pub r12: Reg,
    pub r13: Reg,
    pub r14: Reg,
    pub r15: Reg,
    pub rbx: Reg,
    pub rsp: Reg,
    pub rbp: Reg,
    /// The pinned register on this architecture.
    /// It must be the same as Spidermonkey's HeapReg, as found in this file.
    /// https://searchfox.org/mozilla-central/source/js/src/jit/x64/Assembler-x64.h#99
    pub pinned_reg: Reg,
}

const FPR: &[(u8, &'static str); 16] = &[
    (0, "%xmm0"),
    (1, "%xmm1"),
    (2, "%xmm2"),
    (3, "%xmm3"),
    (4, "%xmm4"),
    (5, "%xmm5"),
    (6, "%xmm6"),
    (7, "%xmm7"),
    (8, "%xmm8"),
    (9, "%xmm9"),
    (10, "%xmm10"),
    (11, "%xmm11"),
    (12, "%xmm12"),
    (13, "%xmm13"),
    (14, "%xmm14"),
    (15, "%xmm15"),
];

const XMM0: (u8, &'static str) = (0, "%xmm0");
const XMM1: (u8, &'static str) = (1, "%xmm1");
const XMM2: (u8, &'static str) = (2, "%xmm2");
const XMM3: (u8, &'static str) = (3, "%xmm3");
const XMM4: (u8, &'static str) = (4, "%xmm4");
const XMM5: (u8, &'static str) = (5, "%xmm5");
const XMM6: (u8, &'static str) = (6, "%xmm6");
const XMM7: (u8, &'static str) = (7, "%xmm7");
const XMM8: (u8, &'static str) = (8, "%xmm8");
const XMM9: (u8, &'static str) = (9, "%xmm9");
const XMM10: (u8, &'static str) = (10, "%xmm10");
const XMM11: (u8, &'static str) = (11, "%xmm11");
const XMM12: (u8, &'static str) = (12, "%xmm12");
const XMM13: (u8, &'static str) = (13, "%xmm13");
const XMM14: (u8, &'static str) = (14, "%xmm14");
const XMM15: (u8, &'static str) = (15, "%xmm15");

const RAX: (u8, &'static str) = (0, "%rax");
const RCX: (u8, &'static str) = (1, "%rcx");
const RDX: (u8, &'static str) = (2, "%rdx");
const RBX: (u8, &'static str) = (3, "%rbx");
const RSP: (u8, &'static str) = (ENC_RSP, "%rsp");
const RBP: (u8, &'static str) = (ENC_RBP, "%rbp");
const RSI: (u8, &'static str) = (6, "%rsi");
const RDI: (u8, &'static str) = (7, "%rdi");
const R8: (u8, &'static str) = (8, "%r8");
const R9: (u8, &'static str) = (9, "%r9");
const R10: (u8, &'static str) = (10, "%r10");
const R11: (u8, &'static str) = (11, "%r11");
const R12: (u8, &'static str) = (12, "%r12");
const R13: (u8, &'static str) = (13, "%r13");
const R14: (u8, &'static str) = (14, "%r14");
const R15: (u8, &'static str) = (15, "%r15");

pub(crate) struct Registers {
    pub defs: RegDefs,
    pub universe: RealRegUniverse,
}

impl Registers {
    pub(crate) fn systemv(use_pinned_reg: bool) -> Self {
        let mut regs: Vec<(RealReg, String)> = Vec::with_capacity(32);

        // First push all the XMM registers.
        let first_fpr = regs.len();

        let xmm0 = fpr(XMM0.0, regs.len() as u8);
        regs.push((xmm0.to_real_reg(), XMM0.1.into()));
        let xmm1 = fpr(XMM1.0, regs.len() as u8);
        regs.push((xmm1.to_real_reg(), XMM1.1.into()));
        let xmm2 = fpr(XMM2.0, regs.len() as u8);
        regs.push((xmm2.to_real_reg(), XMM2.1.into()));
        let xmm3 = fpr(XMM3.0, regs.len() as u8);
        regs.push((xmm3.to_real_reg(), XMM3.1.into()));
        let xmm4 = fpr(XMM4.0, regs.len() as u8);
        regs.push((xmm4.to_real_reg(), XMM4.1.into()));
        let xmm5 = fpr(XMM5.0, regs.len() as u8);
        regs.push((xmm5.to_real_reg(), XMM5.1.into()));
        let xmm6 = fpr(XMM6.0, regs.len() as u8);
        regs.push((xmm6.to_real_reg(), XMM6.1.into()));
        let xmm7 = fpr(XMM7.0, regs.len() as u8);
        regs.push((xmm7.to_real_reg(), XMM7.1.into()));
        let xmm8 = fpr(XMM8.0, regs.len() as u8);
        regs.push((xmm8.to_real_reg(), XMM8.1.into()));
        let xmm9 = fpr(XMM9.0, regs.len() as u8);
        regs.push((xmm9.to_real_reg(), XMM9.1.into()));
        let xmm10 = fpr(XMM10.0, regs.len() as u8);
        regs.push((xmm10.to_real_reg(), XMM10.1.into()));
        let xmm11 = fpr(XMM11.0, regs.len() as u8);
        regs.push((xmm11.to_real_reg(), XMM11.1.into()));
        let xmm12 = fpr(XMM12.0, regs.len() as u8);
        regs.push((xmm12.to_real_reg(), XMM12.1.into()));
        let xmm13 = fpr(XMM13.0, regs.len() as u8);
        regs.push((xmm13.to_real_reg(), XMM13.1.into()));
        let xmm14 = fpr(XMM14.0, regs.len() as u8);
        regs.push((xmm14.to_real_reg(), XMM14.1.into()));
        let xmm15 = fpr(XMM15.0, regs.len() as u8);
        regs.push((xmm15.to_real_reg(), XMM15.1.into()));
        let last_fpr = regs.len() - 1;

        // Integer regs.
        let first_gpr = regs.len();

        // Caller-saved, in the SystemV x86_64 ABI.
        let rsi = gpr(RSI.0, regs.len() as u8);
        regs.push((rsi.to_real_reg(), RSI.1.into()));

        let rdi = gpr(RDI.0, regs.len() as u8);
        regs.push((rdi.to_real_reg(), RDI.1.into()));

        let rax = gpr(RAX.0, regs.len() as u8);
        regs.push((rax.to_real_reg(), RAX.1.into()));

        let rcx = gpr(RCX.0, regs.len() as u8);
        regs.push((rcx.to_real_reg(), RCX.1.into()));

        let rdx = gpr(RDX.0, regs.len() as u8);
        regs.push((rdx.to_real_reg(), RDX.1.into()));

        let r8 = gpr(R8.0, regs.len() as u8);
        regs.push((r8.to_real_reg(), R8.1.into()));

        let r9 = gpr(R9.0, regs.len() as u8);
        regs.push((r9.to_real_reg(), R9.1.into()));

        let r10 = gpr(R10.0, regs.len() as u8);
        regs.push((r10.to_real_reg(), R10.1.into()));

        let r11 = gpr(R11.0, regs.len() as u8);
        regs.push((r11.to_real_reg(), R11.1.into()));

        // Callee-saved, in the SystemV x86_64 ABI.
        let r12 = gpr(R12.0, regs.len() as u8);
        regs.push((r12.to_real_reg(), R12.1.into()));

        let r13 = gpr(R13.0, regs.len() as u8);
        regs.push((r13.to_real_reg(), R13.1.into()));

        let r14 = gpr(R14.0, regs.len() as u8);
        regs.push((r14.to_real_reg(), R14.1.into()));

        let rbx = gpr(RBX.0, regs.len() as u8);
        regs.push((rbx.to_real_reg(), RBX.1.into()));

        // Other regs, not available to the allocator.
        let r15 = gpr(R15.0, regs.len() as u8);
        let (pinned_reg, allocable) = if use_pinned_reg {
            // The pinned register is not allocatable in this case, so record the length before adding
            // it.
            let len = regs.len();
            regs.push((r15.to_real_reg(), "%r15/pinned".into()));
            (r15, len)
        } else {
            regs.push((r15.to_real_reg(), "%r15".into()));
            (Reg::invalid(), regs.len())
        };
        let last_gpr = allocable - 1;

        let rsp = gpr(RSP.0, regs.len() as u8);
        regs.push((rsp.to_real_reg(), RSP.1.into()));

        let rbp = gpr(RBP.0, regs.len() as u8);
        regs.push((rbp.to_real_reg(), RBP.1.into()));

        let mut allocable_by_class = [None; NUM_REG_CLASSES];
        allocable_by_class[RegClass::I64.rc_to_usize()] = Some(RegClassInfo {
            first: first_gpr,
            last: last_gpr,
            suggested_scratch: Some(r12.get_index()),
        });
        allocable_by_class[RegClass::V128.rc_to_usize()] = Some(RegClassInfo {
            first: first_fpr,
            last: last_fpr,
            suggested_scratch: Some(xmm15.get_index()),
        });

        let defs = RegDefs {
            xmm0,
            xmm1,
            xmm2,
            xmm3,
            xmm4,
            xmm5,
            xmm6,
            xmm7,
            xmm8,
            xmm9,
            xmm10,
            xmm11,
            xmm12,
            xmm13,
            xmm14,
            xmm15,
            rsi,
            rdi,
            rax,
            rcx,
            rdx,
            r8,
            r9,
            r10,
            r11,
            r12,
            r13,
            r14,
            rbx,
            r15,
            rsp,
            rbp,
            pinned_reg,
        };

        let universe = RealRegUniverse {
            regs,
            allocable,
            allocable_by_class,
        };

        Self { defs, universe }
    }
}

/// Create the register universe for X64.
///
/// The ordering of registers matters, as commented in the file doc comment: assumes the
/// calling-convention is SystemV, at the moment.
pub(crate) fn create_reg_universe_systemv(flags: &settings::Flags) -> Registers {
    Registers::systemv(flags.enable_pinned_reg())
}

/// If `ireg` denotes an I64-classed reg, make a best-effort attempt to show its name at some
/// smaller size (4, 2 or 1 bytes).
pub fn show_ireg_sized(reg: Reg, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
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
